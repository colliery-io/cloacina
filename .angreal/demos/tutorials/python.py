"""demos tutorials python — run individual Python tutorial examples."""

import os
import shutil
import subprocess
import sys
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
    # All prints flush=True so CI logs land in order — buffered stdout
    # has burned us before (tutorial would exit silently between Step 5
    # and "Executing tutorial N..." with no diagnostic).
    project_root = PROJECT_ROOT
    tutorial_path = project_root / tutorial_rel_path
    python_tutorials_dir = tutorial_path.parent

    if not tutorial_path.exists():
        print(f"ERROR: Tutorial file not found: {tutorial_path}", flush=True)
        return 1

    venv_name = f"tutorial-{tutorial_num}-unified"
    venv_path = project_root / venv_name

    try:
        if backend == "postgres":
            print("Starting PostgreSQL container...", flush=True)
            if docker_up() != 0:
                raise Exception("Failed to start PostgreSQL container")
            print("Waiting for PostgreSQL to be ready...", flush=True)
            time.sleep(10)
            if not check_postgres_container_health():
                raise Exception("PostgreSQL container is not healthy")

        print("Building cloaca wheel and tutorial venv...", flush=True)
        _venv, python_exe, _pip_exe = _build_and_install_cloaca_unified(venv_name)

        print(f"[diagnostic] post-venv: tutorial_num={tutorial_num} backend={backend} "
              f"venv={venv_path} python={python_exe}", flush=True)

        if backend == "sqlite":
            for db_file in project_root.glob(f"python_tutorial_{tutorial_num}.db*"):
                try:
                    db_file.unlink()
                except FileNotFoundError:
                    pass
        elif backend == "postgres":
            print("Resetting PostgreSQL schema...", flush=True)
            reset_ok = smart_postgres_reset()
            print(f"[diagnostic] smart_postgres_reset returned {reset_ok}", flush=True)

        print(f"Executing tutorial {tutorial_num}...", flush=True)
        # The harness owns the DB wiring: the dev stack publishes postgres on
        # host 15432 (not 5432 — that's the user's own DB). Tutorials honor
        # DATABASE_URL and keep a user-facing 5432 fallback.
        env = os.environ.copy()
        if backend == "postgres":
            env["DATABASE_URL"] = "postgres://cloacina:cloacina@localhost:15432/cloacina"
        # `python -u` forces unbuffered stdio in the child so CI sees
        # progress + tracebacks even if the tutorial crashes mid-stream.
        result = subprocess.run(
            [str(python_exe), "-u", str(tutorial_path)],
            cwd=str(python_tutorials_dir),
            capture_output=True,
            text=True,
            timeout=300,
            env=env,
        )

        if result.returncode == 0:
            print(f"SUCCESS: Tutorial {tutorial_num} completed.", flush=True)
            print(result.stdout, flush=True)
            return 0
        print(f"FAILED: Tutorial {tutorial_num} failed (exit {result.returncode}).", flush=True)
        print("--- tutorial stderr ---", flush=True)
        print(result.stderr or "(empty)", flush=True)
        print("--- tutorial stdout ---", flush=True)
        print(result.stdout or "(empty)", flush=True)
        print("--- end tutorial output ---", flush=True)
        return 1

    except subprocess.TimeoutExpired as e:
        print(f"TIMEOUT: Tutorial {tutorial_num} timed out after 5 minutes", flush=True)
        if e.stdout:
            print("--- partial stdout ---", flush=True)
            print(e.stdout.decode("utf-8", errors="replace") if isinstance(e.stdout, bytes) else e.stdout, flush=True)
        if e.stderr:
            print("--- partial stderr ---", flush=True)
            print(e.stderr.decode("utf-8", errors="replace") if isinstance(e.stderr, bytes) else e.stderr, flush=True)
        return 1
    except Exception as e:
        # Print BOTH the exception summary AND the full traceback so CI
        # never has to guess what failed.
        import traceback as _tb
        print(f"ERROR: Tutorial {tutorial_num} setup failed: {type(e).__name__}: {e}", flush=True)
        print("--- traceback ---", flush=True)
        _tb.print_exc()
        sys.stdout.flush()
        sys.stderr.flush()
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
        tool=angreal.ToolDescription(
            f"Run Python Tutorial {number}. With `--backend postgres`, the cleanup path "
            "stops docker services and removes their volumes — any unrelated Postgres "
            "state in the shared compose stack is destroyed. Sqlite backend is volume-safe.",
            risk_level="destructive",
        ),
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
