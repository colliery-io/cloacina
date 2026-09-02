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


def _fresh_tutorial_db(db_name):
    """Drop + recreate a dedicated database on the shared postgres instance.

    The `all` runner gives every tutorial its own database instead of
    resetting one shared schema between runs — no cross-tutorial state, no
    reset races, and a retry gets a genuinely fresh DB. Two `-c` flags:
    CREATE/DROP DATABASE cannot run inside the implicit transaction a single
    multi-statement `-c` would use.
    """
    env = os.environ.copy()
    env.setdefault("PGPASSWORD", "cloacina")
    drop = f'DROP DATABASE IF EXISTS "{db_name}" WITH (FORCE)'
    create = f'CREATE DATABASE "{db_name}"'
    for cmd in (
        ["psql", "-h", "localhost", "-p", "15432", "-U", "cloacina", "-d", "cloacina",
         "-v", "ON_ERROR_STOP=1", "-c", drop, "-c", create],
        ["docker", "exec", "cloacina-postgres", "psql", "-U", "cloacina", "-d", "cloacina",
         "-v", "ON_ERROR_STOP=1", "-c", drop, "-c", create],
    ):
        try:
            proc = subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=30)
            if proc.returncode == 0:
                return True
        except (OSError, subprocess.TimeoutExpired):
            continue
    return False


def _run_python_tutorial(tutorial_num, tutorial_rel_path, backend="sqlite", python_exe=None,
                         db_name=None):
    """Run one Python tutorial.

    Standalone form (``python_exe`` is None): builds a fresh cloaca wheel +
    venv, brings the postgres stack up/down around the run, and removes the
    venv afterwards. Shared form (``python_exe`` given — the `all` command):
    the caller owns the wheel, the venv, and the services; this only resets
    the database state and executes the tutorial. With ``db_name`` the
    tutorial runs against its own dedicated database on the shared instance
    instead of resetting the default schema.
    """
    # All prints flush=True so CI logs land in order — buffered stdout
    # has burned us before (tutorial would exit silently between Step 5
    # and "Executing tutorial N..." with no diagnostic).
    project_root = PROJECT_ROOT
    tutorial_path = project_root / tutorial_rel_path
    python_tutorials_dir = tutorial_path.parent

    if not tutorial_path.exists():
        print(f"ERROR: Tutorial file not found: {tutorial_path}", flush=True)
        return 1

    owns_env = python_exe is None
    venv_name = f"tutorial-{tutorial_num}-unified"
    venv_path = project_root / venv_name

    try:
        if backend == "postgres" and owns_env:
            print("Starting PostgreSQL container...", flush=True)
            if docker_up() != 0:
                raise Exception("Failed to start PostgreSQL container")
            print("Waiting for PostgreSQL to be ready...", flush=True)
            time.sleep(10)
            if not check_postgres_container_health():
                raise Exception("PostgreSQL container is not healthy")

        if owns_env:
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
            if db_name:
                print(f"Provisioning dedicated database {db_name}...", flush=True)
                if not _fresh_tutorial_db(db_name):
                    raise Exception(f"failed to provision database {db_name}")
            else:
                print("Resetting PostgreSQL schema...", flush=True)
                reset_ok = smart_postgres_reset()
                print(f"[diagnostic] smart_postgres_reset returned {reset_ok}", flush=True)

        print(f"Executing tutorial {tutorial_num}...", flush=True)
        # The harness owns the DB wiring: the dev stack publishes postgres on
        # host 15432 (not 5432 — that's the user's own DB). Tutorials honor
        # DATABASE_URL and keep a user-facing 5432 fallback.
        env = os.environ.copy()
        if backend == "postgres":
            env["DATABASE_URL"] = (
                f"postgres://cloacina:cloacina@localhost:15432/{db_name or 'cloacina'}"
            )
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
        if owns_env:
            if backend == "postgres":
                docker_down(remove_volumes=True)
            if venv_path.exists():
                shutil.rmtree(venv_path)


def _resolve_backend(number, backend):
    """The per-tutorial backend rule: 06 (multi-tenancy) is postgres-only."""
    if number == "06" and backend == "sqlite":
        return "postgres"
    return backend


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
        if _resolve_backend(number, backend) != backend:
            print("Tutorial 06 (multi-tenancy) requires PostgreSQL; switching backend.")
            docker_up()
            backend = "postgres"
        return _run_python_tutorial(number, tutorial_rel_path, backend)

    _cmd.__name__ = f"python_tutorial_{number}"
    return _cmd


_commands = {
    fname: _register(fname, path) for fname, path in get_python_tutorial_files()
}


# --- `demos tutorials python all` — every tutorial, one wheel -----------------
#
# CI used to run a 21-leg matrix (11 tutorials x 2 backends), and every leg
# rebuilt the cloaca wheel from scratch in release mode (~7 min) to run a
# tutorial that takes seconds. This builds the wheel ONCE and runs every
# tutorial against it for each requested backend, with the database reset
# between tutorials exactly as the standalone commands do.

# Tutorial 05 (cron scheduling) hardcodes postgres; it has no sqlite form.
_POSTGRES_ONLY = {"05"}


@demos()
@tutorials()
@python_group()
@angreal.command(
    name="all",
    about="run every Python tutorial against one shared cloaca wheel (the CI lane form)",
    when_to_use=["CI tutorial gate", "validating all tutorials after a bindings change"],
    when_not_to_use=["iterating on a single tutorial (use its numbered command)"],
    tool=angreal.ToolDescription(
        "Runs all Python tutorials. When postgres is in scope, the cleanup path stops "
        "docker services and removes their volumes — any unrelated Postgres state in "
        "the shared compose stack is destroyed.",
        risk_level="destructive",
    ),
)
@angreal.argument(
    name="backend",
    long="backend",
    required=False,
    help="postgres, sqlite, or both (default: both)",
)
@angreal.argument(
    name="attempts",
    long="attempts",
    required=False,
    help="attempts per tutorial before it counts as failed (default 1)",
)
def run_all(backend=None, attempts=None):
    backend = backend or "both"
    if backend not in ("postgres", "sqlite", "both"):
        print(f"Error: invalid backend '{backend}' (use 'postgres', 'sqlite' or 'both').")
        return 1
    backends = ["sqlite", "postgres"] if backend == "both" else [backend]
    max_attempts = max(1, int(attempts or 1))

    entries = []
    for fname, rel_path in sorted(get_python_tutorial_files()):
        number = fname.split("_", 1)[0]
        if number.isdigit():
            entries.append((number, rel_path))
    if not entries:
        print("No Python tutorials discovered.")
        return 1

    # (number, path, effective backend) — the sqlite pass skips postgres-only
    # tutorials and routes 06 to postgres, mirroring the standalone commands.
    # Deduped on (number, effective backend): 06's sqlite→postgres rerouting
    # used to schedule it twice on postgres.
    plan = []
    seen = set()
    for b in backends:
        for number, rel_path in entries:
            if b == "sqlite" and number in _POSTGRES_ONLY:
                continue
            eff = _resolve_backend(number, b)
            if (number, eff) in seen:
                continue
            seen.add((number, eff))
            plan.append((number, rel_path, eff))
    needs_postgres = any(b == "postgres" for _, _, b in plan)

    venv_name = "tutorial-all-unified"
    venv_path = PROJECT_ROOT / venv_name
    results = []
    try:
        if needs_postgres:
            print("Starting PostgreSQL container...", flush=True)
            if docker_up() != 0:
                raise Exception("Failed to start PostgreSQL container")
            print("Waiting for PostgreSQL to be ready...", flush=True)
            time.sleep(10)
            if not check_postgres_container_health():
                raise Exception("PostgreSQL container is not healthy")

        print("Building cloaca wheel and shared tutorial venv (once)...", flush=True)
        _venv, python_exe, _pip_exe = _build_and_install_cloaca_unified(venv_name)

        for number, rel_path, b in plan:
            label = f"Python Tutorial {number} ({b})"
            rc = 1
            for attempt in range(1, max_attempts + 1):
                print(f"\n=== {label} (attempt {attempt}/{max_attempts}) ===", flush=True)
                rc = _run_python_tutorial(
                    number, rel_path, b, python_exe=python_exe,
                    db_name=f"tutorial_{number}" if b == "postgres" else None,
                )
                if rc == 0:
                    break
            results.append((label, rc == 0))
    except Exception as e:
        import traceback as _tb
        print(f"ERROR: shared tutorial setup failed: {type(e).__name__}: {e}", flush=True)
        _tb.print_exc()
        sys.stdout.flush()
        return 1
    finally:
        if needs_postgres:
            docker_down(remove_volumes=True)
        if venv_path.exists():
            shutil.rmtree(venv_path, ignore_errors=True)

    print("\n=== Python tutorial summary ===", flush=True)
    for label, ok in results:
        print(f"  {'ok  ' if ok else 'FAIL'} {label}", flush=True)
    failed = [label for label, ok in results if not ok]
    if failed:
        print(f"\n{len(failed)} tutorial run(s) failed: {', '.join(failed)}", flush=True)
        return 1
    print(f"\nAll {len(results)} Python tutorial runs passed.", flush=True)
    return 0
