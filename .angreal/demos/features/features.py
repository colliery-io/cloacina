"""demos features — run feature-focused Cloacina examples."""

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
