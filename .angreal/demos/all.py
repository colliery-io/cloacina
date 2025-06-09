"""
Command to run all demos.
"""

import angreal  # type: ignore

from .demos_utils import (
    normalize_command_name
)

from .rust_demos import rust_example_commands, rust_tutorial_commands
from .python_demos import python_tutorial_commands

# Define command group
demos = angreal.command_group(name="demos", about="run Cloacina demonstration projects")


@demos()
@angreal.command(name="all", about="run all demonstration projects")
@angreal.argument(
    name="rust",
    long="rust",
    help="Run only Rust demos (examples and tutorials)",
    takes_value=False,
    is_flag=True
)
@angreal.argument(
    name="python",
    long="python",
    help="Run only Python tutorials",
    takes_value=False,
    is_flag=True
)
@angreal.argument(
    name="examples",
    long="examples",
    help="Run only Rust examples",
    takes_value=False,
    is_flag=True
)
@angreal.argument(
    name="tutorials",
    long="tutorials",
    help="Run only tutorials (Rust and Python)",
    takes_value=False,
    is_flag=True
)
@angreal.argument(
    name="backend",
    long="backend",
    help="Backend for Python tutorials: postgres or sqlite (default: sqlite)",
    required=False
)
def all_demos(rust=False, python=False, examples=False, tutorials=False, backend=None):
    """Run all demonstration projects."""
    if backend is None:
        backend = "sqlite"

    # Determine what to run based on flags
    # If no flags are set, run everything
    if not any([rust, python, examples, tutorials]):
        run_rust_examples = True
        run_rust_tutorials = True
        run_python_tutorials = True
    else:
        # Otherwise, determine based on flag combinations
        run_rust_examples = rust or examples
        run_rust_tutorials = rust or tutorials
        run_python_tutorials = python or tutorials

    overall_success = True
    results = []

    # Run Rust examples
    if run_rust_examples:
        print("\n" + "="*60)
        print("RUNNING RUST EXAMPLES")
        print("="*60)

        for dir_name, command_func in rust_example_commands.items():
            demo_name = normalize_command_name(dir_name)
            print(f"\n=== Running {demo_name} ===")
            result = command_func()
            results.append(("Rust Example", demo_name, result == 0))
            if result != 0:
                print(f"{demo_name} failed with exit code {result}")
                overall_success = False
            else:
                print(f"{demo_name} completed successfully!")

    # Run Rust tutorials
    if run_rust_tutorials:
        print("\n" + "="*60)
        print("RUNNING RUST TUTORIALS")
        print("="*60)

        for dir_name, command_func in rust_tutorial_commands.items():
            demo_name = normalize_command_name(dir_name)
            print(f"\n=== Running {demo_name} ===")
            result = command_func()
            results.append(("Rust Tutorial", demo_name, result == 0))
            if result != 0:
                print(f"{demo_name} failed with exit code {result}")
                overall_success = False
            else:
                print(f"{demo_name} completed successfully!")

    # Run Python tutorials
    if run_python_tutorials:
        print("\n" + "="*60)
        print(f"RUNNING PYTHON TUTORIALS (backend: {backend})")
        print("="*60)

        for tutorial_file, command_func in python_tutorial_commands.items():
            demo_name = normalize_command_name(tutorial_file)

            # Skip tutorial 05 if using SQLite
            if "05" in tutorial_file and backend == "sqlite":
                print(f"\n=== Skipping {demo_name} (requires PostgreSQL) ===")
                results.append(("Python Tutorial", demo_name, "skipped"))
                continue

            print(f"\n=== Running {demo_name} ===")
            # Call the command with backend argument
            result = command_func(backend=backend)
            results.append(("Python Tutorial", demo_name, result == 0))
            if result != 0:
                print(f"{demo_name} failed with exit code {result}")
                overall_success = False
            else:
                print(f"{demo_name} completed successfully!")

    # Print summary
    print("\n" + "="*60)
    print("DEMO EXECUTION SUMMARY")
    print("="*60)

    passed = sum(1 for _, _, status in results if status is True)
    failed = sum(1 for _, _, status in results if status is False)
    skipped = sum(1 for _, _, status in results if status == "skipped")
    total = len(results)

    print(f"\nTotal: {total} demos")
    print(f"Passed: {passed}")
    print(f"Failed: {failed}")
    if skipped > 0:
        print(f"Skipped: {skipped}")

    print("\nDetailed Results:")
    for demo_type, name, status in results:
        if status is True:
            status_str = "PASS"
        elif status is False:
            status_str = "FAIL"
        else:
            status_str = "SKIP"
        print(f"  [{status_str}] {demo_type}: {name}")

    if overall_success:
        print("\nAll demos completed successfully!")
        return 0
    else:
        print(f"\n{failed} demos failed. See individual results above.")
        return 1
