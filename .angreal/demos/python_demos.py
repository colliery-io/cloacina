"""
Python tutorial demo commands.
"""

import angreal  # type: ignore
import shutil
import subprocess
import time

from cloaca.cloaca_utils import _build_and_install_cloaca_backend
from cloaca.generate import generate
from cloaca.scrub import scrub
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
    examples_dir = project_root / "examples"
    tutorial_path = examples_dir / tutorial_file

    if not tutorial_path.exists():
        print(f"ERROR: Tutorial file not found: {tutorial_path}")
        return 1

    # Create tutorial-specific environment
    venv_name = f"tutorial-{tutorial_num}-{backend}"
    venv_path = project_root / venv_name

    try:
        # Step 1: Generate files for the backend
        print(f"Step 1: Generating files for {backend} backend...")
        result = generate(backend)
        if result != 0:
            raise Exception(f"Failed to generate files for {backend}")

        # Step 2: Setup Docker for postgres backend
        if backend == "postgres":
            print("Step 2: Setting up PostgreSQL container...")
            exit_code = docker_up()
            if exit_code != 0:
                raise Exception("Failed to start PostgreSQL container")
            print("Waiting for PostgreSQL to be ready...")
            time.sleep(10)

            # Verify container health
            if not check_postgres_container_health():
                raise Exception("PostgreSQL container is not healthy")
            print("PostgreSQL ready")

        # Step 3: Build and install cloaca backend
        print("Step 3: Setting up tutorial environment...")
        venv, python_exe, pip_exe = _build_and_install_cloaca_backend(backend, venv_name)

        # Step 4: Run the tutorial
        print(f"Step 4: Executing tutorial {tutorial_num}...")

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
            cwd=str(examples_dir),
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

        # Clean up generated files
        print("  Cleaning generated files...")
        scrub()


def create_python_tutorial_command(tutorial_file):
    """Create a command for a Python tutorial."""
    # Extract tutorial number from filename
    parts = tutorial_file.replace('.py', '').split('_')
    if len(parts) >= 3 and parts[2].isdigit():
        tutorial_num = parts[2]
        command_name = f"python-tutorial-{tutorial_num}"
    else:
        # Fallback
        command_name = normalize_command_name(tutorial_file)
        tutorial_num = "??"

    @demos()
    @angreal.command(
        name=command_name,
        about=f"run Python Tutorial {tutorial_num}"
    )
    @angreal.argument(
        name="backend",
        long="backend",
        help="Backend to use: postgres or sqlite (default: sqlite)",
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

        # Special handling for tutorial 05 (multi-tenancy)
        if tutorial_num == "05" and backend == "sqlite":
            print("Error: Tutorial 05 (multi-tenancy) requires PostgreSQL backend")
            print("Please use --backend postgres to run this tutorial")
            return 1

        return run_python_tutorial(tutorial_num, tutorial_file, backend)

    # Store the function with a unique name to avoid conflicts
    command.__name__ = f"python_tutorial_{tutorial_num}"
    return command


# Create commands for all Python tutorials
python_tutorial_commands = {}
for tutorial_file in get_python_tutorial_files():
    python_tutorial_commands[tutorial_file] = create_python_tutorial_command(tutorial_file)
