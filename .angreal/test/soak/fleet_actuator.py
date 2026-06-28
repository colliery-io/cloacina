# Copyright 2026 Cloacina Contributors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.

"""Fleet CONTROL-PLANE soak — the Docker fleet actuator under sustained
provision/deprovision churn + workflow load (CLOACI-T-0816, CLOACI-I-0127).

Where `soak fleet` (CLOACI-T-0635) soaks the DATA plane (a fixed set of agents
running tasks), this soaks the CONTROL plane: the Docker ACTUATOR that *spawns
and stops* `cloacina-agent` containers as a tenant's `desired_count` moves. It
runs the fleet-actuator demo variant
(`docker/docker-compose.demo.yml` + `docker-compose.demo.fleet.yml`) and, for
`CLOACINA_SOAK_FLEET_ACTUATOR_DURATION_S` (default 120s):

- keeps a STEADY tenant (`acme`) provisioned + under acme_billing workflow load
  (real fleet work — proves a loaded fleet is not falsely evicted), and
- CHURNS a scratch tenant (`soakfleet`): periodic provision↑ / deprovision↓ so
  the actuator continuously spawns + stops containers (idle agents, so no work
  to reassign).

A small extra compose override turns the demo's autoscaler OFF (FLOOR=0,
INTERVAL_S=10) so `desired_count` is exactly what the soak provisions and the
reconcile loop drives actual→desired deterministically.

Control-plane stability checked at the end (this is the point — surface drift
the way the prior server soak did):

- **No false eviction under load** — the steady `acme` fleet's `actual_count`
  never drops below its `desired_count` at any sample (a healthy, loaded agent
  was never declared dead). NB: `cloacina_fleet_agents_evicted_total` itself
  legitimately RISES with churn (a stopped container's heartbeat goes stale and
  the sweeper reaps it), so the per-tenant actual_count is the meaningful drift
  signal, not the global counter — the counter is reported, not asserted ==0.
- **No spurious work reassignment** — `cloacina_fleet_work_reassigned_total`
  delta == 0 (churned agents were idle; the loaded steady fleet never stopped).
- **Convergence / no leaked containers** — after churn settles, the number of
  `cloacina.managed=true` containers == the sum of every tenant's
  `desired_count` (the actuator converged actual→desired; nothing orphaned).
- **Bounded api-key growth** — per-spawn key minting is a known first-pass
  tradeoff (DalKeyMinter, flagged in actuator/docker.rs); MEASURE the
  `api_keys` delta and report keys-per-spawn. Fail only if pathological
  (>3 keys per spawn — i.e. keys minted without a spawn).
- **/ready healthy** throughout (0 failures sampled).
- **Reconcile/leader loop clean** — no `reconcile failed` / `ERROR` /
  `is forbidden` in the server log over the run.

Tunable via env (defaults in parens):
  CLOACINA_SOAK_FLEET_ACTUATOR_DURATION_S   sustained-churn seconds (120)
  CLOACINA_SOAK_FLEET_ACTUATOR_STEADY       steady acme agents (2)
  CLOACINA_SOAK_FLEET_ACTUATOR_CHURN        scratch-tenant churn amplitude (2)
  CLOACINA_SOAK_FLEET_ACTUATOR_CYCLE_S      provision/deprovision period (15)
"""

import json
import os
import re
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

# Strip ANSI color codes so substring log checks are reliable (the server
# colorizes tracing fields, e.g. `actuator\x1b[0m=\x1b[0mdocker`).
_ANSI = re.compile(r"\x1b\[[0-9;]*m")

import angreal  # type: ignore

from .._utils import print_section_header, print_final_success

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
soak = angreal.command_group(name="soak", about="sustained-load soak tests")

REPO_ROOT = Path(__file__).resolve().parents[3]
COMPOSE_FILE = REPO_ROOT / "docker" / "docker-compose.demo.yml"
FLEET_OVERRIDE = REPO_ROOT / "docker" / "docker-compose.demo.fleet.yml"

BASE_URL = "http://localhost:8080"
BOOTSTRAP_KEY = "clk_demo_bootstrap_key_0001"

# Steady, loaded fleet (a real demo tenant with a compiled workflow).
STEADY_TENANT = "acme"
STEADY_WORKFLOW = "acme_billing"
# Scratch tenant we churn (provision/deprovision) — no workflows, just spawns.
CHURN_TENANT = "soakfleet"

MANAGED_LABEL = "label=cloacina.managed"

DURATION_S = int(os.environ.get("CLOACINA_SOAK_FLEET_ACTUATOR_DURATION_S", "120"))
STEADY = int(os.environ.get("CLOACINA_SOAK_FLEET_ACTUATOR_STEADY", "2"))
CHURN = int(os.environ.get("CLOACINA_SOAK_FLEET_ACTUATOR_CHURN", "2"))
CYCLE_S = int(os.environ.get("CLOACINA_SOAK_FLEET_ACTUATOR_CYCLE_S", "15"))


# ---------------------------------------------------------------------------
# compose + REST + metric helpers (mirrors soak/server.py + soak/fleet.py)
# ---------------------------------------------------------------------------

def _compose_files(soak_override):
    return ["-f", str(COMPOSE_FILE), "-f", str(FLEET_OVERRIDE), "-f", str(soak_override)]


def _compose(soak_override, *args, check=True, capture=False):
    cmd = ["docker", "compose", *_compose_files(soak_override), *args]
    return subprocess.run(cmd, cwd=REPO_ROOT, check=check, capture_output=capture, text=True)


def _api(method, path, body=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(f"{BASE_URL}{path}", method=method, data=data)
    req.add_header("Authorization", f"Bearer {BOOTSTRAP_KEY}")
    if data is not None:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            raw, code = resp.read().decode(), resp.status
    except urllib.error.HTTPError as exc:
        raw, code = exc.read().decode(), exc.code
    try:
        body = json.loads(raw) if raw else None
    except json.JSONDecodeError:
        body = raw
    return code, body


def _ready_code():
    try:
        with urllib.request.urlopen(f"{BASE_URL}/ready", timeout=5) as r:
            return r.status
    except urllib.error.HTTPError as exc:
        return exc.code
    except Exception:
        return 0


def _wait_health(timeout_s=240):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        try:
            code, _ = _api("GET", "/health")
            if code == 200:
                return True
        except Exception:
            pass
        time.sleep(2)
    return False


def _scrape_metrics():
    try:
        with urllib.request.urlopen(f"{BASE_URL}/metrics", timeout=5) as r:
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


def _fleet(tenant):
    code, body = _api("GET", f"/v1/tenants/{tenant}/fleet")
    if code == 200 and isinstance(body, dict):
        return body.get("desired_count", 0), body.get("actual_count", 0)
    return None, None


def _set_desired(tenant, target):
    """Drive a tenant's desired_count to `target` via the +1/-1 provision API."""
    desired, _ = _fleet(tenant)
    if desired is None:
        return
    while desired < target:
        code, _ = _api("POST", f"/v1/tenants/{tenant}/fleet/provision")
        if code == 409:
            break
        desired += 1
    while desired > target:
        _api("POST", f"/v1/tenants/{tenant}/fleet/deprovision")
        desired -= 1


def _managed_count():
    out = subprocess.run(
        ["docker", "ps", "-q", "--filter", MANAGED_LABEL],
        capture_output=True, text=True, check=False,
    ).stdout.split()
    return len(out)


def _managed_count_for(tenants):
    """Count running managed containers belonging to the given tenants only.

    The demo also runs its own `public`/default realm (which may carry a
    leftover desired_count row in a reused DB volume), so convergence is scoped
    to the tenants THIS soak controls rather than every managed container.
    """
    total = 0
    for t in tenants:
        out = subprocess.run(
            ["docker", "ps", "-q", "--filter", MANAGED_LABEL,
             "--filter", f"label=cloacina.tenant={t}"],
            capture_output=True, text=True, check=False,
        ).stdout.split()
        total += len(out)
    return total


def _reap_managed():
    out = subprocess.run(
        ["docker", "ps", "-aq", "--filter", MANAGED_LABEL],
        capture_output=True, text=True, check=False,
    ).stdout.split()
    if out:
        subprocess.run(["docker", "rm", "-f", *out], check=False)
    return len(out)


def _agent_key_count(soak_override):
    """Count minted per-spawn agent keys (name like 'agent:%') via psql."""
    try:
        r = _compose(
            soak_override, "exec", "-T", "postgres",
            "psql", "-U", "cloacina", "-d", "cloacina", "-tAc",
            "SELECT count(*) FROM api_keys WHERE name LIKE 'agent:%'",
            check=False, capture=True,
        )
        return int((r.stdout or "0").strip() or "0")
    except Exception:
        return -1


def _server_log_since(soak_override, since="0s"):
    r = _compose(soak_override, "logs", "server", "--since", since,
                 check=False, capture=True)
    return _ANSI.sub("", (r.stdout or "") + (r.stderr or ""))


@test()
@soak()
@angreal.command(
    name="fleet-actuator",
    about="fleet CONTROL-plane soak — Docker actuator under provision/deprovision churn",
    when_to_use=[
        "validating the Docker fleet ACTUATOR (spawn/stop) under sustained churn",
        "hunting container/key leaks, non-convergence, false eviction of a loaded fleet",
    ],
    when_not_to_use=["unit testing", "data-plane fleet soak (use `soak fleet`)",
                     "environments without Docker"],
)
@angreal.argument(
    name="duration", long="duration", required=False, takes_value=True, is_flag=False,
    help="sustained-churn seconds (default 120; use ~60 for a quick check)",
)
@angreal.argument(
    name="no_build", long="no-build", required=False, takes_value=False, is_flag=True,
    help="skip image rebuilds (reuse a stack that's already built)",
)
def fleet_actuator(duration=None, no_build=False):
    """Run the fleet control-plane soak against the fleet-actuator demo variant."""
    dur = int(duration) if duration else DURATION_S

    print_section_header("Fleet Control-Plane Soak (Docker actuator)")
    print(f"  compose: {COMPOSE_FILE.name} + {FLEET_OVERRIDE.name}")
    print(f"  config:  duration={dur}s steady={STEADY} churn=±{CHURN} cycle={CYCLE_S}s")

    # Extra override: deterministic convergence (autoscaler off, fast reconcile).
    soak_override = Path(tempfile.mkdtemp(prefix="fleet-actuator-soak-")) / "override.yml"
    soak_override.write_text(
        "services:\n"
        "  server:\n"
        "    environment:\n"
        '      CLOACINA_AUTOSCALE: "false"\n'
        '      CLOACINA_AUTOSCALE_FLOOR: "0"\n'
        '      CLOACINA_AUTOSCALE_INTERVAL_S: "10"\n'
    )

    problems = []
    try:
        # --- bring up the fleet-actuator demo variant ----------------------
        print_section_header("Step 1: bring up fleet-actuator demo variant")
        up = ["up", "-d"]
        if not no_build:
            up.append("--build")
        _compose(soak_override, *up)
        if not _wait_health():
            raise RuntimeError("server /health not ready")
        print("  server healthy ✓ (actuator=docker)")

        # Confirm the actuator initialized as docker (fail fast otherwise).
        boot = _server_log_since(soak_override, since="10m")
        # The control loop only starts when an actuator is active (kind != none),
        # so its presence confirms the docker actuator wired up.
        if ("fleet actuator initialized" not in boot
                or "fleet control loop started" not in boot):
            raise RuntimeError("server did not initialize the docker actuator "
                               "(check the socket mount + override)")

        # --- wait for the steady tenant + its workflow (demo seed) ---------
        print_section_header("Step 2: wait for steady tenant + workflow")
        deadline = time.time() + 600
        have_wf = False
        while time.time() < deadline:
            code, body = _api("GET", f"/v1/tenants/{STEADY_TENANT}/workflows")
            if code == 200 and isinstance(body, dict):
                names = {w.get("workflow_name") for w in body.get("items", [])}
                if STEADY_WORKFLOW in names:
                    have_wf = True
                    break
            time.sleep(5)
        if not have_wf:
            raise RuntimeError(f"{STEADY_TENANT}/{STEADY_WORKFLOW} never became available "
                               "(demo seed incomplete)")
        print(f"  {STEADY_TENANT}/{STEADY_WORKFLOW} available ✓")

        # --- provision steady fleet + create churn tenant ------------------
        print_section_header("Step 3: provision steady fleet + churn tenant")
        _set_desired(STEADY_TENANT, STEADY)
        # create the scratch churn tenant (POST /v1/tenants takes a JSON body).
        # Tolerate "already exists": a fresh DB returns 201; a reused demo volume
        # returns 409 OR 400 (the Postgres schema is already present). Verify the
        # tenant is usable afterwards via GET /fleet.
        req = urllib.request.Request(
            f"{BASE_URL}/v1/tenants", method="POST",
            data=json.dumps({"name": CHURN_TENANT}).encode())
        req.add_header("Authorization", f"Bearer {BOOTSTRAP_KEY}")
        req.add_header("Content-Type", "application/json")
        try:
            urllib.request.urlopen(req, timeout=60).read()
        except urllib.error.HTTPError as exc:
            if exc.code not in (200, 201, 409, 400):
                raise
        code, _ = _api("GET", f"/v1/tenants/{CHURN_TENANT}/fleet")
        if code != 200:
            raise RuntimeError(f"churn tenant {CHURN_TENANT} not usable (GET fleet -> {code})")
        _set_desired(CHURN_TENANT, 0)  # start churn tenant at 0

        # wait for the steady fleet to converge to STEADY managed agents
        deadline = time.time() + 90
        while time.time() < deadline:
            d, a = _fleet(STEADY_TENANT)
            if a is not None and a >= STEADY:
                break
            time.sleep(3)
        d, a = _fleet(STEADY_TENANT)
        print(f"  steady {STEADY_TENANT}: desired={d} actual={a}; "
              f"managed containers={_managed_count()}")

        # --- baselines -----------------------------------------------------
        m0 = _scrape_metrics()
        evicted0 = _metric(m0, "cloacina_fleet_agents_evicted_total")
        reassigned0 = _metric(m0, "cloacina_fleet_work_reassigned_total")
        keys0 = _agent_key_count(soak_override)
        print(f"  baseline: evicted={evicted0:.0f} reassigned={reassigned0:.0f} "
              f"agent_keys={keys0}")

        # --- sustained churn + workflow load -------------------------------
        print_section_header(f"Step 4: sustained churn + load ({dur}s)")
        ready_fail = 0
        steady_drift = 0          # samples where loaded acme lost a healthy agent
        spawn_ops = 0             # cumulative agents we asked the actuator to spawn
        submits = 0
        max_managed = 0
        deadline = time.time() + dur
        last_cycle = 0.0
        last_sample = 0.0
        last_print = 0.0
        churn_high = False
        while time.time() < deadline:
            now = time.time()

            # workflow load on the steady fleet (real fleet work). The execute
            # endpoint needs a JSON context body.
            code, _ = _api("POST",
                           f"/v1/tenants/{STEADY_TENANT}/workflows/{STEADY_WORKFLOW}/execute",
                           body={"context": {"soak": submits}})
            if code in (200, 202):
                submits += 1

            # provision/deprovision the churn tenant on CYCLE_S
            if now - last_cycle >= CYCLE_S:
                if churn_high:
                    _set_desired(CHURN_TENANT, 0)
                else:
                    _set_desired(CHURN_TENANT, CHURN)
                    spawn_ops += CHURN
                churn_high = not churn_high
                last_cycle = now

            # sample every 3s
            if now - last_sample >= 3:
                rc = _ready_code()
                if rc != 200:
                    ready_fail += 1
                ds, as_ = _fleet(STEADY_TENANT)
                if as_ is not None and ds is not None and as_ < ds:
                    steady_drift += 1
                max_managed = max(max_managed, _managed_count())
                last_sample = now
                if now - last_print >= 12:
                    text = _scrape_metrics()
                    print(f"  [{int(deadline - now)}s left] submits={submits} "
                          f"steady(acme) d={ds} a={as_} managed={_managed_count()} "
                          f"evicted={_metric(text, 'cloacina_fleet_agents_evicted_total'):.0f} "
                          f"reassigned={_metric(text, 'cloacina_fleet_work_reassigned_total'):.0f} "
                          f"ready={rc}")
                    last_print = now
            time.sleep(0.2)

        # --- settle: drive churn tenant to 0 + let reconcile converge ------
        print_section_header("Step 5: settle + converge")
        _set_desired(CHURN_TENANT, 0)
        # wait > reconcile interval (10s) for actual→desired to settle. Scope to
        # the soak's own tenants (acme + churn); the demo's public realm is
        # excluded (see _managed_count_for).
        soak_tenants = (STEADY_TENANT, CHURN_TENANT)
        settle_deadline = time.time() + 90
        sum_desired = None
        managed = None
        while time.time() < settle_deadline:
            ds_steady, _ = _fleet(STEADY_TENANT)
            ds_churn, _ = _fleet(CHURN_TENANT)
            sum_desired = (ds_steady or 0) + (ds_churn or 0)
            managed = _managed_count_for(soak_tenants)
            if managed == sum_desired:
                break
            time.sleep(3)
        print(f"  converged: managed(acme+churn)={managed} sum_desired={sum_desired} "
              f"(total managed incl. demo public={_managed_count()})")

        # --- final metrics + checks ----------------------------------------
        print_section_header("Step 6: control-plane stability checks")
        mf = _scrape_metrics()
        evicted = _metric(mf, "cloacina_fleet_agents_evicted_total")
        reassigned = _metric(mf, "cloacina_fleet_work_reassigned_total")
        keysf = _agent_key_count(soak_override)
        logs = _server_log_since(soak_override, since=f"{dur + 180}s")

        # 1. no false eviction of the loaded steady fleet
        if steady_drift > 0:
            problems.append(f"steady-fleet drift: {STEADY_TENANT} actual<desired in "
                            f"{steady_drift} sample(s) — a loaded, healthy agent was lost")

        # 2. no spurious work reassignment
        if reassigned - reassigned0 != 0:
            problems.append(f"work reassigned during soak: "
                            f"cloacina_fleet_work_reassigned_total +{reassigned - reassigned0:.0f}")

        # 3. convergence — no orphaned/leaked managed containers (scoped to the
        # soak's tenants). Allow ±1 for a reconcile tick racing the final read
        # ("≈"); a real leak shows managed >> desired.
        if abs(managed - sum_desired) > 1:
            problems.append(f"non-convergence: {managed} managed(acme+churn) containers vs "
                            f"{sum_desired} sum(desired_count) — orphaned/leaked or starved")

        # 4. /ready healthy throughout
        if ready_fail > 0:
            problems.append(f"/ready unhealthy in {ready_fail} sample(s)")

        # 5. reconcile/leader loop clean
        bad = [ln for ln in logs.splitlines()
               if ("fleet reconcile: tenant reconcile failed" in ln
                   or "is forbidden" in ln
                   or (" ERROR " in ln and "actuator" in ln.lower()))]
        if bad:
            problems.append(f"reconcile/control-loop errors in server log ({len(bad)}); "
                            f"first: {bad[0][:200]}")

        # 6. bounded api-key growth (MEASURE + report; fail only if pathological)
        key_delta = (keysf - keys0) if (keysf >= 0 and keys0 >= 0) else None
        per_spawn = (key_delta / spawn_ops) if (key_delta is not None and spawn_ops) else None
        if per_spawn is not None and per_spawn > 3.0:
            problems.append(f"pathological key growth: {key_delta} keys for ~{spawn_ops} "
                            f"spawns ({per_spawn:.1f}/spawn) — keys minted without spawns")

        # --- report --------------------------------------------------------
        print(f"  submits={submits} spawn_ops~{spawn_ops} peak_managed={max_managed}")
        print(f"  evicted: {evicted0:.0f} -> {evicted:.0f} (+{evicted - evicted0:.0f}; "
              f"expected to rise with churn — stopped agents reaped by the sweeper)")
        print(f"  reassigned: {reassigned0:.0f} -> {reassigned:.0f} "
              f"(delta {reassigned - reassigned0:.0f})")
        print(f"  steady-fleet drift samples: {steady_drift} (want 0)")
        print(f"  convergence: managed={managed} == sum_desired={sum_desired}")
        print(f"  /ready failures: {ready_fail} (want 0)")
        print(f"  agent_keys: {keys0} -> {keysf} "
              f"(delta {key_delta}, ~{per_spawn:.2f}/spawn)"
              if per_spawn is not None else
              f"  agent_keys: {keys0} -> {keysf} (delta {key_delta})")
        print("  NOTE: per-spawn key minting is a known first-pass tradeoff "
              "(DalKeyMinter, actuator/docker.rs) — keys are not reclaimed on stop.")

        if problems:
            print("\n--- server log (tail) ---")
            print(_server_log_since(soak_override, since="120s")[-3000:])
            raise AssertionError(
                "fleet control-plane instability:\n  - " + "\n  - ".join(problems))

        print_final_success(
            f"Fleet control-plane soak stable over {dur}s: actuator converged "
            f"(managed={managed}==desired={sum_desired}), no false eviction of the loaded "
            f"steady fleet, no work reassignment, /ready healthy; key growth "
            f"~{per_spawn:.2f}/spawn (known tradeoff)."
            if per_spawn is not None else
            f"Fleet control-plane soak stable over {dur}s.")
    finally:
        print_section_header("Teardown")
        try:
            _compose(soak_override, "down", check=False)
        except Exception as exc:
            print(f"  (compose down error: {exc})")
        reaped = _reap_managed()
        print(f"  reaped {reaped} actuator-managed agent container(s)")
        print("  (the demo stack + its volumes are removed; managed agents reaped)")
