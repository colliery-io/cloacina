"""
Integration tests for cloacina-ctl.
"""

import subprocess
import sys
import time
import angreal  # type: ignore

from .cloacina_ctl_utils import (
    validate_backend,
    print_section_header,
    print_test_result
)

# Import Docker utilities
import os
sys.path.append(os.path.join(os.path.dirname(__file__), '..'))
from utils import docker_up, docker_down, docker_clean

# Define command group
ctl = angreal.command_group(name="ctl", about="cloacina-ctl testing and operations")


@ctl()
@angreal.command(
    name="integration",
    about="run integration tests for cloacina-ctl",
    when_to_use=[
        "testing end-to-end functionality",
        "validating database interactions",
        "CI/CD integration test phase",
        "pre-release testing"
    ],
    when_not_to_use=[
        "unit testing isolated functions",
        "performance testing",
        "quick development feedback",
        "testing without database setup"
    ]
)
@angreal.argument(
    name="backend",
    long="backend",
    help="test specific backend: postgres, sqlite, or both (default)",
    required=False
)
@angreal.argument(
    name="filter",
    help="filter tests by name pattern",
    required=False
)
@angreal.argument(
    name="skip_docker",
    long="skip-docker",
    help="skip Docker service management (assume services are running)",
    takes_value=False,
    is_flag=True
)
def integration(backend=None, filter=None, skip_docker=False):
    """Run integration tests for cloacina-ctl."""

    # Validate backend selection
    if backend and not validate_backend(backend):
        return 1

    # Determine which backends to test
    backends_to_test = []
    if backend is None:
        backends_to_test = ["postgres", "sqlite"]
    else:
        backends_to_test = [backend]

    overall_success = True

    # Start Docker services if needed and postgres is being tested
    if not skip_docker and ("postgres" in backends_to_test):
        print_section_header("Starting Docker services for PostgreSQL")
        docker_down()
        docker_clean()
        exit_code = docker_up()
        if exit_code != 0:
            print_test_result("Docker service startup", False, "Failed to start PostgreSQL services")
            return 1
        else:
            print_test_result("Docker service startup", True)
            print("Waiting for PostgreSQL to be ready...")
            time.sleep(15)  # Wait for services to be ready

    for test_backend in backends_to_test:
        print_section_header(f"Running integration tests for {test_backend} backend")

        try:
            # Build the cargo test command for integration tests
            cmd_args = ["test", "--test", "integration", "-p", "cloacina-ctl", "--features", test_backend, "--", "--test-threads=1"]

            if filter:
                cmd_args.append(filter)

            # Run the tests using cargo directly
            result = subprocess.run(
                ["cargo"] + cmd_args,
                cwd=os.path.dirname(angreal.get_root()),
                text=True
            )

            if result.returncode == 0:
                print_test_result(f"{test_backend} integration tests", True)
            else:
                print_test_result(f"{test_backend} integration tests", False, result.stderr)
                overall_success = False

        except Exception as e:
            print_test_result(f"{test_backend} integration tests", False, str(e))
            overall_success = False

    if overall_success:
        print_section_header("ALL INTEGRATION TESTS PASSED")
        return 0
    else:
        print_section_header("SOME INTEGRATION TESTS FAILED")
        return 1
