"""End-to-end test of the compiler service pipeline.

Spins Postgres, server, and compiler as separate subprocesses sharing one
database, then drives the full flow through cloacinactl. Asserts on
DB-observable state (build_status, build_error) and the CLI's own output.

Coverage in v1:
  - Happy path: upload fixture → compiler builds → build_status = success
  - Failed-build: upload broken fixture → build_status = failed, build_error
    non-empty
  - Composite status: daemon + server + compiler side by side

Deferred (tracked as follow-ups, not in this harness):
  - Reconciler end-to-end (upload → success → workflow run → execution
    completes). Needs fixtures that link against cloacina crates, which
    wants a host-path rewrite in the compiler OR published crates from
    T-0501. Compiler-side mechanics above prove the queue + heartbeat +
    artifact-persist contract independently.
  - Stale-heartbeat recovery and content-hash artifact reuse — both are
    DB-observable and worth adding once someone touches this harness next.
"""

import json
import os
import signal
import subprocess
import time
import urllib.request
from pathlib import Path

import angreal  # type: ignore

from .cloacina_utils import print_final_success, print_section_header

cloacina = angreal.command_group(
    name="cloacina", about="commands for Cloacina core engine tests"
)

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURES = REPO_ROOT / "examples" / "fixtures"


def _build_binaries():
    print("Building cloacina-server + cloacina-compiler + cloacinactl (debug)...")
    for pkg in ("cloacina-server", "cloacina-compiler", "cloacinactl"):
        subprocess.run(["cargo", "build", "-p", pkg], cwd=REPO_ROOT, check=True)


def _start_postgres():
    subprocess.run(
        ["docker", "compose", "-f", ".angreal/docker-compose.yaml", "up", "-d", "postgres"],
        cwd=REPO_ROOT,
        check=True,
    )
    for _ in range(30):
        r = subprocess.run(
            [
                "docker", "compose", "-f", ".angreal/docker-compose.yaml",
                "exec", "-T", "postgres", "pg_isready", "-U", "cloacina",
            ],
            cwd=REPO_ROOT,
            capture_output=True,
        )
        if r.returncode == 0:
            return
        time.sleep(1)
    raise RuntimeError("Postgres not ready after 30s")


def _wait_http(url: str, label: str, timeout_s: float = 30.0):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=1.0):
                return
        except Exception:
            time.sleep(0.5)
    raise RuntimeError(f"{label} at {url} never came up within {timeout_s}s")


def _cloacinactl(home: Path, *args, check=True, env=None):
    """Run `cloacinactl` and return (exitcode, stdout, stderr)."""
    cmd = ["target/debug/cloacinactl", "--home", str(home), *args]
    proc = subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        env=env or os.environ.copy(),
    )
    if check and proc.returncode != 0:
        raise AssertionError(
            f"{' '.join(cmd)} exited {proc.returncode}\n"
            f"STDOUT:\n{proc.stdout}\nSTDERR:\n{proc.stderr}"
        )
    return proc.returncode, proc.stdout, proc.stderr


def _upload_fixture(home: Path, fixture_dir: Path) -> str:
    """Pack + upload a fixture. Returns the package UUID printed by upload.

    We use pack + upload (not publish) so local `cargo build` doesn't preempt
    the compiler — we want the compiler service to be the thing that builds.
    """
    archive = home / f"{fixture_dir.name}.cloacina"
    _cloacinactl(home, "package", "pack", str(fixture_dir), "--out", str(archive))
    _, out, _ = _cloacinactl(home, "package", "upload", str(archive))
    pkg_id = out.strip().splitlines()[-1].strip()
    if not pkg_id or len(pkg_id) < 32:
        raise AssertionError(f"upload didn't print a package id; got: {out!r}")
    return pkg_id


def _poll_build_status(
    home: Path,
    pkg_id: str,
    expected: set[str],
    timeout_s: float = 120.0,
) -> dict:
    """Poll `package inspect -o json` until build_status is in `expected`.

    Returns the final parsed JSON body. Raises with a diagnostic if the
    timeout elapses — include the last seen status and error to make CI
    failures debuggable.
    """
    deadline = time.time() + timeout_s
    last_body: dict = {}
    while time.time() < deadline:
        _, out, _ = _cloacinactl(home, "-o", "json", "package", "inspect", pkg_id)
        try:
            last_body = json.loads(out)
        except json.JSONDecodeError:
            time.sleep(1.0)
            continue
        status = last_body.get("build_status")
        if status in expected:
            return last_body
        time.sleep(1.0)
    raise AssertionError(
        f"build_status for {pkg_id} never reached {expected} within {timeout_s}s; "
        f"last body: {json.dumps(last_body, indent=2)}"
    )


def _kill(proc: subprocess.Popen | None):
    if proc is None or proc.poll() is not None:
        return
    proc.send_signal(signal.SIGTERM)
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()


@cloacina()
@angreal.command(
    name="compiler-e2e",
    about="run end-to-end cloacina-compiler integration tests (T-0527)",
)
def compiler_e2e():
    print_section_header("cloacina-compiler e2e")
    _build_binaries()
    _start_postgres()

    db_url = "postgres://cloacina:cloacina@localhost:5432/cloacina"
    bootstrap_key = "test-bootstrap-compiler-e2e"
    server_bind = "127.0.0.1:18083"
    compiler_bind = "127.0.0.1:19003"
    server_url = f"http://{server_bind}"
    compiler_url = f"http://{compiler_bind}"

    import tempfile

    server_proc: subprocess.Popen | None = None
    compiler_proc: subprocess.Popen | None = None

    with tempfile.TemporaryDirectory(prefix="compiler-e2e-") as home_s:
        home = Path(home_s)
        try:
            # --- start server ---
            server_proc = subprocess.Popen(
                [
                    "target/debug/cloacina-server",
                    "--home", str(home),
                    "--database-url", db_url,
                    "--bind", server_bind,
                    "--bootstrap-key", bootstrap_key,
                ],
                cwd=REPO_ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            _wait_http(f"{server_url}/health", "server")
            print("  ok: server up")

            # --- start compiler ---
            compiler_proc = subprocess.Popen(
                [
                    "target/debug/cloacina-compiler",
                    "--home", str(home),
                    "--database-url", db_url,
                    "--bind", compiler_bind,
                    "--poll-interval-ms", "500",
                ],
                cwd=REPO_ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            _wait_http(f"{compiler_url}/health", "compiler")
            print("  ok: compiler up")

            # --- configure CLI profile ---
            _cloacinactl(
                home,
                "config", "profile", "set", "local", server_url,
                "--api-key", bootstrap_key,
                "--default",
            )
            _cloacinactl(
                home, "config", "set", "compiler.local_addr", compiler_bind,
            )

            # --- composite status: all three reachable, exit 0 ---
            code, out, _ = _cloacinactl(home, "status")
            assert code == 0, f"composite status failed: {out!r}"
            assert "server" in out and "compiler" in out, out
            print("  ok: composite status covers server + compiler")

            # --- happy path ---
            happy_id = _upload_fixture(home, FIXTURES / "compiler-happy-rust")
            print(f"  uploaded happy fixture: {happy_id}")
            body = _poll_build_status(
                home, happy_id, {"success"}, timeout_s=180.0
            )
            assert body.get("build_status") == "success", body
            assert body.get("build_error") in (None, "", "null"), body
            print("  ok: happy path → build_status = success")

            # --- failed-build path ---
            broken_id = _upload_fixture(home, FIXTURES / "compiler-broken-rust")
            print(f"  uploaded broken fixture: {broken_id}")
            body = _poll_build_status(
                home, broken_id, {"failed"}, timeout_s=180.0
            )
            assert body.get("build_status") == "failed", body
            err = body.get("build_error") or ""
            assert err, f"expected non-empty build_error, got: {body!r}"
            print(
                f"  ok: failed-build path → build_status = failed "
                f"({len(err)}-byte build_error captured)"
            )

            print_final_success("cloacina-compiler e2e")
        finally:
            _kill(compiler_proc)
            _kill(server_proc)
