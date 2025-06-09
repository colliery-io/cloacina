"""
Release tasks for Cloaca.
"""


import angreal  # type: ignore

# Define command group
cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")

from .generate import generate
from .build import build_and_install_cloaca_backend
from .scrub import scrub

@cloaca()
@angreal.command(name="release", about="build and release Python bindings")
@angreal.argument(name="backend", long="backend", help="Backend to use: postgres or sqlite", required=True)
def release(backend):
    """Build and release Python bindings for Cloaca.

    This command will:
    1. Generate all necessary files
    2. Build the wheel
    3. Clean up generated files
    """
    if backend not in ["postgres", "sqlite"]:
        print(f"Error: Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.")
        return 1

    try:
        # Step 1: Generate files
        print(f"Step 1: Generating files for {backend} backend...")
        result = generate(backend)
        if result != 0:
            raise Exception(f"Failed to generate files for {backend}")

        # Step 2: Build wheel
        print("Step 2: Building wheel...")
        venv_name = f"release-env-{backend}"
        venv, python_exe, pip_exe = build_and_install_cloaca_backend(backend, venv_name)

        # Step 3: Clean up
        print("Step 3: Cleaning up...")
        scrub()

        print("\nRelease completed successfully!")
        return 0

    except Exception as e:
        print(f"Error during release: {e}")
        return 1
