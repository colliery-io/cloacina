"""lint credential-logging — scan Rust sources for unmasked credential references."""

import subprocess
from pathlib import Path

import angreal  # type: ignore

PROJECT_ROOT = Path(angreal.get_root()).parent

lint = angreal.command_group(name="lint", about="format, clippy, and credential-logging guards")


@lint()
@angreal.command(
    name="credential-logging",
    about="scan Rust sources for unmasked credential references in log/print macros (OPS-03 / T-0443)",
    when_to_use=[
        "pre-commit",
        "CI validation",
        "after touching code that handles database URLs or tenant credentials",
    ],
    when_not_to_use=["normal development iteration where no credential-touching code changed"],
)
def credential_logging():
    """Run the credential-logging guard script (scripts/check_credential_logging.py)."""
    script = PROJECT_ROOT / "scripts" / "check_credential_logging.py"
    if not script.exists():
        print(f"Guard script not found: {script}")
        return 1
    result = subprocess.run(
        ["python3", str(script)],
        cwd=PROJECT_ROOT,
    )
    return result.returncode
