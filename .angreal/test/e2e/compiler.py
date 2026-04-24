"""End-to-end test of the compiler service pipeline.

Spins Postgres, server, and compiler as separate subprocesses sharing one
database, then drives the full flow through cloacinactl. Asserts on DB
state (build_status, build_error) and the server's actual runtime
behaviour (workflow run → execution completes).

Coverage:
  1. Happy path     — upload → compile → build_status = success
  2. Failed build   — cargo error → build_status = failed, build_error set
  3. Content-hash   — re-uploading identical bytes is idempotent
  4. Stale heartbeat — poisoned `building` row is swept + re-claimed
  5. Reconciler e2e — reconciler loads the compiled package, workflow run
                     schedules an execution, execution completes

All fixtures under examples/fixtures/ are real packaged workflows
(cloacina-workflow + #[workflow] macro); their Cargo.toml's use
`__WORKSPACE__` placeholders that the harness rewrites to absolute paths
at stage time, so the compiler service's `cargo build` can resolve the
unpublished cloacina path-deps from any unpacked tmpdir.
"""

import json
import os
import signal
import subprocess
import tempfile
import time
import urllib.request
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_final_success, print_section_header

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
e2e = angreal.command_group(name="e2e", about="end-to-end tests against a live server")

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURES = REPO_ROOT / "examples" / "fixtures"


# ---------------------------------------------------------------------------
# Build + service lifecycle
# ---------------------------------------------------------------------------


def _build_binaries():
    print("Building cloacina-server + cloacina-compiler + cloacinactl (debug)...")
    for pkg in ("cloacina-server", "cloacina-compiler", "cloacinactl"):
        subprocess.run(["cargo", "build", "-p", pkg], cwd=REPO_ROOT, check=True)


def _start_postgres():
    # Reset the container + volume so each run gets a fresh DB; otherwise
    # register_workflow's content-hash dedup returns stale rows from prior
    # runs (e.g. a previous failed build with the same fixture bytes).
    subprocess.run(
        ["docker", "compose", "-f", ".angreal/docker-compose.yaml", "down", "-v"],
        cwd=REPO_ROOT,
        check=False,
    )
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


def _wait_http(
    url: str,
    label: str,
    timeout_s: float = 30.0,
    proc: subprocess.Popen | None = None,
):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=1.0):
                return
        except Exception:
            time.sleep(0.5)
        if proc is not None and proc.poll() is not None:
            raise RuntimeError(
                f"{label} exited with code {proc.returncode} before /health came up. "
                "See the service log file in $home for details."
            )
    if proc is not None and proc.poll() is None:
        proc.send_signal(signal.SIGTERM)
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()
    raise RuntimeError(f"{label} at {url} never came up within {timeout_s}s")


def _port_free(port: int) -> bool:
    r = subprocess.run(
        ["lsof", f"-iTCP:{port}", "-sTCP:LISTEN"],
        capture_output=True,
        text=True,
    )
    return r.returncode != 0 or not r.stdout.strip()


def _assert_ports_free(*ports: int):
    for p in ports:
        if not _port_free(p):
            raise RuntimeError(
                f"port {p} is already in use — kill the stale process before re-running."
            )


def _psql(sql: str) -> str:
    r = subprocess.run(
        [
            "docker", "compose",
            "-f", ".angreal/docker-compose.yaml",
            "exec", "-T", "postgres",
            "psql", "-U", "cloacina", "-d", "cloacina",
            "-tA", "-c", sql,
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    return r.stdout.strip()


def _kill(proc: subprocess.Popen | None):
    if proc is None or proc.poll() is not None:
        return
    proc.send_signal(signal.SIGTERM)
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()


# ---------------------------------------------------------------------------
# CLI driver
# ---------------------------------------------------------------------------


def _cloacinactl(home: Path, *args, check=True, env=None):
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


# ---------------------------------------------------------------------------
# Fixture staging
# ---------------------------------------------------------------------------


def _stage_fixture(
    home: Path,
    src_name: str,
    *,
    rename_to: str | None = None,
    version_override: str | None = None,
    stage_suffix: str | None = None,
) -> Path:
    """Copy a fixture from examples/fixtures/<src_name> into the per-run
    home, rewriting `__WORKSPACE__` placeholders to absolute paths that
    point at this checkout. Optionally renames the package (cargo pkg +
    cloacina pkg name) to produce distinct content-hash bytes — used by
    the stale-heartbeat test to avoid dedup collision with the happy
    fixture. `version_override` substitutes `version = "0.1.0"` in
    both package.toml + Cargo.toml for the package-lifecycle e2e
    (upgrade/rollback/concurrent scenarios, T-0497). `stage_suffix`
    changes the staged-dir suffix so multiple copies of the same
    (src_name, rename_to) can coexist in one run.
    """
    src = FIXTURES / src_name
    dst_name = rename_to or src_name
    suffix = stage_suffix or ""
    dst = home / f"staged-{dst_name}{suffix}"
    if dst.exists():
        subprocess.run(["rm", "-rf", str(dst)], check=True)
    (dst / "src").mkdir(parents=True)

    ws = str(REPO_ROOT)
    for rel in ("package.toml", "Cargo.toml", "build.rs", "src/lib.rs"):
        text = (src / rel).read_text().replace("__WORKSPACE__", ws)
        if rename_to is not None:
            text = text.replace(src_name, rename_to)
            text = text.replace(
                src_name.replace("-", "_"), rename_to.replace("-", "_")
            )
        if version_override is not None and rel in ("package.toml", "Cargo.toml"):
            text = text.replace('version = "0.1.0"', f'version = "{version_override}"')
        (dst / rel).write_text(text)
    return dst


def _upload(home: Path, fixture_dir: Path) -> str:
    """Pack + upload a staged fixture. Returns the package UUID."""
    archive = home / f"{fixture_dir.name}.cloacina"
    _cloacinactl(home, "package", "pack", str(fixture_dir), "--out", str(archive))
    _, out, _ = _cloacinactl(home, "package", "upload", str(archive))
    pkg_id = out.strip().splitlines()[-1].strip()
    if not pkg_id or len(pkg_id) < 32:
        raise AssertionError(f"upload didn't print a package id; got: {out!r}")
    return pkg_id


# ---------------------------------------------------------------------------
# Polling helpers
# ---------------------------------------------------------------------------


def _poll_build_status(
    home: Path,
    pkg_id: str,
    expected: set[str],
    timeout_s: float = 120.0,
) -> dict:
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


def _poll_run_workflow(
    home: Path,
    workflow_name: str,
    timeout_s: float = 120.0,
) -> str:
    """Try `workflow run` until the runner has actually loaded the workflow
    (HTTP no longer returns 'Workflow not found in registry'). The
    reconciler loads packages on a periodic tick — until that lands, the
    runtime registry doesn't know about the workflow even though the DB
    does. Returns the execution_id from the first accepted run.
    """
    deadline = time.time() + timeout_s
    last_err = ""
    while time.time() < deadline:
        code, out, err = _cloacinactl(
            home, "-o", "json", "workflow", "run", workflow_name, check=False
        )
        if code == 0:
            try:
                resp = json.loads(out)
                exec_id = resp.get("execution_id")
                if exec_id and len(exec_id) >= 32:
                    return exec_id
            except json.JSONDecodeError:
                pass
            # Non-JSON success — fall back to last line.
            tail = out.strip().splitlines()[-1].strip() if out.strip() else ""
            if len(tail) >= 32:
                return tail
        last_err = err.strip() or out.strip()
        time.sleep(2.0)
    raise AssertionError(
        f"workflow run {workflow_name} never succeeded within {timeout_s}s; "
        f"last error: {last_err}"
    )


def _poll_execution_status(
    home: Path,
    execution_id: str,
    expected: set[str],
    timeout_s: float = 60.0,
) -> str:
    deadline = time.time() + timeout_s
    last_status: str | None = None
    while time.time() < deadline:
        _, out, _ = _cloacinactl(
            home, "-o", "json", "execution", "status", execution_id
        )
        try:
            body = json.loads(out)
        except json.JSONDecodeError:
            time.sleep(1.0)
            continue
        last_status = body.get("status")
        if last_status in expected:
            return last_status
        time.sleep(1.0)
    raise AssertionError(
        f"execution {execution_id} never reached {expected}; last: {last_status!r}"
    )


# ---------------------------------------------------------------------------
# Harness entrypoint
# ---------------------------------------------------------------------------


@test()
@e2e()
@angreal.command(
    name="compiler",
    about="end-to-end cloacina-compiler integration tests (T-0527)",
)
def compiler():
    print_section_header("cloacina-compiler e2e")
    _build_binaries()
    _start_postgres()
    _assert_ports_free(18083, 19003)

    db_url = "postgres://cloacina:cloacina@localhost:5432/cloacina"
    bootstrap_key = "test-bootstrap-compiler-e2e"
    server_bind = "127.0.0.1:18083"
    compiler_bind = "127.0.0.1:19003"
    server_url = f"http://{server_bind}"
    compiler_url = f"http://{compiler_bind}"

    server_proc: subprocess.Popen | None = None
    compiler_proc: subprocess.Popen | None = None

    # Persistent home so logs survive past the assertion for post-mortem.
    home = Path(tempfile.mkdtemp(prefix="compiler-e2e-"))
    print(f"compiler-e2e home: {home}")

    try:
        server_log = open(home / "server.log", "w")
        server_proc = subprocess.Popen(
            [
                "target/debug/cloacina-server",
                "--home", str(home),
                "--database-url", db_url,
                "--bind", server_bind,
                "--bootstrap-key", bootstrap_key,
                "--verbose",
            ],
            cwd=REPO_ROOT,
            stdout=server_log,
            stderr=subprocess.STDOUT,
        )
        _wait_http(f"{server_url}/health", "server", proc=server_proc)
        print("  ok: server up")

        # Shared CARGO_TARGET_DIR so the ~100 transitive deps compile once
        # across the whole harness run (and across re-runs in dev).
        shared_target = REPO_ROOT / "target" / "compiler-e2e-cache"
        shared_target.mkdir(parents=True, exist_ok=True)
        compiler_log = open(home / "compiler.log", "w")
        # Build fixtures in debug so the fidius wire format (JSON in debug,
        # bincode in release) matches the debug-built server we're running
        # here. In prod both server and compiler are release builds and the
        # default --release flag on cargo is fine.
        compiler_proc = subprocess.Popen(
            [
                "target/debug/cloacina-compiler",
                "--home", str(home),
                "--database-url", db_url,
                "--bind", compiler_bind,
                "--poll-interval-ms", "500",
                "--cargo-target-dir", str(shared_target),
                "--cargo-flag=build",
                "--cargo-flag=--lib",
                "--verbose",
            ],
            cwd=REPO_ROOT,
            stdout=compiler_log,
            stderr=subprocess.STDOUT,
        )
        _wait_http(
            f"{compiler_url}/health", "compiler", timeout_s=60.0, proc=compiler_proc
        )
        print("  ok: compiler up")

        _cloacinactl(
            home,
            "config", "profile", "set", "local", server_url,
            "--api-key", bootstrap_key,
            "--default",
        )
        _cloacinactl(home, "config", "set", "compiler.local_addr", compiler_bind)

        code, out, _ = _cloacinactl(home, "status")
        assert code == 0, f"composite status failed: {out!r}"
        assert "server" in out and "compiler" in out, out
        print("  ok: composite status covers server + compiler")

        # --- happy path -----------------------------------------------------
        # First run cold-compiles cloacina + ~100 transitive deps; subsequent
        # runs hit the shared target cache and finish in <30s.
        happy_dir = _stage_fixture(home, "compiler-happy-rust")
        print("  compiling happy fixture "
              "(first run: ~5-10 min cold build; subsequent: <30s)")
        happy_id = _upload(home, happy_dir)
        body = _poll_build_status(home, happy_id, {"success"}, timeout_s=900.0)
        assert body.get("build_status") == "success", body
        assert body.get("build_error") in (None, "", "null"), body
        print("  ok: happy path → build_status = success")

        # --- failed build ---------------------------------------------------
        broken_dir = _stage_fixture(home, "compiler-broken-rust")
        broken_id = _upload(home, broken_dir)
        body = _poll_build_status(home, broken_id, {"failed"}, timeout_s=300.0)
        err = body.get("build_error") or ""
        assert err, f"expected non-empty build_error, got: {body!r}"
        print(
            f"  ok: failed-build path → build_status = failed "
            f"({len(err)}-byte build_error captured)"
        )

        # --- content-hash reuse (idempotent re-upload) ---------------------
        _, out, _ = _cloacinactl(
            home, "package", "upload", str(home / f"{happy_dir.name}.cloacina")
        )
        reupload_id = out.strip().splitlines()[-1].strip()
        assert reupload_id == happy_id, (
            f"re-upload of identical bytes should return the same id; "
            f"got {reupload_id!r} vs original {happy_id!r}"
        )
        body = json.loads(
            _cloacinactl(home, "-o", "json", "package", "inspect", happy_id)[1]
        )
        assert body.get("build_status") == "success", body
        print("  ok: content-hash reuse → idempotent, no re-queue")

        # --- stale-heartbeat recovery --------------------------------------
        stale_dir = _stage_fixture(
            home, "compiler-happy-rust", rename_to="compiler-stale-rust"
        )
        stale_id = _upload(home, stale_dir)
        _psql(
            f"UPDATE public.workflow_packages "
            f"SET build_status='building', "
            f"    build_claimed_at = NOW() - INTERVAL '10 minutes' "
            f"WHERE id = '{stale_id}';"
        )
        _poll_build_status(home, stale_id, {"success"}, timeout_s=300.0)
        print("  ok: stale-heartbeat recovered by sweeper → re-built")

        # --- reconciler end-to-end -----------------------------------------
        # Happy fixture already compiled → success. Wait for the reconciler
        # to actually load it into the runner, then run it and assert the
        # execution completes. `_poll_run_workflow` retries until the
        # runner's registry has the workflow.
        execution_id = _poll_run_workflow(
            home, "compiler_happy_workflow", timeout_s=120.0
        )
        print(f"  triggered execution: {execution_id}")
        status = _poll_execution_status(
            home, execution_id, {"Completed", "Failed", "Cancelled"}, timeout_s=60.0
        )
        assert status == "Completed", (
            f"execution {execution_id} ended in status {status!r}"
        )
        print(f"  ok: reconciler end-to-end → execution {status}")

        # --- package lifecycle: upgrade (T-0497) ---------------------------
        # Upload a new version of the same package. The upload handler
        # should supersede the current active row and insert a new one
        # with its own UUID. DB invariant: one active row per name.
        upgrade_dir = _stage_fixture(
            home,
            "compiler-happy-rust",
            version_override="0.2.0",
            stage_suffix="-v2",
        )
        upgrade_id = _upload(home, upgrade_dir)
        _poll_build_status(home, upgrade_id, {"success"}, timeout_s=300.0)
        assert upgrade_id != happy_id, (
            f"upgrade should yield a new package_id; got {upgrade_id!r} "
            f"same as v1 {happy_id!r}"
        )
        v1_row = _psql(
            f"SELECT superseded FROM public.workflow_packages WHERE id = '{happy_id}';"
        )
        v2_row = _psql(
            f"SELECT superseded FROM public.workflow_packages WHERE id = '{upgrade_id}';"
        )
        assert v1_row.strip() in ("t", "true"), f"v1 should be superseded, got {v1_row!r}"
        assert v2_row.strip() in ("f", "false"), f"v2 should be active, got {v2_row!r}"
        active_count = _psql(
            "SELECT COUNT(*) FROM public.workflow_packages "
            "WHERE package_name = 'compiler-happy-rust' AND NOT superseded;"
        )
        assert active_count.strip() == "1", (
            f"exactly one active row expected for compiler-happy-rust, got {active_count!r}"
        )
        print("  ok: upgrade path → old superseded, new active")

        # --- package lifecycle: rollback (T-0497) --------------------------
        # Versions are monotonic (UNIQUE(name, version)), so rollback means
        # a *new* version string carrying older source. Upload v0.3.0 with
        # the v1 task body — supersedes v0.2.0 and lands as a fresh UUID.
        rollback_dir = _stage_fixture(
            home,
            "compiler-happy-rust",
            version_override="0.3.0",
            stage_suffix="-rollback",
        )
        rollback_id = _upload(home, rollback_dir)
        _poll_build_status(home, rollback_id, {"success"}, timeout_s=300.0)
        assert rollback_id != happy_id and rollback_id != upgrade_id, (
            f"rollback should yield a fresh package_id; got {rollback_id!r}"
        )
        v2_after = _psql(
            f"SELECT superseded FROM public.workflow_packages WHERE id = '{upgrade_id}';"
        )
        rollback_row = _psql(
            f"SELECT superseded FROM public.workflow_packages WHERE id = '{rollback_id}';"
        )
        assert v2_after.strip() in ("t", "true"), (
            f"v2 should be superseded after rollback, got {v2_after!r}"
        )
        assert rollback_row.strip() in ("f", "false"), (
            f"rollback row should be active, got {rollback_row!r}"
        )
        active_count = _psql(
            "SELECT COUNT(*) FROM public.workflow_packages "
            "WHERE package_name = 'compiler-happy-rust' AND NOT superseded;"
        )
        assert active_count.strip() == "1", (
            f"exactly one active row expected after rollback, got {active_count!r}"
        )
        print("  ok: rollback path → v2 superseded, older bytes active under new id")

        # --- package lifecycle: concurrent uploads (T-0497) ----------------
        # Two parallel uploads of a fresh (name, version). Exactly one
        # must succeed; the other must lose cleanly with a user-visible
        # "package already exists" error. DB invariant: one active row.
        # No split-brain, no duplicate rows under the partial unique index.
        concurrent_dir = _stage_fixture(
            home,
            "compiler-happy-rust",
            rename_to="compiler-concurrent-rust",
        )
        archive = home / f"{concurrent_dir.name}.cloacina"
        _cloacinactl(
            home, "package", "pack", str(concurrent_dir), "--out", str(archive)
        )

        from concurrent.futures import ThreadPoolExecutor

        def do_upload() -> tuple[int, str, str]:
            return _cloacinactl(
                home, "package", "upload", str(archive), check=False
            )

        with ThreadPoolExecutor(max_workers=2) as pool:
            f1 = pool.submit(do_upload)
            f2 = pool.submit(do_upload)
            r1 = f1.result()
            r2 = f2.result()

        # Either both succeed (second hit the hash-dedup idempotent branch)
        # or one wins + one loses with 409/PackageExists. Both are correct
        # outcomes per the audit ("only one wins, no corruption").
        outcomes = sorted([(r1[0], r1[1], r1[2]), (r2[0], r2[1], r2[2])])
        success_count = sum(1 for (code, _, _) in outcomes if code == 0)
        assert success_count >= 1, (
            f"at least one concurrent upload must succeed; got {outcomes!r}"
        )
        if success_count == 1:
            loser = [err for (code, _, err) in outcomes if code != 0][0]
            assert "already exists" in loser.lower() or "packageexists" in loser.lower(), (
                f"losing upload must report PackageExists, got: {loser!r}"
            )

        active_count = _psql(
            "SELECT COUNT(*) FROM public.workflow_packages "
            "WHERE package_name = 'compiler-concurrent-rust' AND NOT superseded;"
        )
        assert active_count.strip() == "1", (
            f"exactly one active row expected after concurrent upload, "
            f"got {active_count!r}"
        )
        total_count = _psql(
            "SELECT COUNT(*) FROM public.workflow_packages "
            "WHERE package_name = 'compiler-concurrent-rust';"
        )
        assert total_count.strip() == "1", (
            f"no duplicate rows expected; DB has {total_count!r} rows for "
            "compiler-concurrent-rust"
        )
        print(
            f"  ok: concurrent uploads → {success_count}/2 succeeded, one "
            "active row, no split-brain"
        )

        print_final_success("cloacina-compiler e2e")
    except BaseException:
        # Dump log tails so CI transcripts stand alone.
        for label in ("server", "compiler"):
            log = home / f"{label}.log"
            if log.exists():
                print(f"\n---- last 80 lines of {label}.log ----")
                lines = log.read_text(errors="replace").splitlines()
                for line in lines[-80:]:
                    print(line)
        raise
    finally:
        _kill(compiler_proc)
        _kill(server_proc)
