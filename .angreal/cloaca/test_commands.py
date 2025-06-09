"""
Test execution tasks for Cloaca.
"""

import os
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
from .test import TestResult, TestAggregator
from utils import docker_up, docker_down, check_postgres_container_health, smart_postgres_reset

@cloaca()
@angreal.command(name="test", about="run tests in isolated test environments")
@angreal.argument(name="backend", long="backend", help="Test specific backend: postgres or sqlite (default: both)", required=False)
@angreal.argument(name="filter", short="k", help="Filter tests by expression (pytest -k)")
@angreal.argument(name="file", long="file", help="Run specific test file (e.g. test_scenario_03_function_based_dag_topology.py)")
def test(backend=None, filter=None, file=None):
    """Run Python binding tests in isolated virtual environments.

    Creates fresh virtual environments for each test run to ensure
    no cross-contamination between test cycles.
    """

    # Define backend configurations
    backends_to_test = []
    if backend == "postgres":
        backends_to_test = ["postgres"]
    elif backend == "sqlite":
        backends_to_test = ["sqlite"]
    elif backend is None:
        backends_to_test = ["postgres", "sqlite"]
    else:
        print(f"Error: Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.")
        return 1

    all_passed = True

    # Initialize test result aggregator
    test_aggregator = TestAggregator()

    for backend_name in backends_to_test:
        print(f"\n{'='*50}")
        print(f"Testing {backend_name.title()} backend")
        print(f"{'='*50}")

        project_root = Path(angreal.get_root()).parent
        venv_name = f"test-env-{backend_name}"
        venv_path = project_root / venv_name

        try:
            # Step 1: Generate files
            print("Step 1: Generating files...")
            result = generate(backend_name)
            if result != 0:
                raise Exception(f"Failed to generate files for {backend_name}")

            # Step 2: Setup Docker for postgres backend
            if backend_name == "postgres":
                print("Step 2: Setting up Docker services for postgres...")
                exit_code = docker_up()
                if exit_code != 0:
                    raise Exception("Failed to start Docker services")
                print("Waiting for services to be ready...")
                time.sleep(10)

                # Verify container health
                if not check_postgres_container_health():
                    raise Exception("PostgreSQL container is not healthy")

            # Step 3: Build and install cloaca backend in test environment
            venv, python_exe, pip_exe = build_and_install_cloaca_backend(backend_name, venv_name)

            # Step 4: Run tests with file-level isolation
            print("Step 4: Running tests with file-level isolation...")

            # Discover test files to run
            test_dir = project_root / "python-tests"

            if file:
                # Run specific file
                test_file_path = test_dir / file
                if not test_file_path.exists():
                    print(f"Error: Test file {file} not found in {test_dir}")
                    all_passed = False
                    continue
                test_files = [test_file_path]
            else:
                # Run all test files
                test_files = list(test_dir.glob("test_*.py"))

                if filter:
                    test_files = [f for f in test_files if filter in f.name]

            print(f"Found {len(test_files)} test files to run")

            file_results = []
            pytest_exe = venv.path / "bin" / "pytest"
            env = os.environ.copy()

            for test_file in test_files:
                print(f"\n--- Running {test_file.name} in isolation ---")

                # Clean state between test files
                if backend_name == "postgres":
                    print(f"Resetting PostgreSQL state for {test_file.name}...")

                    # Try smart reset first (much faster)
                    if smart_postgres_reset():
                        print("✓ Fast PostgreSQL reset completed")
                    else:
                        print("Fast reset failed, falling back to Docker restart...")
                        docker_down(remove_volumes=True)
                        exit_code = docker_up()
                        if exit_code != 0:
                            print(f"✗ Failed to restart Docker for {test_file.name}")
                            file_results.append((test_file.name, False))
                            all_passed = False
                            continue
                        print("Waiting for postgres to be ready...")
                        time.sleep(10)

                        # Verify container health after restart
                        if not check_postgres_container_health():
                            print(f"✗ PostgreSQL container unhealthy after restart for {test_file.name}")
                            file_results.append((test_file.name, False))
                            all_passed = False
                            continue

                if backend_name == "sqlite":
                    print("Cleaning SQLite database files...")
                    for db_file in project_root.glob("*.db*"):
                        try:
                            db_file.unlink()
                            print(f"  Removed {db_file.name}")
                        except FileNotFoundError:
                            pass

                # Run single test file
                cmd = [str(pytest_exe), "--timeout=10", str(test_file), "-v"]
                if filter:
                    cmd.extend(["-k", filter])

                try:
                    result = subprocess.run(cmd, env=env, capture_output=True, text=True)

                    if result.returncode == 0:
                        print(f"✓ {test_file.name} PASSED")
                        test_result = TestResult(
                            file_name=test_file.name,
                            backend=backend_name,
                            passed=True,
                            stdout=result.stdout,
                            stderr=result.stderr,
                            return_code=result.returncode
                        )
                        file_results.append((test_file.name, True))
                    else:
                        print(f"✗ {test_file.name} FAILED")
                        test_result = TestResult(
                            file_name=test_file.name,
                            backend=backend_name,
                            passed=False,
                            stdout=result.stdout,
                            stderr=result.stderr,
                            return_code=result.returncode
                        )
                        file_results.append((test_file.name, False))
                        all_passed = False

                    test_aggregator.add_result(test_result)

                except subprocess.CalledProcessError as e:
                    print(f"✗ {test_file.name} FAILED (subprocess error)")
                    test_result = TestResult(
                        file_name=test_file.name,
                        backend=backend_name,
                        passed=False,
                        stdout=e.stdout or "",
                        stderr=e.stderr or "",
                        return_code=e.returncode
                    )
                    test_aggregator.add_result(test_result)
                    file_results.append((test_file.name, False))
                    all_passed = False

            # Report results for this backend
            passed = [name for name, success in file_results if success]
            failed = [name for name, success in file_results if not success]
            print(f"\n{backend_name.title()} Results:")
            print(f"  Passed: {len(passed)} files")
            print(f"  Failed: {len(failed)} files")
            if failed:
                print(f"  Failed files: {failed}")
            if passed:
                print(f"  Passed files: {passed}")

        except subprocess.CalledProcessError as e:
            print(f"✗ Test setup failed for {backend_name}: {e}")
            if e.stdout:
                print("STDOUT:", e.stdout)
            if e.stderr:
                print("STDERR:", e.stderr)
            all_passed = False
        except Exception as e:
            print(f"✗ Failed to setup {backend_name} test environment: {e}")
            all_passed = False
        finally:
            # Clean up Docker services for postgres backend
            if backend_name == "postgres":
                print("Cleaning up Docker services...")
                docker_down(remove_volumes=True)

            # Clean up test environment
            if venv_path.exists():
                print(f"Cleaning up test environment: {venv_name}")
                shutil.rmtree(venv_path)

            # Clean up generated files
            scrub()

    # Generate comprehensive final report
    summary = test_aggregator.get_summary()
    failed_results = test_aggregator.get_failed_results()

    print(f"\n{'='*70}")
    print("FINAL TEST REPORT")
    print(f"{'='*70}")

    # Overall summary
    print(f"Total tests: {summary['total']}")
    print(f"Passed: {summary['passed']}")
    print(f"Failed: {summary['failed']}")

    # Per-backend summary
    print("\nBackend breakdown:")
    for backend, stats in summary['backends'].items():
        print(f"  {backend.title()}: {stats['passed']} passed, {stats['failed']} failed")

    # Detailed failure report
    if failed_results:
        print(f"\n{'='*70}")
        print("DETAILED FAILURE REPORT")
        print(f"{'='*70}")

        for i, failure in enumerate(failed_results, 1):
            print(f"\n{i}. {failure.file_name} ({failure.backend})")
            print(f"   Return code: {failure.return_code}")
            print(f"   {'-'*50}")

            if failure.stdout:
                print("   STDOUT:")
                # Indent each line of output
                for line in failure.stdout.split('\n'):
                    if line.strip():
                        print(f"   {line}")

            if failure.stderr:
                print("   STDERR:")
                for line in failure.stderr.split('\n'):
                    if line.strip():
                        print(f"   {line}")

            print(f"   {'-'*50}")

        print(f"\n{'='*70}")
        print(f"SUMMARY: {len(failed_results)} test files failed across all backends")
        print("Scroll up to see detailed failure information for each test")
        print(f"{'='*70}")

    if all_passed:
        print("\nAll tests passed!")
        return 0
    else:
        print(f"\n{len(failed_results)} test files failed")
        print("See detailed failure report above")
        return 1
