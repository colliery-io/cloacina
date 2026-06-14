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
    print("Building cloacina-server + cloacinactl + cloacina-agent (debug)...")
    subprocess.run(["cargo", "build", "-p", "cloacina-server"], check=True)
    subprocess.run(["cargo", "build", "-p", "cloacinactl"], check=True)
    # CLOACI-T-0634: the fleet e2e scenario needs the agent binary.
    subprocess.run(["cargo", "build", "-p", "cloacina-agent"], check=True)


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


def _psql(sql: str) -> str:
    """Run a SQL statement against the e2e Postgres via docker compose exec and
    return its tuples-only / unaligned stdout. Used by substrate contract tests
    (CLOACI-T-0629) to assert on `delivery_outbox` row state and to inject test
    rows without going through the runner."""
    r = subprocess.run(
        [
            "docker", "compose", "-f", ".angreal/docker-compose.yaml",
            "exec", "-T", "postgres",
            "psql", "-U", "cloacina", "-d", "cloacina",
            "-t", "-A",
            "-c", sql,
        ],
        capture_output=True, text=True, check=True,
    )
    return r.stdout


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

            # Diagnostic: explicit /health probe so we see the actual
            # status + headers if the next CLI call fails to reach the
            # same endpoint. Mismatch here means the seam (CLI vs the
            # raw HTTP) is the bug.
            with urllib.request.urlopen(f"{base_url}/health", timeout=2.0) as resp:
                print(f"  diag: /health → {resp.status}, headers={dict(resp.headers)}")

            # --- server health via cloacinactl ---
            # ClientContext::resolve requires --api-key even though health
            # doesn't use it (the handler's `Err(_) => eprintln!("down")`
            # arm fires before the actual HTTP probe otherwise).
            code, out, stderr = _cloacinactl(
                home,
                "--server", base_url,
                "--api-key", bootstrap_key,
                "server", "health",
                check=False,
            )
            if code != 0 or out.strip() != "up":
                raise AssertionError(
                    f"server health mismatch: code={code} out={out!r} stderr={stderr!r}"
                )
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
            # Provide --api-key so we test actual network failure, not
            # ClientContext resolution failure.
            code, _, _ = _cloacinactl(
                home,
                "--server", "http://127.0.0.1:59999",
                "--api-key", bootstrap_key,
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
            code, out, stderr = _cloacinactl(
                home,
                "-o", "json",
                "tenant", "create", tenant_name,
                "--description", "e2e fixture",
                "--password", "e2e-test-pass",
                check=False,
            )
            if code != 0 or not out.strip():
                print(f"!! API-01 tenant create exit={code}")
                print(f"!! stdout={out!r}")
                print(f"!! stderr={stderr!r}")
                raise AssertionError("API-01: tenant create unexpected (see prints above)")
            try:
                parsed = json.loads(out)
            except json.JSONDecodeError as e:
                print(f"!! API-01 tenant create stdout not JSON: {e}")
                print(f"!! stdout={out!r}")
                print(f"!! stderr={stderr!r}")
                raise AssertionError("API-01: tenant create stdout not JSON (see prints above)")
            assert parsed.get("name") == tenant_name, (
                f"API-01: tenant create response should echo `name`, got {parsed!r}"
            )
            print("  ok: API-01 tenant create round-trips")

            # --- API-03: `tenant list` returns items envelope (rendered via render::list) ---
            code, out, stderr = _cloacinactl(
                home, "-o", "json", "tenant", "list", check=False,
            )
            if code != 0 or not out.strip():
                raise AssertionError(
                    f"API-03: tenant list unexpected\n"
                    f"  exit={code}\n  stdout={out!r}\n  stderr={stderr!r}"
                )
            try:
                parsed = json.loads(out)
            except json.JSONDecodeError as e:
                raise AssertionError(
                    f"API-03: tenant list stdout not JSON: {e}\n"
                    f"  stdout={out!r}\n  stderr={stderr!r}"
                ) from e
            assert isinstance(parsed, list), (
                f"API-03: tenant list JSON should render the items array, got {parsed!r}"
            )
            assert any(t.get("name") == tenant_name for t in parsed), (
                f"API-03: newly created tenant {tenant_name} missing from list"
            )
            print("  ok: API-03 tenant list renders items envelope")

            # --- API-06: 4xx response has `code` + `message` (canonical ApiError) ---
            # Hit an authenticated endpoint with a missing/bad auth header
            # so we exercise an actual handler-emitted ApiError. Note:
            # axum's JSON extractor rejects malformed bodies BEFORE the
            # handler runs and emits plain-text 422; that path is not
            # ApiError-shaped today and is a documented follow-up
            # (custom Json extractor wrapping ApiError).
            req = urllib.request.Request(
                f"{base_url}/v1/tenants",
                method="GET",
                headers={"Authorization": "Bearer not-a-real-key"},
            )
            try:
                urllib.request.urlopen(req)
                raise AssertionError("expected unauth GET /v1/tenants to 401")
            except urllib.error.HTTPError as e:
                raw = e.read()
                try:
                    body = json.loads(raw)
                except json.JSONDecodeError as je:
                    print(f"!! API-06 4xx body not JSON: {je}")
                    print(f"!! status={e.code}")
                    print(f"!! body={raw!r}")
                    raise AssertionError("API-06: 4xx body not JSON (see prints above)")
                assert "code" in body, f"API-06: envelope missing `code`: {body!r}"
                assert "error" in body, f"API-06: envelope missing `error`: {body!r}"
            print("  ok: API-06 envelope has `code` + `error`")

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

            # NOTE: the original API-17 scenario asserted that
            # `execution events --follow` exited non-zero with
            # "not yet implemented" in stderr. That fail-hard behavior was
            # intentionally replaced in CLOACI-T-0629 — `--follow` now
            # subscribes over the substrate WS. The new contract is
            # exercised by the T-0629 substrate scenario at the bottom of
            # this function (insert → push → ack → row=acked). The old
            # scenario was removed rather than refactored because directly
            # running `cloacinactl ... --follow` via `_cloacinactl` (which
            # uses `subprocess.run` with no timeout) blocks the harness
            # forever: the CLI now genuinely connects and waits for events.

            # --- API-02: `execution list --status` filter actually takes effect ---
            # No executions exist yet for this tenant, so any filter returns []
            # but the request must not 4xx (proving the route accepts the query).
            code, out, stderr = _cloacinactl(
                home,
                "-o", "json",
                "--tenant", tenant_name,
                "execution", "list",
                "--status", "Failed",
                check=False,
            )
            if code != 0 or not out.strip():
                raise AssertionError(
                    f"API-02: execution list unexpected\n"
                    f"  exit={code}\n  stdout={out!r}\n  stderr={stderr!r}"
                )
            try:
                parsed = json.loads(out)
            except json.JSONDecodeError as e:
                raise AssertionError(
                    f"API-02: execution list stdout not JSON: {e}\n"
                    f"  stdout={out!r}\n  stderr={stderr!r}"
                ) from e
            assert isinstance(parsed, list), (
                f"API-02: execution list with filter should render array, got {parsed!r}"
            )
            print("  ok: API-02 execution list --status accepted")

            # --- API-10: `trigger list --limit` round-trip ---
            code, out, stderr = _cloacinactl(
                home,
                "-o", "json",
                "--tenant", tenant_name,
                "trigger", "list",
                "--limit", "5",
                "--offset", "0",
                check=False,
            )
            if code != 0 or not out.strip():
                raise AssertionError(
                    f"API-10: trigger list unexpected\n"
                    f"  exit={code}\n  stdout={out!r}\n  stderr={stderr!r}"
                )
            try:
                parsed = json.loads(out)
            except json.JSONDecodeError as e:
                raise AssertionError(
                    f"API-10: trigger list stdout not JSON: {e}\n"
                    f"  stdout={out!r}\n  stderr={stderr!r}"
                ) from e
            assert isinstance(parsed, list), (
                f"API-10: trigger list with pagination should render array, got {parsed!r}"
            )
            print("  ok: API-10 trigger list --limit/--offset accepted")

            # ─────────────────────────────────────────────────────────
            # CLOACI-I-0119: packaged-workflow authoring loop.
            # `package new` → edit the scaffold ("sed in some stuff") →
            # `package validate` (dir AND archive) → `package pack` →
            # `package upload`, then confirm the package registers. Proves
            # the scaffolder emits a server-acceptable package and the whole
            # author DX round-trips against a live server.
            # ─────────────────────────────────────────────────────────
            token = str(int(time.time()))
            pkg_name = f"e2e-authored-{token}"
            module = pkg_name.replace("-", "_")
            with tempfile.TemporaryDirectory() as authored_s:
                pkg_dir = Path(authored_s) / pkg_name

                # 1. scaffold a canonical Python package (T-0678)
                _cloacinactl(
                    home, "package", "new", pkg_name,
                    "--lang", "python", "--path", str(pkg_dir),
                )
                tasks_py = pkg_dir / "workflow" / module / "tasks.py"
                assert tasks_py.exists(), f"scaffold missing {tasks_py}"

                # 2. "sed in some stuff" — an author edits the generated task
                src = tasks_py.read_text()
                src = src.replace(
                    'context.set("hello", "world")',
                    f'context.set("authored_token", "{token}")',
                )
                tasks_py.write_text(src)
                assert "authored_token" in tasks_py.read_text()

                # 3. validate the edited source dir (T-0679)
                code, out, _ = _cloacinactl(home, "package", "validate", str(pkg_dir))
                assert code == 0 and "valid" in out, f"validate(dir) failed: {out!r}"

                # 4. pack into a .cloacina archive (T-0665)
                archive = Path(authored_s) / f"{pkg_name}.cloacina"
                _cloacinactl(home, "package", "pack", str(pkg_dir), "--out", str(archive))
                assert archive.exists(), "pack did not produce an archive"

                # 4b. validate accepts the packed archive too
                code, out, _ = _cloacinactl(home, "package", "validate", str(archive))
                assert code == 0 and "valid" in out, f"validate(archive) failed: {out!r}"

                # 5. upload to the tenant — the server parses package.toml,
                # stores the source package, and returns a package id. NOTE:
                # this lane runs only the server (no cloacina-compiler service),
                # so the package stays `pending` and never reaches `success` /
                # `package list` here — the build→success→list path is covered
                # by the compiler e2e + the demo stack. The author-DX contract
                # this scenario locks is: scaffold → edit → validate → pack →
                # upload produces a package the server *accepts*.
                code, out, stderr = _cloacinactl(
                    home, "--tenant", tenant_name,
                    "package", "upload", str(archive),
                    check=False,
                )
                assert code == 0, (
                    f"package upload failed: code={code} out={out!r} stderr={stderr!r}"
                )
                package_id = out.strip()
                assert package_id, (
                    f"upload returned no package id (stdout empty); stderr={stderr!r}"
                )
                print("  ok: authored python/workflow new→edit→validate→pack→upload "
                      f"accepted (package_id={package_id})")

                # T-0680: the graph and cron kinds round-trip the same way
                # (scaffold → validate → pack → upload-accept). cron is Rust-only.
                for lang, kind in [("python", "graph"), ("rust", "cron")]:
                    kname = f"e2e-{kind}-{token}"
                    kdir = Path(authored_s) / kname
                    _cloacinactl(
                        home, "package", "new", kname,
                        "--lang", lang, "--kind", kind, "--path", str(kdir),
                    )
                    code, out, _ = _cloacinactl(home, "package", "validate", str(kdir))
                    assert code == 0 and "valid" in out, (
                        f"validate({lang}/{kind}) failed: {out!r}"
                    )
                    karchive = Path(authored_s) / f"{kname}.cloacina"
                    _cloacinactl(home, "package", "pack", str(kdir), "--out", str(karchive))
                    code, out, stderr = _cloacinactl(
                        home, "--tenant", tenant_name, "package", "upload",
                        str(karchive), check=False,
                    )
                    assert code == 0 and out.strip(), (
                        f"upload({lang}/{kind}) failed: code={code} out={out!r} stderr={stderr!r}"
                    )
                    print(f"  ok: authored {lang}/{kind} new→validate→pack→upload accepted")

                # T-0680: --kind cron --lang python is rejected with guidance.
                code, _, stderr = _cloacinactl(
                    home, "package", "new", f"e2e-pycron-{token}",
                    "--lang", "python", "--kind", "cron",
                    "--path", str(Path(authored_s) / "pycron"),
                    check=False,
                )
                assert code != 0 and "Rust-only" in stderr, (
                    f"python+cron should be rejected; code={code} stderr={stderr!r}"
                )
                print("  ok: python --kind cron rejected with guidance")

            # ─────────────────────────────────────────────────────────
            # CLOACI-T-0629: substrate contract — end-to-end JIT
            # delivery + ack via `cloacinactl execution events --follow`.
            #
            # We bypass the runner and directly INSERT a delivery_outbox
            # row to keep the test self-contained: the substrate doesn't
            # care how rows arrive (any insert fires the postgres NOTIFY
            # trigger from migration 028), so we exercise the wake →
            # relay → sink → WS push → CLI ack → mark_acked path with a
            # synthetic event. Admin (bootstrap) auth → tenant=NULL →
            # public-schema delivery_outbox.
            # ─────────────────────────────────────────────────────────
            import uuid as _uuid
            fake_exec_id = str(_uuid.uuid4())
            recipient = f"exec_events:{fake_exec_id}"
            event_payload = {
                "id": str(_uuid.uuid4()),
                "workflow_execution_id": fake_exec_id,
                "task_execution_id": None,
                "event_type": "substrate_smoke",
                "event_data": "hello-from-e2e",
                "created_at": "2026-05-28T00:00:00Z",
            }
            payload_bytes = json.dumps(event_payload).encode()
            payload_hex = payload_bytes.hex()

            # Start CLI in --follow mode (background); capture stdout.
            follow_cmd = [
                "target/debug/cloacinactl",
                "--home", str(home),
                "execution", "events", fake_exec_id,
                "--follow",
            ]
            follow_proc = subprocess.Popen(
                follow_cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            try:
                # Give the CLI time to mint a ws-ticket, connect, register
                # with the WsDeliverySink, and send Hello. ~1s is plenty
                # on localhost; the relay's own startup catch-up drain has
                # long since completed by the time the e2e suite reaches here.
                time.sleep(1.5)
                if follow_proc.poll() is not None:
                    raise AssertionError(
                        f"--follow exited unexpectedly before insert: "
                        f"code={follow_proc.returncode} stderr={follow_proc.stderr.read()!r}"
                    )

                # Insert a substrate row directly. tenant_id NULL (admin auth
                # context); payload = JSON event the CLI will print.
                _psql(
                    "INSERT INTO delivery_outbox "
                    "(recipient, kind, tenant_id, payload, delivery_state, "
                    "delivery_attempts, created_at) VALUES "
                    f"('{recipient}', 'execution_event', NULL, "
                    f"'\\x{payload_hex}'::bytea, 'pending', 0, now());"
                )

                # Let the trigger fire → LISTEN delivers → relay drains →
                # WsDeliverySink pushes → CLI prints + acks → mark_acked.
                # Allow a generous budget for the cold WS round-trip.
                deadline = time.time() + 8.0
                state = ""
                while time.time() < deadline:
                    time.sleep(0.5)
                    state = _psql(
                        "SELECT delivery_state FROM delivery_outbox "
                        f"WHERE recipient = '{recipient}' ORDER BY id DESC LIMIT 1;"
                    ).strip()
                    if state == "acked":
                        break
                if state != "acked":
                    # Pull the CLI's stderr to help diagnose.
                    follow_proc.send_signal(signal.SIGTERM)
                    try:
                        out, err = follow_proc.communicate(timeout=2.0)
                    except subprocess.TimeoutExpired:
                        follow_proc.kill()
                        out, err = follow_proc.communicate()
                    raise AssertionError(
                        f"substrate contract: row state never reached `acked` "
                        f"(last seen: {state!r}).\n"
                        f"--follow stdout:\n{out}\n--follow stderr:\n{err}"
                    )
                print("  ok: T-0629 substrate JIT delivery + ack (row reached `acked`)")
            finally:
                if follow_proc.poll() is None:
                    follow_proc.send_signal(signal.SIGTERM)
                    try:
                        follow_proc.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        follow_proc.kill()

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


@test()
@e2e()
@angreal.command(
    name="default-executor",
    about="boot-time --default-executor hard-match (CLOACI-T-0640)",
    when_to_use=[
        "verifying the server fails fast on an unknown default executor",
        "verifying `--default-executor fleet` boots (fleet executor registers)",
    ],
    when_not_to_use=["unit testing", "running without docker"],
)
def default_executor():
    """CLOACI-T-0640: the executor is a single server-level knob, hard-matched
    against the registered executors at boot.

    - Negative: an unknown key (`--default-executor nope`) must abort startup
      with a clear error rather than silently dispatching all work into the void.
    - Positive: `--default-executor fleet` must boot — the server registers the
      fleet executor when opted in, so validation passes (no agents needed).

    The default (`default`/thread) boot path is already covered by the `cli`
    scenario, which boots with no `--default-executor` flag.
    """
    print_section_header("default-executor boot validation")
    _build_binaries()
    _start_postgres()

    db_url = "postgres://cloacina:cloacina@localhost:5432/cloacina"

    # --- negative: unknown executor key aborts startup ----------------------
    with tempfile.TemporaryDirectory() as home_s:
        home = Path(home_s)
        # Validation fires after DB connect + executor registration, so the
        # server reaches it quickly and then `run()` returns Err → exit non-zero.
        # `--bind` is required but never actually served (we bail first).
        proc = subprocess.run(
            [
                "target/debug/cloacina-server",
                "--home", str(home),
                "--database-url", db_url,
                "--bind", "127.0.0.1:18085",
                "--default-executor", "nope",
            ],
            capture_output=True,
            text=True,
            timeout=90,
        )
        if proc.returncode == 0:
            raise AssertionError(
                "server booted with an unknown --default-executor 'nope' "
                f"(expected non-zero exit).\nstdout:\n{proc.stdout}\n"
                f"stderr:\n{proc.stderr}"
            )
        combined = (proc.stderr + proc.stdout).lower()
        if "not a registered executor" not in combined:
            raise AssertionError(
                "expected a hard-match error mentioning 'not a registered "
                f"executor'; got exit={proc.returncode}\nstderr:\n{proc.stderr}"
            )
        print("  ok: unknown --default-executor 'nope' fails fast at boot")

    # --- positive: `fleet` boots (fleet executor registered when opted in) --
    bind = "127.0.0.1:18086"
    base_url = f"http://{bind}"
    with tempfile.TemporaryDirectory() as home_s:
        home = Path(home_s)
        server_stderr = open(home / "server-stderr.log", "wb")
        server = subprocess.Popen(
            [
                "target/debug/cloacina-server",
                "--home", str(home),
                "--database-url", db_url,
                "--bind", bind,
                "--bootstrap-key", "test-bootstrap-fleet-boot",
                "--default-executor", "fleet",
            ],
            stdout=subprocess.DEVNULL,
            stderr=server_stderr,
        )
        try:
            if server.poll() is not None:
                server_stderr.flush()
                tail = open(home / "server-stderr.log").read()[-4096:]
                raise AssertionError(
                    f"server with --default-executor fleet exited "
                    f"{server.returncode} during startup\nstderr tail:\n{tail}"
                )
            _wait_for_health(base_url, server_proc=server)
            print("  ok: --default-executor fleet boots and serves /health")
            print_final_success("default-executor boot validation")
        except Exception:
            server_stderr.flush()
            try:
                tail = open(home / "server-stderr.log").read()[-4096:]
                print(f"--- server stderr tail ---\n{tail}\n--- end ---")
            except Exception as e:
                print(f"(failed to read server-stderr.log: {e})")
            raise
        finally:
            if server.poll() is None:
                server.send_signal(signal.SIGTERM)
                try:
                    server.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    server.kill()
            server_stderr.close()
