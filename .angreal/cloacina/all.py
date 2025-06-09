"""
All test tasks for Cloacina core engine.
"""


import angreal  # type: ignore

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")

from .unit import unit
from .integration import integration
from .macros import macros

@cloacina()
@angreal.command(name="all", about="run all cloacina core tests (unit, integration, and macro tests)")
def all():
    """Run all Cloacina core tests (unit, integration, and macro tests)."""

    print(f"\n{'='*50}")
    print("Running all Cloacina core tests")
    print(f"{'='*50}")

    # Run unit tests
    print("\nRunning unit tests...")
    if unit() != 0:
        print("Unit tests failed")
        return 1

    # Run integration tests
    print("\nRunning integration tests...")
    if integration() != 0:
        print("Integration tests failed")
        return 1

    # Run macro tests
    print("\nRunning macro tests...")
    if macros() != 0:
        print("Macro tests failed")
        return 1

    print(f"\n{'='*50}")
    print("All Cloacina core tests passed!")
    print(f"{'='*50}")
    return 0
