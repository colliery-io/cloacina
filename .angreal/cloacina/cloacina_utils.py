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
    """Return list of backend configurations based on selection."""
    all_backends = [
        ("PostgreSQL", ["cargo", "test", "--lib", "--no-default-features", "--features", "postgres,macros"]),
        ("SQLite", ["cargo", "test", "--lib", "--no-default-features", "--features", "sqlite,macros"])
    ]

    if backend == "postgres":
        return [all_backends[0]]  # PostgreSQL only
    elif backend == "sqlite":
        return [all_backends[1]]  # SQLite only
    elif backend is None:
        return all_backends  # Both (default)
    else:
        return None


def get_check_backends(backend):
    """Return list of backend configurations for cargo check commands."""
    all_backends = [
        ("PostgreSQL", ["cargo", "check", "--no-default-features", "--features", "postgres,macros"]),
        ("SQLite", ["cargo", "check", "--no-default-features", "--features", "sqlite,macros"])
    ]

    if backend == "postgres":
        return [all_backends[0]]  # PostgreSQL only
    elif backend == "sqlite":
        return [all_backends[1]]  # SQLite only
    elif backend is None:
        return all_backends  # Both (default)
    else:
        return None


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
