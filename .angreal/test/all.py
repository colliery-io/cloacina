import angreal  # type: ignore

from .integration import integration
from .macros import macros
from .unit import unit

test = angreal.command_group(name="test", about="Cloacina test suites (unit, integration, e2e, soak)")


@test()
@angreal.command(
    name="all",
    about="run unit, macros, and integration tests",
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
        failed_tests.append(f"Unit tests: {e!s}")

    # Run macro tests
    print("\n=== Running Macro Tests ===")
    try:
        macros()
    except Exception as e:
        failed_tests.append(f"Macro tests: {e!s}")

    # Run integration tests last
    print("\n=== Running Integration Tests ===")
    try:
        integration()
    except Exception as e:
        failed_tests.append(f"Integration tests: {e!s}")

    if failed_tests:
        failure_summary = "\n".join(f"- {test}" for test in failed_tests)
        raise RuntimeError(f"Some cloacina core tests failed:\n{failure_summary}")

    print("\nAll cloacina core tests passed!")
