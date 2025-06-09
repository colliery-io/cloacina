import subprocess
import angreal  # type: ignore

from .cloacina_utils import (
    PROJECT_ROOT,
    validate_backend,
    get_check_backends,
    print_section_header,
    print_final_success
)

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")


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

    # Validate backend selection
    if not validate_backend(backend):
        return 1

    # Get backend configurations for cargo check
    backends = get_check_backends(backend)
    if backends is None:
        return 1

    # Test that invalid examples fail to compile as expected
    failure_examples = [
        "missing_dependency",
        "circular_dependency",
        "duplicate_task_ids"
    ]

    for backend_name, cmd_base in backends:
        print_section_header(f"Running macro tests for {backend_name}")
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

            except subprocess.CalledProcessError as e:
                print(f"Error testing {example}: {e}")
                all_passed = False

        if not all_passed:
            print(f"\n{backend_name} macro tests failed!")
            return 1
        else:
            print(f"\n{backend_name} macro tests passed")

    print_final_success("All macro tests passed for both backends!")
    return 0
