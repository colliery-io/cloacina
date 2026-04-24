"""ci fast — lint + unit tests. No Docker, no backing services."""

import angreal  # type: ignore

from lint.all import all as lint_all
from test.unit import unit

ci = angreal.command_group(name="ci", about="local mirrors of the CI matrix")


@ci()
@angreal.command(
    name="fast",
    about="lint + unit tests (no Docker, no backing services)",
    when_to_use=["pre-push sanity check", "quick CI emulation on a laptop"],
    when_not_to_use=["pre-release validation (use ci full)"],
)
def fast():
    """Run lint suite then unit tests. Fails fast."""
    print("=== ci fast: lint all ===")
    lint_all()
    print("\n=== ci fast: test unit ===")
    unit()
    print("\nci fast: OK")
