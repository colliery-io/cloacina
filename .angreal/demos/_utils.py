"""
Shared utilities for demo commands.
"""

from pathlib import Path
import angreal  # type: ignore

# Project root for accessing examples (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent


def get_rust_tutorial_directories():
    """Get all Rust tutorial directories from examples/tutorials/.

    Scans the hierarchical structure:
      tutorials/workflows/library/01-basic-workflow/
      tutorials/workflows/service/07-packaged-workflows/
      tutorials/computation-graphs/library/01-computation-graph/
      tutorials/computation-graphs/service/...

    Returns (dir_name, relative_path) tuples.
    """
    tutorials_dir = PROJECT_ROOT / "examples" / "tutorials"
    if not tutorials_dir.exists():
        return []
    results = []
    for capability in ["workflows", "computation-graphs"]:
        for mode in ["library", "service"]:
            scan_dir = tutorials_dir / capability / mode
            if scan_dir.exists():
                for d in scan_dir.iterdir():
                    if d.is_dir():
                        rel_path = f"examples/tutorials/{capability}/{mode}/{d.name}"
                        results.append((d.name, rel_path))
    return results


def get_rust_feature_directories():
    """Get all Rust feature example directories from examples/features/.

    Scans the hierarchical structure:
      features/workflows/cron-scheduling/
      features/computation-graphs/continuous-scheduling/
    """
    features_dir = PROJECT_ROOT / "examples" / "features"
    if not features_dir.exists():
        return []
    # Exclude examples that are libraries or not meant to be executed directly
    excluded = {"validation-failures", "complex-dag", "packaged-workflows", "simple-packaged", "packaged-triggers", "python-workflow"}
    results = []
    for capability in ["workflows", "computation-graphs"]:
        scan_dir = features_dir / capability
        if scan_dir.exists():
            for d in scan_dir.iterdir():
                if d.is_dir() and d.name not in excluded:
                    rel_path = f"examples/features/{capability}/{d.name}"
                    results.append((d.name, rel_path))
    return results


def get_rust_performance_directories():
    """Get all Rust performance example directories from examples/performance/."""
    perf_dir = PROJECT_ROOT / "examples" / "performance"
    if not perf_dir.exists():
        return []
    return [d.name for d in perf_dir.iterdir() if d.is_dir()]


def get_python_tutorial_files():
    """Get all Python tutorial files from examples/tutorials/python/.

    Scans the hierarchical structure:
      tutorials/python/workflows/01_first_workflow.py
      tutorials/python/computation-graphs/...
    """
    results = []
    python_dir = PROJECT_ROOT / "examples" / "tutorials" / "python"
    for capability in ["workflows", "computation-graphs"]:
        scan_dir = python_dir / capability
        if scan_dir.exists():
            for f in scan_dir.iterdir():
                if f.is_file() and f.suffix == ".py" and not f.name.startswith("_"):
                    rel_path = f"examples/tutorials/python/{capability}/{f.name}"
                    results.append((f.name, rel_path))
    return results


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
        for fname, rel_path in get_python_tutorial_files():
            if fname.startswith(f"{number}_"):
                return rel_path
        return None

    # Rust tutorials
    if command_name.startswith("tutorial-"):
        number = command_name.split("-")[-1]
        for dname, rel_path in get_rust_tutorial_directories():
            if dname.startswith(f"{number}-"):
                return rel_path
        return None

    # Feature examples
    for dname, rel_path in get_rust_feature_directories():
        if command_name == dname or command_name == dname.replace('_', '-'):
            return rel_path

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
        for fname, rel_path in get_python_tutorial_files():
            if fname.startswith(f"{number}_"):
                return {
                    "type": "python-tutorial",
                    "path": rel_path,
                    "name": f"Python Tutorial {number}",
                    "needs_docker": False,
                    "file": fname
                }
        return None

    # Check if it's a Rust tutorial
    if command_name.startswith("tutorial-"):
        number = command_name.split("-")[-1]
        for dname, rel_path in get_rust_tutorial_directories():
            if dname.startswith(f"{number}-"):
                return {
                    "type": "rust-tutorial",
                    "path": rel_path,
                    "name": f"Rust Tutorial {number}",
                    "needs_docker": number == "06"  # Multi-tenancy needs PostgreSQL
                }
        return None

    # Check feature examples
    for dname, rel_path in get_rust_feature_directories():
        if command_name == dname or command_name == dname.replace('_', '-'):
            return {
                "type": "rust-feature",
                "path": rel_path,
                "name": f"{dname.replace('-', ' ').replace('_', ' ').title()} Example",
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
