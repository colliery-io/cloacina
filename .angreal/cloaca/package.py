import angreal # type: ignore
import shutil
from pathlib import Path

from angreal.integrations.venv import VirtualEnv  # type: ignore
import subprocess  # noqa: F821
from .scrub import scrub  # noqa: F821


from .generate import generate


cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")


@cloaca()
@angreal.command(
    name="package", 
    about="generate files, build wheel, then clean",
    when_to_use=["building individual backend wheels", "testing wheel creation", "local development packaging"],
    when_not_to_use=["release builds", "building multiple backends", "CI/CD pipelines"]
)
@angreal.argument(name="backend", long="backend", required=True, help="target backend: postgres or sqlite")
def package(backend=None):
    """Generate files, build the wheel, then clean up generated files."""
    try:
        # Step 1: Generate files
        print("Step 1: Generating files...")
        result = generate(backend)
        if result != 0:
            return result

        # Step 2: Build wheel
        print("Step 2: Building wheel...")
        project_root = Path(angreal.get_root()).parent
        backend_dir = project_root / "cloaca-backend"

        # Create temporary virtual environment for building
        venv_name = f"build-env-{backend}"
        venv_path = project_root / venv_name

        try:
            # Create virtual environment
            print(f"Creating build environment: {venv_name}")
            venv = VirtualEnv(path=str(venv_path), now=True)

            # Install pip and maturin
            python_exe = venv.path / "bin" / "python"
            print("Installing build dependencies...")
            subprocess.run([str(python_exe), "-m", "ensurepip"], check=True, capture_output=True)

            pip_exe = venv.path / "bin" / "pip3"
            subprocess.run([str(pip_exe), "install", "maturin"], check=True, capture_output=True)

            # Clean up any existing .so files to avoid conflicts
            print("Cleaning existing compiled extensions...")
            for pattern in ["*.so", "*.pyd"]:
                for artifact in backend_dir.rglob(pattern):
                    artifact.unlink()
                    print(f"  Removed {artifact.name}")

            # Build the wheel using maturin
            print(f"Building {backend} wheel...")
            maturin_exe = venv.path / "bin" / "maturin"
            maturin_cmd = [
                str(maturin_exe), "build",
                "--no-default-features",
                "--features", backend,
                "--release"
            ]

            result = subprocess.run(
                maturin_cmd,
                cwd=str(backend_dir),
                capture_output=True,
                text=True,
                check=True
            )
            print("  Build completed successfully")

            # Find the built wheel
            wheel_pattern = f"cloaca_{backend}-*.whl"
            wheel_dir = backend_dir / "target" / "wheels"
            wheel_files = list(wheel_dir.glob(wheel_pattern))

            if wheel_files:
                wheel_file = wheel_files[0]
                print(f"  Built wheel: {wheel_file.name}")
            else:
                print(f"  Warning: No wheel found matching {wheel_pattern} in {wheel_dir}")

        except subprocess.CalledProcessError as e:
            print(f"  Build failed with exit code {e.returncode}")
            if e.stdout:
                print(f"  STDOUT: {e.stdout}")
            if e.stderr:
                print(f"  STDERR: {e.stderr}")
            return 1
        except Exception as e:
            print(f"  Build failed: {e}")
            return 1
        finally:
            # Clean up the build environment
            if venv_path.exists():
                print(f"Cleaning up build environment: {venv_name}")
                shutil.rmtree(venv_path)

        # Step 3: Clean up
        print("Step 3: Cleaning generated files...")
        result = scrub()
        if result != 0:
            return result

        print(f"Successfully built {backend} backend!")
        return 0

    except Exception as e:
        print(f"Build failed: {e}")
        return 1
