"""
Helm chart testing for charts/cloacina-server.

CLOACI-I-0111 / T-0605.

Two surfaces:
  - `angreal helm lint`       — fast: helm lint + helm template variants.
                                Mirrors the .github/workflows/ci.yml job.
  - `angreal helm test`       — slow: builds the cloacina-server image,
                                spins up a kind cluster, loads the image,
                                helm-installs the chart with bundled
                                Postgres, port-forwards, curls /health,
                                tears the cluster down. End-to-end.

Prerequisites for `helm test`:
  - docker (running)
  - kind (`port install kind` or `brew install kind`)
  - helm (`port install kubernetes-helm` or `brew install helm`)
  - kubectl (already on the dev box)
"""

import os
import shutil
import subprocess
import sys
import tempfile
import time
import uuid
from pathlib import Path

import angreal  # type: ignore

PROJECT_ROOT = Path(angreal.get_root()).parent
CHART_DIR = PROJECT_ROOT / "charts" / "cloacina-server"
IMAGE_TAG = "cloacina-server:helm-e2e"
RELEASE_NAME = "cloacina-e2e"
NAMESPACE = "cloacina-e2e"

helm = angreal.command_group(name="helm", about="commands for Helm chart testing")


# ---------------------------------------------------------------------------
# Tool checks
# ---------------------------------------------------------------------------

def _check_tool(tool: str, install_hint: str) -> None:
    if shutil.which(tool) is None:
        print(f"error: required tool '{tool}' not found on $PATH", file=sys.stderr)
        print(f"hint: {install_hint}", file=sys.stderr)
        sys.exit(2)


def _run(cmd, **kwargs):
    """Run a command, stream output, raise on non-zero unless check=False."""
    check = kwargs.pop("check", True)
    print(f"$ {' '.join(str(c) for c in cmd)}", flush=True)
    return subprocess.run(cmd, check=check, **kwargs)


# ---------------------------------------------------------------------------
# helm lint  (fast)
# ---------------------------------------------------------------------------

@helm()
@angreal.command(
    name="lint",
    about="helm lint + helm template variants (no cluster)",
)
def helm_lint():
    _check_tool("helm", "sudo port install kubernetes-helm  # or brew install helm")

    _run(["helm", "lint", str(CHART_DIR)])

    # Variant 1: bring-your-own Postgres via plaintext URL.
    _run([
        "helm", "template", "cloacina", str(CHART_DIR),
        "--set", "database.url=postgres://u:p@h:5432/d",
    ], stdout=subprocess.DEVNULL)

    # Variant 2: bring-your-own Postgres via secret ref + extras.
    _run([
        "helm", "template", "cloacina", str(CHART_DIR),
        "--set", "databaseUrlSecretRef.name=cloacina-db",
        "--set", "ingress.enabled=true",
        "--set", "serviceMonitor.enabled=true",
    ], stdout=subprocess.DEVNULL)

    # Variant 3: validateDatabase fail-fast — must error.
    rc = subprocess.run(
        ["helm", "template", "cloacina", str(CHART_DIR)],
        capture_output=True, text=True,
    )
    if rc.returncode == 0:
        print(
            "error: helm template should have failed when no database is configured",
            file=sys.stderr,
        )
        sys.exit(1)
    if "configure exactly one of" not in rc.stderr:
        print(
            "error: expected validateDatabase fail message in stderr, got:\n"
            + rc.stderr,
            file=sys.stderr,
        )
        sys.exit(1)

    print("\n✓ helm lint + template variants pass")


# ---------------------------------------------------------------------------
# helm test  (slow, end-to-end via kind)
# ---------------------------------------------------------------------------

def _kind_cluster_name() -> str:
    # Short suffix keeps repeated invocations from colliding.
    return f"cloacina-helm-{uuid.uuid4().hex[:6]}"


def _wait_rollout(deploy_name: str, namespace: str, timeout: str = "5m") -> None:
    _run([
        "kubectl", "rollout", "status",
        f"deployment/{deploy_name}",
        "-n", namespace,
        f"--timeout={timeout}",
    ])


def _kubectl_get_health(namespace: str, service: str) -> bool:
    """Run a curl pod against the service and check /health → 200."""
    cmd = [
        "kubectl", "run", f"healthcheck-{uuid.uuid4().hex[:4]}",
        "-n", namespace,
        "--rm", "--restart=Never", "-i",
        "--image=curlimages/curl:8.10.1",
        "--",
        "curl", "-fsSL", "-o", "/dev/null", "-w", "%{http_code}",
        f"http://{service}:8080/health",
    ]
    print(f"$ {' '.join(cmd)}", flush=True)
    proc = subprocess.run(cmd, capture_output=True, text=True)
    print(proc.stdout)
    print(proc.stderr, file=sys.stderr)
    if proc.returncode != 0:
        return False
    # The HTTP code is on the last line of stdout; curlimages sometimes
    # interleaves with pod-cleanup chatter, so search for "200".
    return "200" in proc.stdout


@helm()
@angreal.command(
    name="test",
    about="end-to-end helm chart install on a kind cluster + /health curl",
)
def helm_test():
    _check_tool("docker", "install Docker Desktop or colima")
    _check_tool("kind", "sudo port install kind  # or brew install kind")
    _check_tool("kubectl", "sudo port install kubectl")
    _check_tool("helm", "sudo port install kubernetes-helm")

    cluster = _kind_cluster_name()
    kubeconfig = Path(os.environ.get("KUBECONFIG", str(Path.home() / ".kube" / "config")))
    print(f"\n=== cloacina-server helm e2e ({cluster}) ===\n")

    try:
        # Step 1: build the image.
        print("\n--- 1. Build cloacina-server image (this may take a while) ---\n")
        _run([
            "docker", "build",
            "-t", IMAGE_TAG,
            "-f", str(PROJECT_ROOT / "Dockerfile"),
            str(PROJECT_ROOT),
        ])

        # Step 2: kind cluster.
        print("\n--- 2. Create kind cluster ---\n")
        _run(["kind", "create", "cluster", "--name", cluster, "--wait", "120s"])

        # Step 3: load image.
        print("\n--- 3. Load image into kind ---\n")
        _run(["kind", "load", "docker-image", IMAGE_TAG, "--name", cluster])

        # Step 4: helm dependency update (vendor Bitnami postgres).
        print("\n--- 4. helm dependency update ---\n")
        _run(["helm", "dependency", "update", str(CHART_DIR)])

        # Step 5: install the chart.
        print("\n--- 5. helm install (bundled postgres) ---\n")
        password = uuid.uuid4().hex
        _run([
            "helm", "install", RELEASE_NAME, str(CHART_DIR),
            "--namespace", NAMESPACE, "--create-namespace",
            "--set", f"image.repository={IMAGE_TAG.split(':')[0]}",
            "--set", f"image.tag={IMAGE_TAG.split(':')[1]}",
            "--set", "image.pullPolicy=Never",  # use loaded image, never pull
            "--set", "postgresql.enabled=true",
            "--set", f"postgresql.auth.password={password}",
            "--wait", "--timeout=8m",
        ])

        # Step 6: rollout + health probe.
        print("\n--- 6. Verify pod ready + /health ---\n")
        _wait_rollout(f"{RELEASE_NAME}-cloacina-server", NAMESPACE, timeout="5m")
        # Settle a beat so probes pass at least once.
        time.sleep(3)
        ok = _kubectl_get_health(NAMESPACE, f"{RELEASE_NAME}-cloacina-server")
        if not ok:
            # Dump diagnostics before failing.
            _run(["kubectl", "get", "pods", "-n", NAMESPACE], check=False)
            _run([
                "kubectl", "logs",
                "-l", f"app.kubernetes.io/instance={RELEASE_NAME}",
                "-n", NAMESPACE, "--tail=200",
            ], check=False)
            print("\n✗ /health did not return 200", file=sys.stderr)
            sys.exit(1)

        print("\n✓ chart installed, pod ready, /health = 200")

    finally:
        # Always tear the cluster down — leftovers eat host resources fast.
        print(f"\n--- cleanup: kind delete cluster {cluster} ---\n")
        _run(["kind", "delete", "cluster", "--name", cluster], check=False)
        _ = kubeconfig  # keep linter quiet


# ---------------------------------------------------------------------------
# helm fleet  (slow, end-to-end containerized fleet via kind) — CLOACI-T-0637
# ---------------------------------------------------------------------------

FLEET_RELEASE = "fleet-e2e"
FLEET_NS = "cloacina-fleet-e2e"
FLEET_BOOTSTRAP_KEY = "fleet-e2e-bootstrap-key"
FLEET_AGENT_REPLICAS = 2
FLEET_FWD_PORT = 18090

SERVER_IMG = "cloacina-server:fleet-e2e"
AGENT_IMG = "cloacina-agent:fleet-e2e"
COMPILER_IMG = "cloacina-compiler:fleet-e2e"

# In-container path where the compiler image bakes the cloacina workspace; the
# uploaded package's `__WORKSPACE__` path-deps are rewritten to this so they
# resolve against the baked source. Must match docker/Dockerfile.compiler.
COMPILER_WORKSPACE = "/workspace"


def _compiler_manifest() -> str:
    db = f"postgres://cloacina:cloacina@{FLEET_RELEASE}-postgresql:5432/cloacina"
    return f"""
apiVersion: apps/v1
kind: Deployment
metadata:
  name: cloacina-compiler
  namespace: {FLEET_NS}
  labels:
    app: cloacina-compiler
spec:
  replicas: 1
  selector:
    matchLabels:
      app: cloacina-compiler
  template:
    metadata:
      labels:
        app: cloacina-compiler
    spec:
      containers:
        - name: compiler
          image: {COMPILER_IMG}
          imagePullPolicy: Never
          args:
            - "--bind"
            - "0.0.0.0:9000"
            - "--poll-interval-ms"
            - "500"
            - "--verbose"
            # Tell the compiler the shared target dir explicitly. The image
            # bakes ENV CARGO_TARGET_DIR=/workspace/target so the warm cache
            # (cloacina + ~100 deps from the image build) is reused — BUT the
            # compiler only knows where cargo *writes* the cdylib if it owns
            # the dir via this flag. Without it, config.cargo_target_dir is
            # None and post-build discovery looks in the package-local
            # `target/release` while cargo (inheriting the env) wrote to
            # /workspace/target/release → "expected lib...so in .../target/
            # release" (build.rs:411-418, 531-536). Flag == env keeps both
            # sides pointed at the same dir.
            - "--cargo-target-dir"
            - "/workspace/target"
            # `--cargo-flag=<v>` (equals form) — clap won't take a hyphen-led
            # value in the two-token form, so `--cargo-flag --release` is
            # parsed as a (bogus) --release flag. The `=` assigns verbatim.
            # This set REPLACES the default `build --release --lib --frozen
            # --offline` (main.rs:73), dropping the offline posture for the
            # in-cluster online build (kind pods have egress).
            - "--cargo-flag=build"
            - "--cargo-flag=--release"
            - "--cargo-flag=--lib"
          env:
            - name: DATABASE_URL
              value: "{db}"
          ports:
            - containerPort: 9000
          readinessProbe:
            httpGet:
              path: /health
              port: 9000
            initialDelaySeconds: 3
            periodSeconds: 3
"""


def _agent_manifest() -> str:
    server_url = f"http://{FLEET_RELEASE}-cloacina-server:8080"
    return f"""
apiVersion: apps/v1
kind: Deployment
metadata:
  name: cloacina-agent
  namespace: {FLEET_NS}
  labels:
    app: cloacina-agent
spec:
  replicas: {FLEET_AGENT_REPLICAS}
  selector:
    matchLabels:
      app: cloacina-agent
  template:
    metadata:
      labels:
        app: cloacina-agent
    spec:
      containers:
        - name: agent
          image: {AGENT_IMG}
          imagePullPolicy: Never
          args:
            - "--max-concurrency"
            - "2"
          env:
            - name: CLOACINA_SERVER
              value: "{server_url}"
            - name: CLOACINA_API_KEY
              valueFrom:
                secretKeyRef:
                  name: fleet-bootstrap
                  key: key
"""


def _kubectl_apply_stdin(manifest: str) -> None:
    print("$ kubectl apply -f - <<manifest", flush=True)
    subprocess.run(
        ["kubectl", "apply", "-f", "-"],
        input=manifest, text=True, check=True,
    )


def _ctl(home: Path, *args, check=True):
    """cloacinactl against the port-forwarded server."""
    cmd = ["target/debug/cloacinactl", "--home", str(home), *args]
    proc = subprocess.run(cmd, cwd=PROJECT_ROOT, capture_output=True, text=True)
    if check and proc.returncode != 0:
        raise AssertionError(
            f"{' '.join(cmd)} exited {proc.returncode}\n"
            f"STDOUT:\n{proc.stdout}\nSTDERR:\n{proc.stderr}"
        )
    return proc.returncode, proc.stdout, proc.stderr


def _stage_fleet_fixture(dest: Path, fixture: str = "compiler-happy-rust") -> Path:
    """Copy examples/fixtures/<fixture> and rewrite __WORKSPACE__ to the compiler
    image's baked workspace path so the in-cluster compiler resolves the cloacina
    path-deps."""
    import shutil as _sh
    src = PROJECT_ROOT / "examples" / "fixtures" / fixture
    out = dest / fixture
    if out.exists():
        _sh.rmtree(out)
    (out / "src").mkdir(parents=True)
    for rel in ("package.toml", "Cargo.toml", "build.rs", "src/lib.rs"):
        text = (src / rel).read_text().replace("__WORKSPACE__", COMPILER_WORKSPACE)
        (out / rel).write_text(text)
    return out


def _upload_and_compile(home: Path, fixture: str, timeout_s: float = 900.0) -> str:
    """Stage → pack → upload → poll build_status=success for a fixture. Returns
    the package id. Dumps build_error + compiler logs before raising on failure."""
    import json as _json
    staged = _stage_fleet_fixture(home, fixture)
    archive = home / f"{fixture}.cloacina"
    _ctl(home, "package", "pack", str(staged), "--out", str(archive))
    _, out, _ = _ctl(home, "package", "upload", str(archive))
    pkg_id = out.strip().splitlines()[-1].strip()
    assert len(pkg_id) >= 32, f"no package id from upload of {fixture}: {out!r}"
    deadline = time.time() + timeout_s
    last_body = {}
    while time.time() < deadline:
        _, out, _ = _ctl(home, "-o", "json", "package", "inspect", pkg_id, check=False)
        try:
            last_body = _json.loads(out)
            status = last_body.get("build_status")
        except Exception:
            status = None
        if status == "success":
            return pkg_id
        if status == "failed":
            print(f"\n===== BUILD FAILED ({fixture}) — package inspect =====", flush=True)
            print(_json.dumps(last_body, indent=2), flush=True)
            _dump_diag("cloacina-compiler", "app=cloacina-compiler")
            raise AssertionError(
                f"compiler build failed for {fixture} ({pkg_id}): "
                f"{last_body.get('build_error') or '(no build_error field)'}"
            )
        time.sleep(5)
    raise AssertionError(f"build of {fixture} never succeeded within {timeout_s}s")


def _agent_pods() -> list:
    """Names of LIVE cloacina-agent pods — Running and not Terminating. A
    scaled-down pod stays in the list (phase=Running + deletionTimestamp) for
    its grace period, so filter those out to avoid counting a dying pod."""
    import json as _json
    out = subprocess.run(
        ["kubectl", "get", "pods", "-l", "app=cloacina-agent", "-n", FLEET_NS, "-o", "json"],
        capture_output=True, text=True, check=True,
    ).stdout
    try:
        items = _json.loads(out).get("items", [])
    except Exception:
        return []
    live = []
    for p in items:
        meta, st = p.get("metadata", {}), p.get("status", {})
        if meta.get("deletionTimestamp"):
            continue  # terminating
        if st.get("phase") == "Running":
            live.append(meta.get("name"))
    return live


def _wait_agent_pods(n: int, timeout_s: float = 90.0) -> list:
    """Poll until exactly n live agent pods are present; returns their names."""
    deadline = time.time() + timeout_s
    pods = []
    while time.time() < deadline:
        pods = _agent_pods()
        if len(pods) == n:
            return pods
        time.sleep(3)
    raise AssertionError(f"expected {n} live agent pods within {timeout_s}s; last: {pods}")


def _server_logs() -> str:
    proc = subprocess.run(
        ["kubectl", "logs", f"deploy/{FLEET_RELEASE}-cloacina-server",
         "-n", FLEET_NS, "--tail=400"],
        capture_output=True, text=True,
    )
    return proc.stdout + proc.stderr


def _pod_logs(pod: str, tail: int = 200) -> str:
    proc = subprocess.run(
        ["kubectl", "logs", pod, "-n", FLEET_NS, f"--tail={tail}"],
        capture_output=True, text=True,
    )
    return proc.stdout + proc.stderr


def _dump_diag(deploy: str, label: str) -> None:
    """Dump deployment + pod state and logs for a failing component. Called
    before the finally-block teardown so a rollout timeout isn't a black box
    (the cluster is deleted on exit, taking the pod logs with it)."""
    print(f"\n===== DIAGNOSTICS: {deploy} (-l {label}) =====", flush=True)
    subprocess.run(["kubectl", "get", "pods", "-l", label, "-n", FLEET_NS, "-o", "wide"], check=False)
    subprocess.run(["kubectl", "describe", "deploy", deploy, "-n", FLEET_NS], check=False)
    subprocess.run(["kubectl", "describe", "pods", "-l", label, "-n", FLEET_NS], check=False)
    print("----- current logs -----", flush=True)
    subprocess.run(["kubectl", "logs", "-l", label, "-n", FLEET_NS,
                    "--tail=200", "--all-containers=true"], check=False)
    print("----- previous logs (if the container restarted) -----", flush=True)
    subprocess.run(["kubectl", "logs", "-l", label, "-n", FLEET_NS,
                    "--tail=200", "--all-containers=true", "--previous"], check=False)
    print(f"===== END DIAGNOSTICS: {deploy} =====\n", flush=True)


def _wait_rollout_or_dump(deploy: str, label: str, timeout: str = "5m") -> None:
    """_wait_rollout, but on timeout dump pod diagnostics before re-raising."""
    try:
        _wait_rollout(deploy, FLEET_NS, timeout=timeout)
    except subprocess.CalledProcessError:
        _dump_diag(deploy, label)
        raise


def _run_workflow(home: Path, workflow: str, timeout_s: float = 120.0,
                  context: str = None) -> str:
    """Trigger `workflow run` via cloacinactl, retrying until the reconciler
    has loaded the workflow into the registry. Returns the execution id.
    `context`, if given, is a path to a JSON context file (`--context`).

    The reconciler loads packages on a periodic tick, so the first runs may
    fail with 'Workflow not found in registry' until that lands.

    `workflow run` prints the BARE execution_id via `println!` (workflow/
    mod.rs:102) — a plain UUID line, NOT a JSON object, even under `-o json`.
    So parse JSON first (in case that ever changes), then fall back to the
    last non-empty output line. Mirrors the host e2e `_poll_run_workflow`."""
    import json as _json
    extra = ("--context", context) if context else ()
    deadline = time.time() + timeout_s
    last = ""
    while time.time() < deadline:
        code, out, err = _ctl(home, "-o", "json", "workflow", "run", workflow, *extra, check=False)
        if code == 0:
            exec_id = None
            try:
                exec_id = _json.loads(out).get("execution_id")
            except Exception:
                exec_id = None
            if not exec_id and out.strip():
                tail = out.strip().splitlines()[-1].strip()
                if len(tail) >= 32:
                    exec_id = tail
            if exec_id and len(exec_id) >= 32:
                return exec_id
        last = (err.strip() or out.strip())
        time.sleep(2)
    raise AssertionError(f"workflow run {workflow} never succeeded in {timeout_s}s; last: {last}")


def _exec_status(home: Path, exec_id: str):
    """One-shot `execution status` → the status string (or None if unparsable)."""
    import json as _json
    _, out, _ = _ctl(home, "-o", "json", "execution", "status", exec_id, check=False)
    try:
        return _json.loads(out).get("status")
    except Exception:
        return None


def _wait_exec(home: Path, exec_id: str, timeout_s: float = 120.0):
    """Poll `execution status` until terminal; returns the last status seen."""
    deadline = time.time() + timeout_s
    status = None
    while time.time() < deadline:
        status = _exec_status(home, exec_id)
        if status in ("Completed", "Failed", "Cancelled"):
            return status
        time.sleep(2)
    return status


@helm()
@angreal.command(
    name="fleet",
    about="end-to-end containerized fleet on kind (server + compiler + N agents, in-flight reclaim)",
    when_to_use=[
        "validating the execution-agent fleet in a real k8s topology",
        "confirming compiler->agent->reconcile closes in-cluster",
        "proving dead-agent in-flight reclaim (kill the executing agent)",
    ],
    when_not_to_use=["unit testing", "running without docker/kind"],
)
def helm_fleet():
    _check_tool("docker", "install Docker Desktop or colima")
    _check_tool("kind", "sudo port install kind  # or brew install kind")
    _check_tool("kubectl", "sudo port install kubectl")
    _check_tool("helm", "sudo port install kubernetes-helm")

    cluster = _kind_cluster_name()
    print(f"\n=== cloacina fleet e2e ({cluster}) ===\n")

    fwd = None
    home = Path(tempfile.mkdtemp(prefix="fleet-k8s-"))
    try:
        # 1. Build cloacinactl (host driver) + the three images.
        print("\n--- 1. Build cloacinactl + server/agent/compiler images ---\n")
        _run(["cargo", "build", "-p", "cloacinactl"], cwd=PROJECT_ROOT)
        _run(["docker", "build", "-t", SERVER_IMG, "-f", str(PROJECT_ROOT / "Dockerfile"), str(PROJECT_ROOT)])
        _run(["docker", "build", "-t", AGENT_IMG, "-f", str(PROJECT_ROOT / "docker" / "Dockerfile.agent"), str(PROJECT_ROOT)])
        _run(["docker", "build", "-t", COMPILER_IMG, "-f", str(PROJECT_ROOT / "docker" / "Dockerfile.compiler"), str(PROJECT_ROOT)])

        # 2. Cluster + load all three images.
        print("\n--- 2. kind cluster + load images ---\n")
        _run(["kind", "create", "cluster", "--name", cluster, "--wait", "120s"])
        for img in (SERVER_IMG, AGENT_IMG, COMPILER_IMG):
            _run(["kind", "load", "docker-image", img, "--name", cluster])

        # 3. Namespace + bootstrap-key secret (shared by server + agents + ctl).
        print("\n--- 3. namespace + bootstrap secret ---\n")
        _run(["kubectl", "create", "namespace", FLEET_NS])
        _run([
            "kubectl", "create", "secret", "generic", "fleet-bootstrap",
            "-n", FLEET_NS, f"--from-literal=key={FLEET_BOOTSTRAP_KEY}",
        ])

        # 4. Install the server chart: bundled postgres, fleet routing, known key.
        print("\n--- 4. helm install server (routes **=fleet) ---\n")
        _run(["helm", "dependency", "update", str(CHART_DIR)])
        values = home / "server-values.yaml"
        values.write_text(
            "image:\n"
            f"  repository: {SERVER_IMG.split(':')[0]}\n"
            f"  tag: {SERVER_IMG.split(':')[1]}\n"
            "  pullPolicy: Never\n"
            "postgresql:\n"
            "  enabled: true\n"
            "  auth:\n"
            "    username: cloacina\n"
            "    password: cloacina\n"
            "    database: cloacina\n"
            "apiKeySecretRef:\n"
            "  name: fleet-bootstrap\n"
            "  key: key\n"
            "server:\n"
            "  extraEnv:\n"
            "    - name: CLOACINA_FLEET_ROUTES\n"
            '      value: "**=fleet"\n'
            # Aggressive liveness (CLOACI-T-0639) so the reclaim step doesn't
            # wait the default 45-60s: dead-after = 5s x 2 = 10s. Also exercises
            # the new --agent-heartbeat-interval-s / --agent-liveness-misses
            # config end-to-end.
            "    - name: CLOACINA_AGENT_HEARTBEAT_INTERVAL_S\n"
            '      value: "5"\n'
            "    - name: CLOACINA_AGENT_LIVENESS_MISSES\n"
            '      value: "2"\n'
        )
        _run([
            "helm", "install", FLEET_RELEASE, str(CHART_DIR),
            "--namespace", FLEET_NS,
            "-f", str(values),
            "--wait", "--timeout=8m",
        ])
        _wait_rollout(f"{FLEET_RELEASE}-cloacina-server", FLEET_NS, timeout="5m")

        # 5. Compiler + agents (after server migrated the DB + is ready).
        print("\n--- 5. deploy compiler + agents ---\n")
        _kubectl_apply_stdin(_compiler_manifest())
        _wait_rollout_or_dump("cloacina-compiler", "app=cloacina-compiler", timeout="5m")
        _kubectl_apply_stdin(_agent_manifest())
        _wait_rollout_or_dump("cloacina-agent", "app=cloacina-agent", timeout="3m")

        # 6. Port-forward the server so host cloacinactl can drive it.
        print("\n--- 6. port-forward + configure cloacinactl ---\n")
        fwd = subprocess.Popen(
            ["kubectl", "port-forward", f"svc/{FLEET_RELEASE}-cloacina-server",
             f"{FLEET_FWD_PORT}:8080", "-n", FLEET_NS],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        time.sleep(4)
        base = f"http://127.0.0.1:{FLEET_FWD_PORT}"
        _ctl(home, "config", "profile", "set", "local", base,
             "--api-key", FLEET_BOOTSTRAP_KEY, "--default")

        # 7. Upload + compile both fixtures (rewritten to the compiler's baked
        # workspace path). The cold build pulls ~100 deps; warm cache → ~15s.
        print("\n--- 7. upload + compile workflows (cold build can take minutes) ---\n")
        _upload_and_compile(home, "compiler-happy-rust")
        print("  ok: compiler-happy-rust compiled in-cluster")
        _upload_and_compile(home, "fleet-slow-rust")
        print("  ok: fleet-slow-rust compiled in-cluster")

        # 8. Happy path — workflow runs on an agent and reconciles (multi-agent:
        # FLEET_AGENT_REPLICAS agents are up).
        print("\n--- 8. run workflow on the fleet (happy path) ---\n")
        exec_id = _run_workflow(home, "compiler_happy_workflow")
        print(f"  triggered execution: {exec_id}")
        status = _wait_exec(home, exec_id)
        if status != "Completed":
            print(_server_logs())
            raise AssertionError(f"fleet execution ended {status!r}, expected Completed")
        if "agent reported result" not in _server_logs():
            print(_server_logs())
            raise AssertionError("no 'agent reported result' in server log — task did not run on the fleet")
        print("  ok: workflow executed on an agent + reconciled (Completed)")

        # 9. True in-flight reclaim (CLOACI-T-0638). Run a long task on the fleet,
        # find which agent ACTUALLY executes it, kill that pod, and assert the
        # server reclaims the orphaned in-flight work onto a surviving agent
        # where it completes.
        #
        # Both agents stay live (no scale-down): scaling down races the server's
        # agent registry, which lags k8s pod termination by the heartbeat
        # timeout, so the fleet executor can still dispatch to the dying pod. We
        # identify the executor instead — the agent logs the slow package's
        # cdylib registration when it dlopens it to run — and the OTHER agent is
        # a genuinely-live survivor for the reclaim to land on.
        print("\n--- 9. in-flight reclaim: kill the executing agent ---\n")
        # With the aggressive liveness config above (dead-after ~10s), detection
        # is fast, so the re-run dominates; 45s leaves comfortable margin between
        # the kill (~10-15s in) and the agent finishing on its own.
        SLEEP_SECONDS = 45
        agents = _wait_agent_pods(FLEET_AGENT_REPLICAS)
        assert len(agents) >= 2, f"need >=2 live agents for reclaim, got {agents}"
        print(f"  live agents: {agents}")

        # Pass the sleep duration via --context (a JSON file; inline isn't supported).
        ctx_file = home / "slow-context.json"
        ctx_file.write_text(f'{{"sleep_seconds": {SLEEP_SECONDS}}}')
        exec_id = _run_workflow(home, "fleet_slow_workflow", context=str(ctx_file))
        print(f"  triggered slow execution: {exec_id}")

        # Identify the executor: the pod whose log shows it loaded fleet-slow-rust
        # (only the agent that claimed the work receives + dlopens the cdylib).
        # A fleet workflow execution stays "Pending" while running, so the pod log
        # is the reliable "this agent is executing it" signal.
        deadline = time.time() + 90
        executor = None
        while time.time() < deadline and not executor:
            st = _exec_status(home, exec_id)
            if st in ("Completed", "Failed", "Cancelled"):
                _, body, _ = _ctl(home, "-o", "json", "execution", "status", exec_id, check=False)
                print(f"\n===== SLOW TASK {st} BEFORE KILL — execution status =====", flush=True)
                print(body, flush=True)
                _dump_diag("cloacina-agent", "app=cloacina-agent")
                print(_server_logs(), flush=True)
                raise AssertionError(
                    f"slow task reached {st} before the executing agent could be killed "
                    "(expected it to still be running) — see diagnostics above"
                )
            for pod in _agent_pods():
                if "fleet-slow-rust" in _pod_logs(pod):
                    executor = pod
                    break
            if not executor:
                time.sleep(2)
        if not executor:
            _dump_diag("cloacina-agent", "app=cloacina-agent")
            print(_server_logs(), flush=True)
            raise AssertionError(
                "no agent picked up the slow task within 90s (no 'fleet-slow-rust' in any "
                "agent log) — dispatch may be stuck; see diagnostics above"
            )
        survivors = [p for p in _agent_pods() if p != executor]
        assert survivors, f"no surviving agent to reclaim onto (agents={_agent_pods()})"
        print(f"  slow task is executing on {executor}; survivor(s): {survivors}")

        # Regression guard for the observability fix: the fleet executor marks the
        # workflow execution Running on dispatch (it used to read Pending the whole
        # run). It's executing now, so status must be Running.
        deadline = time.time() + 10
        while time.time() < deadline and _exec_status(home, exec_id) != "Running":
            time.sleep(1)
        st = _exec_status(home, exec_id)
        assert st == "Running", (
            f"expected execution status Running while the agent runs it, got {st!r} "
            "— fleet 'mark execution Running on dispatch' may have regressed"
        )
        print("  execution status is Running while in flight (observability fix OK)")

        # Kill the executing agent. k8s will spin up a replacement, but the
        # already-live survivor is what the reclaim targets.
        kill_t = time.time()
        _run(["kubectl", "delete", "pod", executor, "-n", FLEET_NS, "--wait=false"])
        print(f"  killed executing agent {executor}; waiting for reclaim "
              f"(~detect 10-20s + {SLEEP_SECONDS}s re-run)")

        # Reclaim: the agent sweeper marks the killed agent dead (~10-20s with
        # the aggressive 5s×2 liveness config + the pod's SIGTERM grace), then
        # reassigns its
        # in-flight outbox row to a live agent which re-runs the slow task from
        # scratch and reports → the original rendezvous wakes → Completed.
        status = _wait_exec(home, exec_id, timeout_s=300.0)
        reclaim_secs = time.time() - kill_t
        logs = _server_logs()
        if status != "Completed":
            print(logs)
            raise AssertionError(f"post-reclaim execution ended {status!r}, expected Completed")
        if "reclaimed dead agent's in-flight work" not in logs:
            print(logs)
            raise AssertionError(
                "execution completed but server never logged the reclaim — the task may "
                "have completed on the executor before the kill (timing), not via reclaim"
            )
        # The slow path is the fleet rendezvous timing out at 300s then retrying.
        # We completed under 300s (or _wait_exec would have failed), but surface
        # it explicitly so a regression toward the timeout fallback is visible.
        if "agent result wait exceeded server-side timeout" in logs:
            print("  WARNING: a fleet rendezvous hit the 300s timeout fallback — "
                  "reclaim re-push may not be reaching the rendezvous promptly; investigate")
        print(f"  ok: in-flight work reclaimed onto a survivor — execution Completed "
              f"in {reclaim_secs:.0f}s after kill "
              f"(~detect + {SLEEP_SECONDS}s re-run; well under the 300s rendezvous ceiling)")

        print("\n✓ containerized fleet e2e passed (kind): happy path + in-flight reclaim")

    finally:
        if fwd is not None and fwd.poll() is None:
            fwd.terminate()
        print(f"\n--- cleanup: kind delete cluster {cluster} ---\n")
        _run(["kind", "delete", "cluster", "--name", cluster], check=False)
