import subprocess
import angreal  # type: ignore

from .cloacina_utils import (
    PROJECT_ROOT,
    print_section_header,
    print_final_success
)

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")


@cloacina()
@angreal.command(
    name="macros",
    about="run tests for macro validation system",
    when_to_use=["validating workflow macros", "testing compile-time checks", "ensuring macro safety"],
    when_not_to_use=["runtime testing", "integration testing", "performance testing"]
)
@angreal.argument(
    name="backend",
    long="backend",
    required=False,
    help="(ignored) backend parameter for CI compatibility - tests run with both backends"
)
def macros(backend=None):
    """Run tests for macro validation system with both backends enabled.

    The --backend parameter is accepted for CI compatibility but ignored.
    """

    # Test that invalid examples fail to compile as expected
    failure_examples = [
        "missing_dependency",
        "circular_dependency",
        "duplicate_task_ids",
        "missing_workflow_task"
    ]

    print_section_header("Running macro validation tests")
    print("\nTesting macro validation failure examples...")

    # Use cargo check (no features needed - validation-failures uses path dependencies)
    cmd_base = ["cargo", "check"]

    all_passed = True
    for example in failure_examples:
        print(f"\n   Testing {example} (should fail to compile)...")
        try:
            cmd = cmd_base + ["--bin", example]
            result = subprocess.run(
                cmd,
                cwd=str(PROJECT_ROOT / "examples/features/validation-failures"),
                capture_output=True,
                text=True
            )

            if result.returncode == 0:
                print(f"ERROR: {example} compiled when it should have failed!")
                all_passed = False
            else:
                print(f"SUCCESS: {example} failed to compile as expected")
                # Show the actual error message
                for line in result.stderr.split('\n'):
                    if 'error:' in line.lower() or 'depends on' in line or 'Circular' in line or 'Duplicate' in line or 'not found' in line:
                        print(f"   -> {line.strip()}")

        except subprocess.CalledProcessError as e:
            print(f"Error testing {example}: {e}")
            all_passed = False

    if not all_passed:
        print("\nMacro tests failed!")
        raise RuntimeError("Macro tests failed")

    print_final_success("All macro validation tests passed!")
