"""
Unit test tasks for Cloacina core engine.
"""

import subprocess
import sys

import angreal  # type: ignore

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")

@cloacina()
@angreal.command(name="unit", about="run unit tests")
@angreal.argument(
    name="filter",
    required=False,
    help="Filter tests by name"
)
@angreal.argument(
    name="backend",
    long="backend",
    help="Run tests for specific backend: postgres or sqlite (default: both)",
    required=False
)
def unit(filter=None, backend=None):
    """Run unit tests (tests embedded in src/ modules only) for PostgreSQL and/or SQLite."""

    # Define backend test configurations
    all_backends = [
        ("PostgreSQL", ["cargo", "test", "--lib", "--no-default-features", "--features", "postgres,macros"]),
        ("SQLite", ["cargo", "test", "--lib", "--no-default-features", "--features", "sqlite,macros"])
    ]

    # Filter backends based on selection
    if backend == "postgres":
        backends = [all_backends[0]]  # PostgreSQL only
    elif backend == "sqlite":
        backends = [all_backends[1]]  # SQLite only
    elif backend is None:
        backends = all_backends  # Both (default)
    else:
        print(f"Error: Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.", file=sys.stderr)
        return 1

    for backend_name, cmd_base in backends:
        print(f"\n{'='*50}")
        print(f"Running unit tests for {backend_name}")
        print(f"{'='*50}")

        cmd = cmd_base.copy()
        if filter:
            cmd.append(filter)

        try:
            subprocess.run(cmd, check=True)
            print(f"{backend_name} unit tests passed")
        except subprocess.CalledProcessError as e:
            print(f"{backend_name} unit tests failed with error: {e}", file=sys.stderr)
            return e.returncode

    print(f"\n{'='*50}")
    print("All unit tests passed for both backends!")
    print(f"{'='*50}")
    return 0
