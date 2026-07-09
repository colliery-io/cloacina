"""
Release version management for Cloacina (CLOACI-I-0134).

Thin angreal wrapper over `version_lockstep.py` (which holds the logic and is
also run directly by the pre-commit hook / CI). ONE source of version truth:
the `[workspace.package] version` in the root Cargo.toml; `release bump`
rewrites every core touchpoint from one input, `release check` fails on drift.
Providers under `examples/` are independently versioned (ADR A-0010) and are
NOT touched.
"""

import angreal  # type: ignore

# version_lockstep.py sits beside this file; angreal puts .angreal on sys.path
# (same as task_services.py's `from utils import ...`). It is not a `task_*`
# module, so angreal does not load it as a task.
import version_lockstep  # noqa: E402

release = angreal.command_group(
    name="release", about="version bump + drift guard (single source of version truth)"
)


@release()
@angreal.command(
    name="check",
    about="fail if any version touchpoint disagrees with the workspace version (drift guard)",
    when_to_use=["verifying a bump landed everywhere", "reproducing the pre-commit guard"],
    when_not_to_use=["needing to change the version (use `release bump`)"],
)
def check():
    """Assert every core version touchpoint equals the canonical workspace version.

    (The pre-commit hook runs `python .angreal/version_lockstep.py check` directly
    for readable output — angreal swallows task stdout. Exit code is authoritative.)
    """
    return version_lockstep.run_check()


@release()
@angreal.command(
    name="bump",
    about="set the whole core repo (Rust/npm/python/scaffold + CHANGELOG stub) to a new version",
    when_to_use=["cutting a release", "moving to the next dev version"],
    when_not_to_use=["touching provider versions (independent, A-0010)"],
)
@angreal.argument(name="version", help="target semver, e.g. 0.10.0", required=True, takes_value=True)
def bump(version: str):
    """Rewrite every core version touchpoint to `version`. The git tag stays a human step."""
    return version_lockstep.run_bump(version)
