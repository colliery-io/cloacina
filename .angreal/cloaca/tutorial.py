"""
Tutorial execution tasks for Cloaca.
"""

import time
import subprocess
import shutil
from pathlib import Path

import angreal  # type: ignore

# Define command group
cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")

from .generate import generate
from .build import build_and_install_cloaca_backend
from .scrub import scrub
from utils import docker_up, check_postgres_container_health, smart_postgres_reset

@cloaca()
@angreal.command(name="tutorial", about="run Python tutorial examples with isolated environments")
@angreal.argument(name="tutorial", long="tutorial", help="Run specific tutorial: 01, 02, 03, 04, 05, or 'all' (default: all)", required=False)
@angreal.argument(name="backend", long="backend", help="Backend to use: postgres or sqlite (default: sqlite)", required=False)
def tutorial(tutorial=None, backend=None):
    """Run Python tutorial examples in isolated environments.

    Creates fresh virtual environments for each tutorial to demonstrate
    the Python bindings with real database backends.
    """

    # Set defaults
    if backend is None:
        backend = "sqlite"
    if tutorial is None:
        tutorial = "all"

    # Validate backend
    if backend not in ["postgres", "sqlite"]:
        print(f"Error: Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.")
        return 1

    # Define available tutorials
    available_tutorials = {
        "01": "python_tutorial_01_first_workflow.py",
        "02": "python_tutorial_02_context_handling.py",
        "03": "python_tutorial_03_error_handling.py",
        "04": "python_tutorial_04_complex_workflows.py",
        "05": "python_tutorial_05_multi_tenancy.py"
    }

    # Determine which tutorials to run
    if tutorial == "all":
        tutorials_to_run = list(available_tutorials.items())
    elif tutorial in available_tutorials:
        tutorials_to_run = [(tutorial, available_tutorials[tutorial])]
    else:
        print(f"Error: Invalid tutorial '{tutorial}'. Available: {', '.join(available_tutorials.keys())}, 'all'")
        return 1

    # Special handling for tutorial 05 (multi-tenancy)
    if any(t[0] == "05" for t in tutorials_to_run) and backend == "sqlite":
        print("Warning: Tutorial 05 (multi-tenancy) requires PostgreSQL backend")
        print("Either:")
        print("  1. Use --backend postgres to run with PostgreSQL")
        print("  2. Skip tutorial 05 by specifying a different tutorial")

        # Remove tutorial 05 from the list if running all with sqlite
        if tutorial == "all":
            tutorials_to_run = [(t, f) for t, f in tutorials_to_run if t != "05"]
            print("Skipping tutorial 05 for SQLite backend")
        else:
            return 1

    project_root = Path(angreal.get_root()).parent
    examples_dir = project_root / "examples"

    print(f"Running {len(tutorials_to_run)} tutorial(s) with {backend} backend")
    print("=" * 60)

    tutorial_results = []
    overall_success = True

    for tutorial_num, tutorial_file in tutorials_to_run:
        print(f"\nRunning Tutorial {tutorial_num}: {tutorial_file}")
        print("-" * 50)

        tutorial_path = examples_dir / tutorial_file
        if not tutorial_path.exists():
            print(f"ERROR: Tutorial file not found: {tutorial_path}")
            tutorial_results.append({
                "tutorial": tutorial_num,
                "status": "file_not_found",
                "file": tutorial_file
            })
            overall_success = False
            continue

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
            venv, python_exe, pip_exe = build_and_install_cloaca_backend(backend, venv_name)

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

                tutorial_results.append({
                    "tutorial": tutorial_num,
                    "status": "success",
                    "file": tutorial_file,
                    "output_lines": len(result.stdout.split('\n'))
                })
            else:
                print(f"FAILED: Tutorial {tutorial_num} failed!")
                print("\nError Output:")
                print("-" * 30)
                print(result.stderr)
                if result.stdout:
                    print("\nStandard Output:")
                    print("-" * 30)
                    print(result.stdout)

                tutorial_results.append({
                    "tutorial": tutorial_num,
                    "status": "failed",
                    "file": tutorial_file,
                    "error": result.stderr,
                    "return_code": result.returncode
                })
                overall_success = False

        except subprocess.TimeoutExpired:
            print(f"TIMEOUT: Tutorial {tutorial_num} timed out after 5 minutes")
            tutorial_results.append({
                "tutorial": tutorial_num,
                "status": "timeout",
                "file": tutorial_file
            })
            overall_success = False

        except Exception as e:
            print(f"ERROR: Tutorial {tutorial_num} setup failed: {e}")
            tutorial_results.append({
                "tutorial": tutorial_num,
                "status": "setup_failed",
                "file": tutorial_file,
                "error": str(e)
            })
            overall_success = False

        finally:
            # Cleanup for this tutorial
            print(f"Cleaning up tutorial {tutorial_num} environment...")

            # Clean up PostgreSQL for postgres backend
            if backend == "postgres":
                print("  Cleaning PostgreSQL state...")
                smart_postgres_reset()

            # Clean up virtual environment
            if venv_path.exists():
                print(f"  Removing virtual environment: {venv_name}")
                shutil.rmtree(venv_path)

            # Clean up generated files
            print("  Cleaning generated files...")
            scrub()

    # Final summary
    print(f"\n{'=' * 60}")
    print("TUTORIAL SUMMARY")
    print(f"{'=' * 60}")

    successful = len([r for r in tutorial_results if r["status"] == "success"])
    total = len(tutorial_results)

    print(f"Results: {successful}/{total} tutorials completed successfully")
    print(f"Backend: {backend}")
    print()

    # Show individual results
    for result in tutorial_results:
        status_indicator = {
            "success": "[PASS]",
            "failed": "[FAIL]",
            "timeout": "[TIMEOUT]",
            "setup_failed": "[SETUP_FAILED]",
            "file_not_found": "[NOT_FOUND]"
        }.get(result["status"], "[UNKNOWN]")

        print(f"{status_indicator} Tutorial {result['tutorial']}: {result['status']}")
        if result["status"] == "success":
            print(f"    Output lines: {result.get('output_lines', 'unknown')}")
        elif result["status"] in ["failed", "setup_failed"]:
            error = result.get("error", "Unknown error")
            # Show first line of error for brevity
            error_summary = error.split('\n')[0] if error else "Unknown error"
            print(f"    Error: {error_summary}")

    print()
    if overall_success:
        print("All tutorials completed successfully!")
        print("Ready to explore Cloacina Python bindings!")
    else:
        print("Some tutorials failed - check individual results above")
        print("Common issues:")
        print("  - PostgreSQL not running (try: docker-compose up -d)")
        print("  - Missing dependencies (check pip install)")
        print("  - Database permission issues")

    return 0 if overall_success else 1
