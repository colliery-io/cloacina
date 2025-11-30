"""
Shared utilities for Cloacina core engine test commands.
"""

from pathlib import Path
import angreal  # type: ignore

# Project root for accessing examples, cloacina, etc. (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent


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
