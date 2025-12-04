import angreal # type: ignore
from angreal.integrations.venv import VirtualEnv# type: ignore


import shutil
from pathlib import Path
import subprocess


# Define command group
cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")

@cloaca()
@angreal.command(
    name="release",
    about="build release wheel and sdist for distribution (leaves artifacts for inspection)",
    when_to_use=["preparing production releases", "generating distribution artifacts"],
    when_not_to_use=["development testing", "quick iterations"]
)
def release():
    """Build release wheel and sdist for distribution without cleanup.

    Generates unified cloaca wheel that supports both PostgreSQL and SQLite
    backends at runtime. Leaves all artifacts for inspection.
    Use 'scrub' command to clean up afterward.
    """

    project_root = Path(angreal.get_root()).parent
    backend_dir = project_root / "bindings" / "cloaca-backend"
    venv_name = "release-build-unified"
    venv_path = project_root / venv_name

    built_wheel = None
    built_sdist = None

    try:
        print(f"\n{'='*50}")
        print("Building Unified Cloaca Release")
        print(f"{'='*50}")

        # Step 1: Create build environment
        print("\nStep 1: Creating build environment...")
        venv = VirtualEnv(path=str(venv_path), now=True)

        python_exe = venv.path / "bin" / "python"
        pip_exe = venv.path / "bin" / "pip3"

        # Install pip and build dependencies
        print("Installing build dependencies...")
        subprocess.run([str(python_exe), "-m", "ensurepip"], check=True, capture_output=True)
        subprocess.run([str(pip_exe), "install", "maturin", "build"], check=True, capture_output=True)

        # Step 2: Clean existing extensions
        print("\nStep 2: Cleaning existing compiled extensions...")
        for pattern in ["*.so", "*.pyd"]:
            for artifact in backend_dir.rglob(pattern):
                artifact.unlink()
                print(f"  Removed {artifact.name}")

        # Step 3: Build wheel
        print("\nStep 3: Building unified release wheel...")
        maturin_exe = venv.path / "bin" / "maturin"
        maturin_cmd = [str(maturin_exe), "build", "--release"]

        subprocess.run(
            maturin_cmd,
            cwd=str(backend_dir),
            capture_output=True,
            text=True,
            check=True
        )
        print("Wheel build completed")

        # Find the built wheel
        wheel_pattern = "cloaca-*.whl"
        wheel_dir = backend_dir / "target" / "wheels"
        wheel_files = list(wheel_dir.glob(wheel_pattern))

        if wheel_files:
            built_wheel = wheel_files[0]
            print(f"Built wheel: {built_wheel.name}")
        else:
            raise FileNotFoundError(f"No wheel found matching {wheel_pattern} in {wheel_dir}")

        # Step 4: Build sdist
        print("\nStep 4: Building source distribution...")
        subprocess.run([
            str(python_exe), "-m", "build", "--sdist"
        ], cwd=str(backend_dir), check=True, capture_output=True)

        # Find the built sdist
        dist_dir = backend_dir / "dist"
        sdist_files = list(dist_dir.glob("*.tar.gz"))
        if sdist_files:
            built_sdist = sdist_files[0]
            print(f"Built sdist: {built_sdist.name}")
        else:
            print("Warning: No sdist found in dist directory")

        # Summary
        print(f"\n{'='*50}")
        print("Release build completed successfully!")
        print(f"{'='*50}")
        print("Built artifacts:")
        if built_wheel:
            print(f"  Wheel: {built_wheel}")
        if built_sdist:
            print(f"  Sdist: {built_sdist}")
        print("\nArtifacts preserved for inspection.")
        print("Run 'angreal cloaca scrub' to clean up when ready.")

    except subprocess.CalledProcessError as e:
        print(f"Release build failed: {e}")
        if e.stdout:
            print("STDOUT:", e.stdout)
        if e.stderr:
            print("STDERR:", e.stderr)
        raise RuntimeError("Release build failed")
    except Exception as e:
        print(f"Failed to build release: {e}")
        raise RuntimeError(f"Failed to build release: {e}")
    finally:
        # Clean up build environment (but leave generated files and wheels)
        if venv_path.exists():
            print(f"\nCleaning up build environment: {venv_name}")
            shutil.rmtree(venv_path)
