import subprocess
import sys
import angreal  # type: ignore

from .cloacina_utils import (
    print_section_header,
    print_final_success
)

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")


@cloacina()
@angreal.command(
    name="unit",
    about="run unit tests",
    when_to_use=["testing core functionality", "validating changes", "CI/CD pipelines"],
    when_not_to_use=["integration testing", "end-to-end testing", "performance testing"]
)
@angreal.argument(
    name="filter",
    required=False,
    help="filter tests by name pattern"
)
@angreal.argument(
    name="backend",
    long="backend",
    required=False,
    help="(ignored) backend parameter for CI compatibility - tests run with both backends"
)
def unit(filter=None, backend=None):
    """Run unit tests (tests embedded in src/ modules only).

    Tests are compiled once with both PostgreSQL and SQLite backends enabled.
    The --backend parameter is accepted for CI compatibility but ignored.
    """

    print_section_header("Running unit tests")

    cmd = ["cargo", "test", "-p", "cloacina", "--lib", "--features", "postgres,sqlite,macros"]
    if filter:
        cmd.append(filter)

    try:
        subprocess.run(cmd, check=True)
        print_final_success("All unit tests passed!")
    except subprocess.CalledProcessError as e:
        print(f"Unit tests failed with error: {e}", file=sys.stderr)
        raise RuntimeError(f"Unit tests failed with return code {e.returncode}")
