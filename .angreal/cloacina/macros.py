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
def macros():
    """Run tests for macro validation system with both backends enabled."""

    # Test that invalid examples fail to compile as expected
    failure_examples = [
        "missing_dependency",
        "circular_dependency",
        "duplicate_task_ids"
    ]

    print_section_header("Running macro validation tests")
    print("\nTesting macro validation failure examples...")

    # Use cargo check with both backends enabled
    cmd_base = ["cargo", "check", "--features", "postgres,sqlite,macros"]

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
                    print("   -> Missing dependency error message generated")
                elif "Circular dependency detected" in result.stderr:
                    print("   -> Circular dependency error message generated")
                elif "Duplicate task ID" in result.stderr:
                    print("   -> Duplicate task ID error message generated")

        except subprocess.CalledProcessError as e:
            print(f"Error testing {example}: {e}")
            all_passed = False

    if not all_passed:
        print("\nMacro tests failed!")
        raise RuntimeError("Macro tests failed")

    print_final_success("All macro validation tests passed!")
