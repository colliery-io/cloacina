import angreal  # type: ignore

import os
import shutil
import subprocess
import time
from pathlib import Path

from utils import (
    docker_up,
    docker_down,
    check_postgres_container_health,
    smart_postgres_reset,
)

from .python_utils import (
    _build_and_install_cloaca_unified,
    TestAggregator,
    TestResult,
)


cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")


@cloacina()
@angreal.command(
    name="python-integration",
    about="run Python binding pytest scenarios against built wheel (sqlite + postgres)",
    when_to_use=[
        "comprehensive Python binding validation",
        "CI/CD pipeline",
        "validating API/runtime changes that affect the Python surface",
    ],
    when_not_to_use=[
        "quick Rust-side validation (cloacina unit already covers python::tests)",
        "iterative development on a single Rust module",
    ],
)
@angreal.argument(
    name="backend",
    long="backend",
    help="specific backend to test: postgres, sqlite, or both (default)",
    required=False,
)
@angreal.argument(
    name="filter",
    short="k",
    help="filter tests using pytest -k expression syntax",
)
@angreal.argument(
    name="file",
    long="file",
    help="run a specific test file by filename (under tests/python/)",
)
@angreal.argument(
    name="skip_docker",
    long="skip-docker",
    help="skip Docker setup/teardown (use when postgres is already running)",
    takes_value=False,
    is_flag=True,
)
def python_integration(backend=None, filter=None, file=None, skip_docker=False):
    """Run the pytest scenarios under tests/python/ against a freshly built cloaca wheel.

    Builds the unified cloaca wheel (from crates/cloacina-python) once, then runs
    each scenario file against the requested backend(s). For the postgres backend,
    Docker services are started automatically unless --skip-docker is set.
    """
    if backend == "postgres":
        backends_to_test = ["postgres"]
    elif backend == "sqlite":
        backends_to_test = ["sqlite"]
    elif backend is None:
        backends_to_test = ["sqlite", "postgres"]
    else:
        raise RuntimeError(f"Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.")

    project_root = Path(angreal.get_root()).parent
    venv_name = "test-env-unified"
    venv_path = project_root / venv_name

    all_passed = True
    test_aggregator = TestAggregator()

    try:
        print(f"\n{'='*50}")
        print("Building unified cloaca wheel")
        print(f"{'='*50}")
        venv, python_exe, pip_exe = _build_and_install_cloaca_unified(venv_name)

        for backend_name in backends_to_test:
            print(f"\n{'='*50}", flush=True)
            print(f"Testing {backend_name.title()} backend", flush=True)
            print(f"{'='*50}", flush=True)

            try:
                if backend_name == "postgres" and not skip_docker:
                    print("Setting up Docker services for postgres...")
                    if docker_up() != 0:
                        raise Exception("Failed to start Docker services")
                    print("Waiting for services to be ready...")
                    time.sleep(10)
                    if not check_postgres_container_health():
                        raise Exception("PostgreSQL container is not healthy")
                elif backend_name == "postgres" and skip_docker:
                    print("Skipping Docker setup (--skip-docker flag set)")

                test_dir = project_root / "tests" / "python"
                if file:
                    test_file_path = test_dir / file
                    if not test_file_path.exists():
                        print(f"Error: Test file {file} not found in {test_dir}")
                        all_passed = False
                        continue
                    test_files = [test_file_path]
                else:
                    test_files = sorted(test_dir.glob("test_*.py"))
                    if filter:
                        test_files = [f for f in test_files if filter in f.name]

                print(f"Found {len(test_files)} test files to run")

                file_results = []
                pytest_exe = venv.path / "bin" / "pytest"
                env = os.environ.copy()
                env["CLOACA_BACKEND"] = backend_name

                for test_file in test_files:
                    print(f"\n--- Running {test_file.name} ---", flush=True)

                    if backend_name == "postgres":
                        if skip_docker:
                            try:
                                result = subprocess.run(
                                    [
                                        "psql", "-U", "cloacina", "-d", "cloacina",
                                        "-c", "DROP SCHEMA public CASCADE; CREATE SCHEMA public;",
                                    ],
                                    capture_output=True, text=True,
                                )
                                if result.returncode != 0:
                                    print(f"Warning: PostgreSQL reset failed: {result.stderr}")
                            except Exception as e:
                                print(f"Warning: Could not reset PostgreSQL: {e}")
                        elif smart_postgres_reset():
                            print("PostgreSQL state reset")
                        else:
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

                    cmd = [str(pytest_exe), "--timeout=10", str(test_file), "-v"]
                    if filter:
                        cmd.extend(["-k", filter])

                    try:
                        result = subprocess.run(cmd, env=env, capture_output=True, text=True)
                        passed = result.returncode == 0
                        test_aggregator.add_result(
                            TestResult(
                                file_name=test_file.name,
                                backend=backend_name,
                                passed=passed,
                                stdout=result.stdout,
                                stderr=result.stderr,
                                return_code=result.returncode,
                            )
                        )
                        file_results.append((test_file.name, passed))
                        if passed:
                            print(f"PASSED: {test_file.name}")
                        else:
                            print(f"FAILED: {test_file.name}")
                            print("\n--- PYTEST OUTPUT ---")
                            print(result.stdout)
                            if result.stderr:
                                print("\n--- STDERR ---")
                                print(result.stderr)
                            print("--- END OUTPUT ---\n")
                            all_passed = False
                    except subprocess.CalledProcessError as e:
                        print(f"FAILED: {test_file.name} (subprocess error)")
                        test_aggregator.add_result(
                            TestResult(
                                file_name=test_file.name,
                                backend=backend_name,
                                passed=False,
                                stdout=e.stdout or "",
                                stderr=e.stderr or "",
                                return_code=e.returncode,
                            )
                        )
                        file_results.append((test_file.name, False))
                        all_passed = False

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

    summary = test_aggregator.get_summary()
    failed_results = test_aggregator.get_failed_results()

    print(f"\n{'='*50}")
    print("TEST REPORT")
    print(f"{'='*50}")
    print(f"Total: {summary['total']}, Passed: {summary['passed']}, Failed: {summary['failed']}")

    if summary.get("backends"):
        print("\nBy backend:")
        for backend_label, stats in summary["backends"].items():
            status = "OK" if stats["failed"] == 0 else "FAILED"
            print(f"  {backend_label}: {stats['passed']} passed, {stats['failed']} failed [{status}]")

    if all_passed:
        print("\nAll tests passed!")
    else:
        test_aggregator.print_failure_report()
        print(f"\n{len(failed_results)} test files failed")
        raise RuntimeError(f"{len(failed_results)} Python binding test files failed")
