"""
Integration test tasks for Cloacina core engine.
"""

import subprocess
import sys
import time

import angreal  # type: ignore

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")

from utils import docker_up, docker_down, docker_clean

@cloacina()
@angreal.command(name="integration", about="run integration tests with backing services")
@angreal.argument(
    name="filter",
    required=False,
    help="Filter tests by name"
)
@angreal.argument(
    name="skip_docker",
    long="skip-docker",
    help="Skip Docker setup/teardown",
    takes_value=False,
    is_flag=True
)
@angreal.argument(
    name="backend",
    long="backend",
    help="Run tests for specific backend: postgres or sqlite (default: both)",
    required=False
)
def integration(filter=None, skip_docker=False, backend=None):
    """Run integration tests with backing services for PostgreSQL and/or SQLite."""

    # Validate backend selection
    if backend and backend not in ["postgres", "sqlite"]:
        print(f"Error: Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.", file=sys.stderr)
        return 1

    # Determine which backends to run
    run_postgres = backend is None or backend == "postgres"
    run_sqlite = backend is None or backend == "sqlite"

    postgresql_success = True
    sqlite_success = True

    # Run PostgreSQL integration tests
    if run_postgres:
        print(f"\n{'='*50}")
        print("Running integration tests for PostgreSQL")
        print(f"{'='*50}")

        if not skip_docker:
            # Start Docker services for PostgreSQL
            docker_down()
            docker_clean()
            exit_code = docker_up()
            if exit_code != 0:
                print("PostgreSQL Docker setup failed")
                postgresql_success = False
            else:
                # Wait for services to be ready
                print("Waiting for PostgreSQL to be ready...")
                time.sleep(30)

        if postgresql_success:
            try:
                cmd = ["cargo", "test", "--test", "integration", "--no-default-features", "--features", "postgres,macros", "--verbose", "--", "--test-threads=1", "--nocapture"]
                if filter:
                    cmd.append(filter)

                subprocess.run(cmd, check=True)
                print("PostgreSQL integration tests passed")
            except subprocess.CalledProcessError as e:
                print(f"PostgreSQL integration tests failed with error: {e}", file=sys.stderr)
                postgresql_success = False
            finally:
                if not skip_docker:
                    # Stop Docker services
                    docker_down()
                    docker_clean()

    # Run SQLite integration tests (no Docker needed)
    if run_sqlite:
        print(f"\n{'='*50}")
        print("Running integration tests for SQLite")
        print(f"{'='*50}")

        try:
            cmd = ["cargo", "test", "--test", "integration", "--no-default-features", "--features", "sqlite,macros", "--verbose", "--", "--test-threads=1", "--nocapture"]
            if filter:
                cmd.append(filter)

            subprocess.run(cmd, check=True)
            print("SQLite integration tests passed")
        except subprocess.CalledProcessError as e:
            print(f"SQLite integration tests failed with error: {e}", file=sys.stderr)
            sqlite_success = False

    # Summary
    if (not run_postgres or postgresql_success) and (not run_sqlite or sqlite_success):
        print(f"\n{'='*50}")
        backends_run = []
        if run_postgres:
            backends_run.append("PostgreSQL")
        if run_sqlite:
            backends_run.append("SQLite")
        backends_str = " and ".join(backends_run)
        print(f"All integration tests passed for {backends_str}!")
        print(f"{'='*50}")
        return 0
    else:
        print(f"\n{'='*50}")
        if run_postgres and not postgresql_success:
            print("PostgreSQL integration tests failed")
        if run_sqlite and not sqlite_success:
            print("SQLite integration tests failed")
        print(f"{'='*50}")
        return 1
