"""
Shared utilities for Cloacina core engine test commands.
"""

from pathlib import Path
import angreal  # type: ignore

# Project root for accessing examples, cloacina, etc. (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent


def print_section_header(title):
    """Print a formatted section header."""
    print(f"\n{'='*50}")
    print(title)
    print(f"{'='*50}")


def print_final_success(message):
    """Print a formatted final success message."""
    print(f"\n{'='*50}")
    print(message)
    print(f"{'='*50}")


def wait_for_postgres_stable(
    compose_file=".angreal/docker-compose.yaml",
    cwd=None,
    user="cloacina",
    consecutive=3,
    attempts=60,
    interval=1.0,
):
    """Wait until the compose Postgres is *stably* accepting connections.

    `pg_isready` can pass during Postgres's init-restart window and then the
    server bounces — a caller that fires psql right after a single success
    races the restart and dies with a transient connection error (exit 56;
    this flaked the 0.9.0 release nightly). Require `consecutive` successes,
    `interval` apart, so the bounce window can't slip through. (CLOACI-T-0806;
    the readiness half of PR #145's `_fresh_database` retry.)
    """
    import subprocess
    import time

    streak = 0
    for _ in range(attempts):
        r = subprocess.run(
            [
                "docker", "compose", "-f", str(compose_file),
                "exec", "-T", "postgres", "pg_isready", "-U", user,
            ],
            capture_output=True,
            cwd=cwd,
        )
        if r.returncode == 0:
            streak += 1
            if streak >= consecutive:
                return
        else:
            streak = 0
        time.sleep(interval)
    raise RuntimeError(
        f"Postgres not stably ready after {attempts} checks "
        f"(needed {consecutive} consecutive pg_isready successes)"
    )


def psql_retry(
    sql_args,
    compose_file=".angreal/docker-compose.yaml",
    cwd=None,
    user="cloacina",
    dbname="postgres",
    attempts=10,
    interval=2.0,
):
    """Run IDEMPOTENT psql statements against the compose Postgres, retrying
    transient connection failures (the exit-56 class). Mirrors the hardening
    PR #145 gave the UI e2e `_fresh_database`; shared so every lane gets it
    (CLOACI-T-0806). `sql_args` is the list of trailing psql args (e.g.
    ["-c", "DROP ...", "-c", "CREATE ..."]).
    """
    import subprocess
    import time

    last = None
    for _ in range(attempts):
        last = subprocess.run(
            [
                "docker", "compose", "-f", str(compose_file),
                "exec", "-T", "postgres",
                "psql", "-U", user, "-d", dbname,
            ]
            + list(sql_args),
            capture_output=True,
            cwd=cwd,
        )
        if last.returncode == 0:
            return last
        time.sleep(interval)
    import subprocess as _sp

    raise _sp.CalledProcessError(
        last.returncode, last.args, output=last.stdout, stderr=last.stderr
    )
