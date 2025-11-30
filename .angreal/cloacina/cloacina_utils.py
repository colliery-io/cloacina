"""
Shared utilities for Cloacina core engine test commands.
"""

import sys
from pathlib import Path
import angreal  # type: ignore

# Project root for accessing examples, cloacina, etc. (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent


def validate_backend(backend):
    """Validate backend selection and return error if invalid."""
    if backend and backend not in ["postgres", "sqlite"]:
        print(f"Error: Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.", file=sys.stderr)
        return False
    return True


def get_backends_to_test(backend):
    """Return list of backend configurations based on selection.

    Note: Both postgres and sqlite features are always enabled since the codebase
    no longer supports single-backend builds. The 'backend' parameter now only
    controls which database the tests connect to at runtime, not which code is compiled.
    """
    # Always compile with both backends - runtime selection determines which DB to use
    base_cmd = ["cargo", "test", "-p", "cloacina", "--lib", "--features", "postgres,sqlite,macros"]

    all_backends = [
        ("PostgreSQL", base_cmd.copy()),
        ("SQLite", base_cmd.copy())
    ]

    if backend == "postgres":
        return [all_backends[0]]  # Run with PostgreSQL database
    elif backend == "sqlite":
        return [all_backends[1]]  # Run with SQLite database
    elif backend is None:
        return all_backends  # Both (default)
    else:
        return None


def get_check_backends(backend):
    """Return list of backend configurations for cargo check commands.

    Note: Both postgres and sqlite features are always enabled since the codebase
    no longer supports single-backend builds.
    """
    # Always compile with both backends
    base_cmd = ["cargo", "check", "--features", "postgres,sqlite,macros"]

    all_backends = [
        ("Both backends", base_cmd.copy())
    ]

    # Backend parameter is ignored for check - we always check with both
    return all_backends


def print_section_header(title):
    """Print a formatted section header."""
    print(f"\n{'='*50}")
    print(title)
    print(f"{'='*50}")


def print_final_success(message):
    """Print a formatted final success message."""
    print(f"\n{'='*50}")
    print(message)
    print(f"{'='*50}")
