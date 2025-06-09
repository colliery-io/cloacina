"""
Build-related functionality for Cloaca tasks.
"""

import subprocess
from pathlib import Path

import angreal  # type: ignore
from angreal.integrations.venv import VirtualEnv  # type: ignore


def build_and_install_cloaca_backend(backend_name: str, venv_name: str):
    """Build cloaca backend wheel and install it in a test environment with dispatcher.

    Assumes files are already generated and docker is set up if needed.
    Only handles virtual environment creation and building.
    Returns the VirtualEnv object and paths to executables.
    """
    project_root = Path(angreal.get_root()).parent
    venv_path = project_root / venv_name

    # Create test environment
    print("Creating test environment...")
    venv = VirtualEnv(path=str(venv_path), now=True)

    python_exe = venv.path / "bin" / "python"
    pip_exe = venv.path / "bin" / "pip3"

    # Install pip and dependencies
    print("Installing dependencies...")
    subprocess.run([str(python_exe), "-m", "ensurepip"], check=True, capture_output=True)
    subprocess.run([str(pip_exe), "install", "maturin", "pytest", "pytest-asyncio", "psycopg2", "pytest-timeout"], check=True, capture_output=True)

    # Install dispatcher package
    print("Installing dispatcher package...")
    subprocess.run([str(pip_exe), "install", "-e", str(project_root / "cloaca")], check=True, capture_output=True)

    # Build and install backend wheel
    print(f"Building and installing {backend_name} wheel...")
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

    subprocess.run(
        maturin_cmd,
        cwd=str(backend_dir),
        capture_output=True,
        text=True,
        check=True
    )

    # Find and install the wheel
    wheel_pattern = f"cloaca_{backend_name}-*.whl"
    wheel_dir = backend_dir / "target" / "wheels"
    wheel_files = list(wheel_dir.glob(wheel_pattern))

    if not wheel_files:
        raise FileNotFoundError(f"No wheel found matching {wheel_pattern} in {wheel_dir}")

    wheel_file = wheel_files[0]
    print(f"Installing wheel: {wheel_file.name}")
    subprocess.run([str(pip_exe), "install", str(wheel_file)], check=True, capture_output=True)

    return venv, python_exe, pip_exe
