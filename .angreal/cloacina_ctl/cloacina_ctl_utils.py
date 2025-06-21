"""
Utility functions for cloacina-ctl testing and operations.
"""

import subprocess
from pathlib import Path


def get_project_root():
    """Get the project root directory."""
    return Path(__file__).parent.parent.parent


def run_cloacina_ctl_command(args, backend="postgres", cwd=None, check=True):
    """
    Run a cloacina-ctl command with the specified backend.

    Args:
        args: List of command arguments (excluding the binary name)
        backend: Backend to use ("postgres" or "sqlite")
        cwd: Working directory for the command
        check: Whether to check return code

    Returns:
        subprocess.CompletedProcess
    """
    project_root = get_project_root()

    # Determine which binary to use based on backend
    if backend == "postgres":
        binary = "cloacina-ctl-postgres"
    elif backend == "sqlite":
        binary = "cloacina-ctl-sqlite"
    else:
        raise ValueError(f"Unknown backend: {backend}")

    # Build the command
    cmd = ["cargo", "run", "--bin", binary, "--features", backend, "--"] + args

    if cwd is None:
        cwd = project_root / "cloacina-ctl"

    print(f"Running: {' '.join(cmd)}")
    print(f"Working directory: {cwd}")

    return subprocess.run(
        cmd,
        cwd=cwd,
        capture_output=True,
        text=True,
        check=check
    )


def validate_backend(backend):
    """Validate that the backend is supported."""
    if backend not in ["postgres", "sqlite"]:
        print(f"Error: Invalid backend '{backend}'. Supported backends: postgres, sqlite")
        return False
    return True


def print_section_header(title):
    """Print a formatted section header."""
    print(f"{'='*50}")
    print(title)
    print(f"{'='*50}")


def print_test_result(test_name, success, details=None):
    """Print formatted test results."""
    status = "PASS" if success else "FAIL"
    color = "\033[92m" if success else "\033[91m"  # Green for pass, red for fail
    reset = "\033[0m"

    print(f"{color}[{status}]{reset} {test_name}")
    if details:
        print(f"       {details}")
