import angreal # type: ignore
import shutil
from pathlib import Path
import subprocess
import time

from .cloaca_utils import _build_and_install_cloaca_unified
from utils import docker_up, docker_down


# Define command group
cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")




@cloaca()
@angreal.command(
    name="smoke",
    about="run basic smoke tests to verify Python bindings work",
    when_to_use=["quick validation after changes", "verifying build success", "debugging import issues"],
    when_not_to_use=["comprehensive testing", "CI/CD validation", "performance testing"]
)
@angreal.argument(name="backend", long="backend", help="specific backend to test: postgres, sqlite, or both (default)", required=False)
def smoke(backend=None):
    """Run basic smoke tests to verify unified Python bindings work."""

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
    venv_name = "smoke-test-unified"
    venv_path = project_root / venv_name

    all_passed = True

    try:
        # Step 1: Build and install unified wheel once
        print(f"\n{'='*50}")
        print("Building unified cloaca wheel")
        print(f"{'='*50}")
        venv, python_exe, pip_exe = _build_and_install_cloaca_unified(venv_name)

        # Step 2: Test basic import
        print("\nStep 2: Testing basic import...")
        test_script = '''
import cloaca
print(f"Backend: {cloaca.get_backend()}")
print(f"Hello: {cloaca.hello_world()}")
assert cloaca.get_backend() == "unified", f"Expected 'unified', got {cloaca.get_backend()}"
print("Basic import test passed!")
'''
        result = subprocess.run([str(python_exe), "-c", test_script], capture_output=True, text=True)
        if result.returncode != 0:
            print(f"STDOUT: {result.stdout}")
            print(f"STDERR: {result.stderr}")
            raise RuntimeError("Basic import test failed")
        print(result.stdout)

        # Step 3: Test each backend
        for backend_name in backends_to_test:
            print(f"\n{'='*50}")
            print(f"Testing {backend_name.title()} backend")
            print(f"{'='*50}")

            try:
                # Setup Docker for postgres backend
                if backend_name == "postgres":
                    print("Setting up Docker services for postgres...")
                    exit_code = docker_up()
                    if exit_code != 0:
                        raise Exception("Failed to start Docker services")
                    print("Waiting for services to be ready...")
                    time.sleep(10)

                # Run backend-specific test
                if backend_name == "sqlite":
                    test_script = '''
import cloaca
from cloaca import DefaultRunner

# Test SQLite connection
runner = DefaultRunner("file:smoke_test.db")
print("SQLite DefaultRunner created successfully")
runner.shutdown()
print("SQLite DefaultRunner shutdown successfully")

# Test that admin rejects SQLite
try:
    admin = cloaca.DatabaseAdmin("sqlite:///test.db")
    print("ERROR: Should have rejected SQLite URL for admin")
    exit(1)
except RuntimeError as e:
    if "PostgreSQL" in str(e):
        print("Admin correctly rejected SQLite URL")
    else:
        raise

print("SQLite smoke test passed!")
'''
                else:  # postgres
                    test_script = '''
import cloaca
from cloaca import DefaultRunner, DatabaseAdmin

# Test PostgreSQL connection
runner = DefaultRunner("postgres://cloacina:cloacina@localhost:5432/cloacina")
print("PostgreSQL DefaultRunner created successfully")
runner.shutdown()
print("PostgreSQL DefaultRunner shutdown successfully")

# Test admin accepts PostgreSQL
admin = DatabaseAdmin("postgres://cloacina:cloacina@localhost:5432/cloacina")
print("DatabaseAdmin created successfully with PostgreSQL")

print("PostgreSQL smoke test passed!")
'''

                result = subprocess.run([str(python_exe), "-c", test_script], capture_output=True, text=True)
                print(result.stdout)
                if result.stderr:
                    # Filter out tracing logs
                    stderr_lines = [l for l in result.stderr.split('\n') if not l.strip().startswith('[2m') and l.strip()]
                    if stderr_lines:
                        print("STDERR:", '\n'.join(stderr_lines))

                if result.returncode != 0:
                    print(f"Test failed with return code {result.returncode}")
                    all_passed = False
                else:
                    print(f"{backend_name.title()} smoke test passed!")

            except Exception as e:
                print(f"Failed to test {backend_name}: {e}")
                all_passed = False
            finally:
                # Clean up Docker services for postgres backend
                if backend_name == "postgres":
                    print("Cleaning up Docker services...")
                    docker_down(remove_volumes=True)

    except Exception as e:
        print(f"Failed to setup test environment: {e}")
        all_passed = False
    finally:
        # Clean up test environment
        if venv_path.exists():
            print(f"\nCleaning up test environment: {venv_name}")
            shutil.rmtree(venv_path)

    if all_passed:
        print(f"\n{'='*50}")
        print("All smoke tests passed!")
        print(f"{'='*50}")
    else:
        print(f"\n{'='*50}")
        print("Some smoke tests failed!")
        print(f"{'='*50}")
        raise RuntimeError("Some Python binding smoke tests failed")
