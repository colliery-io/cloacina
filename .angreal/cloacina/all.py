import angreal  # type: ignore

from .unit import unit
from .macros import macros
from .integration import integration

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")


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
