import subprocess
import sys
import time
import os
import angreal  # type: ignore

from utils import docker_up, docker_down, docker_clean

from .cloacina_utils import (
    print_section_header,
    print_final_success
)


def build_test_packages():
    """Pre-build test packages before running integration tests.

    This builds the example workflow packages separately from the test binary,
    avoiding the fork-after-OpenSSL-init issue on Linux that causes SIGSEGV.
    The packages are stored in target/test-packages/ and loaded at test runtime.
    """
    print_section_header("Pre-building test packages")

    # Create output directory
    os.makedirs("target/test-packages", exist_ok=True)

    # Build packaged-workflow-example
    print("Building packaged-workflow-example...")
    subprocess.run(
        ["cargo", "build", "--release", "-p", "packaged-workflow-example"],
        check=True,
        cwd="examples/features/packaged-workflows"
    )

    # Build simple-packaged-demo
    print("Building simple-packaged-demo...")
    subprocess.run(
        ["cargo", "build", "--release", "-p", "simple-packaged-demo"],
        check=True,
        cwd="examples/features/simple-packaged"
    )

    print("Test packages built successfully.")

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
    required=False,
    help="run tests for specific backend: 'postgres', 'sqlite', or both if not specified"
)
@angreal.argument(
    name="features",
    long="features",
    required=False,
    help="cargo features to use (default: 'postgres,sqlite,macros')"
)
def integration(filter=None, skip_docker=False, backend=None, features=None):
    """Run integration tests against PostgreSQL and/or SQLite databases.

    Tests are compiled once with both backends enabled. By default, PostgreSQL
    tests run first, then SQLite tests run separately to avoid cross-backend
    interference. Use --backend to run only one backend's tests.
    """

    run_postgres = backend is None or backend == "postgres"
    run_sqlite = backend is None or backend == "sqlite"

    # Use provided features or default to both backends
    cargo_features = features if features else "postgres,sqlite,macros"
    is_default_features = cargo_features == "postgres,sqlite,macros"

    # Pre-build test packages to avoid fork-after-OpenSSL-init SIGSEGV on Linux
    # Only build for default features since examples depend on both backends
    if is_default_features:
        build_test_packages()
    else:
        print(f"Skipping test package builds for non-default features: {cargo_features}")

    if not skip_docker and run_postgres:
        # Start Docker services for PostgreSQL
        print_section_header("Starting Docker services")
        docker_down()
        docker_clean()
        exit_code = docker_up()
        if exit_code != 0:
            raise RuntimeError("Docker setup failed")
        # Wait for services to be ready
        print("Waiting for PostgreSQL to be ready...")
        time.sleep(30)

    try:
        if run_postgres:
            # Run PostgreSQL tests (exclude sqlite tests)
            print_section_header("Running PostgreSQL integration tests")
            postgres_cmd = ["cargo", "test", "-p", "cloacina", "--test", "integration",
                           "--features", cargo_features, "--",
                           "--test-threads=1", "--nocapture", "--skip", "sqlite"]
            if filter:
                postgres_cmd.append(filter)
            subprocess.run(postgres_cmd, check=True)

        if run_sqlite:
            # Run SQLite tests
            print_section_header("Running SQLite integration tests")
            sqlite_cmd = ["cargo", "test", "-p", "cloacina", "--test", "integration",
                         "--features", cargo_features, "--",
                         "--test-threads=1", "--nocapture", "sqlite"]
            if filter:
                sqlite_cmd.append(filter)
            subprocess.run(sqlite_cmd, check=True)

        print_final_success("All integration tests passed!")
    except subprocess.CalledProcessError as e:
        print(f"Integration tests failed with error: {e}", file=sys.stderr)
        raise RuntimeError(f"Integration tests failed with return code {e.returncode}")
    finally:
        if not skip_docker and run_postgres:
            docker_down()
            docker_clean()
