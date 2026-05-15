"""
Helm chart testing for charts/cloacina-server.

CLOACI-I-0111 / T-0605.

Two surfaces:
  - `angreal helm lint`       — fast: helm lint + helm template variants.
                                Mirrors the .github/workflows/ci.yml job.
  - `angreal helm test`       — slow: builds the cloacina-server image,
                                spins up a kind cluster, loads the image,
                                helm-installs the chart with bundled
                                Postgres, port-forwards, curls /v1/health,
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
    """Run a curl pod against the service and check /v1/health → 200."""
    cmd = [
        "kubectl", "run", f"healthcheck-{uuid.uuid4().hex[:4]}",
        "-n", namespace,
        "--rm", "--restart=Never", "-i",
        "--image=curlimages/curl:8.10.1",
        "--",
        "curl", "-fsSL", "-o", "/dev/null", "-w", "%{http_code}",
        f"http://{service}:8080/v1/health",
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
    about="end-to-end helm chart install on a kind cluster + /v1/health curl",
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
        print("\n--- 6. Verify pod ready + /v1/health ---\n")
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
            print("\n✗ /v1/health did not return 200", file=sys.stderr)
            sys.exit(1)

        print("\n✓ chart installed, pod ready, /v1/health = 200")

    finally:
        # Always tear the cluster down — leftovers eat host resources fast.
        print(f"\n--- cleanup: kind delete cluster {cluster} ---\n")
        _run(["kind", "delete", "cluster", "--name", cluster], check=False)
        _ = kubeconfig  # keep linter quiet
