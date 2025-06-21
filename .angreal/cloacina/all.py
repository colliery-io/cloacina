import angreal  # type: ignore

from .unit import unit
from .macros import macros
from .integration import integration

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")


@cloacina()
@angreal.command(
    name="all",
    about="run all cloacina core tests (unit, integration, and macro tests)",
    when_to_use=["comprehensive testing", "pre-commit validation", "CI/CD full test suite"],
    when_not_to_use=["quick feedback loops", "testing specific features", "debugging individual tests"]
)
def all():
    """Run all cloacina core tests (unit, integration, and macro tests)."""
    failed_tests = []

    # Run unit tests first
    print("=== Running Unit Tests ===")
    try:
        unit()
    except Exception as e:
        failed_tests.append(f"Unit tests: {str(e)}")

    # Run macro tests
    print("\n=== Running Macro Tests ===")
    try:
        macros()
    except Exception as e:
        failed_tests.append(f"Macro tests: {str(e)}")

    # Run integration tests last
    print("\n=== Running Integration Tests ===")
    try:
        integration()
    except Exception as e:
        failed_tests.append(f"Integration tests: {str(e)}")

    if failed_tests:
        failure_summary = "\n".join(f"- {test}" for test in failed_tests)
        raise RuntimeError(f"Some cloacina core tests failed:\n{failure_summary}")

    print("\nAll cloacina core tests passed!")
