import angreal # type: ignore
from angreal.integrations.venv import VirtualEnv# type: ignore


import shutil
from pathlib import Path
import subprocess

from .generate import generate

# Define command group
cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")

@cloaca()
@angreal.command(
    name="release", 
    about="build release wheels for distribution (leaves artifacts for inspection)",
    when_to_use=["preparing production releases", "building all backend variants", "generating distribution artifacts"],
    when_not_to_use=["development testing", "quick iterations", "single backend development"]
)
@angreal.argument(name="backend", long="backend", help="specific backend: postgres, sqlite, or both (default)", required=False)
def release(backend=None):
    """Build release wheels for distribution without cleanup.

    Generates files, builds wheels, but leaves all artifacts for inspection.
    Use 'scrub' command to clean up afterward.
    """

    # Define backend configurations
    backends_to_build = []
    if backend == "postgres":
        backends_to_build = ["postgres"]
    elif backend == "sqlite":
        backends_to_build = ["sqlite"]
    elif backend is None:
        backends_to_build = ["postgres", "sqlite"]
    else:
        print(f"Error: Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.")
        return 1

    all_passed = True
    built_wheels = []
    built_sdist = None
    built_backend_sdists = []

    # Build dispatcher source distribution first
    print(f"\n{'='*50}")
    print("Building Cloaca Dispatcher Source Distribution")
    print(f"{'='*50}")

    try:
        project_root = Path(angreal.get_root()).parent
        cloaca_dir = project_root / "cloaca"

        # Generate dispatcher files first (use any backend since version is the same)
        print("Step 1: Generating dispatcher files...")
        result = generate("postgres")  # Just to generate dispatcher pyproject.toml
        if result != 0:
            print("✗ Failed to generate dispatcher files")
            all_passed = False
        else:
            print("Step 2: Building dispatcher sdist...")
            # Create temporary venv for building sdist
            sdist_venv_name = "sdist-build-env"
            sdist_venv_path = project_root / sdist_venv_name

            try:
                # Create virtual environment for sdist building
                sdist_venv = VirtualEnv(path=str(sdist_venv_path), now=True)
                sdist_python_exe = sdist_venv.path / "bin" / "python"
                sdist_pip_exe = sdist_venv.path / "bin" / "pip3"

                # Install build dependencies
                subprocess.run([str(sdist_python_exe), "-m", "ensurepip"], check=True, capture_output=True)
                subprocess.run([str(sdist_pip_exe), "install", "build"], check=True, capture_output=True)

                # Build the sdist
                subprocess.run([
                    str(sdist_python_exe), "-m", "build", "--sdist"
                ], cwd=str(cloaca_dir), check=True, capture_output=True)

            finally:
                # Clean up sdist build environment
                if sdist_venv_path.exists():
                    shutil.rmtree(sdist_venv_path)

            # Find the built sdist
            dist_dir = cloaca_dir / "dist"
            sdist_files = list(dist_dir.glob("*.tar.gz"))
            if sdist_files:
                built_sdist = sdist_files[0]
                print(f"✓ Built dispatcher sdist: {built_sdist.name}")
            else:
                print("✗ No sdist found in dist directory")
                all_passed = False

    except subprocess.CalledProcessError as e:
        print(f"✗ Dispatcher sdist build failed: {e}")
        if hasattr(e, 'stdout') and e.stdout:
            print("STDOUT:", e.stdout.decode())
        if hasattr(e, 'stderr') and e.stderr:
            print("STDERR:", e.stderr.decode())
        all_passed = False
    except Exception as e:
        print(f"✗ Failed to build dispatcher sdist: {e}")
        all_passed = False

    # Build backend source distributions
    print(f"\n{'='*50}")
    print("Building Backend Source Distributions")
    print(f"{'='*50}")

    for backend_name in backends_to_build:
        print(f"\nBuilding {backend_name} backend sdist...")
        try:
            # Generate backend files
            print("Step 1: Generating backend files...")
            result = generate(backend_name)
            if result != 0:
                print(f"✗ Failed to generate files for {backend_name}")
                all_passed = False
                continue

            print("Step 2: Building backend sdist...")
            backend_dir = project_root / "cloaca-backend"

            # Create temporary venv for building backend sdist
            backend_sdist_venv_name = f"backend-sdist-build-{backend_name}"
            backend_sdist_venv_path = project_root / backend_sdist_venv_name

            try:
                # Create virtual environment for backend sdist building
                backend_sdist_venv = VirtualEnv(path=str(backend_sdist_venv_path), now=True)
                backend_sdist_python_exe = backend_sdist_venv.path / "bin" / "python"
                backend_sdist_pip_exe = backend_sdist_venv.path / "bin" / "pip3"

                # Install build dependencies
                subprocess.run([str(backend_sdist_python_exe), "-m", "ensurepip"], check=True, capture_output=True)
                subprocess.run([str(backend_sdist_pip_exe), "install", "build", "maturin"], check=True, capture_output=True)

                # Build the backend sdist
                subprocess.run([
                    str(backend_sdist_python_exe), "-m", "build", "--sdist"
                ], cwd=str(backend_dir), check=True, capture_output=True)

                # Find the built backend sdist
                backend_dist_dir = backend_dir / "dist"
                backend_sdist_files = list(backend_dist_dir.glob("*.tar.gz"))
                if backend_sdist_files:
                    backend_sdist_file = backend_sdist_files[0]
                    built_backend_sdists.append(backend_sdist_file)
                    print(f"✓ Built {backend_name} backend sdist: {backend_sdist_file.name}")
                else:
                    print(f"✗ No backend sdist found for {backend_name}")
                    all_passed = False

            finally:
                # Clean up backend sdist build environment
                if backend_sdist_venv_path.exists():
                    shutil.rmtree(backend_sdist_venv_path)

        except subprocess.CalledProcessError as e:
            print(f"✗ Backend sdist build failed for {backend_name}: {e}")
            if hasattr(e, 'stdout') and e.stdout:
                print("STDOUT:", e.stdout.decode())
            if hasattr(e, 'stderr') and e.stderr:
                print("STDERR:", e.stderr.decode())
            all_passed = False
        except Exception as e:
            print(f"✗ Failed to build {backend_name} backend sdist: {e}")
            all_passed = False

    for backend_name in backends_to_build:
        print(f"\n{'='*50}")
        print(f"Building {backend_name.title()} release wheel")
        print(f"{'='*50}")

        project_root = Path(angreal.get_root()).parent
        venv_name = f"release-build-{backend_name}"
        venv_path = project_root / venv_name

        try:
            # Step 1: Generate files
            print("Step 1: Generating files...")
            result = generate(backend_name)
            if result != 0:
                all_passed = False
                continue

            # Step 2: Create build environment
            print("Step 2: Creating build environment...")
            venv = VirtualEnv(path=str(venv_path), now=True)

            python_exe = venv.path / "bin" / "python"
            pip_exe = venv.path / "bin" / "pip3"

            # Install pip and maturin
            print("Installing build dependencies...")
            subprocess.run([str(python_exe), "-m", "ensurepip"], check=True, capture_output=True)
            subprocess.run([str(pip_exe), "install", "maturin"], check=True, capture_output=True)

            # Step 3: Build wheel
            print(f"Step 3: Building {backend_name} release wheel...")
            backend_dir = project_root / "cloaca-backend"

            # Clean existing extensions
            for pattern in ["*.so", "*.pyd"]:
                for artifact in backend_dir.rglob(pattern):
                    artifact.unlink()

            # Build wheel
            maturin_exe = venv.path / "bin" / "maturin"
            maturin_cmd = [
                str(maturin_exe), "build",
                "--no-default-features",
                "--features", backend_name,
                "--release"
            ]

            result = subprocess.run(
                maturin_cmd,
                cwd=str(backend_dir),
                capture_output=True,
                text=True,
                check=True
            )

            # Find the built wheel
            wheel_pattern = f"cloaca_{backend_name}-*.whl"
            wheel_dir = backend_dir / "target" / "wheels"
            wheel_files = list(wheel_dir.glob(wheel_pattern))

            if wheel_files:
                wheel_file = wheel_files[0]
                built_wheels.append(wheel_file)
                print(f"✓ Built release wheel: {wheel_file.name}")
            else:
                print(f"✗ No wheel found matching {wheel_pattern} in {wheel_dir}")
                all_passed = False

        except subprocess.CalledProcessError as e:
            print(f"✗ Release build failed for {backend_name}: {e}")
            if e.stdout:
                print("STDOUT:", e.stdout)
            if e.stderr:
                print("STDERR:", e.stderr)
            all_passed = False
        except Exception as e:
            print(f"✗ Failed to build {backend_name} release: {e}")
            all_passed = False
        finally:
            # Clean up build environment (but leave generated files and wheels)
            if venv_path.exists():
                print(f"Cleaning up build environment: {venv_name}")
                shutil.rmtree(venv_path)

    # Summary
    if all_passed:
        print(f"\n{'='*50}")
        print("Release build completed successfully!")
        print(f"{'='*50}")
        print("Built artifacts:")
        if built_sdist:
            print(f"  Dispatcher sdist: {built_sdist}")
        if built_backend_sdists:
            print("  Backend sdists:")
            for backend_sdist in built_backend_sdists:
                print(f"    {backend_sdist}")
        print("  Backend wheels:")
        for wheel in built_wheels:
            print(f"    {wheel}")
        print("\nGenerated files and artifacts preserved for inspection.")
        print("Run 'angreal cloaca scrub' to clean up when ready.")
        return 0
    else:
        print(f"\n{'='*50}")
        print("Some release builds failed!")
        print(f"{'='*50}")
        return 1
