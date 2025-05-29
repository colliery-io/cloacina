"""
Test tasks for Cloacina.
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
tests = angreal.command_group(name="tests", about="commands for test suites")


@tests()
@angreal.command(name="unit", about="run unit tests")
@angreal.argument(
    name="filter",
    required=False,
    help="Filter tests by name"
)
def unit(filter=None):
    """Run unit tests (tests embedded in src/ modules only)."""
    # Run lib tests (unit tests within src/ modules)
    cmd_lib = ["cargo", "test", "--lib", "--features", "postgres,macros"]
    if filter:
        cmd_lib.append(filter)

    try:
        result = subprocess.run(cmd_lib, check=True)
        return result.returncode
    except subprocess.CalledProcessError as e:
        print(f"Lib tests failed with error: {e}", file=sys.stderr)
        return e.returncode


@tests()
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
def integration(filter=None, skip_docker=False):
    """Run integration tests with backing services."""

    if not skip_docker:
        # Start Docker services
        docker_down()
        docker_clean()
        exit_code = docker_up()
        if exit_code != 0:
            return exit_code

        # Wait for services to be ready
        print("Waiting for services to be ready...")
        time.sleep(30)

    try:
        cmd = ["cargo", "test", "--test", "integration", "--features", "postgres,macros", "--verbose", "--", "--test-threads=1", "--nocapture"]
        if filter:
            cmd.append(filter)

        result = subprocess.run(cmd, check=True)
        return_code = result.returncode
    except subprocess.CalledProcessError as e:
        print(f"Integration tests failed with error: {e}", file=sys.stderr)
        return_code = e.returncode
    finally:
        if not skip_docker:
            # Stop Docker services
            docker_down()
            docker_clean()

    return return_code


@tests()
@angreal.command(name="macros", about="run tests for macro validation system")
def macro():
    """Run tests for macro validation system."""
    print("Running macro tests...")

    # Test that invalid examples fail to compile as expected
    print("\nTesting macro validation failure examples...")

    failure_examples = [
        "missing_dependency",
        "circular_dependency",
        "duplicate_task_ids"
    ]

    for example in failure_examples:
        print(f"\n   Testing {example} (should fail to compile)...")
        try:
            result = subprocess.run(
                ["cargo", "check", "--bin", example, "--features", "postgres"],
                cwd=str(PROJECT_ROOT / "examples/validation_failures"),
                capture_output=True,
                text=True
            )

            if result.returncode == 0:
                print(f"ERROR: {example} compiled when it should have failed!")
                return 1
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
            return e.returncode

    print("\nAll macro tests passed!")
    return 0


@tests()
@angreal.command(name="all", about="run all tests (unit, integration, and macro tests)")
def all():
    """Run all tests (unit, integration, and macro tests)."""
    # Run unit tests first
    print("=== Running Unit Tests ===")
    unit_result = unit()
    if unit_result != 0:
        return unit_result

    # Run macro tests
    print("\n=== Running Macro Tests ===")
    macro_result = macro()
    if macro_result != 0:
        return macro_result

    # Run integration tests last
    print("\n=== Running Integration Tests ===")
    integration_result = integration()
    if integration_result != 0:
        return integration_result

    print("\nAll tests passed!")
    return 0
