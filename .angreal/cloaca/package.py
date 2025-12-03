import angreal # type: ignore
import shutil
from pathlib import Path
import subprocess

from angreal.integrations.venv import VirtualEnv  # type: ignore


cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")


@cloaca()
@angreal.command(
    name="package",
    about="build unified cloaca wheel",
    when_to_use=["building the wheel locally", "testing wheel creation", "local development packaging"],
    when_not_to_use=["release builds", "CI/CD pipelines"]
)
def package():
    """Build the unified cloaca wheel."""
    try:
        project_root = Path(angreal.get_root()).parent
        backend_dir = project_root / "cloaca-backend"

        # Create temporary virtual environment for building
        venv_name = "build-env-unified"
        venv_path = project_root / venv_name

        try:
            # Create virtual environment
            print("Creating build environment...")
            venv = VirtualEnv(path=str(venv_path), now=True)

            # Install pip and maturin
            python_exe = venv.path / "bin" / "python"
            print("Installing build dependencies...")
            subprocess.run([str(python_exe), "-m", "ensurepip"], check=True, capture_output=True)

            pip_exe = venv.path / "bin" / "pip3"
            subprocess.run([str(pip_exe), "install", "maturin"], check=True, capture_output=True)

            # Clean up any existing .so files
            print("Cleaning existing compiled extensions...")
            for pattern in ["*.so", "*.pyd"]:
                for artifact in backend_dir.rglob(pattern):
                    artifact.unlink()
                    print(f"  Removed {artifact.name}")

            # Build the unified wheel
            print("Building unified cloaca wheel...")
            maturin_exe = venv.path / "bin" / "maturin"
            maturin_cmd = [str(maturin_exe), "build", "--release"]

            result = subprocess.run(
                maturin_cmd,
                cwd=str(backend_dir),
                capture_output=True,
                text=True,
                check=True
            )
            print("Build completed successfully")

            # Find the built wheel
            wheel_pattern = "cloaca-*.whl"
            wheel_dir = backend_dir / "target" / "wheels"
            wheel_files = list(wheel_dir.glob(wheel_pattern))

            if wheel_files:
                wheel_file = wheel_files[0]
                print(f"Built wheel: {wheel_file}")
            else:
                print(f"Warning: No wheel found matching {wheel_pattern} in {wheel_dir}")

        except subprocess.CalledProcessError as e:
            print(f"Build failed with exit code {e.returncode}")
            if e.stdout:
                print(f"STDOUT: {e.stdout}")
            if e.stderr:
                print(f"STDERR: {e.stderr}")
            raise RuntimeError("Failed to build unified wheel")
        finally:
            # Clean up the build environment
            if venv_path.exists():
                print(f"Cleaning up build environment: {venv_name}")
                shutil.rmtree(venv_path)

        print("Successfully built unified cloaca wheel!")

    except Exception as e:
        print(f"Build failed: {e}")
        raise RuntimeError(f"Failed to package unified wheel: {e}")
