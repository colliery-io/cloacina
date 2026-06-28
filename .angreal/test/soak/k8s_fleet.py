# Copyright 2026 Cloacina Contributors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.

"""Kubernetes fleet-actuator SOAK — the long-running control-plane stability
test we actually care about (k8s, not Docker) (CLOACI-T-0816, CLOACI-I-0127).

Where `soak fleet-actuator` soaks the *Docker* actuator, THIS soaks the
**Kubernetes** actuator under the chart's REAL RBAC: it stands up the same k3s
e2e platform as `e2e k8s-fleet` (k3s + registry + a helm-installed
cloacina-server bound to the fleet ServiceAccount, `fleet.actuator=kubernetes`),
seeds 2-3 tenants, and then for `--duration` seconds (default 24h) drives:

- a STEADY load: two tenants are provisioned once and held — their agent
  Deployments stay at desired replicas, their pods register + heartbeat, and
  the leader-gated reconcile loop runs continuously. A loaded, healthy fleet
  that gets falsely evicted is the #1 thing this hunts.
- periodic CHURN: a scratch tenant is scaled up/down on `--snapshot`-cadence so
  the K8s actuator continuously creates/patches/scales Deployments + Secrets
  (the real RBAC path), giving the reconcile + namespace lifecycle a sustained
  workout.

Every `--snapshot-interval` (default 60s) it APPENDS one JSON line to a DURABLE,
tailable log under `.angreal/test/soak/runs/` (a path that survives
`angreal purge` / `target` cleaning) capturing: server `/ready`, per-tenant
`GET /v1/agents` counts, kubectl counts (tenant namespaces, Deployment
spec/ready replicas, Secrets, pods by phase), `cloacina_fleet_agents_evicted_total`
+ `cloacina_fleet_work_reassigned_total`, the `api_keys` row count, the
leader/advisory-lock holder, and the **Docker VM disk free %**. The file is
flushed each snapshot so a crash / disk-full is captured WITH its cause.

VERDICT (computed continuously for abort + once at the end, written as a final
SUMMARY block to the log):
- steady fleets' per-tenant `actual_count` never spuriously dropped below
  `desired_count` after convergence (no eviction drift),
- convergence held — managed Deployment replicas ≈ sum(desired_count) and NO
  orphaned namespaces / Deployments / Secrets accumulate across churn cycles,
- `api_keys` growth quantified + bounded (keys-per-spawn),
- `/ready` failure count == 0,
- reconcile / leader-loop error count == 0 (server log scrape),
- no pods stuck CrashLoopBackOff / Pending.

DISK-SAFETY (critical for a 24h unattended run): every snapshot checks the
Docker VM disk free %; if it falls below `--disk-floor` (default 12%) the soak
ABORTS GRACEFULLY — scales every tenant to 0, reaps the managed namespaces,
logs `ABORT: disk-pressure` with the last-good state, tears the platform down
to reclaim space, and exits non-zero — rather than crash-filling the machine.
SIGTERM/SIGINT do the same graceful scale-to-0 + teardown.

Run with:
  angreal test soak k8s-fleet --duration 120              # SHORT validation
  angreal test soak k8s-fleet                              # 24h (default)
  angreal test soak k8s-fleet --reuse-cluster              # attach to a live platform
  angreal test soak k8s-fleet --no-teardown                # leave platform up at end

Tunable via env (flags override):
  CLOACINA_SOAK_K8S_DURATION_S    soak seconds (86400 = 24h)
  CLOACINA_SOAK_K8S_STEADY        agents per steady tenant (2)
  CLOACINA_SOAK_K8S_CHURN         churn-tenant amplitude (2)
  CLOACINA_SOAK_K8S_DISK_FLOOR    disk-free abort threshold, percent (12)
"""

import json
import os
import shutil
import signal
import subprocess
import sys
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_section_header, print_final_success

# Reuse the e2e platform bring-up + API/kubectl helpers verbatim — the soak MUST
# exercise the SAME real-RBAC path as the e2e (don't duplicate brittle logic).
from ..e2e.k8s_fleet import (
    RELEASE,
    NS,
    BOOTSTRAP_KEY,
    _api,
    _check_tool,
    _compose,
    _kube_env,
    _kubectl,
    _kubectl_json,
    _prepare_images,
    _server_logs,
    _wait_http,
    bring_up_cluster,
    helm_deploy_server,
    start_port_forward,
)

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
soak = angreal.command_group(name="soak", about="sustained-load soak tests")

# --- stable platform identity so --reuse-cluster can re-attach ---------------
# A FIXED compose project + host kubeconfig dir (the e2e uses a random project +
# temp dir per run; the soak owns a single long-lived platform instead). The
# platform dir lives under .angreal/ so it survives `angreal purge` / target
# cleaning. It holds the kubeconfig + the rendered helm values.
PROJECT = "cloacina-k8s-soak"
PLATFORM_DIR = Path(__file__).resolve().parent / ".k8s-platform"
KUBECONFIG = PLATFORM_DIR / "kubeconfig.host.yaml"
RUNS_DIR = Path(__file__).resolve().parent / "runs"

# Distinct from the e2e port-forward (18092) so a stray e2e forward can't clash.
FWD_PORT = 18094
TAG = "k8s-soak"

# Advisory-lock key the fleet control loop leader-elects on
# (crates/cloacina-server/src/autoscaler/leader.rs::FLEET_CONTROL_LOCK_KEY).
FLEET_LOCK_KEY = 8110127

# Tenants (namespaces become cloacina-tenant-<name>). 2 steady + 1 churn = 3.
STEADY_TENANTS = ["soaksteadya", "soaksteadyb"]
CHURN_TENANT = "soakchurn"
ALL_TENANTS = STEADY_TENANTS + [CHURN_TENANT]

DURATION_S = int(os.environ.get("CLOACINA_SOAK_K8S_DURATION_S", "86400"))
STEADY = int(os.environ.get("CLOACINA_SOAK_K8S_STEADY", "2"))
CHURN = int(os.environ.get("CLOACINA_SOAK_K8S_CHURN", "2"))
DISK_FLOOR = float(os.environ.get("CLOACINA_SOAK_K8S_DISK_FLOOR", "12"))

POSTGRES_DEPLOY = f"deploy/{RELEASE}-postgresql"
SERVER_DEPLOY = f"deploy/{RELEASE}-cloacina-server"

# Graceful-abort signalling: the handler sets a reason; the main loop and the
# finally block both honour it.
_ABORT = {"reason": None, "force_teardown": False}


# ---------------------------------------------------------------------------
# REST helpers (fleet provision/deprovision + roster) — base URL bound per run
# ---------------------------------------------------------------------------

def _fleet(base, tenant):
    code, body = _api("GET", f"/v1/tenants/{tenant}/fleet", expect=None, base=base)
    if code == 200 and isinstance(body, dict):
        return body.get("desired_count"), body.get("actual_count")
    return None, None


def _set_desired(base, tenant, target):
    """Drive a tenant's desired_count to `target` via the +1/-1 provision API."""
    desired, _ = _fleet(base, tenant)
    if desired is None:
        return 0
    moved = 0
    while desired < target:
        code, _ = _api("POST", f"/v1/tenants/{tenant}/fleet/provision",
                       expect=None, base=base)
        if code not in (200, 201):
            break
        desired += 1
        moved += 1
    while desired > target:
        _api("POST", f"/v1/tenants/{tenant}/fleet/deprovision", expect=None, base=base)
        desired -= 1
    return moved


def _ensure_tenant(base, tenant):
    """Create a tenant (idempotent) + lift its agent limit so we can provision."""
    _api("POST", "/v1/tenants", {"name": tenant}, expect=None, base=base)
    _api("POST", f"/v1/tenants/{tenant}/limits", {"max_agents": STEADY + CHURN + 5},
         expect=None, base=base)
    code, _ = _api("GET", f"/v1/tenants/{tenant}/fleet", expect=None, base=base)
    if code != 200:
        raise RuntimeError(f"tenant {tenant} not usable after create (GET fleet -> {code})")


def _roster_by_tenant(base):
    code, roster = _api("GET", "/v1/agents", expect=None, base=base)
    counts = {}
    if code == 200 and isinstance(roster, dict):
        for a in roster.get("items", []):
            t = a.get("tenant_id")
            counts[t] = counts.get(t, 0) + 1
    return counts


def _ready_code(base):
    try:
        import urllib.request
        with urllib.request.urlopen(f"{base}/ready", timeout=5) as r:
            return r.status
    except Exception as exc:  # HTTPError carries .code
        return getattr(exc, "code", 0)


def _metrics(base):
    try:
        import urllib.request
        with urllib.request.urlopen(f"{base}/metrics", timeout=5) as r:
            return r.read().decode()
    except Exception:
        return ""


def _metric(text, name):
    for line in text.splitlines():
        if line.startswith("#"):
            continue
        if line.startswith(name + " ") or line.startswith(name + "{"):
            try:
                return float(line.rsplit(None, 1)[-1])
            except (ValueError, IndexError):
                pass
    return 0.0


# ---------------------------------------------------------------------------
# kubectl / psql / disk probes
# ---------------------------------------------------------------------------

def _tenant_ns(tenant):
    return f"cloacina-tenant-{tenant}"


def _tenant_namespaces():
    """All actuator-created tenant namespaces (cloacina-tenant-*)."""
    data = _kubectl_json(["get", "ns"], KUBECONFIG)
    if not data:
        return []
    return [i["metadata"]["name"] for i in data.get("items", [])
            if i["metadata"]["name"].startswith("cloacina-tenant-")]


def _tenant_k8s(tenant):
    """Deployment replicas (spec/ready), secret count, pods-by-phase for a tenant
    namespace. Returns a dict; zeros if the namespace doesn't exist yet."""
    ns = _tenant_ns(tenant)
    out = {"deploy_spec": 0, "deploy_ready": 0, "deployments": 0,
           "secrets": 0, "pods": {"Running": 0, "Pending": 0,
                                  "CrashLoopBackOff": 0, "other": 0}}
    deploys = _kubectl_json(["get", "deploy", "-n", ns], KUBECONFIG)
    if deploys:
        items = deploys.get("items", [])
        out["deployments"] = len(items)
        for d in items:
            out["deploy_spec"] += d.get("spec", {}).get("replicas", 0) or 0
            out["deploy_ready"] += d.get("status", {}).get("readyReplicas", 0) or 0
    # The actuator's per-tenant Secret is named `cloacina-agent-key` and is NOT
    # labelled app.kubernetes.io/name=cloacina-agent, so count actuator secrets
    # by name (any extra cloacina-agent-key* secret => orphaned managed object).
    secrets = _kubectl_json(["get", "secret", "-n", ns], KUBECONFIG)
    if secrets:
        out["secrets"] = sum(1 for s in secrets.get("items", [])
                             if s["metadata"]["name"].startswith("cloacina-agent-key"))
    pods = _kubectl_json(["get", "pods", "-n", ns,
                          "-l", "app.kubernetes.io/name=cloacina-agent"], KUBECONFIG)
    if pods:
        for p in pods.get("items", []):
            phase = p.get("status", {}).get("phase", "other")
            # surface CrashLoopBackOff from container statuses (phase stays Running)
            crash = False
            for cs in p.get("status", {}).get("containerStatuses", []) or []:
                waiting = (cs.get("state", {}).get("waiting") or {})
                if waiting.get("reason") == "CrashLoopBackOff":
                    crash = True
            if crash:
                out["pods"]["CrashLoopBackOff"] += 1
            elif phase in out["pods"]:
                out["pods"][phase] += 1
            else:
                out["pods"]["other"] += 1
    return out


def _psql(sql):
    """Run a scalar query against the in-cluster postgres; '' on failure."""
    r = subprocess.run(
        ["kubectl", "exec", POSTGRES_DEPLOY, "-n", NS, "--",
         "env", "PGPASSWORD=cloacina", "psql", "-U", "cloacina", "-d", "cloacina",
         "-tAc", sql],
        env=_kube_env(KUBECONFIG), capture_output=True, text=True, check=False,
    )
    return (r.stdout or "").strip()


def _api_key_count():
    try:
        return int(_psql("SELECT count(*) FROM api_keys") or "0")
    except ValueError:
        return -1


def _leader_holder():
    """Best-effort leader/advisory-lock holder.

    The fleet control loop takes a *session-level* advisory lock per tick and
    releases it, so pg_locks is usually empty between ticks; the running
    single-replica server pod IS the de-facto leader. Report both: the holder
    pid (when a tick is mid-flight) and the server pod name."""
    holder = _psql(
        f"SELECT a.application_name||':'||a.pid FROM pg_locks l "
        f"JOIN pg_stat_activity a ON l.pid=a.pid "
        f"WHERE l.locktype='advisory' AND l.objid={FLEET_LOCK_KEY} LIMIT 1")
    pod = subprocess.run(
        ["kubectl", "get", "pods", "-n", NS, "-l",
         "app.kubernetes.io/name=cloacina-server",
         "-o", "jsonpath={.items[0].metadata.name}"],
        env=_kube_env(KUBECONFIG), capture_output=True, text=True, check=False,
    ).stdout.strip()
    return {"server_pod": pod or "?", "advisory_holder": holder or None}


def _disk_free_pct():
    """Docker VM disk free %, probed via a throwaway alpine container.

    Returns free-percent as a float, or -1.0 if the probe fails (treated as a
    non-abort 'unknown' so a flaky probe never crash-aborts a 24h run)."""
    r = subprocess.run(
        ["docker", "run", "--rm", "alpine", "sh", "-c", "df -P / | tail -1"],
        capture_output=True, text=True, check=False, timeout=30,
    )
    try:
        # filesystem size used avail use% mount  ->  use% is the 5th field
        use_pct = int(r.stdout.split()[4].rstrip("%"))
        return float(100 - use_pct)
    except (IndexError, ValueError):
        return -1.0


# ---------------------------------------------------------------------------
# durable log
# ---------------------------------------------------------------------------

def _now_iso():
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


class DurableLog:
    """Append-and-flush JSON-line log. Every write hits disk so a crash /
    disk-full leaves the cause on durable storage."""

    def __init__(self, path):
        self.path = path
        self._f = open(path, "a", buffering=1)  # line-buffered

    def write(self, obj):
        self._f.write(json.dumps(obj, separators=(",", ":")) + "\n")
        self._f.flush()
        os.fsync(self._f.fileno())

    def close(self):
        try:
            self._f.close()
        except Exception:
            pass


# ---------------------------------------------------------------------------
# graceful scale-to-0 + reap
# ---------------------------------------------------------------------------

def _scale_all_to_zero(base):
    """Drive every soak tenant's desired_count to 0 (best effort)."""
    for t in ALL_TENANTS:
        try:
            _set_desired(base, t, 0)
        except Exception as exc:
            print(f"  (scale-to-0 {t}: {exc})")


def _reap_tenant_namespaces():
    """Delete the actuator-managed tenant namespaces (reaps deploys/secrets/pods)."""
    for ns in _tenant_namespaces():
        _kubectl(["delete", "namespace", ns, "--wait=false"], KUBECONFIG, check=False)


# ---------------------------------------------------------------------------
# the command
# ---------------------------------------------------------------------------

@test()
@soak()
@angreal.command(
    name="k8s-fleet",
    about="K8s fleet-actuator SOAK — sustained load + churn under real chart RBAC (CLOACI-T-0816)",
    when_to_use=[
        "long-running (24h) stability soak of the Kubernetes fleet actuator",
        "hunting eviction drift, non-convergence, orphaned namespaces/secrets, key leaks under churn",
    ],
    when_not_to_use=[
        "the quick e2e correctness check (use `e2e k8s-fleet`)",
        "the Docker actuator soak (use `soak fleet-actuator`)",
        "environments without docker/kubectl/helm",
    ],
)
@angreal.argument(name="duration", long="duration", required=False, takes_value=True,
                  is_flag=False, help="soak seconds (default 86400 = 24h; use 120 for validation)")
@angreal.argument(name="snapshot_interval", long="snapshot-interval", required=False,
                  takes_value=True, is_flag=False,
                  help="seconds between durable snapshots + churn flips (default 60)")
@angreal.argument(name="reuse_cluster", long="reuse-cluster", takes_value=False, is_flag=True,
                  help="attach to an already-running soak platform (skip rebuild/redeploy)")
@angreal.argument(name="no_teardown", long="no-teardown", takes_value=False, is_flag=True,
                  help="leave the platform up at the end (managed pods scaled to 0)")
@angreal.argument(name="disk_floor", long="disk-floor", required=False, takes_value=True,
                  is_flag=False, help="abort when Docker VM disk free %% drops below this (default 12)")
@angreal.argument(name="skip_build", long="skip-build", takes_value=False, is_flag=True,
                  help="reuse already-built server/agent images (no docker build)")
def k8s_fleet(duration=None, snapshot_interval=None, reuse_cluster=False,
              no_teardown=False, disk_floor=None, skip_build=False):
    # Line-buffer stdout so a 24h run redirected to a file is live-tailable
    # (the DURABLE LOG is still the fsync'd source of truth either way).
    try:
        sys.stdout.reconfigure(line_buffering=True)
    except Exception:
        pass

    _check_tool("docker", "install Docker Desktop or colima")
    _check_tool("kubectl", "sudo port install kubectl")
    _check_tool("helm", "sudo port install kubernetes-helm")

    dur = int(duration) if duration else DURATION_S
    snap_s = int(snapshot_interval) if snapshot_interval else 60
    floor = float(disk_floor) if disk_floor else DISK_FLOOR

    PLATFORM_DIR.mkdir(parents=True, exist_ok=True)
    RUNS_DIR.mkdir(parents=True, exist_ok=True)
    ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    log_path = RUNS_DIR / f"k8s-soak-{ts}.log"
    log = DurableLog(log_path)

    print_section_header("Kubernetes Fleet-Actuator SOAK (real chart RBAC)")
    print(f"  duration={dur}s ({dur / 3600:.2f}h)  snapshot={snap_s}s  disk-floor={floor}%")
    print(f"  steady tenants={STEADY_TENANTS} @ {STEADY} agents each; churn={CHURN_TENANT} ±{CHURN}")
    print(f"  DURABLE LOG: {log_path}")
    print(f"  platform: project={PROJECT} kubeconfig={KUBECONFIG} reuse={reuse_cluster}")

    log.write({"event": "start", "ts": _now_iso(), "duration_s": dur,
               "snapshot_interval_s": snap_s, "disk_floor_pct": floor,
               "steady_tenants": STEADY_TENANTS, "steady": STEADY,
               "churn_tenant": CHURN_TENANT, "churn": CHURN,
               "reuse_cluster": bool(reuse_cluster), "no_teardown": bool(no_teardown)})

    # SIGTERM/SIGINT -> graceful abort.
    def _on_signal(signum, _frame):
        _ABORT["reason"] = f"signal-{signal.Signals(signum).name}"
        print(f"\n  !! {_ABORT['reason']} received — aborting gracefully")
    try:
        signal.signal(signal.SIGINT, _on_signal)
        signal.signal(signal.SIGTERM, _on_signal)
    except ValueError:
        # not the main thread (unusual invocation) — fall back to KeyboardInterrupt
        print("  (signal handlers unavailable off main thread; SIGINT still aborts)")

    compose_env = os.environ.copy()
    compose_env["CLOACINA_K8S_E2E_HOSTDIR"] = str(PLATFORM_DIR)

    fwd = None
    base = None
    verdict = {}
    aborted = False
    try:
        # --- platform bring-up (or re-attach) ------------------------------
        print_section_header("Step 1: platform (k3s + registry + server, real RBAC)")
        bring_up_cluster(PROJECT, PLATFORM_DIR, compose_env, KUBECONFIG, reuse=reuse_cluster)
        server_ref, agent_ref = _prepare_images(TAG, skip_build or reuse_cluster)
        helm_deploy_server(KUBECONFIG, PLATFORM_DIR, TAG, agent_ref, reuse=reuse_cluster)

        print_section_header("Step 2: port-forward + readiness")
        fwd, base = start_port_forward(KUBECONFIG, port=FWD_PORT)
        if not _wait_http(f"{base}/ready", timeout_s=90, proc=fwd):
            print(_server_logs(KUBECONFIG))
            raise RuntimeError("server /ready never became healthy via port-forward")
        print("  server /ready healthy ✓ (actuator=kubernetes, fleet RBAC)")

        # --- seed tenants + provision the steady fleets --------------------
        print_section_header("Step 3: seed tenants + provision steady fleets")
        for t in ALL_TENANTS:
            _ensure_tenant(base, t)
        for t in STEADY_TENANTS:
            _set_desired(base, t, STEADY)
        _set_desired(base, CHURN_TENANT, 0)

        # wait for steady fleets to converge (actual>=desired) before we start
        # counting eviction drift — initial pod spin-up is not drift.
        print("  waiting for steady fleets to converge...")
        conv_deadline = time.time() + 240
        while time.time() < conv_deadline:
            ok = all((_fleet(base, t)[1] or 0) >= STEADY for t in STEADY_TENANTS)
            if ok:
                break
            time.sleep(5)
        for t in STEADY_TENANTS:
            d, a = _fleet(base, t)
            print(f"    {t}: desired={d} actual={a}")

        # --- baselines -----------------------------------------------------
        m0 = _metrics(base)
        evicted0 = _metric(m0, "cloacina_fleet_agents_evicted_total")
        reassigned0 = _metric(m0, "cloacina_fleet_work_reassigned_total")
        keys0 = _api_key_count()
        log.write({"event": "baseline", "ts": _now_iso(), "evicted": evicted0,
                   "reassigned": reassigned0, "api_keys": keys0})
        print(f"  baseline: evicted={evicted0:.0f} reassigned={reassigned0:.0f} "
              f"api_keys={keys0}")

        # --- sustained churn + snapshots -----------------------------------
        print_section_header(f"Step 4: sustained load + churn ({dur}s)")
        ready_fail = 0
        drift_samples = 0          # steady tenant actual<desired (post-convergence)
        worst_drift = []           # (tenant, desired, actual) at drift
        crashloop_samples = 0
        spawn_ops = 0              # cumulative agents asked to spawn via churn
        max_ns = 0
        snapshots = 0
        churn_high = False
        last_good = None
        start = time.time()
        deadline = start + dur
        next_snap = start  # snapshot immediately, then every snap_s

        while time.time() < deadline:
            if _ABORT["reason"]:
                break
            now = time.time()
            if now < next_snap:
                time.sleep(min(1.0, next_snap - now))
                continue
            next_snap = now + snap_s

            # flip the churn tenant each snapshot tick (provision <-> deprovision)
            if churn_high:
                _set_desired(base, CHURN_TENANT, 0)
            else:
                moved = _set_desired(base, CHURN_TENANT, CHURN)
                spawn_ops += moved
            churn_high = not churn_high

            # --- gather snapshot -------------------------------------------
            ready = _ready_code(base)
            if ready != 200:
                ready_fail += 1
            roster = _roster_by_tenant(base)
            fleet_state = {t: dict(zip(("desired", "actual"), _fleet(base, t)))
                           for t in ALL_TENANTS}
            k8s = {t: _tenant_k8s(t) for t in ALL_TENANTS}
            tenant_ns = _tenant_namespaces()
            max_ns = max(max_ns, len(tenant_ns))
            mtext = _metrics(base)
            evicted = _metric(mtext, "cloacina_fleet_agents_evicted_total")
            reassigned = _metric(mtext, "cloacina_fleet_work_reassigned_total")
            keys = _api_key_count()
            leader = _leader_holder()
            disk = _disk_free_pct()

            # eviction drift on the STEADY fleets (desired held at STEADY)
            for t in STEADY_TENANTS:
                d = fleet_state[t].get("desired")
                a = fleet_state[t].get("actual")
                if d is not None and a is not None and a < d:
                    drift_samples += 1
                    if len(worst_drift) < 20:
                        worst_drift.append({"tenant": t, "desired": d, "actual": a})
            # CrashLoop across any tenant
            if any(k8s[t]["pods"]["CrashLoopBackOff"] > 0 for t in ALL_TENANTS):
                crashloop_samples += 1

            snap = {
                "event": "snapshot", "ts": _now_iso(),
                "elapsed_s": int(now - start), "ready": ready,
                "agents_by_tenant": roster,
                "agents_total": sum(roster.values()),
                "fleet": fleet_state,
                "k8s": {"tenant_namespaces": len(tenant_ns), "tenants": k8s},
                "metrics": {"evicted": evicted, "reassigned": reassigned},
                "api_keys": keys,
                "leader": leader,
                "disk_free_pct": disk,
            }
            log.write(snap)
            last_good = snap
            snapshots += 1
            print(f"  [{int(deadline - now)}s left] ready={ready} "
                  f"agents={snap['agents_total']} ns={len(tenant_ns)} "
                  f"evicted={evicted:.0f} reassigned={reassigned:.0f} "
                  f"keys={keys} disk_free={disk:.0f}% drift={drift_samples}")

            # --- DISK-SAFETY abort -----------------------------------------
            if disk >= 0 and disk < floor:
                _ABORT["reason"] = "disk-pressure"
                _ABORT["force_teardown"] = True
                log.write({"event": "ABORT", "ts": _now_iso(), "reason": "disk-pressure",
                           "disk_free_pct": disk, "disk_floor_pct": floor,
                           "last_good": last_good})
                print(f"\n  !! ABORT: disk-pressure ({disk:.0f}% < {floor}%) — "
                      f"scaling to 0 + reaping + tearing down to protect the machine")
                break

        # --- settle churn to 0 + converge ----------------------------------
        if not _ABORT["reason"]:
            print_section_header("Step 5: settle + converge")
            _set_desired(base, CHURN_TENANT, 0)
            settle_deadline = time.time() + 120
            while time.time() < settle_deadline:
                conv = all(
                    (_tenant_k8s(t)["deploy_spec"]) == (_fleet(base, t)[0] or 0)
                    for t in ALL_TENANTS)
                if conv:
                    break
                time.sleep(5)

        # --- VERDICT -------------------------------------------------------
        print_section_header("Step 6: verdict")
        sum_desired = sum((_fleet(base, t)[0] or 0) for t in ALL_TENANTS)
        sum_replicas = sum(_tenant_k8s(t)["deploy_spec"] for t in ALL_TENANTS)
        tenant_ns = _tenant_namespaces()
        mf = _metrics(base)
        evicted_f = _metric(mf, "cloacina_fleet_agents_evicted_total")
        reassigned_f = _metric(mf, "cloacina_fleet_work_reassigned_total")
        keys_f = _api_key_count()
        logs = _server_logs(KUBECONFIG, tail=4000)
        log_errors = [ln for ln in logs.splitlines()
                      if ("tenant reconcile failed" in ln
                          or "is forbidden" in ln
                          or "fleet leadership" in ln and "failed" in ln
                          or (" ERROR " in ln and "fleet" in ln.lower()))]
        # orphan accumulation: we churn ONE scratch tenant, so the namespace set
        # should never exceed the tenants we control (3); a Deployment/Secret
        # count > 1 per namespace means stale managed objects piling up.
        orphan_objs = []
        for t in ALL_TENANTS:
            k = _tenant_k8s(t)
            if k["deployments"] > 1:
                orphan_objs.append(f"{t}:{k['deployments']} deployments")
            if k["secrets"] > 1:
                orphan_objs.append(f"{t}:{k['secrets']} secrets")
        # pods stuck Pending at the end (post-settle Pending is a real problem)
        stuck_pending = [t for t in ALL_TENANTS if _tenant_k8s(t)["pods"]["Pending"] > 0]

        key_delta = (keys_f - keys0) if (keys_f >= 0 and keys0 >= 0) else None
        per_spawn = (key_delta / spawn_ops) if (key_delta is not None and spawn_ops) else None

        problems = []
        if drift_samples > 0:
            problems.append(f"eviction drift: steady fleet actual<desired in "
                            f"{drift_samples} sample(s) — a loaded healthy agent was lost")
        if abs(sum_replicas - sum_desired) > 1:
            problems.append(f"non-convergence: {sum_replicas} managed replicas vs "
                            f"{sum_desired} sum(desired_count)")
        if len(tenant_ns) > len(ALL_TENANTS):
            problems.append(f"orphaned namespaces accumulated: {len(tenant_ns)} "
                            f"cloacina-tenant-* ns > {len(ALL_TENANTS)} controlled tenants")
        if orphan_objs:
            problems.append("orphaned managed objects: " + ", ".join(orphan_objs))
        if ready_fail > 0:
            problems.append(f"/ready unhealthy in {ready_fail} sample(s)")
        if reassigned_f - reassigned0 != 0:
            problems.append(f"work reassigned: +{reassigned_f - reassigned0:.0f}")
        if log_errors:
            problems.append(f"reconcile/leader-loop errors in server log "
                            f"({len(log_errors)}); first: {log_errors[0][:200]}")
        if crashloop_samples > 0:
            problems.append(f"pods CrashLoopBackOff in {crashloop_samples} sample(s)")
        if stuck_pending:
            problems.append(f"pods stuck Pending after settle: {stuck_pending}")
        if per_spawn is not None and per_spawn > 3.0:
            problems.append(f"pathological key growth: {key_delta} keys / ~{spawn_ops} "
                            f"spawns ({per_spawn:.1f}/spawn)")

        verdict = {
            "event": "SUMMARY", "ts": _now_iso(),
            "aborted": bool(_ABORT["reason"]), "abort_reason": _ABORT["reason"],
            "duration_s": dur, "snapshots": snapshots,
            "steady_eviction_drift_samples": drift_samples,
            "worst_drift": worst_drift,
            "convergence": {"managed_replicas": sum_replicas, "sum_desired": sum_desired},
            "tenant_namespaces": len(tenant_ns), "controlled_tenants": len(ALL_TENANTS),
            "orphan_objects": orphan_objs,
            "ready_failures": ready_fail,
            "evicted": {"start": evicted0, "end": evicted_f, "delta": evicted_f - evicted0},
            "reassigned": {"start": reassigned0, "end": reassigned_f,
                           "delta": reassigned_f - reassigned0},
            "api_keys": {"start": keys0, "end": keys_f, "delta": key_delta,
                         "spawn_ops": spawn_ops, "per_spawn": per_spawn},
            "crashloop_samples": crashloop_samples, "stuck_pending": stuck_pending,
            "reconcile_leader_errors": len(log_errors),
            "problems": problems,
            "verdict": "FAIL" if (problems or _ABORT["reason"]) else "PASS",
        }
        log.write(verdict)

        print(f"  snapshots={snapshots} drift={drift_samples} "
              f"convergence(managed={sum_replicas}==desired={sum_desired}) "
              f"ns={len(tenant_ns)}/{len(ALL_TENANTS)} ready_fail={ready_fail}")
        print(f"  evicted {evicted0:.0f}->{evicted_f:.0f} (churn raises this — informational) "
              f"reassigned delta={reassigned_f - reassigned0:.0f}")
        print(f"  api_keys {keys0}->{keys_f} (delta {key_delta}, ~{per_spawn}"
              f"{'' if per_spawn is None else '/spawn'} over {spawn_ops} spawns)")
        print(f"  reconcile/leader errors={len(log_errors)} crashloop={crashloop_samples} "
              f"stuck_pending={stuck_pending}")

        if _ABORT["reason"]:
            aborted = True
            raise SystemExit(
                f"SOAK ABORTED ({_ABORT['reason']}). Summary + last-good state in {log_path}")
        if problems:
            raise AssertionError(
                "K8s fleet-actuator soak instability:\n  - " + "\n  - ".join(problems))

        print_final_success(
            f"K8s fleet-actuator soak STABLE over {dur}s: actuator converged "
            f"(managed={sum_replicas}==desired={sum_desired}), no eviction drift on the "
            f"steady fleets, no work reassignment, /ready healthy, no orphaned ns/objects; "
            f"key growth ~{per_spawn}/spawn. Durable log: {log_path}")

    except SystemExit:
        raise
    except BaseException as exc:  # noqa: BLE001 — log the cause durably, then re-raise
        log.write({"event": "ERROR", "ts": _now_iso(), "error": repr(exc)})
        raise
    finally:
        # graceful scale-to-0 always (idempotent, protects against leaked agents)
        if base is not None:
            print_section_header("Teardown")
            _scale_all_to_zero(base)
        if fwd is not None and fwd.poll() is None:
            fwd.terminate()

        force_td = _ABORT["force_teardown"]
        if force_td:
            _reap_tenant_namespaces()

        compose_path = Path(angreal.get_root()) / "files" / "docker-compose.k8s.yaml"
        if no_teardown and not force_td:
            print(f"  --no-teardown: platform left UP (tenants scaled to 0).\n"
                  f"    KUBECONFIG={KUBECONFIG}\n"
                  f"    kubectl --kubeconfig {KUBECONFIG} get pods -A\n"
                  f"    reuse next run: angreal test soak k8s-fleet --reuse-cluster\n"
                  f"    teardown: docker compose -f {compose_path} -p {PROJECT} down -v "
                  f"&& rm -rf {PLATFORM_DIR}")
        else:
            print(f"  tearing down platform (project={PROJECT})"
                  + (" [forced: disk-pressure]" if force_td else ""))
            try:
                _compose(["down", "-v"], PROJECT, env=compose_env)
            except Exception as exc:
                print(f"  (compose down error: {exc})")
            shutil.rmtree(PLATFORM_DIR, ignore_errors=True)
        log.write({"event": "end", "ts": _now_iso(),
                   "aborted": aborted, "torn_down": bool(not (no_teardown and not force_td))})
        log.close()
        print(f"  DURABLE LOG: {log_path}")
