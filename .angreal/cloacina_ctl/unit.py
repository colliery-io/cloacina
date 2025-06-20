"""
Unit tests for cloacina-ctl.
"""

import subprocess
import angreal  # type: ignore

from .cloacina_ctl_utils import (
    validate_backend,
    print_section_header,
    print_test_result
)

# Define command group
ctl = angreal.command_group(name="ctl", about="cloacina-ctl testing and operations")


@ctl()
@angreal.command(
    name="unit",
    about="run unit tests for cloacina-ctl",
    when_to_use=[
        "testing cloacina-ctl library functions",
        "validating core CLI functionality",
        "CI/CD unit test phase",
        "development testing"
    ],
    when_not_to_use=[
        "integration testing with databases",
        "end-to-end workflow testing",
        "performance testing",
        "manual testing workflows"
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
def unit(backend=None, filter=None, verbose=False):
    """Run unit tests for cloacina-ctl."""

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

    for test_backend in backends_to_test:
        print_section_header(f"Running unit tests for {test_backend} backend")

        try:
            # Build the cargo test command
            cmd_args = ["test", "--lib", "--features", test_backend]

            if filter:
                cmd_args.extend(["--", filter])

            if verbose:
                cmd_args.append("--verbose")

            # Run the tests using cargo directly since these are unit tests
            result = subprocess.run(
                ["cargo"] + cmd_args,
                cwd="/Users/dstorey/Desktop/colliery/cloacina/cloacina-ctl",
                capture_output=True,
                text=True
            )

            if result.returncode == 0:
                print_test_result(f"{test_backend} unit tests", True)
                if verbose:
                    print("STDOUT:", result.stdout)
            else:
                print_test_result(f"{test_backend} unit tests", False, result.stderr)
                overall_success = False

        except Exception as e:
            print_test_result(f"{test_backend} unit tests", False, str(e))
            overall_success = False

    if overall_success:
        print_section_header("ALL UNIT TESTS PASSED")
        return 0
    else:
        print_section_header("SOME UNIT TESTS FAILED")
        return 1
