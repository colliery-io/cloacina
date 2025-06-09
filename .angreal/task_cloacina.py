"""
Cloacina core engine test tasks.

These tasks specifically test the Cloacina workflow orchestration engine itself,
separate from the Python bindings (which are tested via the cloaca command group).
"""

import subprocess
import sys
import time
from pathlib import Path

import angreal  # type: ignore

from utils import docker_up, docker_down, docker_clean

# Project root for accessing examples, cloacina, etc. (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")

# Import command implementations
from cloacina.unit import unit
from cloacina.integration import integration
from cloacina.macros import macros
from cloacina.all import all


@cloacina()
@angreal.command(name="unit", about="run unit tests")
@angreal.argument(
    name="filter",
    required=False,
    help="Filter tests by name"
)
@angreal.argument(
    name="backend",
    long="backend",
    help="Run tests for specific backend: postgres or sqlite (default: both)",
    required=False
)
def unit(filter=None, backend=None):
    """Run unit tests (tests embedded in src/ modules only) for PostgreSQL and/or SQLite."""

    # Define backend test configurations
    all_backends = [
        ("PostgreSQL", ["cargo", "test", "--lib", "--no-default-features", "--features", "postgres,macros"]),
        ("SQLite", ["cargo", "test", "--lib", "--no-default-features", "--features", "sqlite,macros"])
    ]

    # Filter backends based on selection
    if backend == "postgres":
        backends = [all_backends[0]]  # PostgreSQL only
    elif backend == "sqlite":
        backends = [all_backends[1]]  # SQLite only
    elif backend is None:
        backends = all_backends  # Both (default)
    else:
        print(f"Error: Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.", file=sys.stderr)
        return 1

    for backend_name, cmd_base in backends:
        print(f"\n{'='*50}")
        print(f"Running unit tests for {backend_name}")
        print(f"{'='*50}")

        cmd = cmd_base.copy()
        if filter:
            cmd.append(filter)

        try:
            subprocess.run(cmd, check=True)
            print(f"{backend_name} unit tests passed")
        except subprocess.CalledProcessError as e:
            print(f"{backend_name} unit tests failed with error: {e}", file=sys.stderr)
            return e.returncode

    print(f"\n{'='*50}")
    print("All unit tests passed for both backends!")
    print(f"{'='*50}")
    return 0


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


@cloacina()
@angreal.command(name="macros", about="run tests for macro validation system")
@angreal.argument(
    name="backend",
    long="backend",
    help="Run tests for specific backend: postgres or sqlite (default: both)",
    required=False
)
def macros(backend=None):
    """Run tests for macro validation system for PostgreSQL and/or SQLite."""

    # Define backend test configurations
    all_backends = [
        ("PostgreSQL", ["cargo", "check", "--no-default-features", "--features", "postgres,macros"]),
        ("SQLite", ["cargo", "check", "--no-default-features", "--features", "sqlite,macros"])
    ]

    # Filter backends based on selection
    if backend == "postgres":
        backends = [all_backends[0]]  # PostgreSQL only
    elif backend == "sqlite":
        backends = [all_backends[1]]  # SQLite only
    elif backend is None:
        backends = all_backends  # Both (default)
    else:
        print(f"Error: Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.", file=sys.stderr)
        return 1

    # Test that invalid examples fail to compile as expected
    failure_examples = [
        "missing_dependency",
        "circular_dependency",
        "duplicate_task_ids"
    ]

    for backend_name, cmd_base in backends:
        print(f"\n{'='*50}")
        print(f"Running macro tests for {backend_name}")
        print(f"{'='*50}")
        print("\nTesting macro validation failure examples...")

        all_passed = True
        for example in failure_examples:
            print(f"\n   Testing {example} (should fail to compile)...")
            try:
                cmd = cmd_base + ["--bin", example]
                result = subprocess.run(
                    cmd,
                    cwd=str(PROJECT_ROOT / "examples/validation_failures"),
                    capture_output=True,
                    text=True
                )

                if result.returncode == 0:
                    print(f"ERROR: {example} compiled when it should have failed!")
                    all_passed = False
                else:
                    print(f"SUCCESS: {example} failed to compile as expected")
                    # Show brief indication of what was detected
                    if "depends on undefined task" in result.stderr:
                        print("   → Missing dependency error message generated")
                    elif "Circular dependency detected" in result.stderr:
                        print("   → Circular dependency error message generated")
                    elif "Duplicate task ID" in result.stderr:
                        print("   → Duplicate task ID error message generated")

            except subprocess.CalledProcessError as e:
                print(f"Error testing {example}: {e}")
                all_passed = False

        if not all_passed:
            print(f"\n{backend_name} macro tests failed!")
            return 1
        else:
            print(f"\n{backend_name} macro tests passed")

    print(f"\n{'='*50}")
    print("All macro tests passed for both backends!")
    print(f"{'='*50}")
    return 0


@cloacina()
@angreal.command(name="all", about="run all cloacina core tests (unit, integration, and macro tests)")
def all():
    """Run all cloacina core tests (unit, integration, and macro tests)."""
    # Run unit tests first
    print("=== Running Unit Tests ===")
    unit_result = unit()
    if unit_result != 0:
        return unit_result

    # Run macro tests
    print("\n=== Running Macro Tests ===")
    macros_result = macros()
    if macros_result != 0:
        return macros_result

    # Run integration tests last
    print("\n=== Running Integration Tests ===")
    integration_result = integration()
    if integration_result != 0:
        return integration_result

    print("\nAll cloacina core tests passed!")
    return 0
