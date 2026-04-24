"""demos tutorials python — run individual Python tutorial examples."""

import shutil
import subprocess
import time

import angreal  # type: ignore

from test._python_utils import _build_and_install_cloaca_unified
from utils import (
    docker_up,
    docker_down,
    check_postgres_container_health,
    smart_postgres_reset,
)

from .._utils import (
    PROJECT_ROOT,
    get_python_tutorial_files,
)

demos = angreal.command_group(name="demos", about="run Cloacina demonstration projects")
tutorials = angreal.command_group(name="tutorials", about="run tutorial examples")
python_group = angreal.command_group(name="python", about="Python tutorial examples")


def _run_python_tutorial(tutorial_num, tutorial_rel_path, backend="sqlite"):
    project_root = PROJECT_ROOT
    tutorial_path = project_root / tutorial_rel_path
    python_tutorials_dir = tutorial_path.parent

    if not tutorial_path.exists():
        print(f"ERROR: Tutorial file not found: {tutorial_path}")
        return 1

    venv_name = f"tutorial-{tutorial_num}-unified"
    venv_path = project_root / venv_name

    try:
        if backend == "postgres":
            print("Starting PostgreSQL container...")
            if docker_up() != 0:
                raise Exception("Failed to start PostgreSQL container")
            print("Waiting for PostgreSQL to be ready...")
            time.sleep(10)
            if not check_postgres_container_health():
                raise Exception("PostgreSQL container is not healthy")

        print("Building cloaca wheel and tutorial venv...")
        _venv, python_exe, _pip_exe = _build_and_install_cloaca_unified(venv_name)

        print(f"Executing tutorial {tutorial_num}...")
        if backend == "sqlite":
            for db_file in project_root.glob(f"python_tutorial_{tutorial_num}.db*"):
                try:
                    db_file.unlink()
                except FileNotFoundError:
                    pass
        elif backend == "postgres":
            smart_postgres_reset()

        result = subprocess.run(
            [str(python_exe), str(tutorial_path)],
            cwd=str(python_tutorials_dir),
            capture_output=True,
            text=True,
            timeout=300,
        )

        if result.returncode == 0:
            print(f"SUCCESS: Tutorial {tutorial_num} completed.")
            print(result.stdout)
            return 0
        print(f"FAILED: Tutorial {tutorial_num} failed (exit {result.returncode}).")
        print(result.stderr)
        if result.stdout:
            print(result.stdout)
        return 1

    except subprocess.TimeoutExpired:
        print(f"TIMEOUT: Tutorial {tutorial_num} timed out after 5 minutes")
        return 1
    except Exception as e:
        print(f"ERROR: Tutorial {tutorial_num} setup failed: {e}")
        return 1
    finally:
        if backend == "postgres":
            docker_down(remove_volumes=True)
        if venv_path.exists():
            shutil.rmtree(venv_path)


def _register(tutorial_file, tutorial_rel_path):
    parts = tutorial_file.replace(".py", "").split("_")
    if parts[0].isdigit():
        number = parts[0]
    else:
        number = "??"
    leaf = number

    @demos()
    @tutorials()
    @python_group()
    @angreal.command(
        name=leaf,
        about=f"run Python Tutorial {number}",
        when_to_use=["learning Cloacina's Python surface", "validating a tutorial change"],
        when_not_to_use=["production deployment", "performance benchmarking"],
    )
    @angreal.argument(
        name="backend",
        long="backend",
        help="Database backend (postgres/sqlite, default: sqlite)",
        required=False,
    )
    def _cmd(backend=None):
        backend = backend or "sqlite"
        if backend not in ("postgres", "sqlite"):
            print(f"Error: invalid backend '{backend}' (use 'postgres' or 'sqlite').")
            return 1
        if number == "06" and backend == "sqlite":
            print("Tutorial 06 (multi-tenancy) requires PostgreSQL; switching backend.")
            docker_up()
            backend = "postgres"
        return _run_python_tutorial(number, tutorial_rel_path, backend)

    _cmd.__name__ = f"python_tutorial_{number}"
    return _cmd


_commands = {
    fname: _register(fname, path) for fname, path in get_python_tutorial_files()
}
