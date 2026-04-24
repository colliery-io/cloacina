"""ci full — lint + test all + coverage. Requires Docker + Postgres."""

import angreal  # type: ignore

from lint.all import all as lint_all
from test.all import all as test_all
from test.coverage import coverage

ci = angreal.command_group(name="ci", about="local mirrors of the CI matrix")


@ci()
@angreal.command(
    name="full",
    about="lint + full test suite + coverage (requires Docker)",
    when_to_use=["pre-release validation", "investigating CI failures locally"],
    when_not_to_use=["quick feedback loops (use ci fast)"],
)
def full():
    """Run lint, full test suite, and coverage. Fails fast."""
    print("=== ci full: lint all ===")
    lint_all()
    print("\n=== ci full: test all ===")
    test_all()
    print("\n=== ci full: test coverage ===")
    coverage()
    print("\nci full: OK")
