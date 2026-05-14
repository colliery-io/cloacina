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
import urllib.error
import urllib.request
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_section_header, print_final_success

test = angreal.command_group(name="test", about="Cloacina test suites (unit, integration, e2e, soak)")
e2e = angreal.command_group(name="e2e", about="end-to-end tests against a live server")


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


def _wait_for_health(base_url: str, timeout_s: float = 30.0, server_proc=None):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if server_proc is not None and server_proc.poll() is not None:
            raise RuntimeError(
                f"server exited {server_proc.returncode} while waiting for /health"
            )
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


@test()
@e2e()
@angreal.command(
    name="cli",
    about="end-to-end cloacinactl integration tests (T-0518)",
    when_to_use=[
        "validating cloacinactl against a live server",
        "pre-release CLI regression check",
    ],
    when_not_to_use=["unit testing", "running without docker"],
)
def cli():
    print_section_header("cloacinactl e2e")
    _build_binaries()
    _start_postgres()

    db_url = "postgres://cloacina:cloacina@localhost:5432/cloacina"
    bootstrap_key = "test-bootstrap-e2e"
    bind = "127.0.0.1:18082"
    base_url = f"http://{bind}"

    with tempfile.TemporaryDirectory() as home_s:
        home = Path(home_s)

        # Drain stderr to a file so the buffer doesn't fill (causing the
        # server to block on writes and the /health probe to flap).
        # Captured for the failure diagnostic if the test fails.
        server_stderr = open(home / "server-stderr.log", "wb")
        server = subprocess.Popen(
            [
                "target/debug/cloacina-server",
                "--home", str(home),
                "--database-url", db_url,
                "--bind", bind,
                "--bootstrap-key", bootstrap_key,
            ],
            stdout=subprocess.DEVNULL,
            stderr=server_stderr,
        )

        def _dump_server_stderr():
            server_stderr.flush()
            try:
                with open(home / "server-stderr.log", "r") as f:
                    tail = f.read()[-4096:]
                print(f"--- server stderr tail ---\n{tail}\n--- end ---")
            except Exception as e:
                print(f"(failed to read server-stderr.log: {e})")

        try:
            # Catch a server that exits during startup before we even probe.
            if server.poll() is not None:
                _dump_server_stderr()
                raise RuntimeError(f"server exited {server.returncode} before health probe")
            _wait_for_health(base_url, server_proc=server)

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

            # ─────────────────────────────────────────────────────────
            # CLOACI-I-0107 regression coverage. Every test below pins
            # one or more of the seven API-XX findings from the May
            # 2026 review. Failure traces back to T-0594/T-0595/T-0596.
            # ─────────────────────────────────────────────────────────

            # --- API-01: `tenant create` happy path ---
            tenant_name = f"e2e_tenant_{int(time.time())}"
            code, out, _ = _cloacinactl(
                home,
                "-o", "json",
                "tenant", "create", tenant_name,
                "--description", "e2e fixture",
                "--password", "e2e-test-pass",
            )
            parsed = json.loads(out)
            assert parsed.get("name") == tenant_name, (
                f"API-01: tenant create response should echo `name`, got {parsed!r}"
            )
            print("  ok: API-01 tenant create round-trips")

            # --- API-03: `tenant list` returns items envelope (rendered via render::list) ---
            code, out, _ = _cloacinactl(home, "-o", "json", "tenant", "list")
            parsed = json.loads(out)
            assert isinstance(parsed, list), (
                f"API-03: tenant list JSON should render the items array, got {parsed!r}"
            )
            assert any(t.get("name") == tenant_name for t in parsed), (
                f"API-03: newly created tenant {tenant_name} missing from list"
            )
            print("  ok: API-03 tenant list renders items envelope")

            # --- API-06: 4xx response has `code` + `message` (canonical ApiError) ---
            # Hit a known-bad endpoint via raw urllib so we see the body shape.
            req = urllib.request.Request(
                f"{base_url}/v1/tenants",
                method="POST",
                headers={
                    "Authorization": f"Bearer {bootstrap_key}",
                    "Content-Type": "application/json",
                },
                data=b"{}",  # missing required `name` field
            )
            try:
                urllib.request.urlopen(req)
                raise AssertionError("expected POST /v1/tenants with empty body to 4xx")
            except urllib.error.HTTPError as e:
                body = json.loads(e.read())
                assert "code" in body, f"API-06: error envelope missing `code`: {body!r}"
                assert "message" in body, f"API-06: error envelope missing `message`: {body!r}"
            print("  ok: API-06 error envelope has `code` + `message`")

            # --- API-08: /health (under no nest) returns x-request-id header.
            # The middleware is global so health probes get tagged too.
            req = urllib.request.Request(f"{base_url}/health")
            with urllib.request.urlopen(req) as resp:
                xrid = resp.headers.get("x-request-id")
                assert xrid is not None and len(xrid) > 0, (
                    "API-08: /health response missing x-request-id"
                )
            print("  ok: API-08 health response carries x-request-id")

            # --- API-05: `package pack --sign` exits non-zero ---
            with tempfile.TemporaryDirectory() as fakepkg_s:
                fakepkg = Path(fakepkg_s)
                (fakepkg / "package.toml").write_text("[package]\nname = 'x'\nversion = '0.1.0'\n")
                fake_key = fakepkg / "fake.key"
                fake_key.write_text("not-a-real-key")
                code, _, stderr = _cloacinactl(
                    home,
                    "package", "pack",
                    str(fakepkg),
                    "--sign", str(fake_key),
                    check=False,
                )
                assert code != 0, "API-05: --sign must fail-hard"
                assert "not yet implemented" in stderr.lower(), (
                    f"API-05: error should mention 'not yet implemented', got: {stderr!r}"
                )
            print("  ok: API-05 --sign fails hard with clear message")

            # --- API-17: `execution events --follow` exits non-zero ---
            code, _, stderr = _cloacinactl(
                home,
                "execution", "events",
                "00000000-0000-0000-0000-000000000000",
                "--follow",
                check=False,
            )
            assert code != 0, "API-17: --follow must fail-hard"
            assert "not yet implemented" in stderr.lower(), (
                f"API-17: error should mention 'not yet implemented', got: {stderr!r}"
            )
            print("  ok: API-17 --follow fails hard with clear message")

            # --- API-02: `execution list --status` filter actually takes effect ---
            # No executions exist yet for this tenant, so any filter returns []
            # but the request must not 4xx (proving the route accepts the query).
            code, out, _ = _cloacinactl(
                home,
                "-o", "json",
                "--tenant", tenant_name,
                "execution", "list",
                "--status", "Failed",
            )
            parsed = json.loads(out)
            assert isinstance(parsed, list), (
                f"API-02: execution list with filter should render array, got {parsed!r}"
            )
            print("  ok: API-02 execution list --status accepted")

            # --- API-10: `trigger list --limit` round-trip ---
            code, out, _ = _cloacinactl(
                home,
                "-o", "json",
                "--tenant", tenant_name,
                "trigger", "list",
                "--limit", "5",
                "--offset", "0",
            )
            parsed = json.loads(out)
            assert isinstance(parsed, list), (
                f"API-10: trigger list with pagination should render array, got {parsed!r}"
            )
            print("  ok: API-10 trigger list --limit/--offset accepted")

            print_final_success("cloacinactl e2e")
        except Exception:
            _dump_server_stderr()
            raise
        finally:
            if server.poll() is None:
                server.send_signal(signal.SIGTERM)
                try:
                    server.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    server.kill()
            server_stderr.close()
