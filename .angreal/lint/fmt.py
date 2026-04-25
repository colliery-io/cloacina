"""lint fmt — cargo fmt across the workspace."""

import subprocess
from pathlib import Path

import angreal  # type: ignore

PROJECT_ROOT = Path(angreal.get_root()).parent

lint = angreal.command_group(name="lint", about="format, clippy, and credential-logging guards")


@lint()
@angreal.command(
    name="fmt",
    about="run cargo fmt across the workspace",
    when_to_use=["pre-commit", "CI validation"],
    when_not_to_use=["runtime testing"],
)
@angreal.argument(
    name="check",
    long="check",
    help="check formatting without modifying files (fails on diff)",
    takes_value=False,
    is_flag=True,
)
def fmt(check=False):
    """Run `cargo fmt --all`, optionally in --check mode for CI."""
    cmd = ["cargo", "fmt", "--all"]
    if check:
        cmd += ["--", "--check"]
    return subprocess.run(cmd, cwd=PROJECT_ROOT).returncode
