import subprocess
import sys
import angreal  # type: ignore

from .cloacina_utils import (
    validate_backend,
    get_backends_to_test,
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
    help="test specific backend: postgres, sqlite, or both (default)",
    required=False
)
def unit(filter=None, backend=None):
    """Run unit tests (tests embedded in src/ modules only) for PostgreSQL and/or SQLite."""

    # Validate backend selection
    if not validate_backend(backend):
        return 1

    # Get backend configurations
    backends = get_backends_to_test(backend)
    if backends is None:
        return 1

    for backend_name, cmd_base in backends:
        print_section_header(f"Running unit tests for {backend_name}")

        cmd = cmd_base.copy()
        if filter:
            cmd.append(filter)

        try:
            subprocess.run(cmd, check=True)
            print(f"{backend_name} unit tests passed")
        except subprocess.CalledProcessError as e:
            print(f"{backend_name} unit tests failed with error: {e}", file=sys.stderr)
            return e.returncode

    print_final_success("All unit tests passed for both backends!")
    return 0
