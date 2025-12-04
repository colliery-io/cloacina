"""
Shared utilities for demo commands.
"""

from pathlib import Path
import angreal  # type: ignore

# Project root for accessing examples (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent


def get_rust_tutorial_directories():
    """Get all Rust tutorial directories from examples/tutorials/."""
    tutorials_dir = PROJECT_ROOT / "examples" / "tutorials"
    if not tutorials_dir.exists():
        return []
    return [
        d.name for d in tutorials_dir.iterdir()
        if d.is_dir() and not d.name.startswith("python")
    ]


def get_rust_feature_directories():
    """Get all Rust feature example directories from examples/features/."""
    features_dir = PROJECT_ROOT / "examples" / "features"
    if not features_dir.exists():
        return []
    # Exclude validation-failures as it has multiple binaries and is not meant to be executed directly
    # Exclude packaged workflow examples as they are libraries, not runnable binaries
    excluded = {"validation-failures", "complex-dag", "packaged-workflows", "simple-packaged"}
    return [
        d.name for d in features_dir.iterdir()
        if d.is_dir() and d.name not in excluded
    ]


def get_rust_performance_directories():
    """Get all Rust performance example directories from examples/performance/."""
    perf_dir = PROJECT_ROOT / "examples" / "performance"
    if not perf_dir.exists():
        return []
    return [d.name for d in perf_dir.iterdir() if d.is_dir()]


def get_python_tutorial_files():
    """Get all Python tutorial files from examples/tutorials/python/."""
    python_dir = PROJECT_ROOT / "examples" / "tutorials" / "python"
    if not python_dir.exists():
        return []
    return [
        f.name for f in python_dir.iterdir()
        if f.is_file() and f.suffix == ".py" and not f.name.startswith("_")
    ]


def normalize_command_name(name):
    """Normalize a demo name for use as a command.

    Examples:
        multi-tenant -> multi-tenant
        01-basic-workflow -> tutorial-01
        01_first_workflow.py -> python-tutorial-01
    """
    # For Python tutorial files (e.g., 01_first_workflow.py)
    if name.endswith(".py"):
        parts = name.replace('.py', '').split("_")
        if len(parts) >= 1 and parts[0].isdigit():
            return f"python-tutorial-{parts[0]}"

    # For Rust tutorials (e.g., 01-basic-workflow)
    if name[0:2].isdigit() and "-" in name:
        return f"tutorial-{name.split('-')[0]}"

    # For other demos, just use the name with underscores replaced
    return name.replace('_', '-')


def get_demo_path(command_name):
    """Get the full path for a demo from its command name.

    Returns the relative path from project root.
    """
    # Python tutorials
    if command_name.startswith("python-tutorial-"):
        number = command_name.split("-")[-1]
        for f in get_python_tutorial_files():
            if f.startswith(f"{number}_"):
                return f"examples/tutorials/python/{f}"
        return None

    # Rust tutorials
    if command_name.startswith("tutorial-"):
        number = command_name.split("-")[-1]
        for d in get_rust_tutorial_directories():
            if d.startswith(f"{number}-"):
                return f"examples/tutorials/{d}"
        return None

    # Feature examples
    features_dir = PROJECT_ROOT / "examples" / "features"
    dir_name = command_name.replace('-', '_')
    if (features_dir / command_name).exists():
        return f"examples/features/{command_name}"
    if (features_dir / dir_name).exists():
        return f"examples/features/{dir_name}"

    # Performance examples
    perf_dir = PROJECT_ROOT / "examples" / "performance"
    if (perf_dir / command_name).exists():
        return f"examples/performance/{command_name}"

    return None


def get_demo_info(command_name):
    """Get information about a demo from its command name.

    Returns a dict with:
        - type: 'rust-tutorial', 'rust-feature', 'rust-performance', or 'python-tutorial'
        - path: relative path from project root
        - name: display name
        - needs_docker: whether it needs Docker services
    """
    # Check if it's a Python tutorial
    if command_name.startswith("python-tutorial-"):
        number = command_name.split("-")[-1]
        for f in get_python_tutorial_files():
            if f.startswith(f"{number}_"):
                return {
                    "type": "python-tutorial",
                    "path": f"examples/tutorials/python/{f}",
                    "name": f"Python Tutorial {number}",
                    "needs_docker": False,
                    "file": f
                }
        return None

    # Check if it's a Rust tutorial
    if command_name.startswith("tutorial-"):
        number = command_name.split("-")[-1]
        for d in get_rust_tutorial_directories():
            if d.startswith(f"{number}-"):
                return {
                    "type": "rust-tutorial",
                    "path": f"examples/tutorials/{d}",
                    "name": f"Rust Tutorial {number}",
                    "needs_docker": number == "06"  # Multi-tenancy needs PostgreSQL
                }
        return None

    # Check feature examples
    for d in get_rust_feature_directories():
        if command_name == d or command_name == d.replace('_', '-'):
            return {
                "type": "rust-feature",
                "path": f"examples/features/{d}",
                "name": f"{d.replace('-', ' ').replace('_', ' ').title()} Example",
                "needs_docker": True
            }

    # Check performance examples
    for d in get_rust_performance_directories():
        if command_name == d:
            return {
                "type": "rust-performance",
                "path": f"examples/performance/{d}",
                "name": f"Performance {d.title()}",
                "needs_docker": False
            }

    return None
