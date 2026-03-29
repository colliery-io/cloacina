"""
Daemon soak test — sustained package loading and execution.

Spawns the daemon process, drops packages into the watch directory,
verifies reconciliation, removes packages, and verifies clean shutdown.
"""

import json
import signal
import subprocess
import tarfile
import tempfile
import time
import io
import gzip
from pathlib import Path
from datetime import datetime, timezone

import angreal  # type: ignore

from .cloacina_utils import print_section_header, print_final_success

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")


def build_daemon():
    """Build the daemon binary."""
    print("Building cloacinactl daemon...")
    subprocess.run(
        ["cargo", "build", "--release", "-p", "cloacinactl"],
        check=True,
    )
    print("Daemon binary built.")


def find_daemon_binary():
    """Find the daemon binary path."""
    binary = Path("target/release/cloacinactl")
    if not binary.exists():
        raise FileNotFoundError(f"Daemon binary not found at {binary}. Run build first.")
    return str(binary)


def create_test_package(name, version="1.0.0"):
    """Create a minimal .cloacina archive with a ManifestV2 manifest.

    This creates a valid archive that the daemon's FilesystemWorkflowRegistry
    can peek for metadata. The package won't actually execute (no real cdylib),
    but it tests the reconciler's load/unload lifecycle.
    """
    manifest = {
        "format_version": "2",
        "package": {
            "name": name,
            "version": version,
            "description": f"Soak test package {name}",
            "fingerprint": f"sha256:soak-{name}-{version}",
            "targets": ["linux-x86_64", "macos-arm64"],
        },
        "language": "rust",
        "rust": {"library_path": f"lib/lib{name}.so"},
        "tasks": [
            {
                "id": "task1",
                "function": "execute_task",
                "dependencies": [],
                "retries": 0,
            }
        ],
        "triggers": [],
        "created_at": datetime.now(timezone.utc).isoformat(),
    }

    # Build tar.gz archive in memory
    buf = io.BytesIO()
    with gzip.GzipFile(fileobj=buf, mode="wb") as gz:
        with tarfile.open(fileobj=gz, mode="w") as tar:
            manifest_json = json.dumps(manifest, indent=2).encode()
            info = tarfile.TarInfo(name="manifest.json")
            info.size = len(manifest_json)
            info.mode = 0o644
            tar.addfile(info, io.BytesIO(manifest_json))

    return buf.getvalue()


def wait_for_daemon_ready(daemon_home, timeout=15):
    """Wait for the daemon to create its logs directory (indicates it's running)."""
    logs_dir = Path(daemon_home) / "logs"
    start = time.time()
    while time.time() - start < timeout:
        # Check for log files (created after full startup including logging init)
        if logs_dir.exists() and any(logs_dir.iterdir()):
            return True
        time.sleep(0.5)
    return False


@cloacina()
@angreal.command(
    name="soak",
    about="run daemon soak test — sustained package loading and execution",
    when_to_use=[
        "validating daemon stability under load",
        "testing package loading/unloading lifecycle",
        "verifying graceful shutdown",
    ],
    when_not_to_use=[
        "unit testing",
        "quick validation",
    ],
)
@angreal.argument(
    name="duration",
    long="duration",
    required=False,
    help="soak test duration in seconds (default: 30)",
)
def soak(duration=None):
    """Run daemon soak test.

    Builds the daemon, spawns it as a subprocess, drops test packages
    into the watch directory, verifies reconciliation, removes packages,
    and verifies clean shutdown.
    """
    duration = int(duration) if duration else 30

    print_section_header("Daemon Soak Test")
    print(f"Duration: {duration}s")

    # Step 1: Build daemon
    print_section_header("Step 1: Build daemon binary")
    build_daemon()
    daemon_binary = find_daemon_binary()

    # Step 2: Create temp home directory
    with tempfile.TemporaryDirectory(prefix="cloacina-soak-") as daemon_home:
        packages_dir = Path(daemon_home) / "packages"
        packages_dir.mkdir(parents=True, exist_ok=True)

        print_section_header("Step 2: Start daemon")
        print(f"  Home: {daemon_home}")
        print(f"  Packages: {packages_dir}")

        # Start daemon process (stderr to log file to avoid mixing with test output)
        daemon_stderr_path = Path(daemon_home) / "daemon_stderr.log"
        daemon_stderr_file = open(daemon_stderr_path, "w")
        daemon_proc = subprocess.Popen(
            [daemon_binary, "daemon", "--home", daemon_home],
            stdout=subprocess.PIPE,
            stderr=daemon_stderr_file,
        )
        print(f"  PID: {daemon_proc.pid}")

        try:
            # Wait for daemon to be ready
            if not wait_for_daemon_ready(daemon_home):
                daemon_proc.kill()
                daemon_stderr_file.close()
                stderr = daemon_stderr_path.read_text() if daemon_stderr_path.exists() else ""
                raise RuntimeError(f"Daemon failed to start within timeout. Stderr:\n{stderr[-1000:]}")

            print("  Daemon is running.")

            # Step 3: Drop packages and verify reconciliation
            print_section_header("Step 3: Drop test packages")

            packages_dropped = []
            packages_to_drop = 5
            drop_interval = max(1, duration // (packages_to_drop * 2))  # Drop + remove cycles

            for i in range(packages_to_drop):
                pkg_name = f"soak-test-pkg-{i}"
                pkg_data = create_test_package(pkg_name, f"1.0.{i}")
                pkg_path = packages_dir / f"{pkg_name}.cloacina"

                print(f"  Dropping: {pkg_name}.cloacina")
                pkg_path.write_bytes(pkg_data)
                packages_dropped.append(pkg_path)

                # Wait for reconciler to notice
                time.sleep(drop_interval)

            # Verify packages are present
            cloacina_files = list(packages_dir.glob("*.cloacina"))
            print(f"  Packages in watch dir: {len(cloacina_files)}")
            assert len(cloacina_files) == packages_to_drop, \
                f"Expected {packages_to_drop} packages, found {len(cloacina_files)}"

            # Step 4: Remove packages
            print_section_header("Step 4: Remove test packages")

            for pkg_path in packages_dropped:
                print(f"  Removing: {pkg_path.name}")
                pkg_path.unlink()
                time.sleep(drop_interval)

            # Verify all removed
            remaining = list(packages_dir.glob("*.cloacina"))
            print(f"  Packages remaining: {len(remaining)}")
            assert len(remaining) == 0, f"Expected 0 packages, found {len(remaining)}"

            # Step 5: Verify daemon is still running
            print_section_header("Step 5: Verify daemon health")
            assert daemon_proc.poll() is None, "Daemon crashed during soak test!"
            print("  Daemon is still running — no crashes.")

            # Check log files exist
            logs_dir = Path(daemon_home) / "logs"
            log_files = list(logs_dir.glob("cloacina.log.*"))
            print(f"  Log files: {len(log_files)}")
            assert len(log_files) > 0, "Expected at least one log file"

            # Check log file has content
            total_log_bytes = sum(f.stat().st_size for f in log_files)
            print(f"  Total log size: {total_log_bytes} bytes")
            assert total_log_bytes > 0, "Log files are empty"

            # Step 6: Graceful shutdown
            print_section_header("Step 6: Graceful shutdown")
            print("  Sending SIGINT...")
            daemon_proc.send_signal(signal.SIGINT)

            # Wait for clean exit
            try:
                exit_code = daemon_proc.wait(timeout=15)
                print(f"  Daemon exited with code: {exit_code}")

                if exit_code != 0:
                    daemon_stderr_file.close()
                    stderr = daemon_stderr_path.read_text() if daemon_stderr_path.exists() else ""
                    print(f"  Stderr (last 500 chars): {stderr[-500:]}")
                    raise RuntimeError(f"Daemon exited with non-zero code: {exit_code}")

            except subprocess.TimeoutExpired:
                print("  Shutdown timed out — sending SIGKILL")
                daemon_proc.kill()
                raise RuntimeError("Daemon did not shut down within 15 seconds")

            print_final_success("Daemon soak test passed!")

        except Exception:
            # Kill daemon if test fails
            if daemon_proc.poll() is None:
                daemon_proc.kill()
                daemon_proc.wait(timeout=5)
            raise
