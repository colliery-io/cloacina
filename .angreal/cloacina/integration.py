import subprocess
import sys
import time
import angreal  # type: ignore

from utils import docker_up, docker_down, docker_clean

from .cloacina_utils import (
    validate_backend,
    print_section_header,
    print_final_success
)

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")


@cloacina()
@angreal.command(
    name="integration",
    about="run integration tests with backing services",
    when_to_use=["testing with real databases", "validating service integrations", "end-to-end testing"],
    when_not_to_use=["unit testing", "quick validation", "environments without Docker"]
)
@angreal.argument(
    name="filter",
    required=False,
    help="filter tests by name pattern"
)
@angreal.argument(
    name="skip_docker",
    long="skip-docker",
    help="skip Docker setup/teardown for manual service management",
    takes_value=False,
    is_flag=True
)
@angreal.argument(
    name="backend",
    long="backend",
    help="test specific backend: postgres, sqlite, or both (default)",
    required=False
)
def integration(filter=None, skip_docker=False, backend=None):
    """Run integration tests with backing services for PostgreSQL and/or SQLite."""

    # Validate backend selection
    if not validate_backend(backend):
        raise RuntimeError("Invalid backend specified")

    # Determine which backends to run
    run_postgres = backend is None or backend == "postgres"
    run_sqlite = backend is None or backend == "sqlite"

    postgresql_success = True
    sqlite_success = True

    # Run PostgreSQL integration tests
    if run_postgres:
        print_section_header("Running integration tests for PostgreSQL")

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
                cmd = ["cargo", "test", "-p", "cloacina", "--test", "integration", "--no-default-features", "--features", "postgres,macros", "--verbose", "--", "--test-threads=1", "--nocapture"]
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
        print_section_header("Running integration tests for SQLite")

        try:
            cmd = ["cargo", "test", "-p", "cloacina", "--test", "integration", "--no-default-features", "--features", "sqlite,macros", "--verbose", "--", "--test-threads=1", "--nocapture"]
            if filter:
                cmd.append(filter)

            subprocess.run(cmd, check=True)
            print("SQLite integration tests passed")
        except subprocess.CalledProcessError as e:
            print(f"SQLite integration tests failed with error: {e}", file=sys.stderr)
            sqlite_success = False

    # Summary
    if (not run_postgres or postgresql_success) and (not run_sqlite or sqlite_success):
        backends_run = []
        if run_postgres:
            backends_run.append("PostgreSQL")
        if run_sqlite:
            backends_run.append("SQLite")
        backends_str = " and ".join(backends_run)
        print_final_success(f"All integration tests passed for {backends_str}!")
    else:
        print_section_header("INTEGRATION TEST FAILURES")
        failed_backends = []
        if run_postgres and not postgresql_success:
            failed_backends.append("PostgreSQL")
        if run_sqlite and not sqlite_success:
            failed_backends.append("SQLite")
        print(f"{'='*50}")
        raise RuntimeError(f"Integration tests failed for: {', '.join(failed_backends)}")
