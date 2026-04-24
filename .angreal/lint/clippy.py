"""lint clippy — cargo clippy across the workspace."""

import subprocess
from pathlib import Path

import angreal  # type: ignore

PROJECT_ROOT = Path(angreal.get_root()).parent

lint = angreal.command_group(name="lint", about="format, clippy, and credential-logging guards")


@lint()
@angreal.command(
    name="clippy",
    about="run cargo clippy across the workspace",
    when_to_use=["pre-commit", "CI validation", "surface code quality issues"],
    when_not_to_use=["runtime testing"],
)
@angreal.argument(
    name="deny_warnings",
    long="deny-warnings",
    help="treat warnings as errors (-D warnings)",
    takes_value=False,
    is_flag=True,
)
def clippy(deny_warnings=False):
    """Run `cargo clippy --all-targets --all-features`."""
    cmd = ["cargo", "clippy", "--all-targets", "--all-features"]
    if deny_warnings:
        cmd += ["--", "-D", "warnings"]
    return subprocess.run(cmd, cwd=PROJECT_ROOT).returncode
