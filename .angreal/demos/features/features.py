"""demos features — run feature-focused Cloacina examples."""

import json
import shutil
import subprocess

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
_BESPOKE_FEATURES = ["python-workflow", "simple-packaged"]


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


# --- canonical packaged-workflow demo (the gold path) ------------------------
#
# `simple-packaged` is the CANONICAL example (CLOACI-I-0138): it runs through
# the PRIMARY interface — pack → upload → (server compiles & reconciles) →
# workflow run → execution Completed — never an in-process runner. It's in the
# auto-registration exclude list because `cargo run` is the wrong verb for it;
# this bespoke command drives the real lifecycle instead, reusing the
# service-lifecycle helpers from the compiler e2e harness.

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
    import tempfile
    from pathlib import Path

    from test.e2e.compiler import (
        _assert_ports_free,
        _build_binaries,
        _cloacinactl,
        _kill,
        _poll_build_status,
        _poll_execution_status,
        _poll_run_workflow,
        _start_postgres,
        _upload,
        _wait_http,
    )

    print("=== simple-packaged: packaged-workflow gold path ===")
    _build_binaries()
    _start_postgres()
    # Distinct ports from the other harnesses (compiler e2e 18083/19003,
    # ui e2e 18085/19001) so lanes can't collide on a shared machine.
    _assert_ports_free(18087, 19005)

    db_url = "postgres://cloacina:cloacina@localhost:15432/cloacina"
    bootstrap_key = "demo-simple-packaged-key"
    server_bind = "127.0.0.1:18087"
    compiler_bind = "127.0.0.1:19005"

    example_dir = PROJECT_ROOT / "examples" / "features" / "workflows" / "simple-packaged"
    home = Path(tempfile.mkdtemp(prefix="simple-packaged-demo-"))
    print(f"demo home (service logs): {home}")

    server_proc = None
    compiler_proc = None
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
            cwd=PROJECT_ROOT,
            stdout=server_log,
            stderr=subprocess.STDOUT,
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
                # DEV ESCAPE HATCH (CLOACI-T-0887): the example ships crates.io
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

        exec_id = _poll_run_workflow(home, "data_processing", timeout_s=180.0)
        print(f"  ok: workflow run accepted (execution {exec_id})")

        _poll_execution_status(home, exec_id, {"Completed"}, timeout_s=300.0)
        print("  ok: execution Completed")

        print(
            "\nSUCCESS: gold path verified — "
            "pack → upload → compile → reconcile → execute → Completed"
        )
        return 0
    finally:
        _kill(compiler_proc)
        _kill(server_proc)
