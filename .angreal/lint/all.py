"""lint all — fmt check, clippy, and credential-logging in one pass."""

import angreal  # type: ignore

from .fmt import fmt
from .clippy import clippy
from .credential_logging import credential_logging

lint = angreal.command_group(name="lint", about="format, clippy, and credential-logging guards")


@lint()
@angreal.command(
    name="all",
    about="run fmt --check, clippy, and credential-logging",
    when_to_use=["pre-commit", "CI validation", "before opening a PR"],
    when_not_to_use=["iterating on a single file"],
)
def all():
    """Run every lint. Fail-fast — exits non-zero on first failure."""
    for name, fn, kwargs in (
        ("fmt --check", fmt, {"check": True}),
        ("clippy", clippy, {"deny_warnings": True}),
        ("credential-logging", credential_logging, {}),
    ):
        print(f"\n=== lint: {name} ===")
        rc = fn(**kwargs)
        if rc:
            raise RuntimeError(f"lint {name} failed (exit {rc})")
    print("\nAll lints passed.")
