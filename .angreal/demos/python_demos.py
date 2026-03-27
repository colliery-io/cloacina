"""
Python tutorial demo commands.
"""

import angreal  # type: ignore
import shutil
import subprocess
import time

from cloaca.cloaca_utils import _build_and_install_cloaca_unified
from utils import docker_up, docker_down, check_postgres_container_health, smart_postgres_reset

from .demos_utils import (
    PROJECT_ROOT,
    get_python_tutorial_files,
    normalize_command_name
)

# Define command group
demos = angreal.command_group(name="demos", about="run Cloacina demonstration projects")


def run_python_tutorial(tutorial_num, tutorial_file, backend="sqlite"):
    """Run a single Python tutorial with the specified backend."""
    project_root = PROJECT_ROOT
    python_tutorials_dir = project_root / "examples" / "tutorials" / "python"
    tutorial_path = python_tutorials_dir / tutorial_file

    if not tutorial_path.exists():
        print(f"ERROR: Tutorial file not found: {tutorial_path}")
        return 1

    # Create tutorial-specific environment
    venv_name = f"tutorial-{tutorial_num}-unified"
    venv_path = project_root / venv_name

    try:
        # Step 1: Setup Docker for postgres backend
        if backend == "postgres":
            print("Step 1: Setting up PostgreSQL container...")
            exit_code = docker_up()
            if exit_code != 0:
                raise Exception("Failed to start PostgreSQL container")
            print("Waiting for PostgreSQL to be ready...")
            time.sleep(10)

            # Verify container health
            if not check_postgres_container_health():
                raise Exception("PostgreSQL container is not healthy")
            print("PostgreSQL ready")

        # Step 2: Build and install unified cloaca wheel
        print("Step 2: Setting up tutorial environment...")
        venv, python_exe, pip_exe = _build_and_install_cloaca_unified(venv_name)

        # Step 3: Run the tutorial
        print(f"Step 3: Executing tutorial {tutorial_num}...")

        # Clean any existing database files for this tutorial
        if backend == "sqlite":
            db_pattern = f"python_tutorial_{tutorial_num}.db*"
            for db_file in project_root.glob(db_pattern):
                try:
                    db_file.unlink()
                    print(f"  Cleaned existing database: {db_file.name}")
                except FileNotFoundError:
                    pass
        elif backend == "postgres":
            # Reset postgres state for clean tutorial run
            if smart_postgres_reset():
                print("  PostgreSQL state reset for clean tutorial run")

        # Execute the tutorial
        cmd = [str(python_exe), str(tutorial_path)]

        print(f"  Running: {' '.join(cmd)}")
        result = subprocess.run(
            cmd,
            cwd=str(python_tutorials_dir),
            capture_output=True,
            text=True,
            timeout=300  # 5 minute timeout
        )

        if result.returncode == 0:
            print(f"SUCCESS: Tutorial {tutorial_num} completed successfully!")
            print("\nTutorial Output:")
            print("-" * 30)
            print(result.stdout)
            return 0
        else:
            print(f"FAILED: Tutorial {tutorial_num} failed!")
            print("\nError Output:")
            print("-" * 30)
            print(result.stderr)
            if result.stdout:
                print("\nStandard Output:")
                print("-" * 30)
                print(result.stdout)
            return 1

    except subprocess.TimeoutExpired:
        print(f"TIMEOUT: Tutorial {tutorial_num} timed out after 5 minutes")
        return 1

    except Exception as e:
        print(f"ERROR: Tutorial {tutorial_num} setup failed: {e}")
        return 1

    finally:
        # Cleanup for this tutorial
        print(f"Cleaning up tutorial {tutorial_num} environment...")

        # Clean up PostgreSQL for postgres backend
        if backend == "postgres":
            print("  Cleaning PostgreSQL state...")
            docker_down(remove_volumes=True)

        # Clean up virtual environment
        if venv_path.exists():
            print(f"  Removing virtual environment: {venv_name}")
            shutil.rmtree(venv_path)


def create_python_tutorial_command(tutorial_file):
    """Create a command for a Python tutorial."""
    # Extract tutorial number from filename (e.g., 01_first_workflow.py)
    parts = tutorial_file.replace('.py', '').split('_')
    if len(parts) >= 1 and parts[0].isdigit():
        tutorial_num = parts[0]
        command_name = f"python-tutorial-{tutorial_num}"
    else:
        # Fallback
        command_name = normalize_command_name(tutorial_file)
        tutorial_num = "??"

    @demos()
    @angreal.command(
        name=command_name,
        about=f"run Python Tutorial {tutorial_num}",
        when_to_use=[
            "Testing Cloacina functionality with Python examples",
            "Learning workflow orchestration concepts",
            "Validating backend integrations",
            "Demonstrating framework capabilities"
        ],
        when_not_to_use=[
            "Production workflow deployment",
            "Performance benchmarking",
            "Complex multi-step workflows",
            "Long-running production tasks"
        ]
    )
    @angreal.argument(
        name="backend",
        long="backend",
        help="Database backend (postgres/sqlite, default: sqlite)",
        required=False
    )
    def command(backend=None):
        """Run the Python tutorial."""
        if backend is None:
            backend = "sqlite"

        # Validate backend
        if backend not in ["postgres", "sqlite"]:
            print(f"Error: Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.")
            return 1

        # Special handling for tutorials that require PostgreSQL
        if tutorial_num == "06" and backend == "sqlite":
            print("Tutorial 06 (multi-tenancy) requires PostgreSQL - starting Docker services...")
            docker_up()
            backend = "postgres"

        return run_python_tutorial(tutorial_num, tutorial_file, backend)

    # Store the function with a unique name to avoid conflicts
    command.__name__ = f"python_tutorial_{tutorial_num}"
    return command


# Create commands for all Python tutorials
python_tutorial_commands = {}
for tutorial_file in get_python_tutorial_files():
    python_tutorial_commands[tutorial_file] = create_python_tutorial_command(tutorial_file)


# --------------------------------------------------------------------------
# Python workflow feature example
# --------------------------------------------------------------------------

@demos()
@angreal.command(
    name="python-workflow",
    about="run Python Workflow Example",
    when_to_use=[
        "Testing Python workflow packaging end-to-end",
        "Validating data pipeline task execution",
        "Verifying cloaca Context API with real tasks",
    ],
    when_not_to_use=[
        "Production deployment",
        "Performance benchmarking",
    ],
)
def python_workflow_demo():
    """Run the Python workflow feature example end-to-end."""
    project_root = PROJECT_ROOT
    example_dir = project_root / "examples" / "features" / "python-workflow"
    runner_script = example_dir / "run_pipeline.py"

    if not runner_script.exists():
        print(f"ERROR: Runner script not found: {runner_script}")
        return 1

    venv_name = "python-workflow-demo"
    venv_path = project_root / venv_name

    try:
        print("Step 1: Setting up demo environment...")
        venv, python_exe, pip_exe = _build_and_install_cloaca_unified(venv_name)

        print("Step 2: Executing python-workflow example...")
        cmd = [str(python_exe), str(runner_script)]
        print(f"  Running: {' '.join(cmd)}")

        result = subprocess.run(
            cmd,
            cwd=str(example_dir),
            capture_output=True,
            text=True,
            timeout=120,
        )

        if result.returncode == 0:
            print("SUCCESS: Python workflow example completed!")
            print("\nOutput:")
            print("-" * 30)
            # Only print the assertion lines, skip noisy runtime logs
            for line in result.stdout.splitlines():
                if not line.strip().startswith("[") and not line.startswith("THREAD:") and not line.startswith("TASK:") and not line.startswith("THREADS:"):
                    print(line)
            return 0
        else:
            print("FAILED: Python workflow example failed!")
            print("\nStderr:")
            print(result.stderr)
            if result.stdout:
                print("\nStdout:")
                print(result.stdout)
            return 1

    except subprocess.TimeoutExpired:
        print("TIMEOUT: Python workflow example timed out after 2 minutes")
        return 1

    except Exception as e:
        print(f"ERROR: Setup failed: {e}")
        return 1

    finally:
        print("Cleaning up demo environment...")
        if venv_path.exists():
            shutil.rmtree(venv_path)
