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

# Bespoke feature demos defined below (excluded from auto-registration because
# `cargo run` is the wrong verb for them). Keep this list next to their
# definitions — `demos matrix` includes it so CI runs them too.
_BESPOKE_FEATURES = [
    "parameterized-workflow",
    "python-workflow",
    "simple-packaged",
    # "workflow-secrets" — command exists but is EXCLUDED from CI until
    # CLOACI-T-0895 lands (the fidius task protocol has no secrets channel, so
    # a packaged task cannot resolve `context.secret(...)` yet; the lane is the
    # verification vehicle for that fix).
]

# 32-byte demo KEK (base64) for the secrets lane — the same value the demo
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
    hand-maintained workflow list to drift."""
    names = sorted(
        [name.replace("_", "-") for name in _rust_feature_commands] + _BESPOKE_FEATURES
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

def _run_gold_path(label, example_dirname, run_steps, server_env=None):
    """Stand up dev-stack postgres + a host server + a host compiler
    (--dev-workspace so the examples' crates.io version deps resolve against
    this checkout), pack + upload the example, wait for the build, then call
    `run_steps(ctl, home)` for the example-specific run/observe assertions.
    `ctl(*args, check=True)` is a bound cloacinactl invoker. `server_env`
    adds/overrides env vars on the server process (e.g. the secrets KEK)."""
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
    # Distinct ports from the other harnesses (compiler e2e 18083/19003,
    # ui e2e 18085/19001) so lanes can't collide on a shared machine.
    _assert_ports_free(18087, 19005)

    db_url = "postgres://cloacina:cloacina@localhost:15432/cloacina"
    bootstrap_key = f"demo-{label}-key"
    server_bind = "127.0.0.1:18087"
    compiler_bind = "127.0.0.1:19005"

    example_dir = PROJECT_ROOT / "examples" / "features" / "workflows" / example_dirname
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


@demos()
@features()
@angreal.command(
    name="simple-packaged",
    about="run the canonical packaged example through the primary interface (pack → upload → compile → run)",
    long_about=(
        "Drives examples/features/workflows/simple-packaged the way the README "
        "says a user does: postgres (dev stack) + cloacina-server + "
        "cloacina-compiler (with --dev-workspace so the example's crates.io "
        "version deps resolve against this checkout), then cloacinactl "
        "pack → upload → poll the build to success → workflow run "
        "data_processing → poll the execution to Completed. First run "
        "cold-compiles the package deps (~5-10 min); warm cache finishes in "
        "under a minute."
    ),
    when_to_use=[
        "verifying the packaged/server gold path end to end",
        "validating the canonical example after compiler/server changes",
    ],
    when_not_to_use=[
        "compiler-pipeline regression assertions (use `test e2e compiler`)",
        "running without docker",
    ],
)
def simple_packaged():
    def steps(ctl, home):
        _run_to_completed(ctl, home, "data_processing")

    return _run_gold_path("simple-packaged", "simple-packaged", steps)


@demos()
@features()
@angreal.command(
    name="parameterized-workflow",
    about="run the parameterized-workflow example — params(...) declared, validated, and bound per run (CLOACI-T-0889)",
    long_about=(
        "Drives examples/features/workflows/parameterized-workflow through the "
        "primary interface: pack → upload → build, then runs the sync_file "
        "template TWICE with different --context param bindings (both must "
        "reach Completed) and once with a missing required param (the server "
        "must reject it with a typed validation error before anything runs)."
    ),
    when_to_use=[
        "verifying declared workflow params end to end (I-0116 surface)",
        "validating typed run-input validation after server changes",
    ],
    when_not_to_use=["running without docker"],
)
def parameterized_workflow():
    def steps(ctl, home):
        prod = home / "prod.json"
        prod.write_text('{"source": "/data/prod", "dst": "/backup/prod"}')
        _run_to_completed(ctl, home, "sync_file", context_path=prod)

        archive = home / "archive.json"
        archive.write_text(
            '{"source": "/data/archive", "dst": "/cold", "mode": "move", "max_files": 10}'
        )
        _run_to_completed(ctl, home, "sync_file", context_path=archive)

        # Missing required param `source` → the server must REJECT the run
        # before anything executes (typed input-interface validation, T-0757).
        bad = home / "bad.json"
        bad.write_text('{"dst": "/backup"}')
        code, out, err = ctl(
            "workflow", "run", "sync_file", "--context", str(bad), check=False
        )
        if code == 0:
            raise AssertionError(
                "run with a missing required param was ACCEPTED — declared-param "
                f"validation did not fire: {out!r}"
            )
        print("  ok: missing required param rejected before execution")

    return _run_gold_path("parameterized-workflow", "parameterized-workflow", steps)


@demos()
@features()
@angreal.command(
    name="workflow-secrets",
    about="run the workflow-secrets example — tenant secret created, bound via $secret, resolved at execution, never persisted (CLOACI-T-0890)",
    long_about=(
        "Drives examples/features/workflows/workflow-secrets through the "
        "primary interface with a secrets-enabled server (CLOACINA_SECRET_KEK "
        "set): cloacinactl secret create → run with a {\"$secret\": ...} "
        "binding → execution Completed (the task resolves the value through "
        "the side channel) → rotate → run again → assert `secret get` never "
        "returns values and a LITERAL value for the secret slot is rejected."
    ),
    when_to_use=[
        "verifying tenant secrets end to end (I-0133 surface)",
        "validating the secret side channel after server changes",
    ],
    when_not_to_use=["running without docker"],
)
def workflow_secrets():
    def steps(ctl, home):
        # Create the tenant secret; the value comes from a file — cloacinactl
        # refuses argv literals so secrets never land in shell history.
        token_file = home / "token.txt"
        token_file.write_text("s3cr3t-demo-token-value")
        ctl("secret", "create", "oncall_api", "--field", f"token=@{token_file}")
        print("  ok: tenant secret created (value from file, never argv)")

        # Bind the declared secret with a `$secret` reference and run.
        bind = home / "bind.json"
        bind.write_text(
            '{"channel": "#oncall", "api_token": {"$secret": "oncall_api"}}'
        )
        _run_to_completed(ctl, home, "notify_oncall", context_path=bind)

        # Rotate and run again — the next execution sees the new value.
        token_file.write_text("r0tated-demo-token-value!")
        ctl("secret", "rotate", "oncall_api", "--field", f"token=@{token_file}")
        print("  ok: secret rotated")
        _run_to_completed(ctl, home, "notify_oncall", context_path=bind)

        # Metadata-only reads: `secret get` must NEVER return a value.
        _, out, _ = ctl("-o", "json", "secret", "get", "oncall_api")
        if "s3cr3t" in out or "r0tated" in out:
            raise AssertionError(f"secret get leaked a value: {out!r}")
        print("  ok: secret get returns metadata only")

        # A LITERAL value for the declared secret slot must be rejected —
        # plaintext in the durable context is exactly what the design forbids.
        bad = home / "bad.json"
        bad.write_text('{"api_token": "plaintext-token"}')
        code, out, err = ctl(
            "workflow", "run", "notify_oncall", "--context", str(bad), check=False
        )
        if code == 0:
            raise AssertionError(
                f"literal secret value was ACCEPTED — validation did not fire: {out!r}"
            )
        print("  ok: literal secret value rejected before execution")

    return _run_gold_path(
        "workflow-secrets",
        "workflow-secrets",
        steps,
        server_env={"CLOACINA_SECRET_KEK": _DEMO_SECRET_KEK},
    )
