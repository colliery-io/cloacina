import angreal  # type: ignore
import subprocess
from pathlib import Path


# Define command group
cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")


@cloaca()
@angreal.command(
    name="smoke",
    about="run basic smoke tests to verify Python bindings work",
    when_to_use=["quick validation after changes", "verifying build success", "debugging import issues"],
    when_not_to_use=["comprehensive testing", "CI/CD validation", "performance testing"],
)
def smoke():
    """Run smoke tests for the native Python integration in cloacina core.

    The cloaca Python module is now embedded in cloacina core via PyO3.
    This runs the Rust-side Python integration tests which verify:
    - ensure_cloaca_module registers cloaca in sys.modules
    - @task decorator works and registers tasks in the global registry
    - @trigger decorator works
    - WorkflowBuilder context manager works
    - stdlib shadowing validation works
    """
    project_root = Path(angreal.get_root()).parent

    print("=" * 50)
    print("Running native Python integration smoke tests")
    print("=" * 50)

    # Run the Python module tests in cloacina core
    cmd = [
        "cargo",
        "test",
        "-p",
        "cloacina",
        "--lib",
        "--features",
        "postgres,sqlite,macros",
        "--",
        "python::tests",
        "--nocapture",
    ]

    print(f"Running: {' '.join(cmd)}")
    print()

    result = subprocess.run(cmd, cwd=str(project_root), text=True)

    if result.returncode != 0:
        raise RuntimeError("Python integration smoke tests failed")

    print()
    print("=" * 50)
    print("All Python smoke tests passed!")
    print("=" * 50)
