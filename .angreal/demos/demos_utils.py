"""
Shared utilities for demo commands.
"""

from pathlib import Path
import angreal  # type: ignore

# Project root for accessing examples (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent


def get_rust_example_directories():
    """Get all Rust example directories (non-tutorials)."""
    examples_dir = PROJECT_ROOT / "examples"
    # Exclude validation_failures as it has multiple binaries and is not meant to be executed directly
    excluded_examples = {"validation_failures"}
    return [
        d.name for d in examples_dir.iterdir()
        if d.is_dir()
        and not d.name.startswith("tutorial")
        and not d.name.startswith("python_tutorial")
        and d.name not in excluded_examples
    ]


def get_rust_tutorial_directories():
    """Get all Rust tutorial directories."""
    examples_dir = PROJECT_ROOT / "examples"
    return [
        d.name for d in examples_dir.iterdir()
        if d.is_dir() and d.name.startswith("tutorial-")
    ]


def get_python_tutorial_files():
    """Get all Python tutorial files."""
    examples_dir = PROJECT_ROOT / "examples"
    return [
        f.name for f in examples_dir.iterdir()
        if f.is_file() and f.name.startswith("python_tutorial_") and f.suffix == ".py"
    ]


def normalize_command_name(name):
    """Normalize a demo name for use as a command.

    Examples:
        multi_tenant -> multi-tenant
        tutorial-01 -> tutorial-01
        python_tutorial_01_first_workflow.py -> python-tutorial-01
    """
    if name.startswith("python_tutorial_"):
        # Extract the number from python tutorial files
        parts = name.split("_")
        if len(parts) >= 3 and parts[2].isdigit():
            return f"python-tutorial-{parts[2]}"

    # For Rust demos, just replace underscores with hyphens
    return name.replace('_', '-')


def get_demo_info(command_name):
    """Get information about a demo from its command name.

    Returns a dict with:
        - type: 'rust-example', 'rust-tutorial', or 'python-tutorial'
        - path: relative path from project root
        - name: display name
        - needs_docker: whether it needs Docker services
    """
    examples_dir = PROJECT_ROOT / "examples"

    # Check if it's a Python tutorial
    if command_name.startswith("python-tutorial-"):
        number = command_name.split("-")[-1]
        # Find the actual file
        for f in get_python_tutorial_files():
            if f"_{number}_" in f:
                return {
                    "type": "python-tutorial",
                    "path": f"examples/{f}",
                    "name": f"Python Tutorial {number}",
                    "needs_docker": False,  # Will be determined by backend
                    "file": f
                }

    # Check if it's a Rust tutorial
    if command_name.startswith("tutorial-"):
        return {
            "type": "rust-tutorial",
            "path": f"examples/{command_name}",
            "name": f"Rust {command_name.title()}",
            "needs_docker": False  # Tutorials use SQLite
        }

    # Otherwise it's a Rust example
    # Convert command name back to directory name
    dir_name = command_name.replace('-', '_')
    if (examples_dir / dir_name).exists():
        return {
            "type": "rust-example",
            "path": f"examples/{dir_name}",
            "name": f"{dir_name.replace('_', ' ').title()} Example",
            "needs_docker": True  # Examples typically use PostgreSQL
        }

    return None
