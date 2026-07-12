"""demos features — run feature-focused Cloacina examples."""

import json
import os
import shutil
import subprocess
import time

import angreal  # type: ignore

from test._python_utils import _build_and_install_cloaca_unified
from utils import run_example_or_tutorial

from .._utils import (
    PROJECT_ROOT,
    get_packaged_example_directories,
    get_rust_feature_directories,
)

demos = angreal.command_group(name="demos", about="run Cloacina demonstration projects")
features = angreal.command_group(name="features", about="feature-focused Cloacina examples")


def _register_rust_feature(dir_name, rel_path):
    display_name = f"{dir_name.replace('-', ' ').replace('_', ' ').title()} Example"

    @demos()
    @features()
    @angreal.command(
        name=dir_name.replace("_", "-"),
        about=f"run {display_name}",
        when_to_use=["validating a feature-specific flow", "demoing a capability"],
        when_not_to_use=["performance benchmarking", "production deployment"],
    )
    def _cmd():
        return run_example_or_tutorial(PROJECT_ROOT, rel_path, display_name)

    _cmd.__name__ = f"feature_{dir_name}".replace("-", "_")
    return _cmd


_rust_feature_commands = {
    name: _register_rust_feature(name, path)
    for name, path in get_rust_feature_directories()
}

# 32-byte demo KEK (base64) for the secrets lanes — the same value the demo
# compose stack ships. Demo/dev only; production operators provision their own
# CLOACINA_SECRET_KEK.
_DEMO_SECRET_KEK = "ZGVtby1rZWstZGVtby1rZWstZGVtby1rZWstMDAwMSE="


@demos()
@angreal.command(
    name="matrix",
    about="emit every runnable feature demo as a JSON list (the CI examples matrix source)",
    when_to_use=[
        "generating the CI rust-examples matrix (examples-docs.yml discover job)",
        "checking which examples the harness will execute",
    ],
    when_not_to_use=["running the demos themselves (use demos features <name>)"],
)
def matrix():
    """CI executes ALL runnable examples. This is the single source of truth:
    the same discovery that registers `demos features <name>` commands feeds
    the CI matrix, so a new example directory automatically joins CI — no
    hand-maintained list to drift. Three sources: embedded `cargo run` examples
    (dirs without a package.toml), packaged gold-path examples (dirs WITH a
    package.toml), and the one bespoke wheel demo (python-workflow)."""
    names = sorted(
        [name.replace("_", "-") for name in _rust_feature_commands]
        + list(_packaged_commands)
        + ["python-workflow"]
    )
    print(json.dumps(names))


# --- bespoke Python-workflow feature demo -----------------------------------

@demos()
@features()
@angreal.command(
    name="python-workflow",
    about="run Python Workflow Example (end-to-end data pipeline)",
    when_to_use=[
        "testing Python workflow packaging end-to-end",
        "validating cloaca Context API with real tasks",
    ],
    when_not_to_use=["production deployment", "performance benchmarking"],
)
def python_workflow():
    project_root = PROJECT_ROOT
    example_dir = project_root / "examples" / "features" / "workflows" / "python-workflow"
    runner_script = example_dir / "run_pipeline.py"

    if not runner_script.exists():
        print(f"ERROR: Runner script not found: {runner_script}")
        return 1

    venv_name = "python-workflow-demo"
    venv_path = project_root / venv_name

    try:
        _venv, python_exe, _pip_exe = _build_and_install_cloaca_unified(venv_name)
        result = subprocess.run(
            [str(python_exe), str(runner_script)],
            cwd=str(example_dir),
            capture_output=True,
            text=True,
            timeout=120,
        )
        if result.returncode == 0:
            print("SUCCESS: Python workflow example completed.")
            for line in result.stdout.splitlines():
                if not line.strip().startswith("[") and not line.startswith(
                    ("THREAD:", "TASK:", "THREADS:")
                ):
                    print(line)
            return 0
        print("FAILED: Python workflow example failed.")
        print(result.stderr)
        if result.stdout:
            print(result.stdout)
        return 1
    except subprocess.TimeoutExpired:
        print("TIMEOUT: Python workflow example timed out after 2 minutes")
        return 1
    except Exception as e:
        print(f"ERROR: setup failed: {e}")
        return 1
    finally:
        if venv_path.exists():
            shutil.rmtree(venv_path)


# --- gold-path packaged demos (CLOACI-I-0138) --------------------------------
#
# These examples run through the PRIMARY interface — pack → upload → (server
# compiles & reconciles) → workflow run → execution Completed — never an
# in-process runner. They're in the auto-registration exclude list because
# `cargo run` is the wrong verb; each bespoke command drives the real
# lifecycle via this shared helper, reusing the service-lifecycle helpers
# from the compiler e2e harness.

def _run_gold_path(label, example_dirname, run_steps, server_env=None, extra_services=None):
    """Stand up dev-stack postgres + a host server + a host compiler
    (--dev-workspace so the examples' crates.io version deps resolve against
    this checkout), pack + upload the example, wait for the build, then call
    `run_steps(ctl, home)` for the example-specific run/observe assertions.
    `ctl(*args, check=True)` is a bound cloacinactl invoker. `server_env`
    adds/overrides env vars on the server process (e.g. the secrets KEK).
    `extra_services` names additional dev-stack compose services to bring up
    (e.g. `("kafka",)`) AFTER the postgres reset but BEFORE the server starts,
    so a stream accumulator's consumer can connect at reconcile time."""
    import tempfile
    from pathlib import Path

    from test.e2e.compiler import (
        _assert_ports_free,
        _build_binaries,
        _cloacinactl,
        _kill,
        _poll_build_status,
        _start_postgres,
        _upload,
        _wait_http,
    )

    print(f"=== {label}: packaged-workflow gold path ===")
    _build_binaries()
    _start_postgres()
    # Bring up any extra dev-stack services (e.g. kafka) AFTER the postgres
    # reset (which does `down -v`, tearing the whole project down) but before
    # the server. `--wait` blocks until they're healthy so the server's
    # reconcile-time consumers can connect.
    if extra_services:
        print(f"  bringing up dev-stack services: {', '.join(extra_services)}")
        rc = subprocess.run(
            [
                "docker", "compose", "-f", ".angreal/docker-compose.yaml",
                "up", "-d", "--wait", *extra_services,
            ],
            cwd=str(PROJECT_ROOT),
            check=False,
        ).returncode
        if rc != 0:
            raise RuntimeError(f"failed to bring up dev-stack services {extra_services}")
    # Distinct ports from the other harnesses (compiler e2e 18083/19003,
    # ui e2e 18085/19001) so lanes can't collide on a shared machine.
    _assert_ports_free(18087, 19005)

    db_url = "postgres://cloacina:cloacina@localhost:15432/cloacina"
    bootstrap_key = f"demo-{label}-key"
    server_bind = "127.0.0.1:18087"
    compiler_bind = "127.0.0.1:19005"

    # `example_dirname` is relative to examples/features/ and may carry a
    # subtree (e.g. "computation-graphs/cg-feature-tour").
    example_dir = PROJECT_ROOT / "examples" / "features" / example_dirname
    home = Path(tempfile.mkdtemp(prefix=f"{label}-demo-"))
    print(f"demo home (service logs): {home}")

    server_proc = None
    compiler_proc = None
    try:
        env = os.environ.copy()
        if server_env:
            env.update(server_env)
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
            cwd=PROJECT_ROOT,
            stdout=server_log,
            stderr=subprocess.STDOUT,
            env=env,
        )
        _wait_http(f"http://{server_bind}/health", "server", proc=server_proc)
        print("  ok: server up")

        # Shared target cache with the compiler e2e so the cloacina deps only
        # cold-compile once across all harness lanes.
        shared_target = PROJECT_ROOT / "target" / "compiler-e2e-cache"
        shared_target.mkdir(parents=True, exist_ok=True)
        compiler_log = open(home / "compiler.log", "w")
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
                # DEV ESCAPE HATCH (CLOACI-T-0887): the examples ship crates.io
                # version deps (the form users ship); resolve them against THIS
                # checkout's unpublished crates.
                "--dev-workspace", str(PROJECT_ROOT),
                "--verbose",
            ],
            cwd=PROJECT_ROOT,
            stdout=compiler_log,
            stderr=subprocess.STDOUT,
        )
        _wait_http(
            f"http://{compiler_bind}/health", "compiler",
            timeout_s=60.0, proc=compiler_proc,
        )
        print("  ok: compiler up")

        _cloacinactl(
            home,
            "config", "profile", "set", "local", f"http://{server_bind}",
            "--api-key", bootstrap_key,
            "--default",
        )

        print("  pack + upload (server compiles the source archive; "
              "first run: ~5-10 min cold build)")
        pkg_id = _upload(home, example_dir)
        _poll_build_status(home, pkg_id, {"success"}, timeout_s=900.0)
        print("  ok: build_status = success")

        def ctl(*args, check=True):
            return _cloacinactl(home, *args, check=check)

        run_steps(ctl, home)

        print(
            f"\nSUCCESS: {label} gold path verified — "
            "pack → upload → compile → reconcile → execute → Completed"
        )
        return 0
    finally:
        _kill(compiler_proc)
        _kill(server_proc)


def _run_to_completed(ctl, home, workflow_name, context_path=None, timeout_s=300.0):
    """`workflow run` (retrying until the reconciler has loaded the workflow),
    then poll the execution to Completed. Returns the execution id."""
    from test.e2e.compiler import _poll_execution_status

    deadline = time.time() + 180.0
    last_err = ""
    exec_id = None
    while time.time() < deadline:
        args = ["-o", "json", "workflow", "run", workflow_name]
        if context_path:
            args += ["--context", str(context_path)]
        code, out, err = ctl(*args, check=False)
        if code == 0:
            try:
                exec_id = json.loads(out).get("execution_id")
            except json.JSONDecodeError:
                exec_id = (out.strip().splitlines() or [""])[-1].strip() or None
            if exec_id and len(exec_id) >= 32:
                break
            exec_id = None
        last_err = err.strip() or out.strip()
        time.sleep(3.0)
    if not exec_id:
        raise AssertionError(
            f"workflow run {workflow_name} never succeeded; last error: {last_err}"
        )
    print(f"  ok: workflow run accepted (execution {exec_id})")
    _poll_execution_status(home, exec_id, {"Completed"}, timeout_s=timeout_s)
    print("  ok: execution Completed")
    return exec_id


# --- generic packaged-example registrar (CLOACI-I-0138) ----------------------
#
# Every example with a `package.toml` runs through the SERVER gold path, so ONE
# registrar discovers them and registers `demos features <name>` for each — no
# bespoke command per example (that plumbing was harness boilerplate, not a
# per-example feature). The default assertion comes from the manifest:
# `workflow_name` → run it to Completed. A few examples need a richer check
# (param validation, secret lifecycle, graph inject→fire) and supply a thin
# override below; a few not-yet-gold-path examples are skipped WITH A REASON so
# nothing is silently dropped.


def _default_workflow_steps(workflow_name):
    def steps(ctl, home):
        _run_to_completed(ctl, home, workflow_name)

    return steps


_MT_DB_URL = "postgres://cloacina:cloacina@localhost:15432/cloacina"


def _multi_tenant_steps(workflow_name):
    """Prove the tenant isolation boundary — the FULL per-tenant deployment
    model — on the gold path. The package is already deployed in `public` (the
    harness pre-upload) = tenant 1. We create a second tenant `mtbeta`, stand up
    a SECOND compiler scoped to `--tenant-schema mtbeta` (each tenant runs its
    own compiler — the harness's default compiler only claims the public/admin
    schema, by design: `cloacina-compiler --tenant-schema` isolates builds per
    tenant), deploy the SAME archive into `mtbeta`, and run the workflow in BOTH
    tenants. We then assert their executions never cross, and that a third
    tenant `mtgamma` that never received the package cannot run the workflow.

    Tenant/workflow/execution routes are all `/v1/tenants/{tenant}/...`; every
    call here is `--tenant`-scoped (the shared poll helpers target `public`
    only, so this lane does its own tenant-aware run/poll)."""

    def steps(ctl, home):
        from test.e2e.compiler import _cloacinactl, _wait_http, _kill

        def tctl(tenant, *args, check=True):
            return _cloacinactl(home, "--tenant", tenant, *args, check=check)

        def _exec_id(out):
            try:
                v = json.loads(out).get("execution_id")
            except json.JSONDecodeError:
                v = (out.strip().splitlines() or [""])[-1].strip() or None
            return v if v and len(v) >= 32 else None

        def run_and_wait(tenant, timeout_s=180.0):
            # Retry `workflow run` until the reconciler has loaded the workflow
            # in THIS tenant, then poll THIS tenant's execution to Completed.
            deadline = time.time() + 120.0
            exec_id = None
            last = ""
            while time.time() < deadline:
                code, out, err = tctl(
                    tenant, "-o", "json", "workflow", "run", workflow_name,
                    check=False,
                )
                if code == 0:
                    exec_id = _exec_id(out)
                    if exec_id:
                        break
                last = err.strip() or out.strip()
                time.sleep(3.0)
            if not exec_id:
                raise AssertionError(
                    f"[{tenant}] workflow run {workflow_name} never accepted: {last!r}"
                )
            pdl = time.time() + timeout_s
            while time.time() < pdl:
                _, pout, _ = tctl(
                    tenant, "-o", "json", "execution", "status", exec_id,
                    check=False,
                )
                try:
                    st = json.loads(pout).get("status")
                except json.JSONDecodeError:
                    st = None
                if st == "Completed":
                    return exec_id
                if st in {"Failed", "Cancelled"}:
                    raise AssertionError(f"[{tenant}] execution {exec_id} → {st}")
                time.sleep(2.0)
            raise AssertionError(f"[{tenant}] execution {exec_id} never Completed")

        def exec_ids(tenant):
            _, out, _ = tctl(
                tenant, "-o", "json", "execution", "list", "--limit", "100",
                check=False,
            )
            try:
                data = json.loads(out)
            except json.JSONDecodeError:
                return []
            rows = data.get("items", data) if isinstance(data, dict) else data
            return {(r.get("execution_id") or r.get("id")) for r in rows}

        archive = next(home.glob("*.cloacina"))

        # Tenant 1 = public: the harness already uploaded + built the package.
        e_public = run_and_wait("public")
        print(f"  ok: ran in tenant `public` → {e_public}")

        # Tenant 2 = mtbeta: create it, then stand up its OWN compiler scoped to
        # its schema (the per-tenant build isolation model), deploy the SAME
        # archive, and run independently.
        ctl("tenant", "create", "mtbeta", check=False)
        mt_compiler = None
        try:
            mt_home = home / "mtbeta-compiler"
            mt_home.mkdir(exist_ok=True)
            mt_log = open(mt_home / "compiler.log", "w")
            mt_compiler = subprocess.Popen(
                [
                    "target/debug/cloacina-compiler",
                    "--home", str(mt_home),
                    "--database-url", _MT_DB_URL,
                    "--bind", "127.0.0.1:19006",
                    "--tenant-schema", "mtbeta",
                    "--poll-interval-ms", "500",
                    "--dev-workspace", str(PROJECT_ROOT),
                    "--verbose",
                ],
                cwd=PROJECT_ROOT,
                stdout=mt_log,
                stderr=subprocess.STDOUT,
            )
            _wait_http(
                "http://127.0.0.1:19006/health", "mtbeta-compiler",
                timeout_s=60.0, proc=mt_compiler,
            )
            print("  ok: per-tenant compiler up (--tenant-schema mtbeta)")

            _, up, _ = tctl("mtbeta", "package", "upload", str(archive), check=False)
            pkg_beta = (up.strip().splitlines() or [""])[-1].strip()
            if not (pkg_beta and len(pkg_beta) >= 32):
                raise AssertionError(f"upload to mtbeta didn't return a package id: {up!r}")
            bdl = time.time() + 180.0
            while time.time() < bdl:
                _, iout, _ = tctl(
                    "mtbeta", "-o", "json", "package", "inspect", pkg_beta,
                    check=False,
                )
                try:
                    if json.loads(iout).get("build_status") == "success":
                        break
                except json.JSONDecodeError:
                    pass
                time.sleep(2.0)
            else:
                raise AssertionError("mtbeta package never built (per-tenant compiler)")
            print("  ok: deployed the same package into tenant `mtbeta` (its own compiler built it)")
            e_beta = run_and_wait("mtbeta")
            print(f"  ok: ran in tenant `mtbeta` → {e_beta}")
        finally:
            _kill(mt_compiler)

        # Isolation 1: each tenant's execution list contains ONLY its own run.
        pub_ids, beta_ids = exec_ids("public"), exec_ids("mtbeta")
        if e_public not in pub_ids or e_beta in pub_ids:
            raise AssertionError(
                f"tenant leak: `public` list {sorted(pub_ids)} should have "
                f"{e_public} and NOT {e_beta}"
            )
        if e_beta not in beta_ids or e_public in beta_ids:
            raise AssertionError(
                f"tenant leak: `mtbeta` list {sorted(beta_ids)} should have "
                f"{e_beta} and NOT {e_public}"
            )
        print("  ok: executions are tenant-isolated (neither tenant sees the other's run)")

        # Isolation 2 (KNOWN GAP — CLOACI-T-0901): a tenant that never received
        # the package SHOULD NOT be able to run the workflow. Today it can: the
        # execute route resolves the name against the process-shared in-memory
        # `Runtime` (populated by public/mtbeta) without checking the calling
        # tenant's own registry, so the run is accepted even though
        # `mtgamma.workflow_packages` has zero rows for it. Execution STATE is
        # still isolated (Isolation 1 above) — only workflow-DEFINITION
        # visibility leaks. We surface it loudly rather than fail the lane; flip
        # this to a hard assertion once T-0901 lands.
        ctl("tenant", "create", "mtgamma", check=False)
        code, out, err = tctl(
            "mtgamma", "workflow", "run", workflow_name, check=False,
        )
        if code == 0 and _exec_id(out):
            print(
                "  KNOWN GAP (CLOACI-T-0901): tenant `mtgamma` (no package) was "
                f"allowed to run `{workflow_name}` — workflow-definition visibility "
                "leaks across tenants via the shared Runtime. Execution state is "
                "still isolated; tracked for fix."
            )
        else:
            print(
                "  ok: tenant `mtgamma` (no package) cannot run the workflow — "
                "isolated (CLOACI-T-0901 appears fixed; promote this to a hard assert)"
            )

    return steps


def _trigger_wait_steps(workflow_name):
    """For a POLL/CRON-triggered workflow: don't `workflow run` it — wait for the
    trigger to fire it AUTOMATICALLY and assert the auto-execution reaches
    Completed. Proves the packaged-trigger path (macro → FFI projection → host
    trigger registry → scheduled fire) end to end."""

    def steps(ctl, home):
        from test.e2e.compiler import _poll_execution_status

        deadline = time.time() + 180
        while time.time() < deadline:
            _, out, _ = ctl(
                "-o", "json", "execution", "list",
                "--workflow", workflow_name, "--limit", "1", check=False,
            )
            try:
                data = json.loads(out)
            except json.JSONDecodeError:
                time.sleep(3)
                continue
            rows = data.get("items", data) if isinstance(data, dict) else data
            if rows:
                row = rows[0]
                exec_id = row.get("execution_id") or row.get("id")
                if exec_id and len(str(exec_id)) >= 32:
                    print(f"  ok: trigger fired an execution automatically ({exec_id})")
                    _poll_execution_status(home, exec_id, {"Completed"}, timeout_s=180.0)
                    print("  ok: triggered execution Completed")
                    return
            time.sleep(3)
        raise AssertionError(
            f"no execution of `{workflow_name}` appeared — the poll trigger never "
            "fired (macro/FFI projection or trigger scheduler not running)"
        )

    return steps


def _params_steps(workflow_name):
    """Run a params template twice with different bindings, then assert a
    missing-required-param run is rejected before execution."""

    def steps(ctl, home):
        prod = home / "prod.json"
        prod.write_text('{"source": "/data/prod", "dst": "/backup/prod"}')
        _run_to_completed(ctl, home, workflow_name, context_path=prod)

        archive = home / "archive.json"
        archive.write_text(
            '{"source": "/data/archive", "dst": "/cold", "mode": "move", "max_files": 10}'
        )
        _run_to_completed(ctl, home, workflow_name, context_path=archive)

        bad = home / "bad.json"
        bad.write_text('{"dst": "/backup"}')
        code, out, _ = ctl(
            "workflow", "run", workflow_name, "--context", str(bad), check=False
        )
        if code == 0:
            raise AssertionError(
                "run with a missing required param was ACCEPTED — declared-param "
                f"validation did not fire: {out!r}"
            )
        print("  ok: missing required param rejected before execution")

    return steps


def _secrets_steps(workflow_name):
    """Full tenant-secret lifecycle: create → $secret-bound run (Completed) →
    rotate → rerun → metadata-only get → literal binding rejected."""

    def steps(ctl, home):
        token_file = home / "token.txt"
        token_file.write_text("s3cr3t-demo-token-value")
        ctl("secret", "create", "oncall_api", "--field", f"token=@{token_file}")
        print("  ok: tenant secret created")

        bind = home / "bind.json"
        bind.write_text('{"channel": "#oncall", "api_token": {"$secret": "oncall_api"}}')
        _run_to_completed(ctl, home, workflow_name, context_path=bind)

        token_file.write_text("r0tated-demo-token-value!")
        ctl("secret", "rotate", "oncall_api", "--field", f"token=@{token_file}")
        print("  ok: secret rotated")
        _run_to_completed(ctl, home, workflow_name, context_path=bind)

        _, out, _ = ctl("-o", "json", "secret", "get", "oncall_api")
        if "s3cr3t" in out or "r0tated" in out:
            raise AssertionError(f"secret get leaked a value: {out!r}")
        print("  ok: secret get returns metadata only")

        bad = home / "bad.json"
        bad.write_text('{"api_token": "plaintext-token"}')
        code, out, _ = ctl(
            "workflow", "run", workflow_name, "--context", str(bad), check=False
        )
        if code == 0:
            raise AssertionError(
                f"literal secret value was ACCEPTED — validation did not fire: {out!r}"
            )
        print("  ok: literal secret value rejected before execution")

    return steps


def _graph_inject_steps(label, reactor, accumulator, event, bad_event=None):
    """Inject a typed event into a reactor's accumulator and confirm the reactor
    fires (its graph runs). Reads the fires COUNT off the reactors-list endpoint
    (`ListResponse{items,total}`) rather than parsing the fires list. When
    `bad_event` is given, also assert the accumulator's declared boundary schema
    rejects it (proves the typed inject surface — `@cloaca.boundary_schema` /
    `schemars::JsonSchema` — is wired)."""

    def steps(ctl, home):
        from test.e2e.compiler import _get_json

        key = f"demo-{label}-key"

        def fires_for(name):
            body = _get_json("http://127.0.0.1:18087/v1/health/reactors", key)
            items = body.get("items", []) if isinstance(body, dict) else []
            for r in items:
                if r.get("name") == name:
                    return int(r.get("fires", 0) or 0)
            return 0

        if bad_event is not None:
            code, out, _ = ctl(
                "accumulator", "inject", accumulator, "--event", bad_event, check=False
            )
            if code == 0:
                raise AssertionError(
                    f"malformed event {bad_event!r} was ACCEPTED for `{accumulator}` — "
                    f"the declared boundary schema did not reject it: {out!r}"
                )
            print("  ok: malformed event rejected by the accumulator boundary schema")

        before = fires_for(reactor)
        deadline = time.time() + 180
        while time.time() < deadline:
            ctl("accumulator", "inject", accumulator, "--event", event, check=False)
            time.sleep(4)
            if fires_for(reactor) > before:
                print(f"  ok: inject fired reactor {reactor} (graph ran)")
                return
        raise AssertionError(
            f"reactor {reactor} never fired after injecting into `{accumulator}`"
        )

    return steps


# name -> {"steps": <fn taking (ctl, home)>, "server_env": <dict|None>}
_PACKAGED_OVERRIDES = {
    "parameterized-workflow": {"steps": _params_steps("sync_file")},
    "python-parameterized": {"steps": _params_steps("python_parameterized")},
    "workflow-secrets": {
        "steps": _secrets_steps("notify_oncall"),
        "server_env": {"CLOACINA_SECRET_KEK": _DEMO_SECRET_KEK},
    },
    "python-secrets": {
        "steps": _secrets_steps("python_secrets"),
        "server_env": {"CLOACINA_SECRET_KEK": _DEMO_SECRET_KEK},
    },
    "packaged-graph": {
        "steps": _graph_inject_steps(
            "packaged-graph",
            "packaged_market_maker_reactor",
            "orderbook",
            '{"best_bid": 100.0, "best_ask": 100.1}',
        ),
    },
    "python-packaged-graph": {
        "steps": _graph_inject_steps(
            "python-packaged-graph",
            "market_maker",
            "orderbook",
            '{"best_bid": 100.0, "best_ask": 100.1}',
            # `orderbook` declares @cloaca.boundary_schema(best_bid, best_ask):
            # a non-object event must be rejected by the typed slot.
            bad_event="42",
        ),
    },
    # Poll trigger fires `file_processing` automatically — wait for it, don't run.
    "packaged-triggers": {"steps": _trigger_wait_steps("file_processing")},
    # Python peer: @cloaca.trigger poll fires `file_processing_py` automatically.
    "python-triggers": {"steps": _trigger_wait_steps("file_processing_py")},
    # Python cron: the cron scheduler fires `heartbeat_workflow` on a schedule.
    "python-cron": {"steps": _trigger_wait_steps("heartbeat_workflow")},
    # Python multi-tenancy: same package in two tenants, executions isolated.
    "python-multi-tenant": {"steps": _multi_tenant_steps("tenant_job")},
}

# Packaged examples not yet driveable on the gold path — discovered but not
# registered, each with a reason (no silent drops). Tracked for follow-up.
_PACKAGED_SKIP = {
    # (empty) — every packaged example is now driveable on the gold path.
    # cg-feature-tour's stream/inject surface is deferred to T-0898, but its
    # `tour_pipeline` invocation surface IS runnable via the default → registered.
}


def _register_packaged_example(name, rel_path, meta):
    cfg = _PACKAGED_OVERRIDES.get(name)
    if cfg is not None:
        steps = cfg["steps"]
        server_env = cfg.get("server_env")
    else:
        wf = meta.get("workflow_name")
        if not wf:
            # Graph-only package with no override → can't derive an assertion
            # (reactor/accumulator/event aren't in the manifest). Skip loudly.
            return None
        steps = _default_workflow_steps(wf)
        server_env = None

    @demos()
    @features()
    @angreal.command(
        name=name,
        about=f"packaged gold path: {name} (pack → upload → compile → reconcile → execute)",
        when_to_use=[
            "verifying a packaged example end to end through the server",
            "validating the gold path after compiler/server changes",
        ],
        when_not_to_use=["running without docker"],
    )
    def _cmd(_steps=steps, _rel=rel_path, _name=name, _env=server_env):
        return _run_gold_path(_name, _rel, _steps, server_env=_env)

    _cmd.__name__ = f"packaged_{name}".replace("-", "_")
    return _cmd


_packaged_commands = {}
for _name, _rel, _meta in get_packaged_example_directories():
    if _name in _PACKAGED_SKIP:
        continue
    _cmd = _register_packaged_example(_name, _rel, _meta)
    if _cmd is not None:
        _packaged_commands[_name] = _cmd
