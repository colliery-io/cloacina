import shutil
import subprocess
import sys
import time
import os
from pathlib import Path

import angreal  # type: ignore

from utils import docker_up, docker_down, docker_clean

from .cloacina_utils import (
    print_section_header,
    print_final_success
)
from .python_utils import (
    TestAggregator,
    _build_and_install_cloaca_unified,
    run_pytest_scenarios,
)


def build_test_packages(backend=None):
    """Pre-build test packages before running integration tests.

    This builds the example workflow packages separately from the test binary,
    avoiding the fork-after-OpenSSL-init issue on Linux that causes SIGSEGV.
    The packages are stored in target/test-packages/ and loaded at test runtime.

    Note: Example packages are backend-agnostic (they only use cloacina-macros
    and cloacina-workflow), so we don't pass backend features to them.
    """
    print_section_header("Pre-building test packages")

    # Create output directory
    os.makedirs("target/test-packages", exist_ok=True)

    # Build packaged-workflow-example (debug mode to match test binary wire format)
    print("Building packaged-workflow-example...")
    subprocess.run(
        ["cargo", "build", "-p", "packaged-workflow-example"],
        check=True,
        cwd="examples/features/workflows/packaged-workflows"
    )

    # Build simple-packaged-demo (debug mode to match test binary wire format)
    print("Building simple-packaged-demo...")
    subprocess.run(
        ["cargo", "build", "-p", "simple-packaged-demo"],
        check=True,
        cwd="examples/features/workflows/simple-packaged"
    )

    print("Test packages built successfully.")

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")


@cloacina()
@angreal.command(
    name="integration",
    about="run integration tests with backing services (Rust + Python pytest scenarios)",
    when_to_use=["testing with real databases", "validating service integrations", "end-to-end testing"],
    when_not_to_use=["unit testing", "quick validation", "environments without Docker"]
)
@angreal.argument(
    name="filter",
    required=False,
    help="filter tests by name pattern (cargo test substring + pytest -k)"
)
@angreal.argument(
    name="skip_docker",
    long="skip-docker",
    help="skip Docker setup/teardown for manual service management",
    takes_value=False,
    is_flag=True
)
@angreal.argument(
    name="backend",
    long="backend",
    required=False,
    help="run tests for specific backend: 'postgres', 'sqlite', or both if not specified"
)
@angreal.argument(
    name="features",
    long="features",
    required=False,
    help="cargo features to use (default: 'postgres,sqlite,macros')"
)
@angreal.argument(
    name="skip_python",
    long="skip-python",
    help="skip Python pytest scenarios (run only Rust integration tests)",
    takes_value=False,
    is_flag=True,
)
@angreal.argument(
    name="python_file",
    long="python-file",
    required=False,
    help="run a single tests/python/<name>.py scenario file (still scoped per-backend)",
)
def integration(
    filter=None,
    skip_docker=False,
    backend=None,
    features=None,
    skip_python=False,
    python_file=None,
):
    """Run integration tests against PostgreSQL and/or SQLite databases.

    Two layers run per backend:
      1. Rust integration tests (cargo test -p cloacina --test integration ...).
      2. Python pytest scenarios under tests/python/ against a freshly built
         cloaca wheel — these exercise the Python binding surface end-to-end.
    Use --skip-python to run only the Rust layer.

    Tests are compiled once with both backends enabled. By default, PostgreSQL
    tests run first, then SQLite tests run separately to avoid cross-backend
    interference. Use --backend to run only one backend's tests.
    """

    run_postgres = backend is None or backend == "postgres"
    run_sqlite = backend is None or backend == "sqlite"
    backends_to_run = [b for b, on in (("postgres", run_postgres), ("sqlite", run_sqlite)) if on]

    cargo_features = features if features else "postgres,sqlite,macros"
    is_default_features = cargo_features == "postgres,sqlite,macros"

    if is_default_features:
        build_test_packages()
    else:
        build_test_packages(backend=backend)

    project_root = Path(angreal.get_root()).parent
    venv_name = "test-env-unified"
    venv_path = project_root / venv_name
    py_venv = None
    py_aggregator = TestAggregator()
    python_failures = 0

    if not skip_python:
        try:
            print_section_header("Building unified cloaca wheel for Python scenarios")
            # Pass the cargo feature set through so the wheel matches the
            # lane. Otherwise a sqlite-only lane builds the wheel with
            # maturin's defaults (postgres+sqlite+macros) and the resulting
            # libcloacina.so fails to link when libpq has been removed from
            # the runner to verify sqlite-only purity.
            py_venv, _python_exe, _pip_exe = _build_and_install_cloaca_unified(
                venv_name, cargo_features=cargo_features if not is_default_features else None,
            )
        except Exception as e:
            print(f"Failed to build cloaca wheel for Python scenarios: {e}", file=sys.stderr)
            if venv_path.exists():
                shutil.rmtree(venv_path, ignore_errors=True)
            raise

    if not skip_docker and run_postgres:
        # Start Docker services for PostgreSQL
        print_section_header("Starting Docker services")
        docker_down()
        docker_clean()
        exit_code = docker_up()
        if exit_code != 0:
            raise RuntimeError("Docker setup failed")
        # Wait for services to be ready
        print("Waiting for PostgreSQL to be ready...")
        time.sleep(30)

    try:
        # Build feature flags - use --no-default-features for non-default feature sets
        feature_args = ["--features", cargo_features]
        if not is_default_features:
            feature_args = ["--no-default-features"] + feature_args

        for backend_name in backends_to_run:
            print_section_header(f"Running {backend_name.title()} Rust integration tests")
            cargo_cmd = ["cargo", "test", "-p", "cloacina", "--test", "integration"] + feature_args
            if backend_name == "postgres":
                cargo_cmd += ["--", "--test-threads=1", "--nocapture", "--skip", "sqlite"]
            else:
                cargo_cmd += ["--", "--test-threads=1", "--nocapture", "sqlite"]
            if filter:
                cargo_cmd.append(filter)
            subprocess.run(cargo_cmd, check=True)

            if not skip_python:
                print_section_header(f"Running {backend_name.title()} Python pytest scenarios")
                ok = run_pytest_scenarios(
                    venv=py_venv,
                    project_root=project_root,
                    backend_name=backend_name,
                    aggregator=py_aggregator,
                    filter=filter,
                    file=python_file,
                )
                if not ok:
                    python_failures += 1

        if python_failures:
            py_aggregator.print_failure_report()
            failed = len(py_aggregator.get_failed_results())
            raise RuntimeError(f"{failed} Python pytest scenario file(s) failed")

        print_final_success("All integration tests passed!")
    except subprocess.CalledProcessError as e:
        print(f"Integration tests failed with error: {e}", file=sys.stderr)
        raise RuntimeError(f"Integration tests failed with return code {e.returncode}")
    finally:
        if not skip_docker and run_postgres:
            docker_down()
            docker_clean()
        if py_venv is not None and venv_path.exists():
            print(f"\nCleaning up Python test environment: {venv_name}")
            shutil.rmtree(venv_path, ignore_errors=True)
