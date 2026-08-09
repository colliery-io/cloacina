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
import re
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
    # The down above only frees 15432 if OUR dev stack held it. The dev stack
    # publishes postgres on 15432 (NOT the postgres default 5432) precisely so
    # it can't collide with other projects' databases; if something still holds
    # it, fail with a remedy instead of docker's opaque port-bind error.
    if not _port_free(15432):
        raise RuntimeError(
            "port 15432 is held by another process — check `lsof -iTCP:15432 "
            "-sTCP:LISTEN` / `docker ps`, stop whatever owns it, and re-run."
        )
    subprocess.run(
        ["docker", "compose", "-f", ".angreal/docker-compose.yaml", "up", "-d", "postgres"],
        cwd=REPO_ROOT,
        check=True,
    )
    # CLOACI-T-0806: consecutive-success readiness so the follow-on psql
    # can't race the init-restart bounce (exit 56).
    from .._utils import wait_for_postgres_stable

    wait_for_postgres_stable(cwd=REPO_ROOT)


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


# Matches a cloacina workspace path-dep — `{ path = "__WORKSPACE__/crates/…"
# [, features = […]] }` — so the `--version-deps` staging mode can rewrite it to
# the crates.io VERSION-dep form a real distributed package ships.
_WS_PATH_DEP = re.compile(
    r'\{\s*path\s*=\s*"__WORKSPACE__/crates/[^"]+"'
    r'(?:\s*,\s*(?P<feats>features\s*=\s*\[[^\]]*\]))?\s*\}'
)

# Workspace crate version the version-dep form pins to. Caret "0.10" matches the
# workspace's 0.10.x; the compiler's `--dev-workspace` patch supplies the actual
# (unpublished) crate, so this need not exist on crates.io.
_PROD_DEP_VERSION = "0.10"


def _to_version_deps(text: str) -> str:
    """Rewrite `__WORKSPACE__` path-deps to crates.io version-deps in a Cargo.toml.

    `{ path = "__WORKSPACE__/crates/cloacina-workflow", features = ["packaged"] }`
    → `{ version = "0.10", features = ["packaged"] }`, and a bare path-dep →
    `"0.10"`. This is the exact dep form `cloacinactl package new` emits — the
    shipping form — which only resolves under a compiler started with
    `--dev-workspace` (it injects `[patch.crates-io]` → the local crates).
    """

    def repl(m: re.Match) -> str:
        feats = m.group("feats")
        if feats:
            return f'{{ version = "{_PROD_DEP_VERSION}", {feats} }}'
        return f'"{_PROD_DEP_VERSION}"'

    return _WS_PATH_DEP.sub(repl, text)


def _stage_fixture(
    home: Path,
    src_name: str,
    *,
    rename_to: str | None = None,
    version_override: str | None = None,
    stage_suffix: str | None = None,
    version_deps: bool = False,
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
        raw = (src / rel).read_text()
        # version_deps: author the Cargo.toml with crates.io version deps (the
        # manifest shape users get from `cloacinactl package new`) instead of
        # rewriting `__WORKSPACE__` path-deps to absolute paths. The compiler must
        # run with `--dev-workspace` so these resolve to the LOCAL crates.
        if version_deps and rel == "Cargo.toml":
            # Deps become version deps; then scrub residual placeholders (the
            # fixture's own comments mention __WORKSPACE__, and cloacinactl's
            # pack guard rejects the literal anywhere in the manifest).
            text = _to_version_deps(raw).replace("__WORKSPACE__", ws)
        else:
            text = raw.replace("__WORKSPACE__", ws)
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


def _get_json(url: str, bootstrap_key: str) -> dict:
    """Authenticated GET → parsed JSON. Used to assert on the server's actual
    HTTP response bodies (the API surface the UI/SDK consume)."""
    req = urllib.request.Request(
        url, headers={"Authorization": f"Bearer {bootstrap_key}"}
    )
    with urllib.request.urlopen(req, timeout=5.0) as resp:
        return json.loads(resp.read().decode())


def _poll_graph_topology(
    server_url: str,
    bootstrap_key: str,
    graph_name: str,
    timeout_s: float = 120.0,
) -> dict:
    """Poll GET /v1/health/graphs/{name} until the reactor-bound CG is loaded
    and carries a non-empty node/edge topology, or fail. (CLOACI-T-0673)"""
    deadline = time.time() + timeout_s
    last: dict = {}
    while time.time() < deadline:
        try:
            last = _get_json(
                f"{server_url}/v1/health/graphs/{graph_name}", bootstrap_key
            )
            topo = last.get("topology")
            if topo and topo.get("nodes"):
                return last
        except Exception:
            pass
        time.sleep(1.0)
    raise AssertionError(
        f"graph '{graph_name}' never reported a topology within {timeout_s}s; "
        f"last body: {json.dumps(last, indent=2)}"
    )


def _poll_run_workflow(
    home: Path,
    workflow_name: str,
    timeout_s: float = 120.0,
    context: dict | None = None,
) -> str:
    """Try `workflow run` until the runner has actually loaded the workflow
    (HTTP no longer returns 'Workflow not found in registry'). The
    reconciler loads packages on a periodic tick — until that lands, the
    runtime registry doesn't know about the workflow even though the DB
    does. Returns the execution_id from the first accepted run.

    `context` is REQUIRED for a workflow that declares required params: the
    execute route validates before dispatching, so a bare run of such a
    workflow is rejected forever and this would spin until timeout on a
    validation error rather than a load error (CLOACI-T-0927).
    """
    deadline = time.time() + timeout_s
    last_err = ""
    ctx_args: list[str] = []
    if context is not None:
        ctx_path = home / f"run-context-{workflow_name}.json"
        ctx_path.write_text(json.dumps(context))
        ctx_args = ["--context", str(ctx_path)]
    while time.time() < deadline:
        code, out, err = _cloacinactl(
            home, "-o", "json", "workflow", "run", workflow_name, *ctx_args,
            check=False,
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


def _poll_instance_fire(
    workflow_name: str,
    instance_name: str,
    timeout_s: float = 120.0,
) -> tuple[str, dict]:
    """Wait for a NAMED INSTANCE's cron schedule to actually fire, and return
    `(execution_id, context)` for the run it produced (CLOACI-T-0927).

    This is the assertion T-0894 could not make: that an instance created over
    HTTP is picked up by the scheduler and fires with its bound params merged
    into the run's context. Nothing short of a live server proves it.

    The context is read straight from the DB because the executions API exposes
    only status — `ExecutionDetail` carries no context — so there is no HTTP
    channel for this. The schedule row is matched on `instance_name` so a
    concurrent anonymous schedule for the same workflow can't be mistaken for
    the instance's own fire.
    """
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        rows = _psql(
            "SELECT we.id::text || '|' || c.value "
            "FROM public.workflow_executions we "
            "JOIN public.contexts c ON c.id = we.context_id "
            "JOIN public.schedule_executions se "
            "  ON se.workflow_execution_id = we.id "
            "JOIN public.schedules s ON s.id = se.schedule_id "
            f"WHERE s.workflow_name = '{workflow_name}' "
            f"  AND s.instance_name = '{instance_name}' "
            "ORDER BY we.created_at DESC LIMIT 1;"
        ).strip()
        if rows:
            exec_id, _, raw = rows.partition("|")
            try:
                return exec_id.strip(), json.loads(raw)
            except json.JSONDecodeError:
                pass
        time.sleep(2.0)
    raise AssertionError(
        f"instance '{instance_name}' of '{workflow_name}' never fired within "
        f"{timeout_s}s (no workflow_execution linked to its schedule)"
    )


# ---------------------------------------------------------------------------
# Harness entrypoint
# ---------------------------------------------------------------------------


@test()
@e2e()
@angreal.command(
    name="compiler",
    about="end-to-end cloacina-compiler integration tests (T-0527)",
    when_to_use=[
        "validating cloacina-compiler against reconciler end-to-end",
        "pre-release build-queue regression check",
    ],
    when_not_to_use=["unit testing", "running without docker"],
)
@angreal.argument(
    name="version_deps",
    long="version-deps",
    help=(
        "Author the happy-path fixture with version deps (`cloacina-workflow = "
        '"0.10"` — the manifest shape `cloacinactl package new` gives users) '
        "instead of __WORKSPACE__ path deps. Still compiles THIS checkout: the "
        "compiler gets --dev-workspace, which patches those deps to the local "
        "crates/ (offline; crates.io is never contacted). CLOACI-T-0887."
    ),
    takes_value=False,
    is_flag=True,
)
def compiler(version_deps=False):
    print_section_header("cloacina-compiler e2e")
    if version_deps:
        print(
            "  MODE: --version-deps — user-shaped manifest (version deps), "
            "resolved to the LOCAL dev workspace via --dev-workspace (offline; "
            "crates.io never contacted)"
        )
    _build_binaries()
    _start_postgres()
    _assert_ports_free(18083, 19003)

    db_url = "postgres://cloacina:cloacina@localhost:15432/cloacina"
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
        compiler_argv = [
            "target/debug/cloacina-compiler",
            "--home", str(home),
            "--database-url", db_url,
            "--bind", compiler_bind,
            "--poll-interval-ms", "500",
            "--cargo-target-dir", str(shared_target),
            "--cargo-flags-replace=build",
            "--cargo-flags-replace=--lib",
            "--verbose",
        ]
        if version_deps:
            # DEV ESCAPE HATCH (CLOACI-T-0887): inject [patch.crates-io] → the
            # local unpublished crates so the version-dep fixture resolves —
            # dev code, user-shaped manifest. No-op for the path-dep fixtures
            # (their deps aren't crates-io deps → patch unused).
            compiler_argv += ["--dev-workspace", str(REPO_ROOT)]
        compiler_proc = subprocess.Popen(
            compiler_argv,
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
        happy_dir = _stage_fixture(
            home, "compiler-happy-rust", version_deps=version_deps
        )
        print("  compiling happy fixture "
              f"({'version-dep manifest' if version_deps else 'path-dep escape hatch'}; "
              "first run: ~5-10 min cold build; subsequent: <30s)")
        happy_id = _upload(home, happy_dir)
        body = _poll_build_status(home, happy_id, {"success"}, timeout_s=900.0)
        assert body.get("build_status") == "success", body
        assert body.get("build_error") in (None, "", "null"), body
        print("  ok: happy path → build_status = success")

        # --- workflow metadata: name + task graph -------------------------
        # Guards CLOACI-T-0663 / T-0671 / T-0672: after a successful build the
        # row must persist the executable workflow name (distinct from the
        # package name) and the task list with dependency edges, and the
        # workflow-detail API must surface both (this is what the UI executes
        # by and renders as a DAG).
        detail = _get_json(
            f"{server_url}/v1/tenants/public/workflows/{happy_id}", bootstrap_key
        )
        assert detail.get("workflow_name") == "compiler_happy_workflow", (
            "workflow_name must be the #[workflow(name=...)] value, not the "
            f"package name: {detail!r}"
        )
        assert detail["workflow_name"] != detail.get("package_name"), detail
        assert detail.get("tasks"), f"expected non-empty tasks: {detail!r}"
        task_graph = detail.get("task_graph") or []
        assert task_graph, f"expected non-empty task_graph: {detail!r}"
        assert {n["id"] for n in task_graph} == set(detail["tasks"]), detail
        print(
            "  ok: workflow_name + tasks + task_graph populated "
            "(T-0663/0671/0672)"
        )

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

        # --- named workflow instances (CLOACI-T-0927) ----------------------
        # T-0894 shipped the instance surface with only unit-level proof. This
        # is the lane that proves the FEATURE rather than the endpoint: create
        # a named instance over HTTP against a live server and watch its cron
        # schedule fire with the bound params merged into the run's context.
        inst_dir = _stage_fixture(
            home, "instance-params-rust", version_deps=version_deps
        )
        print("  compiling instance-params fixture")
        inst_id = _upload(home, inst_dir)
        body = _poll_build_status(home, inst_id, {"success"}, timeout_s=900.0)
        assert body.get("build_status") == "success", body

        # Wait for the reconciler to load it — declared params are read from
        # the registry, so creating an instance before the load would skip
        # validation (it fails OPEN by design) and prove nothing.
        #
        # A VALID context is mandatory here: `region` is declared required, so
        # a bare run is rejected by the execute route's own validation and this
        # would spin to timeout on a validation error while the workflow was
        # loaded all along. (That rejection is itself proof the execute route
        # validates against a live server — asserted explicitly below for the
        # instance path.)
        _poll_run_workflow(
            home,
            "instance_params_workflow",
            timeout_s=120.0,
            context={"region": "warmup", "batch_size": 1},
        )
        print("  ok: instance fixture built + loaded")

        # Validation is real: `region` is declared REQUIRED with no default,
        # so an instance that omits it must be refused at CREATE time. This is
        # the whole point of validating at creation rather than at 3am on the
        # first fire.
        code, out, err = _cloacinactl(
            home,
            "instance", "create", "instance_params_workflow", "bad_instance",
            "--param", "batch_size=7",
            "--cron", "*/2 * * * * *",
            check=False,
        )
        assert code != 0, (
            "creating an instance without the required param 'region' should "
            f"fail; got exit 0 with: {out!r}"
        )
        combined = (out + err).lower()
        assert "region" in combined, (
            f"rejection should name the missing param; got: {(out + err)!r}"
        )
        print("  ok: missing required param rejected at create time")

        # Every 2 seconds (6-field cron = seconds precision) so the lane
        # observes a real fire without a long wait.
        _cloacinactl(
            home,
            "instance", "create", "instance_params_workflow", "e2e_prod",
            "--param", "region=eu-west",
            "--param", "batch_size=7",
            "--cron", "*/2 * * * * *",
        )
        print("  ok: instance created")

        listed = json.loads(
            _cloacinactl(
                home, "-o", "json",
                "instance", "list", "instance_params_workflow",
            )[1]
        )
        # `-o json` renders a list as a bare JSON array; tolerate an enveloped
        # shape too so this doesn't break if the renderer gains one.
        rows = (
            listed
            if isinstance(listed, list)
            else (listed.get("items") or listed.get("data") or [])
        )
        names = [i.get("instance_name") for i in rows]
        assert "e2e_prod" in names, f"created instance not listed: {listed!r}"
        assert "bad_instance" not in names, (
            f"rejected instance must not have been persisted: {listed!r}"
        )
        print("  ok: instance listed (and the rejected one was never stored)")

        inst_exec_id, ctx = _poll_instance_fire(
            "instance_params_workflow", "e2e_prod", timeout_s=120.0
        )
        print(f"  instance fired: execution {inst_exec_id}")

        # The bound params must arrive as top-level context keys...
        assert ctx.get("region") == "eu-west", (
            f"bound param 'region' missing/wrong in fired context: {ctx!r}"
        )
        assert ctx.get("batch_size") == 7, (
            f"bound param 'batch_size' missing/wrong in fired context: {ctx!r}"
        )
        # ...and the TASK must have actually seen them, not just the row.
        assert ctx.get("observed_region") == "eu-west", (
            f"task did not observe the bound region: {ctx!r}"
        )
        assert ctx.get("observed_batch_size") == 7, (
            f"task did not observe the bound batch_size: {ctx!r}"
        )
        # Scheduler-reserved keys still win over any binding.
        assert ctx.get("schedule_id"), f"scheduler keys missing: {ctx!r}"
        print(
            "  ok: instance fire delivered bound params to the task "
            "(region=eu-west, batch_size=7)"
        )

        status = _poll_execution_status(
            home, inst_exec_id, {"Completed", "Failed", "Cancelled"}, timeout_s=60.0
        )
        assert status == "Completed", (
            f"instance execution {inst_exec_id} ended in status {status!r}"
        )
        print(f"  ok: instance execution {status}")

        # Delete stops future fires. Assert the row is gone rather than
        # counting fires, which would race the 2-second schedule.
        _cloacinactl(
            home,
            "instance", "delete", "instance_params_workflow", "e2e_prod",
        )
        remaining = _psql(
            "SELECT count(*) FROM public.schedules "
            "WHERE workflow_name = 'instance_params_workflow' "
            "AND instance_name = 'e2e_prod';"
        ).strip()
        assert remaining == "0", f"instance row survived delete: {remaining!r}"
        print("  ok: instance deleted")

        # --- packaged defer_until (CLOACI-T-0897) -------------------------
        # A PACKAGED task taking a handle parameter could not even COMPILE
        # before this ticket (the macro emitted an ungated `::cloacina::`
        # path). Compiling is proven by a unit-level fixture build; what only
        # a live server can show is the rest: that the plugin receives a real
        # task-execution id, calls back into the host mid-execution, and that
        # its concurrency slot is genuinely RELEASED while it waits — the
        # entire point of defer_until.
        defer_dir = _stage_fixture(
            home, "defer-handle-rust", version_deps=version_deps
        )
        print("  compiling defer-handle fixture")
        defer_id = _upload(home, defer_dir)
        body = _poll_build_status(home, defer_id, {"success"}, timeout_s=900.0)
        assert body.get("build_status") == "success", body
        print("  ok: packaged task with a handle parameter BUILT")

        defer_exec = _poll_run_workflow(
            home, "defer_handle_workflow", timeout_s=120.0
        )
        print(f"  defer execution: {defer_exec}")

        # While it is deferred the row must say so. The fixture waits ~1.2s
        # with a 200ms poll, so this window is real but short — poll for it
        # rather than sleeping a fixed amount and hoping.
        saw_deferred = False
        deadline = time.time() + 30.0
        while time.time() < deadline:
            sub = _psql(
                "SELECT COALESCE(sub_status,'') FROM public.task_executions "
                f"WHERE workflow_execution_id = '{defer_exec}';"
            ).strip()
            if "Deferred" in sub:
                saw_deferred = True
                break
            if _cloacinactl(
                home, "-o", "json", "execution", "status", defer_exec
            )[1].find("Completed") != -1:
                break
            time.sleep(0.2)
        assert saw_deferred, (
            "task never reported sub_status=Deferred — the host callback that "
            "marks a deferral did not run"
        )
        print("  ok: task observed in Deferred state (host callback ran)")

        status = _poll_execution_status(
            home, defer_exec, {"Completed", "Failed", "Cancelled"}, timeout_s=60.0
        )
        assert status == "Completed", (
            f"deferred execution {defer_exec} ended in {status!r} — it should "
            "reclaim its slot and finish"
        )

        # The final context proves BOTH halves of the round trip: the plugin
        # was told a real task-execution id (not an empty string), and it
        # resumed after the deferral rather than erroring out of it.
        ctx = json.loads(
            _psql(
                "SELECT c.value FROM public.contexts c "
                "JOIN public.workflow_executions we ON we.context_id = c.id "
                f"WHERE we.id = '{defer_exec}';"
            ).strip()
        )
        observed_id = ctx.get("observed_task_execution_id") or ""
        assert len(observed_id) >= 32, (
            "plugin did not receive a real task-execution id "
            f"(got {observed_id!r}) — the v6 wire field is not arriving"
        )
        assert ctx.get("deferred_and_resumed") is True, (
            f"task did not resume after deferring: {ctx!r}"
        )
        print(
            "  ok: packaged defer_until round-tripped "
            f"(task id {observed_id[:8]}…, slot released and reclaimed)"
        )

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
