import angreal # type: ignore

import shutil
from pathlib import Path
import subprocess
import time
import os


from utils import docker_up, docker_down, check_postgres_container_health, smart_postgres_reset

from .cloaca_utils import (
    _build_and_install_cloaca_unified,
    TestAggregator,
    TestResult,
)



# Define command group
cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")



@cloaca()
@angreal.command(
    name="test",
    about="run tests in isolated test environments",
    when_to_use=["testing Python bindings", "validating API changes", "CI/CD verification"],
    when_not_to_use=["unit testing core Rust", "testing without clean environment", "quick development iterations"]
)
@angreal.argument(name="backend", long="backend", help="specific backend to test: postgres, sqlite, or both (default)", required=False)
@angreal.argument(name="filter", short="k", help="filter tests using pytest -k expression syntax")
@angreal.argument(name="file", long="file", help="run specific test file by filename")
@angreal.argument(name="skip_docker", long="skip-docker", help="skip Docker setup/teardown (use when postgres is already running)", takes_value=False, is_flag=True)
def test(backend=None, filter=None, file=None, skip_docker=False):
    """Run Python binding tests in isolated virtual environments.

    Creates a fresh virtual environment with the unified wheel and runs
    tests against the specified backend(s).
    """

    # Define backend configurations to test
    backends_to_test = []
    if backend == "postgres":
        backends_to_test = ["postgres"]
    elif backend == "sqlite":
        backends_to_test = ["sqlite"]
    elif backend is None:
        backends_to_test = ["sqlite", "postgres"]  # SQLite first (no docker needed)
    else:
        raise RuntimeError(f"Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.")

    project_root = Path(angreal.get_root()).parent
    venv_name = "test-env-unified"
    venv_path = project_root / venv_name

    all_passed = True
    test_aggregator = TestAggregator()

    try:
        # Step 1: Build and install unified wheel once
        print(f"\n{'='*50}")
        print("Building unified cloaca wheel")
        print(f"{'='*50}")
        venv, python_exe, pip_exe = _build_and_install_cloaca_unified(venv_name)

        # Step 2: Test each backend
        for backend_name in backends_to_test:
            print(f"\n{'='*50}")
            print(f"Testing {backend_name.title()} backend")
            print(f"{'='*50}")

            try:
                # Setup Docker for postgres backend
                if backend_name == "postgres" and not skip_docker:
                    print("Setting up Docker services for postgres...")
                    exit_code = docker_up()
                    if exit_code != 0:
                        raise Exception("Failed to start Docker services")
                    print("Waiting for services to be ready...")
                    time.sleep(10)

                    if not check_postgres_container_health():
                        raise Exception("PostgreSQL container is not healthy")
                elif backend_name == "postgres" and skip_docker:
                    print("Skipping Docker setup (--skip-docker flag set)")

                # Run tests
                print("Running tests...")
                test_dir = project_root / "python-tests"

                if file:
                    test_file_path = test_dir / file
                    if not test_file_path.exists():
                        print(f"Error: Test file {file} not found in {test_dir}")
                        all_passed = False
                        continue
                    test_files = [test_file_path]
                else:
                    test_files = list(test_dir.glob("test_*.py"))
                    if filter:
                        test_files = [f for f in test_files if filter in f.name]

                print(f"Found {len(test_files)} test files to run")

                file_results = []
                pytest_exe = venv.path / "bin" / "pytest"
                env = os.environ.copy()
                # Set backend hint for tests
                env["CLOACA_TEST_BACKEND"] = backend_name

                for test_file in test_files:
                    print(f"\n--- Running {test_file.name} ---")

                    # Clean state between test files
                    if backend_name == "postgres":
                        if smart_postgres_reset():
                            print("PostgreSQL state reset")
                        elif not skip_docker:
                            print("Fast reset failed, restarting Docker...")
                            docker_down(remove_volumes=True)
                            docker_up()
                            time.sleep(10)
                            if not check_postgres_container_health():
                                print(f"PostgreSQL unhealthy for {test_file.name}")
                                file_results.append((test_file.name, False))
                                all_passed = False
                                continue

                    if backend_name == "sqlite":
                        for db_file in project_root.glob("*.db*"):
                            try:
                                db_file.unlink()
                            except FileNotFoundError:
                                pass

                    # Run test
                    cmd = [str(pytest_exe), "--timeout=10", str(test_file), "-v"]
                    if filter:
                        cmd.extend(["-k", filter])

                    try:
                        result = subprocess.run(cmd, env=env, capture_output=True, text=True)

                        if result.returncode == 0:
                            print(f"PASSED: {test_file.name}")
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
                            print(f"FAILED: {test_file.name}")
                            if result.stdout:
                                print("STDOUT:", result.stdout[:500])
                            if result.stderr:
                                print("STDERR:", result.stderr[:500])

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
                        print(f"FAILED: {test_file.name} (subprocess error)")
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
                print(f"\n{backend_name.title()} Results: {len(passed)} passed, {len(failed)} failed")

            except Exception as e:
                print(f"Failed to test {backend_name}: {e}")
                all_passed = False
            finally:
                if backend_name == "postgres" and not skip_docker:
                    print("Cleaning up Docker services...")
                    docker_down(remove_volumes=True)

    except Exception as e:
        print(f"Failed to setup test environment: {e}")
        all_passed = False
    finally:
        if venv_path.exists():
            print(f"\nCleaning up test environment: {venv_name}")
            shutil.rmtree(venv_path)

    # Final report
    summary = test_aggregator.get_summary()
    failed_results = test_aggregator.get_failed_results()

    print(f"\n{'='*50}")
    print("TEST REPORT")
    print(f"{'='*50}")
    print(f"Total: {summary['total']}, Passed: {summary['passed']}, Failed: {summary['failed']}")

    if all_passed:
        print("\nAll tests passed!")
    else:
        print(f"\n{len(failed_results)} test files failed")
        raise RuntimeError(f"{len(failed_results)} Python binding test files failed")
