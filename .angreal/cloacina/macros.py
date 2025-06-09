"""
Macro validation test tasks for Cloacina core engine.
"""

import subprocess
import sys
from pathlib import Path

import angreal  # type: ignore

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")

# Project root for accessing examples, cloacina, etc. (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent

@cloacina()
@angreal.command(name="macros", about="run tests for macro validation system")
@angreal.argument(
    name="backend",
    long="backend",
    help="Run tests for specific backend: postgres or sqlite (default: both)",
    required=False
)
def macros(backend=None):
    """Run tests for macro validation system for PostgreSQL and/or SQLite."""

    # Define backend test configurations
    all_backends = [
        ("PostgreSQL", ["cargo", "check", "--no-default-features", "--features", "postgres,macros"]),
        ("SQLite", ["cargo", "check", "--no-default-features", "--features", "sqlite,macros"])
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

    # Test that invalid examples fail to compile as expected
    failure_examples = [
        "missing_dependency",
        "circular_dependency",
        "duplicate_task_ids"
    ]

    for backend_name, cmd_base in backends:
        print(f"\n{'='*50}")
        print(f"Running macro tests for {backend_name}")
        print(f"{'='*50}")
        print("\nTesting macro validation failure examples...")

        all_passed = True
        for example in failure_examples:
            print(f"\n   Testing {example} (should fail to compile)...")
            try:
                cmd = cmd_base + ["--bin", example]
                result = subprocess.run(
                    cmd,
                    cwd=str(PROJECT_ROOT / "examples/validation_failures"),
                    capture_output=True,
                    text=True
                )

                if result.returncode == 0:
                    print(f"ERROR: {example} compiled when it should have failed!")
                    all_passed = False
                else:
                    print(f"SUCCESS: {example} failed to compile as expected")
                    # Show brief indication of what was detected
                    if "depends on undefined task" in result.stderr:
                        print("   → Missing dependency error message generated")
                    elif "Circular dependency detected" in result.stderr:
                        print("   → Circular dependency error message generated")
                    elif "Duplicate task ID" in result.stderr:
                        print("   → Duplicate task ID error message generated")
                    else:
                        print("   → Unknown error message generated")

            except Exception as e:
                print(f"ERROR: Failed to test {example}: {e}")
                all_passed = False

        if all_passed:
            print(f"\nAll macro validation tests passed for {backend_name}!")
        else:
            print(f"\nSome macro validation tests failed for {backend_name}")
            return 1

    print(f"\n{'='*50}")
    print("All macro validation tests passed for both backends!")
    print(f"{'='*50}")
    return 0
