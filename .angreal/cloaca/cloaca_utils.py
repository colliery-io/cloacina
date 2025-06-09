import angreal #type: ignore
from angreal.integrations.venv import VirtualEnv# type: ignore


from dataclasses import dataclass
from typing import List, Optional
import shutil
from pathlib import Path
import re
import subprocess



@dataclass
class TestResult:
    """Represents the result of running a test file."""
    file_name: str
    backend: str
    passed: bool
    stdout: str = ""
    stderr: str = ""
    return_code: Optional[int] = None


class TestAggregator:
    """Aggregates test results across all backends."""

    def __init__(self):
        self.results: List[TestResult] = []

    def add_result(self, result: TestResult):
        self.results.append(result)

    def get_failed_results(self) -> List[TestResult]:
        return [r for r in self.results if not r.passed]

    def get_summary(self) -> dict:
        total = len(self.results)
        failed = len(self.get_failed_results())
        passed = total - failed

        backends = {}
        for result in self.results:
            if result.backend not in backends:
                backends[result.backend] = {"passed": 0, "failed": 0}
            if result.passed:
                backends[result.backend]["passed"] += 1
            else:
                backends[result.backend]["failed"] += 1

        return {
            "total": total,
            "passed": passed,
            "failed": failed,
            "backends": backends
        }


def write_file_safe(path: Path, content: str, encoding: str = "utf-8", backup: bool = False):
    """Safely write a file with error handling.

    Args:
        path: File path to write
        content: Content to write
        encoding: File encoding
        backup: Whether to backup existing file

    Returns:
        Path to backup file if backup=True and file existed, None otherwise

    Raises:
        Exception: If any error occurs during file operations
    """
    try:
        backup_path = None

        if backup and path.exists():
            backup_path = path.with_suffix(path.suffix + ".backup")
            shutil.copy2(path, backup_path)

        # Ensure parent directory exists
        path.parent.mkdir(parents=True, exist_ok=True)

        path.write_text(content, encoding=encoding)
        return backup_path

    except Exception as e:
        raise Exception(f"Failed to write file {path}: {e}")

def normalize_version_for_python(cargo_version: str) -> str:
    """Convert Cargo SemVer to PEP 440 compliant version.

    Args:
        cargo_version: Version string from Cargo.toml (e.g., "0.2.0-alpha.4")

    Returns:
        PEP 440 compliant version string (e.g., "0.2.0a4")

    Examples:
        >>> normalize_version_for_python("0.2.0-alpha.4")
        "0.2.0a4"
        >>> normalize_version_for_python("0.2.0-beta.3")
        "0.2.0b3"
        >>> normalize_version_for_python("1.0.0")
        "1.0.0"
    """
    # Convert alpha pre-releases: 0.2.0-alpha.4 -> 0.2.0a4
    version = re.sub(r'-alpha\.(\d+)', r'a\1', cargo_version)

    # Convert beta pre-releases: 0.2.0-beta.3 -> 0.2.0b3
    version = re.sub(r'-beta\.(\d+)', r'b\1', version)

    return version


def get_workspace_version() -> str:
    """Extract version from workspace Cargo.toml.

    Returns:
        Version string from workspace configuration

    Raises:
        ValueError: If version cannot be found in workspace Cargo.toml
    """
    project_root = Path(angreal.get_root()).parent
    cargo_toml = project_root / "Cargo.toml"

    if not cargo_toml.exists():
        raise FileNotFoundError(f"Workspace Cargo.toml not found at {cargo_toml}")

    content = cargo_toml.read_text()
    match = re.search(r'\[workspace\.package\].*?version\s*=\s*"([^"]+)"', content, re.DOTALL)

    if match:
        return match.group(1)

    raise ValueError("Could not find version in workspace Cargo.toml")


def _build_and_install_cloaca_backend(backend_name, venv_name):
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
