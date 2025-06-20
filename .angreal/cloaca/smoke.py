import angreal # type: ignore
import shutil
from pathlib import Path
import subprocess
import time

from .cloaca_utils import _build_and_install_cloaca_backend
from utils import docker_up, docker_down
from .generate import generate
from .scrub import scrub


# Define command group
cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")




@cloaca()
@angreal.command(
    name="smoke", 
    about="run basic smoke tests to verify Python bindings work",
    when_to_use=["quick validation after changes", "verifying build success", "debugging import issues"],
    when_not_to_use=["comprehensive testing", "CI/CD validation", "performance testing"]
)
@angreal.argument(name="backend", long="backend", help="specific backend: postgres, sqlite, or both (default)", required=False)
def smoke(backend=None):
    """Run basic smoke tests to verify Python bindings work."""

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

    for backend_name in backends_to_test:
        print(f"\n{'='*50}")
        print(f"Smoke testing {backend_name.title()} backend")
        print(f"{'='*50}")

        project_root = Path(angreal.get_root()).parent
        venv_name = f"smoke-test-{backend_name}"
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

            # Step 3: Build and install cloaca backend in test environment
            venv, python_exe, pip_exe = _build_and_install_cloaca_backend(backend_name, venv_name)

            # Step 4: Run smoke test
            print("Step 4: Running smoke test...")
            test_script = f'''
try:
    import cloaca_{backend_name}
    print("✓ Successfully imported cloaca_{backend_name}")
    print("✓ Smoke test passed!")
except Exception as e:
    print(f"✗ Smoke test failed: {{e}}")
    import traceback
    traceback.print_exc()
    exit(1)
'''

            result = subprocess.run([
                str(python_exe), "-c", test_script
            ], check=True, capture_output=True, text=True)
            print(result.stdout)
            print(f"✓ {backend_name.title()} smoke test passed")

        except subprocess.CalledProcessError as e:
            print(f"✗ Smoke test failed for {backend_name}: {e}")
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

    if all_passed:
        print(f"\n{'='*50}")
        print("All smoke tests passed!")
        print(f"{'='*50}")
        return 0
    else:
        print(f"\n{'='*50}")
        print("Some smoke tests failed!")
        print(f"{'='*50}")
        return 1
