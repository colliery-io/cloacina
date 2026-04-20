import angreal #type: ignore
from angreal.integrations.venv import VirtualEnv# type: ignore


from dataclasses import dataclass
from typing import List, Optional
import shutil
from pathlib import Path
import re
import subprocess


def run_pytest_scenarios(
    venv,
    project_root: Path,
    backend_name: str,
    aggregator: "TestAggregator",
    filter: Optional[str] = None,
    file: Optional[str] = None,
) -> bool:
    """Run all (or filtered) tests/python/test_scenario_*.py against an already-built wheel.

    Caller is responsible for:
      - Building/installing the cloaca wheel into `venv` (use _build_and_install_cloaca_unified).
      - Bringing up the database for `backend_name` (Docker postgres or local sqlite).

    For postgres, this resets the schema between scenario files via smart_postgres_reset
    (falling back to docker restart). For sqlite, it deletes lingering *.db files.

    Returns True if all scenarios passed, False otherwise. Per-file results are added
    to `aggregator`.
    """
    import os
    from utils import (  # local import to avoid making this module depend on top-level utils at import time
        docker_up,
        docker_down,
        check_postgres_container_health,
        smart_postgres_reset,
    )
    import time

    test_dir = project_root / "tests" / "python"
    if file:
        test_file_path = test_dir / file
        if not test_file_path.exists():
            print(f"Error: Test file {file} not found in {test_dir}")
            return False
        test_files = [test_file_path]
    else:
        test_files = sorted(test_dir.glob("test_*.py"))
        if filter:
            test_files = [f for f in test_files if filter in f.name]

    print(f"Found {len(test_files)} python scenario files to run for {backend_name}")

    pytest_exe = venv.path / "bin" / "pytest"
    env = os.environ.copy()
    env["CLOACA_BACKEND"] = backend_name

    all_passed = True
    file_results = []

    for test_file in test_files:
        print(f"\n--- pytest {test_file.name} ({backend_name}) ---", flush=True)

        if backend_name == "postgres":
            if smart_postgres_reset():
                print("PostgreSQL state reset")
            else:
                print("Fast reset failed, restarting Docker...")
                docker_down(remove_volumes=True)
                docker_up()
                time.sleep(10)
                if not check_postgres_container_health():
                    print(f"PostgreSQL unhealthy for {test_file.name}")
                    file_results.append((test_file.name, False))
                    all_passed = False
                    continue
        elif backend_name == "sqlite":
            for db_file in project_root.glob("*.db*"):
                try:
                    db_file.unlink()
                except FileNotFoundError:
                    pass

        cmd = [str(pytest_exe), "--timeout=10", str(test_file), "-v"]
        if filter:
            cmd.extend(["-k", filter])

        result = subprocess.run(cmd, env=env, capture_output=True, text=True)
        passed = result.returncode == 0
        aggregator.add_result(
            TestResult(
                file_name=test_file.name,
                backend=backend_name,
                passed=passed,
                stdout=result.stdout,
                stderr=result.stderr,
                return_code=result.returncode,
            )
        )
        file_results.append((test_file.name, passed))
        if passed:
            print(f"PASSED: {test_file.name}")
        else:
            print(f"FAILED: {test_file.name}")
            print("\n--- PYTEST OUTPUT ---")
            print(result.stdout)
            if result.stderr:
                print("\n--- STDERR ---")
                print(result.stderr)
            print("--- END OUTPUT ---\n")
            all_passed = False

    passed = [n for n, ok in file_results if ok]
    failed = [n for n, ok in file_results if not ok]
    print(f"\nPython {backend_name} scenarios: {len(passed)} passed, {len(failed)} failed")
    return all_passed


def scrub_python_artifacts(deep: bool = False) -> int:
    """Clean Python build artifacts and test environments.

    Used by `cloacina purge` (deep=True). Removes:
    - test venvs (smoke-test-*, test-env-*, debug-env-*, tutorial-*)
    - __pycache__ directories
    - SQLite *.db files (project root + /tmp/cloacina_*.db)
    Optionally runs `cargo clean` when deep=True.

    Returns 0 on success, non-zero on failure.
    """
    try:
        project_root = Path(angreal.get_root()).parent

        envs_cleaned = 0
        for env_pattern in ["smoke-test-*", "test-env-*", "debug-env-*", "tutorial-*"]:
            for env_dir in project_root.glob(env_pattern):
                if env_dir.is_dir():
                    shutil.rmtree(env_dir)
                    envs_cleaned += 1
        if envs_cleaned:
            print(f"Cleaned {envs_cleaned} test environments")

        caches_cleaned = 0
        for cache_dir in project_root.rglob("__pycache__"):
            shutil.rmtree(cache_dir)
            caches_cleaned += 1
        if caches_cleaned:
            print(f"Cleaned {caches_cleaned} __pycache__ directories")

        db_files_cleaned = 0
        for db_file in project_root.glob("*.db*"):
            db_file.unlink()
            db_files_cleaned += 1
        for tmp_db in ["/tmp/cloacina_demo.db", "/tmp/cloacina_debug.db"]:
            p = Path(tmp_db)
            if p.exists():
                p.unlink()
                db_files_cleaned += 1
        if db_files_cleaned:
            print(f"Cleaned {db_files_cleaned} database files")

        if deep:
            print("Running cargo clean...")
            result = subprocess.run(
                ["cargo", "clean"], cwd=str(project_root), capture_output=True, text=True
            )
            if result.returncode != 0:
                print(f"cargo clean warning: {result.stderr}")

        return 0
    except Exception as e:
        print(f"Python scrub failed: {e}")
        return 1



@dataclass
class TestResult:
    """Represents the result of running a test file."""
    file_name: str
    backend: str
    passed: bool
    stdout: str = ""
    stderr: str = ""
    return_code: Optional[int] = None

    def get_failure_summary(self) -> str:
        """Extract a concise failure summary from pytest output."""
        if self.passed:
            return ""

        lines = []

        # Look for FAILED lines in stdout
        for line in self.stdout.split('\n'):
            if 'FAILED' in line or 'ERROR' in line:
                lines.append(line.strip())
            # Capture assertion errors
            elif 'AssertionError' in line or 'assert ' in line:
                lines.append(line.strip())
            # Capture exception messages
            elif 'Error:' in line or 'Exception:' in line:
                lines.append(line.strip())

        # Also check stderr
        for line in self.stderr.split('\n'):
            if 'Error' in line or 'Exception' in line:
                lines.append(line.strip())

        return '\n'.join(lines[:10]) if lines else "No specific error extracted"

    def get_short_failures(self) -> str:
        """Extract the pytest short test summary from output."""
        if self.passed:
            return ""

        # Find the short test summary section
        in_summary = False
        summary_lines = []

        for line in self.stdout.split('\n'):
            if '= short test summary info =' in line or '= FAILURES =' in line:
                in_summary = True
                continue
            if in_summary:
                if line.startswith('=') and '=' in line[1:]:
                    break
                if line.strip():
                    summary_lines.append(line)

        return '\n'.join(summary_lines[:20]) if summary_lines else ""


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

    def print_failure_report(self):
        """Print a detailed report of all failures."""
        failed = self.get_failed_results()
        if not failed:
            return

        print(f"\n{'='*60}")
        print("DETAILED FAILURE REPORT")
        print(f"{'='*60}")

        for i, result in enumerate(failed, 1):
            print(f"\n[{i}/{len(failed)}] {result.file_name} ({result.backend})")
            print("-" * 50)

            # Print the short test summary (most useful)
            short_failures = result.get_short_failures()
            if short_failures:
                print("PYTEST FAILURES:")
                print(short_failures)
            else:
                # Fall back to extracted error lines
                failure_summary = result.get_failure_summary()
                if failure_summary:
                    print("ERROR SUMMARY:")
                    print(failure_summary)

            # Print return code
            print(f"\nReturn code: {result.return_code}")

            # Offer to show full output
            print(f"\nFull stdout length: {len(result.stdout)} chars")
            print(f"Full stderr length: {len(result.stderr)} chars")

        print(f"\n{'='*60}")


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


def _build_and_install_cloaca_unified(venv_name):
    """Build unified cloaca wheel and install it in a test environment.

    The unified wheel supports both PostgreSQL and SQLite at runtime.
    Returns the VirtualEnv object and paths to executables.
    """
    project_root = Path(angreal.get_root()).parent
    venv_path = project_root / venv_name

    # Create test environment
    print("[DEBUG] Step 1: Creating test environment...", flush=True)
    venv = VirtualEnv(path=str(venv_path), now=True)
    print(f"[DEBUG] Step 1 complete: venv at {venv.path}", flush=True)

    python_exe = venv.path / "bin" / "python"
    pip_exe = venv.path / "bin" / "pip3"

    # Install pip and dependencies
    print("[DEBUG] Step 2: Installing pip via ensurepip...", flush=True)
    subprocess.run([str(python_exe), "-m", "ensurepip"], check=True, capture_output=True)
    print("[DEBUG] Step 2 complete", flush=True)

    # Base dependencies for all backends
    print("[DEBUG] Step 3: Installing dependencies...", flush=True)
    deps = ["maturin", "pytest", "pytest-asyncio", "pytest-timeout", "psycopg2-binary"]
    subprocess.run([str(pip_exe), "install"] + deps, check=True, capture_output=True)
    print("[DEBUG] Step 3 complete", flush=True)

    # Build and install unified wheel from cloacina-python
    # (pyproject.toml moved there in CLOACI-T-0529 so the Python bindings
    # stop dragging pyo3 through cloacina core).
    print("[DEBUG] Step 4: Building cloaca wheel from cloacina-python...", flush=True)
    crate_dir = project_root / "crates" / "cloacina-python"

    # Build wheel using maturin (pyproject.toml is in crates/cloacina-python/)
    maturin_exe = venv.path / "bin" / "maturin"
    maturin_cmd = [
        str(maturin_exe), "build",
        "--release",
        "--manylinux", "off",  # skip auditwheel repair (avoids libpq.so resolution)
    ]

    print(f"[DEBUG] Running: {' '.join(maturin_cmd)} in {crate_dir}", flush=True)
    result = subprocess.run(
        maturin_cmd,
        cwd=str(crate_dir),
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"[DEBUG] Maturin STDERR: {result.stderr}", flush=True)
        print(f"[DEBUG] Maturin STDOUT: {result.stdout}", flush=True)
        raise subprocess.CalledProcessError(result.returncode, maturin_cmd)
    print("[DEBUG] Step 4 complete: wheel built", flush=True)

    # Find and install the wheel
    print("[DEBUG] Step 5: Finding and installing wheel...", flush=True)
    wheel_pattern = "cloaca-*.whl"
    # Maturin puts wheels in the workspace target, not the crate target
    wheel_dir = project_root / "target" / "wheels"
    wheel_files = list(wheel_dir.glob(wheel_pattern))

    if not wheel_files:
        raise FileNotFoundError(f"No wheel found matching {wheel_pattern} in {wheel_dir}")

    wheel_file = wheel_files[0]
    print(f"[DEBUG] Installing wheel: {wheel_file.name}", flush=True)
    subprocess.run([str(pip_exe), "install", str(wheel_file)], check=True, capture_output=True)
    print("[DEBUG] Step 5 complete: wheel installed", flush=True)

    return venv, python_exe, pip_exe
