# Copyright 2026 Cloacina Contributors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.

"""Multi-replica leadership e2e for the fleet control plane (CLOACI-T-0818).

Proves the design recorded in ADR CLOACI-A-0008 against a REAL 2-replica k3s
deployment: an *in-process advisory-lock leader* gates ONLY the fleet control
loop (autoscale + reconcile), while the API and the per-task scheduler scale
freely across every replica. The T-0815 1-replica soak could not test this —
its `advisory_holder` was always null. At replicaCount=2 the advisory lock is
the real test.

Reuses the T-0815/T-0816 k3s platform helpers verbatim
(`.angreal/test/e2e/k8s_fleet.py`): the same `docker-compose.k8s.yaml` (k3s +
registry), the same image build/push, the same chart RBAC. The ONLY deltas are
`replicaCount=2` and a distinct compose project + port (18096) so this lane
cannot clash with the e2e (18092) / soak (18094) platforms.

Assertions (priority order):
  1. BOTH replicas Ready — 2/2 server pods Running + readyReplicas==2, `/ready`
     healthy through the Service.
  2. SINGLE LEADER — the fleet advisory lock (key 8110127) is held by AT MOST
     one Postgres connection at any instant. Sampled at high frequency against
     `pg_locks`; the holder's `client_addr` is mapped to the owning server pod.
     (Leadership is per-tick: the loop takes `pg_try_advisory_lock` at the start
     of each control tick and releases it at the end — see
     crates/cloacina-server/src/autoscaler/leader.rs — so the validated
     invariant is "never two simultaneous holders", and the leader may legitimately
     differ tick-to-tick.)
  3. SINGLE-WRITER PROVISIONING — create a tenant, set its limit, provision N
     agents via REST; the (leader-only) reconcile actuates the tenant Deployment
     to EXACTLY N (not 2N) despite two server replicas. Deprovision → scales down.
  4. DISJOINT CLAIMING — drive workflow executions and assert each task runs
     exactly once across both replicas' schedulers. BEST-EFFORT / opt-in
     (`--claiming`): a helm-only cloacina-server deploy ships NO compiler, so
     source `.cloacina` packages never build → cannot execute. See the BLOCKED
     note + the per-claim SQL in `_assert_disjoint_claiming`.
  5. FAILOVER — delete the lock-holding replica; assert the surviving replica
     acquires the lock (control plane keeps working: provisioning still scales
     correctly) and the killed replica reschedules + rejoins as a follower
     (2/2 Ready again, lock holder count stays <=1).

Run with:  angreal test e2e k8s-leader               # build images + full run
           angreal test e2e k8s-leader --skip-build  # reuse retained k8s-soak images
           angreal test e2e k8s-leader --no-cleanup   # keep the cluster
           angreal test e2e k8s-leader --claiming     # also attempt assertion 4
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_section_header, print_final_success

# Reuse the k3s platform bring-up + image/kubectl helpers verbatim — this lane
# MUST exercise the SAME real-RBAC chart path as the e2e/soak (don't duplicate
# brittle logic). Only the helm values (replicaCount=2) + identity differ.
from .k8s_fleet import (
    CHART_DIR,
    COMPOSE_FILE,
    REGISTRY_HOST,
    REGISTRY_K8S,
    _check_tool,
    _compose,
    _image_exists,
    _kube_env,
    _kubectl,
    _kubectl_json,
    _prepare_images,
    _run,
    _server_logs,
    _server_values,
    _wait_http,
    bring_up_cluster,
    start_port_forward,
)

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
e2e = angreal.command_group(name="e2e", about="end-to-end tests against a live server")

# --- this lane's identity (distinct from e2e/soak so platforms never clash) ---
RELEASE = "fleet-leader"
NS = "cloacina-leader-e2e"
TENANT = "acme"
TENANT_NS = f"cloacina-tenant-{TENANT}"
BOOTSTRAP_KEY = "k8s-leader-e2e-bootstrap-key"
BOOTSTRAP_SECRET = "cloacina-bootstrap"
FWD_PORT = 18096
REPLICAS = 2

# Advisory-lock key the fleet control loop leader-elects on
# (crates/cloacina-server/src/autoscaler/leader.rs::FLEET_CONTROL_LOCK_KEY).
FLEET_LOCK_KEY = 8110127

# The exact psql query used for assertion 2 (single leader). Reported verbatim.
LOCK_QUERY = (
    "SELECT a.client_addr, a.pid FROM pg_locks l "
    "JOIN pg_stat_activity a ON l.pid=a.pid "
    f"WHERE l.locktype='advisory' AND l.objid={FLEET_LOCK_KEY} AND l.granted;"
)

SERVER_SELECTOR = "app.kubernetes.io/name=cloacina-server"


# ---------------------------------------------------------------------------
# REST helper (urllib, this lane's bootstrap key)
# ---------------------------------------------------------------------------

def _api(method, path, body=None, expect=(200, 201), base=None):
    url = f"{base}{path}"
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Authorization", f"Bearer {BOOTSTRAP_KEY}")
    if data is not None:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            raw = resp.read().decode()
            code = resp.status
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode()
        code = exc.code
    if expect is not None and code not in expect:
        raise AssertionError(f"{method} {path} -> {code} (expected {expect}); body: {raw[:400]}")
    try:
        return code, json.loads(raw) if raw else None
    except json.JSONDecodeError:
        return code, raw


# ---------------------------------------------------------------------------
# helm deploy at replicaCount=2 (real chart RBAC, fleet.actuator=kubernetes)
# ---------------------------------------------------------------------------

def _leader_values(tag, agent_ref, *, interval_s=1):
    """Reuse the shared server values, then force replicaCount=2 + a fast control
    tick so the advisory lock is observable and failover is quick."""
    base = _server_values(tag, agent_ref, BOOTSTRAP_SECRET)
    # Override the autoscale interval (shared default is 5s) for a faster tick.
    base = base.replace(
        '    - {name: CLOACINA_AUTOSCALE_INTERVAL_S, value: "5"}\n',
        f'    - {{name: CLOACINA_AUTOSCALE_INTERVAL_S, value: "{interval_s}"}}\n',
    )
    return f"replicaCount: {REPLICAS}\n" + base


def _helm_deploy(kubeconfig, hostdir, tag, agent_ref):
    _kubectl(["create", "namespace", NS], kubeconfig)
    _kubectl(["create", "secret", "generic", BOOTSTRAP_SECRET, "-n", NS,
              f"--from-literal=bootstrap-key={BOOTSTRAP_KEY}"], kubeconfig)
    values = hostdir / "values.yaml"
    values.write_text(_leader_values(tag, agent_ref))
    try:
        _run(["helm", "install", RELEASE, str(CHART_DIR), "-n", NS,
              "-f", str(values), "--wait", "--timeout=8m"], env=_kube_env(kubeconfig))
    except subprocess.CalledProcessError:
        _dump_diag(kubeconfig, "helm install failed")
        raise
    _kubectl(["rollout", "status", f"deploy/{RELEASE}-cloacina-server", "-n", NS,
              "--timeout=5m"], kubeconfig)


def _dump_diag(kubeconfig, label):
    print(f"\n===== DIAGNOSTICS: {label} =====", flush=True)
    _kubectl(["get", "pods", "-A", "-o", "wide"], kubeconfig, check=False)
    print("----- server logs (tail 200) -----", flush=True)
    print(_server_logs_leader(kubeconfig, tail=200), flush=True)
    print(f"===== END DIAGNOSTICS: {label} =====\n", flush=True)


def _server_logs_leader(kubeconfig, tail=300):
    # --all-containers + the leader release/ns (the shared _server_logs targets
    # the e2e release/ns). --prefix tags each line with its pod so we can see
    # leader vs follower behaviour across both replicas.
    proc = _run(["kubectl", "logs", f"deploy/{RELEASE}-cloacina-server", "-n", NS,
                 f"--tail={tail}", "--prefix=true", "--all-containers=true"],
                env=_kube_env(kubeconfig), check=False, capture=True)
    return (proc.stdout or "") + (proc.stderr or "")


# ---------------------------------------------------------------------------
# postgres / advisory-lock probes
# ---------------------------------------------------------------------------

def _postgres_target(kubeconfig):
    """Return a `kubectl exec` target for the chart's bundled postgres.

    Prefers the conventional `deploy/<release>-postgresql`; falls back to the
    first pod matching the subchart label if the name differs."""
    probe = _run(["kubectl", "get", f"deploy/{RELEASE}-postgresql", "-n", NS],
                 env=_kube_env(kubeconfig), check=False, capture=True)
    if probe.returncode == 0:
        return f"deploy/{RELEASE}-postgresql"
    pod = _run(["kubectl", "get", "pods", "-n", NS, "-l",
                "app.kubernetes.io/name=postgresql",
                "-o", "jsonpath={.items[0].metadata.name}"],
               env=_kube_env(kubeconfig), check=False, capture=True).stdout.strip()
    if not pod:
        raise AssertionError("could not locate the chart's postgresql pod "
                             "(tried deploy/<release>-postgresql + label "
                             "app.kubernetes.io/name=postgresql)")
    return pod


def _psql(kubeconfig, target, sql, capture=True):
    r = _run(["kubectl", "exec", target, "-n", NS, "--",
              "env", "PGPASSWORD=cloacina", "psql", "-U", "cloacina", "-d", "cloacina",
              "-tAc", sql], env=_kube_env(kubeconfig), check=False, capture=capture)
    return (r.stdout or "").strip()


def _lock_holders(kubeconfig, target):
    """Return list of (client_addr, pid) currently granted the fleet advisory lock."""
    out = _psql(kubeconfig, target, LOCK_QUERY)
    rows = []
    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        parts = line.split("|")
        rows.append((parts[0], parts[1] if len(parts) > 1 else "?"))
    return rows


def _server_pod_ips(kubeconfig):
    """Map server pod IP -> pod name (for client_addr -> pod resolution)."""
    data = _kubectl_json(["get", "pods", "-n", NS, "-l", SERVER_SELECTOR], kubeconfig)
    ip_to_pod = {}
    if data:
        for p in data.get("items", []):
            ip = p.get("status", {}).get("podIP")
            name = p.get("metadata", {}).get("name")
            if ip:
                ip_to_pod[ip] = name
    return ip_to_pod


def _server_pods_running(kubeconfig):
    data = _kubectl_json(["get", "pods", "-n", NS, "-l", SERVER_SELECTOR], kubeconfig)
    running = []
    if data:
        for p in data.get("items", []):
            phase = p.get("status", {}).get("phase")
            ready = all(c.get("ready") for c in p.get("status", {}).get("containerStatuses", []) or [])
            if phase == "Running" and ready:
                running.append(p["metadata"]["name"])
    return running


# ---------------------------------------------------------------------------
# assertion 2 helper: sample the lock, with churn to extend lock-hold windows
# ---------------------------------------------------------------------------

def _churn(base, stop_evt, n):
    """Flip the tenant's desired_count to force the (leader) reconcile to actuate
    each tick — this lengthens each lock-hold window, raising sampling catch
    probability. Best-effort; ignores transient errors."""
    high = True
    while not stop_evt.is_set():
        try:
            target = n if high else max(0, n - 1)
            _, fleet = _api("GET", f"/v1/tenants/{TENANT}/fleet", expect=None, base=base)
            desired = fleet.get("desired_count") if isinstance(fleet, dict) else None
            if desired is not None:
                if desired < target:
                    _api("POST", f"/v1/tenants/{TENANT}/fleet/provision", expect=None, base=base)
                elif desired > target:
                    _api("POST", f"/v1/tenants/{TENANT}/fleet/deprovision", expect=None, base=base)
            high = not high
        except Exception:
            pass
        stop_evt.wait(1.0)


def _sample_lock(kubeconfig, target, window_s, base=None, churn_n=0):
    """High-frequency sample of the advisory-lock holders over `window_s`.

    Returns (max_simultaneous, observed_holders) where observed_holders is a dict
    pod_name -> sample_count for every distinct holder seen."""
    ip_to_pod = _server_pod_ips(kubeconfig)
    max_simul = 0
    observed = {}
    catches = 0
    stop_evt = threading.Event()
    churn_t = None
    if base is not None and churn_n > 0:
        churn_t = threading.Thread(target=_churn, args=(base, stop_evt, churn_n), daemon=True)
        churn_t.start()
    deadline = time.time() + window_s
    try:
        while time.time() < deadline:
            holders = _lock_holders(kubeconfig, target)
            max_simul = max(max_simul, len(holders))
            if holders:
                catches += 1
                for addr, _pid in holders:
                    pod = ip_to_pod.get(addr, addr)
                    observed[pod] = observed.get(pod, 0) + 1
            time.sleep(0.1)
    finally:
        stop_evt.set()
        if churn_t is not None:
            churn_t.join(timeout=3)
    return max_simul, observed, catches


def _wait_lock_holder(kubeconfig, target, want_pod=None, not_pod=None, timeout_s=40):
    """Poll until a single lock holder is observed that matches the predicate.

    `want_pod`: holder must equal this pod. `not_pod`: holder must NOT equal this
    pod. Returns (pod, addr, pid) of the first matching catch, or None on timeout.
    Also asserts we never see >1 simultaneous holder while polling."""
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        ip_to_pod = _server_pod_ips(kubeconfig)
        holders = _lock_holders(kubeconfig, target)
        if len(holders) > 1:
            raise AssertionError(f"TWO simultaneous fleet-lock holders observed: {holders}")
        if len(holders) == 1:
            addr, pid = holders[0]
            pod = ip_to_pod.get(addr, addr)
            if want_pod is not None and pod != want_pod:
                pass
            elif not_pod is not None and pod == not_pod:
                pass
            else:
                return pod, addr, pid
        time.sleep(0.1)
    return None


# ---------------------------------------------------------------------------
# assertion 4 (best-effort / opt-in): in-cluster compiler + real executions
# ---------------------------------------------------------------------------

CLAIMING_BLOCKED_REASON = (
    "not validated end-to-end: a helm-only cloacina-server deploy ships NO compiler "
    "(charts/cloacina-server has no compiler template), so uploaded source .cloacina "
    "packages stay build_status='pending' forever and never execute. The disjoint-"
    "claiming property itself is enforced in cloacina-core by the task_outbox claim "
    "'DELETE ... FOR UPDATE SKIP LOCKED' + the claimed_by CAS "
    "(crates/cloacina/src/dal/unified/task_execution/claiming.rs) and BOTH replicas "
    "run the per-tenant scheduler unconditionally (services.rs: 'Always: per-runner "
    "task scheduler'; lib.rs:689 global runner per replica) — it is NOT leader-gated. "
    "To validate behaviourally: deploy a matching-ABI compiler Deployment against the "
    "same Postgres, upload a package whose path-deps resolve in-container, await "
    "build_status='success', drive M executions, then assert no (workflow_execution_id, "
    "task_name) has >1 task_executions row. Pass --claiming to attempt it."
)


def _prepare_compiler_image(tag, skip_build):
    """Tag+push a locally-present demo compiler image to the registry.

    Returns the in-cluster ref, or None if no compiler image is available (we do
    NOT trigger the ~2GB Dockerfile.compiler build implicitly)."""
    compiler_host = f"{REGISTRY_HOST}/cloacina-compiler:{tag}"
    if _image_exists(compiler_host):
        _run(["docker", "push", compiler_host])
        return f"{REGISTRY_K8S}/cloacina-compiler:{tag}"
    for src in ("cloacina-demo-fleet-compiler:latest", "docker-compiler:latest"):
        if _image_exists(src):
            _run(["docker", "tag", src, compiler_host])
            _run(["docker", "push", compiler_host])
            return f"{REGISTRY_K8S}/cloacina-compiler:{tag}"
    return None


def _deploy_compiler(kubeconfig, compiler_ref):
    db_url = f"postgres://cloacina:cloacina@{RELEASE}-postgresql:5432/cloacina"
    manifest = {
        "apiVersion": "apps/v1", "kind": "Deployment",
        "metadata": {"name": "cloacina-compiler", "namespace": NS},
        "spec": {
            "replicas": 1,
            "selector": {"matchLabels": {"app": "cloacina-compiler"}},
            "template": {
                "metadata": {"labels": {"app": "cloacina-compiler"}},
                "spec": {"containers": [{
                    "name": "compiler",
                    "image": compiler_ref,
                    "imagePullPolicy": "IfNotPresent",
                    "args": ["--bind", "0.0.0.0:9000", "--database-url", db_url,
                             "--poll-interval-ms", "1000",
                             "--cargo-target-dir", "/workspace/target",
                             "--cargo-flag=build", "--cargo-flag=--lib"],
                    "env": [{"name": "CARGO_PROFILE_DEV_DEBUG", "value": "0"}],
                }]},
            },
        },
    }
    proc = subprocess.run(["kubectl", "apply", "-n", NS, "-f", "-"],
                          input=json.dumps(manifest), env=_kube_env(kubeconfig),
                          text=True, capture_output=True)
    print(proc.stdout + proc.stderr)
    if proc.returncode != 0:
        raise AssertionError("failed to apply compiler Deployment")


def _upload_package(base, fixture_path):
    boundary = "----CloacinaLeaderE2E"
    body = (f"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; "
            f"filename=\"package.cloacina\"\r\nContent-Type: application/octet-stream\r\n\r\n").encode()
    body += Path(fixture_path).read_bytes()
    body += f"\r\n--{boundary}--\r\n".encode()
    req = urllib.request.Request(f"{base}/v1/tenants/{TENANT}/workflows", data=body, method="POST")
    req.add_header("Authorization", f"Bearer {BOOTSTRAP_KEY}")
    req.add_header("Content-Type", f"multipart/form-data; boundary={boundary}")
    with urllib.request.urlopen(req, timeout=30) as resp:
        return resp.status, json.loads(resp.read())


def _task_exec_schemas(kubeconfig, target):
    out = _psql(kubeconfig, target,
                "SELECT table_schema FROM information_schema.tables "
                "WHERE table_name='task_executions';")
    return [s.strip() for s in out.splitlines() if s.strip()]


def _assert_disjoint_claiming(kubeconfig, target, base, results, tag, skip_build,
                              workflow="data_processing",
                              fixture="examples/fixtures/dist/simple-packaged.cloacina",
                              m_execs=8):
    """Best-effort behavioural proof of disjoint claiming. Reports BLOCKED with a
    precise reason if the compiler path is unavailable / does not converge."""
    key = "4. disjoint claiming (scheduler scales)"
    compiler_ref = _prepare_compiler_image(tag, skip_build)
    if compiler_ref is None:
        results[key] = f"BLOCKED: no compiler image present; {CLAIMING_BLOCKED_REASON}"
        print(f"  BLOCKED [4]: no compiler image present to drive executions")
        return
    print(f"  deploying in-cluster compiler {compiler_ref} (best-effort)")
    _deploy_compiler(kubeconfig, compiler_ref)
    fixture_path = Path(angreal.get_root()).parent / fixture
    if not fixture_path.exists():
        results[key] = f"BLOCKED: fixture {fixture} missing"
        return
    try:
        code, body = _upload_package(base, str(fixture_path))
        print(f"  upload {fixture} -> {code} ({body})")
    except Exception as exc:
        results[key] = f"BLOCKED: package upload failed: {exc}"
        return
    # Await build_status=success (bounded). With cold ABI/path constraints this
    # frequently never converges in a helm-only deploy — hence best-effort.
    print("  waiting for build_status=success (bounded 6m)...")
    deadline = time.time() + 360
    built = False
    while time.time() < deadline:
        _, wf = _api("GET", f"/v1/tenants/{TENANT}/workflows", expect=None, base=base)
        items = wf.get("items", []) if isinstance(wf, dict) else []
        if any(w.get("build_status") == "success" for w in items):
            built = True
            break
        time.sleep(10)
    if not built:
        results[key] = (f"BLOCKED: package never reached build_status=success within 6m; "
                        f"{CLAIMING_BLOCKED_REASON}")
        print("  BLOCKED [4]: package never built (no/incompatible compiler)")
        return
    # Drive M executions; both replicas' schedulers race for the ready tasks.
    print(f"  driving {m_execs} executions of '{workflow}'")
    for i in range(m_execs):
        _api("POST", f"/v1/tenants/{TENANT}/workflows/{workflow}/execute",
             {"context": {"i": i}}, expect=(200, 201, 202), base=base)
    time.sleep(30)  # let them complete
    # Duplicate-dispatch check: any (workflow_execution_id, task_name) with >1 row.
    dup_found = None
    for schema in _task_exec_schemas(kubeconfig, target):
        dup = _psql(kubeconfig, target,
                    f"SELECT count(*) FROM (SELECT workflow_execution_id, task_name "
                    f"FROM {schema}.task_executions GROUP BY 1,2 HAVING count(*)>1) d;")
        total = _psql(kubeconfig, target, f"SELECT count(*) FROM {schema}.task_executions;")
        print(f"  schema {schema}: task_executions={total} duplicate_groups={dup}")
        if dup and dup.isdigit() and int(dup) > 0:
            dup_found = f"{schema}:{dup} duplicate (workflow_execution_id,task_name) groups"
    if dup_found:
        results[key] = f"FAIL: double-dispatch detected — {dup_found}"
        print(f"  FAIL [4]: {dup_found}")
    else:
        results[key] = "PASS"
        print("  PASS [4]: each task ran exactly once across both replicas' schedulers")


# ---------------------------------------------------------------------------
# the command
# ---------------------------------------------------------------------------

@test()
@e2e()
@angreal.command(
    name="k8s-leader",
    about="2-replica leadership e2e: single fleet leader + scaling API/scheduler (CLOACI-T-0818)",
    when_to_use=[
        "validating ADR CLOACI-A-0008's in-process advisory-lock leader at replicaCount=2",
        "proving single-writer fleet provisioning + leader failover on a real cluster",
    ],
    when_not_to_use=["unit testing", "running without docker/kubectl/helm",
                     "the 1-replica fleet-actuator correctness check (use `e2e k8s-fleet`)"],
)
@angreal.argument(name="no_cleanup", long="no-cleanup", takes_value=False, is_flag=True,
                  help="Leave the k3s cluster + helm release up for inspection")
@angreal.argument(name="skip_build", long="skip-build", takes_value=False, is_flag=True,
                  help="Reuse already-built server/agent images (no rebuild)")
@angreal.argument(name="claiming", long="claiming", takes_value=False, is_flag=True,
                  help="Also attempt assertion 4 (disjoint claiming) via an in-cluster compiler")
@angreal.argument(name="agents", long="agents", default_value="3",
                  help="Number of agents to provision for the single-writer test (default 3)")
@angreal.argument(name="tag", long="tag", default_value="k8s-soak",
                  help="Image tag in the local registry (default k8s-soak — the retained images)")
def k8s_leader(no_cleanup=False, skip_build=False, claiming=False, agents="3", tag="k8s-soak"):
    _check_tool("docker", "install Docker Desktop or colima")
    _check_tool("kubectl", "sudo port install kubectl")
    _check_tool("helm", "sudo port install kubernetes-helm")

    n_agents = int(agents)
    project = f"cloacina-k8s-leader-{uuid.uuid4().hex[:8]}"
    hostdir = Path(tempfile.mkdtemp(prefix="cloacina-k8s-leader-"))
    kubeconfig = hostdir / "kubeconfig.host.yaml"

    print_section_header(f"cloacina multi-replica leadership e2e (project: {project})")
    print(f"host kubeconfig dir: {hostdir}")
    print(f"replicaCount={REPLICAS}  port-forward={FWD_PORT}  fleet-lock-key={FLEET_LOCK_KEY}")

    results = {}
    fwd = None
    compose_env = os.environ.copy()
    compose_env["CLOACINA_K8S_E2E_HOSTDIR"] = str(hostdir)

    try:
        print("\n--- 1. bring up k3s + registry (docker compose) ---\n")
        bring_up_cluster(project, hostdir, compose_env, kubeconfig)

        print("\n--- 2. build/push images ---\n")
        server_ref, agent_ref = _prepare_images(tag, skip_build)

        print(f"\n--- 3. helm install cloacina-server (replicaCount={REPLICAS}, "
              f"fleet.actuator=kubernetes) ---\n")
        _helm_deploy(kubeconfig, hostdir, tag, agent_ref)

        print("\n--- 4. port-forward the server Service ---\n")
        fwd, base = start_port_forward(kubeconfig, release=RELEASE, ns=NS, port=FWD_PORT)
        if not _wait_http(f"{base}/ready", timeout_s=90, proc=fwd):
            print(_server_logs_leader(kubeconfig))
            raise AssertionError("server /ready never became healthy via the Service port-forward")

        postgres = _postgres_target(kubeconfig)
        print(f"  postgres exec target: {postgres}")

        # ===== ASSERTION 1: both replicas Ready ==============================
        print("\n--- ASSERTION 1: both replicas Ready ---\n")
        deploy = _kubectl_json(["get", "deploy", f"{RELEASE}-cloacina-server", "-n", NS], kubeconfig)
        ready = deploy.get("status", {}).get("readyReplicas") if deploy else None
        running = _server_pods_running(kubeconfig)
        if ready == REPLICAS and len(running) == REPLICAS:
            results["1. both replicas Ready (2/2) + /ready healthy"] = "PASS"
            print(f"  PASS [1]: {ready}/{REPLICAS} readyReplicas, {len(running)} pods Running "
                  f"({running}), /ready healthy through the Service")
        else:
            _dump_diag(kubeconfig, "replicas not both ready")
            raise AssertionError(f"expected {REPLICAS} ready/running server pods; "
                                 f"readyReplicas={ready} running={running}")

        # ===== ASSERTION 3a: provision N (single-writer) =====================
        # Provision BEFORE the lock sampling so the reconcile loop has real work
        # (which lengthens lock-hold windows for assertion 2).
        print("\n--- ASSERTION 3a: create tenant + provision N agents (REST) ---\n")
        code, _ = _api("POST", "/v1/tenants", {"name": TENANT}, expect=(200, 201, 409), base=base)
        print(f"  create tenant '{TENANT}' -> {code}")
        _api("POST", f"/v1/tenants/{TENANT}/limits", {"max_agents": n_agents + 5}, base=base)
        for i in range(n_agents):
            _api("POST", f"/v1/tenants/{TENANT}/fleet/provision", expect=(200, 201), base=base)
        _, fleet = _api("GET", f"/v1/tenants/{TENANT}/fleet", base=base)
        if fleet.get("desired_count") != n_agents:
            raise AssertionError(f"desired_count={fleet.get('desired_count')} != {n_agents}")
        # Reconcile (leader-only) must drive the tenant Deployment to EXACTLY N.
        deadline = time.time() + 120
        replicas = None
        deploys = None
        while time.time() < deadline:
            d = _kubectl_json(["get", "deploy", "cloacina-agent", "-n", TENANT_NS], kubeconfig)
            replicas = d.get("spec", {}).get("replicas") if d else None
            deploys = _kubectl_json(["get", "deploy", "-n", TENANT_NS], kubeconfig)
            if replicas == n_agents:
                break
            time.sleep(3)
        n_deploys = len(deploys.get("items", [])) if deploys else 0
        if replicas == n_agents and n_deploys == 1:
            print(f"  provision N: desired_count={n_agents}, agent Deployment replicas={replicas}, "
                  f"deployments-in-ns={n_deploys} (exactly N, not {REPLICAS}xN — single writer)")
        else:
            _dump_diag(kubeconfig, "single-writer provision mismatch")
            raise AssertionError(f"single-writer provision: replicas={replicas} (want {n_agents}), "
                                 f"deployments={n_deploys} (want 1)")

        # ===== ASSERTION 2: single leader (advisory lock) ====================
        print("\n--- ASSERTION 2: single fleet-lock holder (sampled) ---\n")
        print(f"  psql lock query: {LOCK_QUERY}")
        max_simul, observed, catches = _sample_lock(
            kubeconfig, postgres, window_s=60, base=base, churn_n=n_agents)
        print(f"  samples with a holder: {catches}; max simultaneous holders: {max_simul}; "
              f"holders observed (pod -> samples): {observed}")
        if max_simul > 1:
            raise AssertionError(f"TWO replicas held the fleet lock simultaneously ({observed}) — "
                                 f"single-writer leadership VIOLATED")
        if catches == 0:
            results["2. single fleet-lock holder"] = (
                "BLOCKED: never caught the lock held (tick window too small to sample); "
                "max simultaneous holders stayed 0")
            print("  BLOCKED [2]: could not catch the transient per-tick lock during sampling")
        else:
            leader_pods = sorted(observed.keys())
            results["2. single fleet-lock holder"] = "PASS"
            print(f"  PASS [2]: fleet lock held by at most ONE connection at a time over {catches} "
                  f"catches; holder pod(s) seen: {leader_pods} "
                  f"(per-tick election: may differ tick-to-tick, never simultaneous)")

        # ===== ASSERTION 3b: deprovision scales down =========================
        print("\n--- ASSERTION 3b: deprovision -> scale down ---\n")
        # churn during sampling may have moved desired_count; drive it explicitly to 0.
        for _ in range(n_agents + 2):
            _, fl = _api("GET", f"/v1/tenants/{TENANT}/fleet", expect=None, base=base)
            if (fl.get("desired_count") or 0) <= 0:
                break
            _api("POST", f"/v1/tenants/{TENANT}/fleet/deprovision", expect=None, base=base)
        deadline = time.time() + 90
        replicas = None
        while time.time() < deadline:
            d = _kubectl_json(["get", "deploy", "cloacina-agent", "-n", TENANT_NS], kubeconfig)
            replicas = d.get("spec", {}).get("replicas") if d else None
            if replicas == 0:
                break
            time.sleep(3)
        _, fleet = _api("GET", f"/v1/tenants/{TENANT}/fleet", base=base)
        if (fleet.get("desired_count") or 0) == 0 and replicas == 0:
            results["3. single-writer provisioning (N, then scale down)"] = "PASS"
            print("  PASS [3]: provisioned to exactly N then deprovisioned to 0 — single writer")
        else:
            results["3. single-writer provisioning (N, then scale down)"] = (
                f"BLOCKED: desired_count={fleet.get('desired_count')} replicas={replicas}")
            print(f"  BLOCKED [3]: desired_count={fleet.get('desired_count')} replicas={replicas}")

        # ===== ASSERTION 4: disjoint claiming (best-effort) ==================
        print("\n--- ASSERTION 4: disjoint claiming ---\n")
        if claiming:
            _assert_disjoint_claiming(kubeconfig, postgres, base, results, tag, skip_build)
        else:
            results["4. disjoint claiming (scheduler scales)"] = (
                f"BLOCKED: {CLAIMING_BLOCKED_REASON}")
            print("  BLOCKED [4]: not attempted (--claiming off); see report for the architectural reason")

        # ===== ASSERTION 5: failover =========================================
        print("\n--- ASSERTION 5: leader failover ---\n")
        # Re-provision so the reconcile loop has work (and so we can prove
        # provisioning still works AFTER the failover).
        _api("POST", f"/v1/tenants/{TENANT}/limits", {"max_agents": n_agents + 5}, base=base)
        for _ in range(2):
            _api("POST", f"/v1/tenants/{TENANT}/fleet/provision", expect=None, base=base)
        # Identify the current lock holder (the leader for the tick we catch).
        holder = _wait_lock_holder(kubeconfig, postgres, timeout_s=40)
        if holder is None:
            results["5. leader failover"] = (
                "BLOCKED: could not catch a lock holder to target for the kill")
            print("  BLOCKED [5]: never caught the lock holder pre-kill")
        else:
            old_pod, old_addr, old_pid = holder
            print(f"  current lock holder: pod={old_pod} addr={old_addr} pid={old_pid} — deleting it")
            _kubectl(["delete", "pod", old_pod, "-n", NS, "--wait=false"], kubeconfig)
            # The surviving replica must acquire the lock (different pod than killed).
            survivor = _wait_lock_holder(kubeconfig, postgres, not_pod=old_pod, timeout_s=60)
            if survivor is None:
                _dump_diag(kubeconfig, "no survivor acquired the lock")
                raise AssertionError("after killing the leader, NO surviving replica acquired the "
                                     "fleet lock within 60s — failover FAILED")
            new_pod, new_addr, new_pid = survivor
            print(f"  failover: lock re-acquired by pod={new_pod} addr={new_addr} pid={new_pid} "
                  f"(was pod={old_pod} pid={old_pid})")
            # `kubectl port-forward svc/...` pins to ONE pod; if it was pinned to
            # the killed leader it is now dead, so the next API call would hit a
            # closed connection (RemoteDisconnected). Re-establish the forward
            # against the Service (the survivor is still a Ready endpoint) before
            # the post-failover provisioning check.
            if fwd is not None and fwd.poll() is None:
                fwd.terminate()
                try:
                    fwd.wait(timeout=5)
                except Exception:
                    fwd.kill()
            fwd, base = start_port_forward(kubeconfig, release=RELEASE, ns=NS, port=FWD_PORT)
            if not _wait_http(f"{base}/ready", timeout_s=60, proc=fwd):
                raise AssertionError("server /ready not healthy via the refreshed port-forward "
                                     "after the leader kill")
            print("  port-forward re-established after the kill; verifying provisioning still works")
            # Provisioning still works under the new leader.
            _, fbefore = _api("GET", f"/v1/tenants/{TENANT}/fleet", base=base)
            d_before = (fbefore.get("desired_count") or 0)
            _api("POST", f"/v1/tenants/{TENANT}/fleet/provision", expect=(200, 201), base=base)
            want = d_before + 1
            deadline = time.time() + 90
            post_replicas = None
            while time.time() < deadline:
                d = _kubectl_json(["get", "deploy", "cloacina-agent", "-n", TENANT_NS], kubeconfig)
                post_replicas = d.get("spec", {}).get("replicas") if d else None
                if post_replicas == want:
                    break
                time.sleep(3)
            # Killed replica reschedules + rejoins as a follower (2/2 Ready again).
            _kubectl(["rollout", "status", f"deploy/{RELEASE}-cloacina-server", "-n", NS,
                      "--timeout=3m"], kubeconfig, check=False)
            rejoined = len(_server_pods_running(kubeconfig))
            post_holders = _lock_holders(kubeconfig, postgres)
            if post_replicas == want and rejoined == REPLICAS and len(post_holders) <= 1:
                results["5. leader failover"] = "PASS"
                print(f"  PASS [5]: survivor {new_pod} leads; provision scaled to {post_replicas}; "
                      f"killed replica rescheduled → {rejoined}/{REPLICAS} Ready; "
                      f"lock holders still <=1 ({len(post_holders)})")
            else:
                results["5. leader failover"] = (
                    f"BLOCKED: post-failover provision replicas={post_replicas} (want {want}), "
                    f"rejoined={rejoined}/{REPLICAS}, lock_holders={len(post_holders)}")
                print(f"  BLOCKED [5]: provision={post_replicas} rejoined={rejoined} "
                      f"holders={len(post_holders)}")

        # --- summary ---------------------------------------------------------
        print("\n" + "=" * 70)
        print("ASSERTION RESULTS")
        print("=" * 70)
        for label, status in results.items():
            print(f"  [{status.split(':')[0]:7}] {label}")
        print("=" * 70)
        blocked = [k for k, v in results.items() if v.startswith("BLOCKED")]
        failed = [k for k, v in results.items() if v.startswith("FAIL")]
        if not blocked and not failed:
            print_final_success("cloacina multi-replica leadership e2e — ALL assertions green")
        else:
            print(f"\n{len(results) - len(blocked) - len(failed)}/{len(results)} green; "
                  f"blocked: {[b.split('.')[0] for b in blocked]}; failed: {[f.split('.')[0] for f in failed]}")

        # Core leadership assertions (1, 2, 3, 5) must be green for a pass. 4 is
        # best-effort (BLOCKED is tolerated; an actual FAIL is not).
        core = ["1. both replicas Ready (2/2) + /ready healthy",
                "2. single fleet-lock holder",
                "3. single-writer provisioning (N, then scale down)",
                "5. leader failover"]
        bad_core = [c for c in core if results.get(c, "").startswith(("BLOCKED", "FAIL")) or c not in results]
        if bad_core or failed:
            print(f"\nFAILED core assertions: {bad_core or failed}")
            sys.exit(1)

    finally:
        if fwd is not None and fwd.poll() is None:
            fwd.terminate()
        if no_cleanup:
            print(f"\n--no-cleanup: cluster left up.\n"
                  f"  KUBECONFIG={kubeconfig}\n"
                  f"  kubectl --kubeconfig {kubeconfig} get pods -A -o wide\n"
                  f"  teardown: docker compose -f {COMPOSE_FILE} -p {project} down -v")
        else:
            print(f"\n--- cleanup: docker compose -p {project} down -v ---\n")
            _compose(["down", "-v"], project, env=compose_env)
            shutil.rmtree(hostdir, ignore_errors=True)
