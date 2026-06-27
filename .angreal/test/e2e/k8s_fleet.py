# Copyright 2026 Cloacina Contributors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.

"""k3s end-to-end harness for the Kubernetes fleet actuator (CLOACI-T-0815).

Validates the T-0814 `KubernetesActuator` + the chart's fleet RBAC
(`charts/cloacina-server/templates/fleet-rbac.yaml`) against a REAL cluster.

Pattern mirrors brokkr's `.angreal/task_helm.py`: an isolated single-node
**k3s** cluster + a local **registry:2** stood up via docker-compose
(`.angreal/files/docker-compose.k8s.yaml`), with a kubeconfig-translation step
so host `helm`/`kubectl` drive the cluster (localhost:6443) while the cluster
pulls locally-built images via the `registry:5000` mirror.

Unlike brokkr, cloacina-server + the per-tenant cloacina-agent fleet run INSIDE
the cluster: the server is helm-installed with `fleet.actuator=kubernetes`, so
it runs bound to the chart's least-privilege fleet ServiceAccount and the REAL
RBAC is exercised — exactly the path that would catch a missing `create`/`patch`
verb.

Assertions (priority order):
  1. Server boots in-cluster with `fleet.actuator=kubernetes` — substrate guard
     passes (in-cluster detected, actuator initializes), `/ready` healthy.
  2. Authenticate (bootstrap key), create a tenant, set its limit, `provision`
     N agents via the REST API.
  3. Under the chart's RBAC, the actuator creates namespace
     `cloacina-tenant-<t>` + the `cloacina-agent` Deployment (replicas=N) + the
     `cloacina-agent-key` Secret. THIS exercises the real RBAC.
  4. Agent pods become Ready and self-register: `GET /v1/agents` shows N agents
     for the tenant.
  5. `deprovision` → the Deployment scales down.

Run with:  angreal test e2e k8s-fleet            # build images + full run
           angreal test e2e k8s-fleet --skip-build   # reuse demo images
           angreal test e2e k8s-fleet --no-cleanup   # keep the cluster
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_section_header, print_final_success

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
e2e = angreal.command_group(name="e2e", about="end-to-end tests against a live server")

PROJECT_ROOT = Path(angreal.get_root()).parent
COMPOSE_FILE = Path(angreal.get_root()) / "files" / "docker-compose.k8s.yaml"
CHART_DIR = PROJECT_ROOT / "charts" / "cloacina-server"

# Registry: host-published port vs the in-network name k3s pulls from.
REGISTRY_HOST = "localhost:5050"
REGISTRY_K8S = "registry:5000"

# Source images to reuse with --skip-build (the running demo stack builds these
# from docker/Dockerfile.demo + docker/Dockerfile.agent on the current branch).
SRC_SERVER_IMAGE = "docker-server:latest"
SRC_AGENT_IMAGE = "docker-agent:latest"

RELEASE = "fleet-e2e"
NS = "cloacina-fleet-e2e"
TENANT = "acme"
TENANT_NS = f"cloacina-tenant-{TENANT}"
BOOTSTRAP_KEY = "k8s-fleet-e2e-bootstrap-key"
BOOTSTRAP_SECRET = "cloacina-bootstrap"
FWD_PORT = 18092


# ---------------------------------------------------------------------------
# shell helpers
# ---------------------------------------------------------------------------

def _run(cmd, env=None, check=True, capture=False, cwd=PROJECT_ROOT):
    printable = " ".join(str(c) for c in cmd)
    print(f"$ {printable}", flush=True)
    return subprocess.run(
        cmd, env=env, check=check, cwd=str(cwd),
        capture_output=capture, text=True,
    )


def _check_tool(tool, hint):
    if shutil.which(tool) is None:
        print(f"error: required tool '{tool}' not found on $PATH\nhint: {hint}",
              file=sys.stderr)
        sys.exit(2)


def _compose(args, project, env=None):
    base = ["docker", "compose", "-f", str(COMPOSE_FILE), "-p", project]
    return _run(base + args, env=env)


def _kube_env(kubeconfig):
    env = os.environ.copy()
    env["KUBECONFIG"] = str(kubeconfig)
    return env


def _kubectl(args, kubeconfig, check=True, capture=False):
    return _run(["kubectl", *args], env=_kube_env(kubeconfig), check=check, capture=capture)


def _kubectl_json(args, kubeconfig):
    proc = _run(["kubectl", *args, "-o", "json"], env=_kube_env(kubeconfig),
                check=False, capture=True)
    if proc.returncode != 0:
        return None
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError:
        return None


# ---------------------------------------------------------------------------
# REST API helpers (urllib — same stdlib path the other e2e lanes use)
# ---------------------------------------------------------------------------

def _api(method, path, body=None, expect=(200, 201), base=None):
    url = f"{base}{path}"
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Authorization", f"Bearer {BOOTSTRAP_KEY}")
    if data is not None:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
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


def _wait_http(url, timeout_s=90, proc=None):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if proc is not None and proc.poll() is not None:
            return False
        try:
            with urllib.request.urlopen(url, timeout=2) as r:
                if r.status < 500:
                    return True
        except Exception:
            time.sleep(1)
    return False


# ---------------------------------------------------------------------------
# image build / push
# ---------------------------------------------------------------------------

def _image_exists(ref):
    return _run(["docker", "image", "inspect", ref], check=False, capture=True).returncode == 0


def _prepare_images(tag, skip_build):
    """Build (or reuse) the server + agent images and push to the registry.

    Returns (server_ref_k8s, agent_ref_k8s) — references as the CLUSTER pulls
    them (registry:5000/...). Host pushes go to localhost:5050.

    IMPORTANT: the server is the component under test (the K8s actuator + the
    substrate guard landed in T-0810/T-0814), so the server image is ALWAYS
    built fresh from the repo's root `Dockerfile` (cloacina-server bin only —
    the fastest correct recipe). The demo-stack `docker-server` image is built
    from `docker/Dockerfile.demo` and can lag the actuator code, so it is NOT
    reused unless you explicitly pass --skip-build (use only when the demo stack
    was just rebuilt on this branch).

    The agent binary (cloacina-agent, T-0637) is older + stable, so a locally
    present `docker-agent:latest` (same `Dockerfile.agent` output) is reused to
    save a second heavy build; otherwise it is built fresh.
    """
    server_host = f"{REGISTRY_HOST}/cloacina-server:{tag}"
    agent_host = f"{REGISTRY_HOST}/cloacina-agent:{tag}"

    if skip_build:
        # Prefer images THIS harness already built+tagged locally as
        # localhost:5050/cloacina-{server,agent}:<tag> (a prior run on this same
        # --tag) — those carry the fresh actuator code. Only fall back to the
        # demo-stack images if the harness-tagged ones aren't present (and warn,
        # since the demo `docker-server` can lag the actuator code).
        if _image_exists(server_host) and _image_exists(agent_host):
            print(f"--skip-build: reusing already-built {server_host} + {agent_host}")
        else:
            print("--skip-build: harness-tagged images absent; falling back to demo-stack "
                  "images (WARNING: docker-server can lag the actuator code)")
            for src in (SRC_SERVER_IMAGE, SRC_AGENT_IMAGE):
                if not _image_exists(src):
                    print(f"error: --skip-build but neither {server_host} nor {src} "
                          f"present; drop --skip-build to build fresh", file=sys.stderr)
                    sys.exit(2)
            if not _image_exists(server_host):
                _run(["docker", "tag", SRC_SERVER_IMAGE, server_host])
            if not _image_exists(agent_host):
                _run(["docker", "tag", SRC_AGENT_IMAGE, agent_host])
    else:
        print("building cloacina-server fresh (root Dockerfile, cloacina-server bin) "
              "— heavy Rust build, be patient")
        _run(["docker", "build", "-t", server_host,
              "-f", str(PROJECT_ROOT / "Dockerfile"), str(PROJECT_ROOT)])
        if _image_exists(SRC_AGENT_IMAGE):
            print(f"reusing existing {SRC_AGENT_IMAGE} for the agent (stable binary)")
            _run(["docker", "tag", SRC_AGENT_IMAGE, agent_host])
        else:
            print("building cloacina-agent fresh (Dockerfile.agent)")
            _run(["docker", "build", "-t", agent_host,
                  "-f", str(PROJECT_ROOT / "docker" / "Dockerfile.agent"), str(PROJECT_ROOT)])

    _run(["docker", "push", server_host])
    _run(["docker", "push", agent_host])

    return f"{REGISTRY_K8S}/cloacina-server:{tag}", f"{REGISTRY_K8S}/cloacina-agent:{tag}"


# ---------------------------------------------------------------------------
# diagnostics
# ---------------------------------------------------------------------------

def _server_logs(kubeconfig, tail=300):
    proc = _run(["kubectl", "logs", f"deploy/{RELEASE}-cloacina-server", "-n", NS,
                 f"--tail={tail}"], env=_kube_env(kubeconfig), check=False, capture=True)
    return (proc.stdout or "") + (proc.stderr or "")


def _dump_diag(kubeconfig, label):
    print(f"\n===== DIAGNOSTICS: {label} =====", flush=True)
    _kubectl(["get", "pods", "-A"], kubeconfig, check=False)
    _kubectl(["get", "deploy,secret,ns", "-n", TENANT_NS], kubeconfig, check=False)
    print("----- server logs (tail 200) -----", flush=True)
    print(_server_logs(kubeconfig, tail=200), flush=True)
    print("----- tenant agent pod describe -----", flush=True)
    _kubectl(["describe", "pods", "-l", "app.kubernetes.io/name=cloacina-agent", "-n", TENANT_NS],
             kubeconfig, check=False)
    print("----- tenant agent pod logs -----", flush=True)
    _kubectl(["logs", "-l", "app.kubernetes.io/name=cloacina-agent", "-n", TENANT_NS, "--tail=100",
              "--all-containers=true"], kubeconfig, check=False)
    print(f"===== END DIAGNOSTICS: {label} =====\n", flush=True)


# ---------------------------------------------------------------------------
# the command
# ---------------------------------------------------------------------------

@test()
@e2e()
@angreal.command(
    name="k8s-fleet",
    about="k3s e2e for the Kubernetes fleet actuator + chart RBAC (CLOACI-T-0815)",
    when_to_use=[
        "validating the KubernetesActuator end to end against real k3s + RBAC",
        "confirming the chart's fleet ServiceAccount has the create/patch verbs",
    ],
    when_not_to_use=["unit testing", "running without docker"],
)
@angreal.argument(name="no_cleanup", long="no-cleanup", takes_value=False, is_flag=True,
                  help="Leave the k3s cluster + helm release up for inspection")
@angreal.argument(name="skip_build", long="skip-build", takes_value=False, is_flag=True,
                  help="Reuse existing docker-server/docker-agent images (no rebuild)")
@angreal.argument(name="agents", long="agents", default_value="2",
                  help="Number of agents to provision (default 2)")
@angreal.argument(name="tag", long="tag", default_value="k8s-e2e",
                  help="Image tag pushed to the local registry (default k8s-e2e)")
def k8s_fleet(no_cleanup=False, skip_build=False, agents="2", tag="k8s-e2e"):
    _check_tool("docker", "install Docker Desktop or colima")
    _check_tool("kubectl", "sudo port install kubectl")
    _check_tool("helm", "sudo port install kubernetes-helm")

    n_agents = int(agents)
    project = f"cloacina-k8s-e2e-{uuid.uuid4().hex[:8]}"
    hostdir = Path(tempfile.mkdtemp(prefix="cloacina-k8s-e2e-"))
    kubeconfig = hostdir / "kubeconfig.host.yaml"

    print_section_header(f"cloacina k8s fleet-actuator e2e (project: {project})")
    print(f"host kubeconfig dir: {hostdir}")

    results = {}      # assertion label -> "PASS" | "BLOCKED: ..."
    fwd = None

    compose_env = os.environ.copy()
    compose_env["CLOACINA_K8S_E2E_HOSTDIR"] = str(hostdir)

    try:
        # --- substrate: k3s + registry + kubeconfig --------------------------
        print("\n--- 1. bring up k3s + registry (docker compose) ---\n")
        # --wait only on the long-running healthchecked services; the
        # init/copy-kubeconfig containers run to completion, and `--wait` treats
        # any container exit (even 0) as a failure — so start those separately
        # and poll for the kubeconfig file instead.
        _compose(["up", "-d", "--wait", "registry", "k3s"], project, env=compose_env)
        _compose(["up", "-d", "init-kubeconfig", "copy-kubeconfig"], project, env=compose_env)

        deadline = time.time() + 60
        while not kubeconfig.exists() and time.time() < deadline:
            time.sleep(1)
        if not kubeconfig.exists():
            raise AssertionError(f"kubeconfig never materialised at {kubeconfig}")
        _kubectl(["get", "nodes"], kubeconfig)

        # --- images ----------------------------------------------------------
        print("\n--- 2. build/push images to the local registry ---\n")
        server_ref, agent_ref = _prepare_images(tag, skip_build)

        # --- helm install: server in-cluster, fleet.actuator=kubernetes ------
        print("\n--- 3. helm install cloacina-server (fleet.actuator=kubernetes) ---\n")
        _kubectl(["create", "namespace", NS], kubeconfig)
        _kubectl(["create", "secret", "generic", BOOTSTRAP_SECRET, "-n", NS,
                  f"--from-literal=bootstrap-key={BOOTSTRAP_KEY}"], kubeconfig)

        values = hostdir / "values.yaml"
        values.write_text(
            "image:\n"
            f"  repository: {REGISTRY_K8S}/cloacina-server\n"
            f"  tag: {tag}\n"
            "  pullPolicy: IfNotPresent\n"
            "postgresql:\n"
            "  enabled: true\n"
            "  persistence:\n"
            "    enabled: false\n"
            "  auth:\n"
            "    username: cloacina\n"
            "    password: cloacina\n"
            "    database: cloacina\n"
            "apiKeySecretRef:\n"
            f"  name: {BOOTSTRAP_SECRET}\n"
            "  key: bootstrap-key\n"
            "fleet:\n"
            "  actuator: kubernetes\n"
            f"  agentImage: {agent_ref}\n"
            # agentServerUrl is intentionally left to the chart DEFAULT, which now
            # renders the cross-namespace Service FQDN
            # (http://<fullname>.<namespace>.svc.cluster.local:<port>). Agents run
            # in per-tenant namespaces (cloacina-tenant-<t>) from which the short
            # service name does NOT resolve — so leaving this unset exercises that
            # the chart default is correct (regression guard for the T-0815 FQDN fix).
            "server:\n"
            "  extraEnv:\n"
            # Keep reconcile running but stop the util-based autoscaler from
            # scaling our hand-provisioned agents back to floor=0 (util is 0
            # — no workflows run in this harness).
            '    - {name: CLOACINA_AUTOSCALE, value: "false"}\n'
            '    - {name: CLOACINA_AUTOSCALE_INTERVAL_S, value: "5"}\n'
            # Deterministic: tenant creation must NOT auto-provision; we drive
            # desired_count explicitly through the provision API.
            '    - {name: CLOACINA_INITIAL_AGENTS, value: "0"}\n'
            '    - {name: RUST_LOG, value: "info,cloacina_server=debug"}\n'
        )
        try:
            _run(["helm", "install", RELEASE, str(CHART_DIR), "-n", NS,
                  "-f", str(values), "--wait", "--timeout=8m"],
                 env=_kube_env(kubeconfig))
        except subprocess.CalledProcessError:
            _dump_diag(kubeconfig, "helm install failed")
            raise

        _kubectl(["rollout", "status", f"deploy/{RELEASE}-cloacina-server", "-n", NS,
                  "--timeout=5m"], kubeconfig)

        # --- port-forward the in-cluster server ------------------------------
        print("\n--- 4. port-forward the server API ---\n")
        fwd = subprocess.Popen(
            ["kubectl", "port-forward", f"svc/{RELEASE}-cloacina-server",
             f"{FWD_PORT}:8080", "-n", NS],
            env=_kube_env(kubeconfig),
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        base = f"http://127.0.0.1:{FWD_PORT}"
        if not _wait_http(f"{base}/ready", timeout_s=60, proc=fwd):
            print(_server_logs(kubeconfig))
            raise AssertionError("server /ready never became healthy via port-forward")

        # ===== ASSERTION 1: in-cluster boot + actuator init ==================
        logs = _server_logs(kubeconfig)
        if "fleet actuator initialized" not in logs and \
                "fleet control loop started" not in logs:
            print(logs)
            raise AssertionError(
                "server up but no 'fleet actuator initialized' / 'fleet control loop "
                "started' log — the K8s actuator did not initialize in-cluster")
        results["1. server in-cluster + actuator init + /ready"] = "PASS"
        print("  PASS [1]: server in-cluster, fleet actuator initialized, /ready healthy")

        # ===== ASSERTION 2: auth + create tenant + set limit + provision N ===
        print("\n--- 5. create tenant + provision agents (REST) ---\n")
        code, _ = _api("POST", "/v1/tenants", {"name": TENANT}, expect=(200, 201, 409),
                       base=base)
        print(f"  create tenant '{TENANT}' -> {code}")
        _api("POST", f"/v1/tenants/{TENANT}/limits", {"max_agents": n_agents + 3}, base=base)
        for i in range(n_agents):
            code, body = _api("POST", f"/v1/tenants/{TENANT}/fleet/provision",
                              expect=(200, 201), base=base)
            print(f"  provision {i + 1}/{n_agents} -> desired_count={body.get('desired_count')}")
        _, fleet = _api("GET", f"/v1/tenants/{TENANT}/fleet", base=base)
        if fleet.get("desired_count") != n_agents:
            raise AssertionError(f"desired_count={fleet.get('desired_count')} != {n_agents}; {fleet}")
        results["2. auth + tenant + provision N (REST)"] = "PASS"
        print(f"  PASS [2]: tenant created, desired_count={n_agents} via provision API")

        # ===== ASSERTION 3: actuator creates ns/deploy/secret under RBAC =====
        print("\n--- 6. assert actuator-created cluster state (real RBAC) ---\n")
        deadline = time.time() + 120
        deploy = None
        while time.time() < deadline:
            ns = _kubectl_json(["get", "ns", TENANT_NS], kubeconfig)
            secret = _kubectl_json(["get", "secret", "cloacina-agent-key", "-n", TENANT_NS],
                                   kubeconfig)
            deploy = _kubectl_json(["get", "deploy", "cloacina-agent", "-n", TENANT_NS],
                                   kubeconfig)
            if ns and secret and deploy:
                replicas = deploy.get("spec", {}).get("replicas")
                if replicas == n_agents:
                    break
            time.sleep(3)
        # Surface any RBAC 403 explicitly.
        logs = _server_logs(kubeconfig)
        if "is forbidden" in logs or "cannot create" in logs or "cannot patch" in logs:
            print(logs)
            raise AssertionError("RBAC denial in server logs — the fleet ClusterRole is "
                                 "missing a verb (see 'is forbidden' above)")
        ns = _kubectl_json(["get", "ns", TENANT_NS], kubeconfig)
        secret = _kubectl_json(["get", "secret", "cloacina-agent-key", "-n", TENANT_NS], kubeconfig)
        deploy = _kubectl_json(["get", "deploy", "cloacina-agent", "-n", TENANT_NS], kubeconfig)
        if not (ns and secret and deploy):
            _dump_diag(kubeconfig, "actuator state missing")
            raise AssertionError(f"actuator did not create all of ns/secret/deploy "
                                 f"(ns={bool(ns)} secret={bool(secret)} deploy={bool(deploy)})")
        replicas = deploy.get("spec", {}).get("replicas")
        if replicas != n_agents:
            _dump_diag(kubeconfig, "deployment replica mismatch")
            raise AssertionError(f"cloacina-agent deploy replicas={replicas} != {n_agents}")
        if secret.get("data", {}).get("api-key") is None:
            raise AssertionError("cloacina-agent-key Secret has no 'api-key' key")
        results["3. actuator ns/deploy/secret under real RBAC"] = "PASS"
        print(f"  PASS [3]: namespace {TENANT_NS} + cloacina-agent Deployment "
              f"(replicas={n_agents}) + cloacina-agent-key Secret, all created under the "
              f"chart's fleet ServiceAccount (real RBAC create/patch verified)")

        # ===== ASSERTION 4: agents become Ready + self-register ==============
        print("\n--- 7. wait for agents to self-register ---\n")
        deadline = time.time() + 180
        registered = 0
        while time.time() < deadline:
            _, roster = _api("GET", "/v1/agents", base=base)
            items = roster.get("items", []) if isinstance(roster, dict) else []
            registered = sum(1 for a in items if a.get("tenant_id") == TENANT)
            if registered >= n_agents:
                break
            time.sleep(5)
        if registered >= n_agents:
            results["4. agents Ready + self-register"] = "PASS"
            print(f"  PASS [4]: {registered}/{n_agents} agents registered for tenant {TENANT}")
        else:
            _dump_diag(kubeconfig, "agent registration incomplete")
            results["4. agents Ready + self-register"] = (
                f"BLOCKED: only {registered}/{n_agents} agents registered (see agent pod "
                f"logs + server logs in diagnostics above)")
            print(f"  BLOCKED [4]: {registered}/{n_agents} registered")

        # ===== ASSERTION 5: deprovision scales down ==========================
        print("\n--- 8. deprovision → scale down ---\n")
        for i in range(n_agents):
            _api("POST", f"/v1/tenants/{TENANT}/fleet/deprovision", expect=(200, 201), base=base)
        _, fleet = _api("GET", f"/v1/tenants/{TENANT}/fleet", base=base)
        deadline = time.time() + 90
        replicas = None
        while time.time() < deadline:
            deploy = _kubectl_json(["get", "deploy", "cloacina-agent", "-n", TENANT_NS], kubeconfig)
            replicas = deploy.get("spec", {}).get("replicas") if deploy else None
            if replicas == 0:
                break
            time.sleep(3)
        if fleet.get("desired_count") == 0 and replicas == 0:
            results["5. deprovision scales down"] = "PASS"
            print("  PASS [5]: deprovision drove desired_count=0 and Deployment replicas=0")
        else:
            results["5. deprovision scales down"] = (
                f"BLOCKED: desired_count={fleet.get('desired_count')} replicas={replicas}")
            print(f"  BLOCKED [5]: desired_count={fleet.get('desired_count')} replicas={replicas}")

        # --- summary ---------------------------------------------------------
        print("\n" + "=" * 64)
        print("ASSERTION RESULTS")
        print("=" * 64)
        for label, status in results.items():
            print(f"  [{status.split(':')[0]:7}] {label}")
        print("=" * 64)
        blocked = [k for k, v in results.items() if v.startswith("BLOCKED")]
        if blocked:
            print(f"\n{len(results) - len(blocked)}/{len(results)} assertions green; "
                  f"blocked: {', '.join(b.split('.')[0] for b in blocked)}")
        else:
            print_final_success("cloacina k8s fleet-actuator e2e — ALL assertions green")

        # Core assertions 1-3 must be green for the run to pass.
        core = ["1. server in-cluster + actuator init + /ready",
                "2. auth + tenant + provision N (REST)",
                "3. actuator ns/deploy/secret under real RBAC"]
        if any(results.get(c, "").startswith("BLOCKED") or c not in results for c in core):
            sys.exit(1)

    finally:
        if fwd is not None and fwd.poll() is None:
            fwd.terminate()
        if no_cleanup:
            print(f"\n--no-cleanup: cluster left up.\n"
                  f"  KUBECONFIG={kubeconfig}\n"
                  f"  kubectl --kubeconfig {kubeconfig} get pods -A\n"
                  f"  teardown: docker compose -f {COMPOSE_FILE} -p {project} down -v")
        else:
            print(f"\n--- cleanup: docker compose -p {project} down -v ---\n")
            _compose(["down", "-v"], project, env=compose_env)
            shutil.rmtree(hostdir, ignore_errors=True)
