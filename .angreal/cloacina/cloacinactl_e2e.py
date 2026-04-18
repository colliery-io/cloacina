"""End-to-end CLI tests for cloacinactl.

Builds cloacina-server + cloacinactl, spins the server up against Postgres,
and drives it exclusively through the `cloacinactl` subprocess. Asserts on
stdout / stderr / exit-code so the ADR-0003 contract is regression-tested.

Coverage in v1 is deliberately narrow — one happy path per noun, plus the
three key error-path checks (unreachable server, invalid key, not-found).
Broader matrix coverage is follow-up work once the server-side routes under
test are stable.
"""

import json
import signal
import subprocess
import tempfile
import time
import urllib.request
from pathlib import Path

import angreal  # type: ignore

from .cloacina_utils import print_section_header, print_final_success

cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")


def _build_binaries():
    print("Building cloacina-server + cloacinactl (debug)...")
    subprocess.run(["cargo", "build", "-p", "cloacina-server"], check=True)
    subprocess.run(["cargo", "build", "-p", "cloacinactl"], check=True)


def _start_postgres():
    subprocess.run(
        ["docker", "compose", "-f", ".angreal/docker-compose.yaml", "up", "-d"],
        check=True,
    )
    for _ in range(30):
        r = subprocess.run(
            [
                "docker", "compose", "-f", ".angreal/docker-compose.yaml",
                "exec", "-T", "postgres", "pg_isready", "-U", "cloacina",
            ],
            capture_output=True,
        )
        if r.returncode == 0:
            return
        time.sleep(1)
    raise RuntimeError("Postgres not ready")


def _wait_for_health(base_url: str, timeout_s: float = 30.0):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(f"{base_url}/health", timeout=1.0):
                return
        except Exception:
            time.sleep(0.5)
    raise RuntimeError(f"server at {base_url} never came up")


def _cloacinactl(home: Path, *args, env=None, check=True):
    """Run `cloacinactl` as a subprocess and return (exitcode, stdout, stderr)."""
    cmd = ["target/debug/cloacinactl", "--home", str(home), *args]
    proc = subprocess.run(cmd, capture_output=True, text=True, env=env)
    if check and proc.returncode != 0:
        raise AssertionError(
            f"{' '.join(cmd)} exited {proc.returncode}\nSTDOUT:\n{proc.stdout}\nSTDERR:\n{proc.stderr}"
        )
    return proc.returncode, proc.stdout, proc.stderr


@cloacina.command(name="cli-e2e", about="run end-to-end cloacinactl integration tests (T-0518)")
def cli_e2e():
    print_section_header("cloacinactl e2e")
    _build_binaries()
    _start_postgres()

    db_url = "postgres://cloacina:cloacina@localhost:5432/cloacina"
    bootstrap_key = "test-bootstrap-e2e"
    bind = "127.0.0.1:18082"
    base_url = f"http://{bind}"

    with tempfile.TemporaryDirectory() as home_s:
        home = Path(home_s)

        server = subprocess.Popen(
            [
                "target/debug/cloacina-server",
                "--home", str(home),
                "--database-url", db_url,
                "--bind", bind,
                "--bootstrap-key", bootstrap_key,
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        try:
            _wait_for_health(base_url)

            # --- server health via cloacinactl ---
            code, out, _ = _cloacinactl(home, "--server", base_url, "server", "health")
            assert code == 0 and out.strip() == "up", f"server health mismatch: {out!r}"
            print("  ok: server health")

            # --- profile: set + use, then drop --server everywhere ---
            _cloacinactl(
                home,
                "config", "profile", "set", "local", base_url,
                "--api-key", bootstrap_key,
                "--default",
            )
            code, out, _ = _cloacinactl(home, "server", "health")
            assert code == 0, f"profile-resolved server health failed: {out!r}"
            print("  ok: profile-resolved server health")

            # --- error: unreachable server → exit 2 ---
            code, _, _ = _cloacinactl(
                home,
                "--server", "http://127.0.0.1:59999",
                "server", "health",
                check=False,
            )
            assert code == 2, f"expected exit 2 for unreachable server, got {code}"
            print("  ok: exit 2 on unreachable server")

            # --- invalid key → exit 4 on an authed call ---
            code, _, stderr = _cloacinactl(
                home,
                "--api-key", "not-a-real-key",
                "package", "list",
                check=False,
            )
            assert code == 4, f"expected exit 4 for bad key, got {code} (stderr={stderr!r})"
            print("  ok: exit 4 on invalid api key")

            # --- not-found → exit 3 ---
            code, _, _ = _cloacinactl(
                home,
                "package", "inspect", "00000000-0000-0000-0000-000000000000",
                check=False,
            )
            assert code == 3, f"expected exit 3 for missing package, got {code}"
            print("  ok: exit 3 on not-found")

            # --- package list (empty, table format) ---
            code, out, _ = _cloacinactl(home, "package", "list")
            assert code == 0 and ("No items" in out or out.strip() == ""), out
            print("  ok: package list empty")

            # --- package list JSON ---
            code, out, _ = _cloacinactl(home, "-o", "json", "package", "list")
            parsed = json.loads(out)
            assert isinstance(parsed, list)
            print("  ok: package list -o json parses")

            print_final_success("cloacinactl e2e")
        finally:
            if server.poll() is None:
                server.send_signal(signal.SIGTERM)
                try:
                    server.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    server.kill()
