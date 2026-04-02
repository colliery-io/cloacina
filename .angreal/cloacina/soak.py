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
from pathlib import Path

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
    """Create a fidius source package (.cloacina bzip2 tar) with a real compilable Rust project.

    The package contains package.toml + Cargo.toml + src/lib.rs with a minimal
    #[workflow] that the daemon's reconciler can compile and load via fidius.
    """
    safe_name = name.replace("-", "_")
    prefix = f"{name}-{version}"

    package_toml = f"""[package]
name = "{name}"
version = "{version}"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
workflow_name = "{safe_name}"
language = "rust"
description = "Soak test package {name}"
author = "soak-test"
"""

    cargo_toml = f"""[package]
name = "{name}"
version = "{version}"
edition = "2021"

[workspace]

[features]
default = ["packaged"]
packaged = []

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
cloacina-macros = {{ path = "../../../crates/cloacina-macros" }}
cloacina-workflow = {{ path = "../../../crates/cloacina-workflow", features = ["packaged"] }}
cloacina-workflow-plugin = {{ path = "../../../crates/cloacina-workflow-plugin" }}
serde_json = "1.0"
async-trait = "0.1"

[build-dependencies]
cloacina-build = {{ path = "../../../crates/cloacina-build" }}
"""

    lib_rs = f"""use cloacina_macros::workflow;
use cloacina_workflow::{{Context, TaskError, task}};

#[workflow(name = "{safe_name}")]
pub mod {safe_name} {{
    use super::*;

    #[task(id = "task1", dependencies = [])]
    pub async fn task1(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {{
        context.set("soak_test", serde_json::json!(true));
        Ok(())
    }}
}}
"""

    build_rs = """fn main() {
    cloacina_build::configure();
}
"""

    # Build bzip2 tar archive matching fidius pack_package format
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:bz2") as tar:
        for rel_path, content in [
            ("package.toml", package_toml),
            ("Cargo.toml", cargo_toml),
            ("src/lib.rs", lib_rs),
            ("build.rs", build_rs),
        ]:
            data = content.encode()
            archive_path = f"{prefix}/{rel_path}"
            entry = tarfile.TarInfo(name=archive_path)
            entry.size = len(data)
            entry.mode = 0o644
            tar.addfile(entry, io.BytesIO(data))

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

            # Step 5: Verify daemon is still running and parse logs
            print_section_header("Step 5: Verify daemon health")
            assert daemon_proc.poll() is None, "Daemon crashed during soak test!"
            print("  Daemon is still running — no crashes.")

            # Parse JSON log files for reconciliation results
            logs_dir = Path(daemon_home) / "logs"
            log_files = sorted(logs_dir.glob("cloacina.log.*"))
            assert len(log_files) > 0, "Expected at least one log file"

            total_log_bytes = sum(f.stat().st_size for f in log_files)
            print(f"  Log files: {len(log_files)} ({total_log_bytes} bytes)")

            # Parse structured JSON logs for verification
            reconcile_events = []
            errors = []
            warnings = []
            for log_file in log_files:
                for line in log_file.read_text().splitlines():
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        entry = json.loads(line)
                        level = entry.get("level", "")
                        msg = entry.get("fields", {}).get("message", "")

                        if "econcil" in msg.lower():
                            reconcile_events.append(msg)
                        if level == "ERROR":
                            errors.append(msg)
                        elif level == "WARN":
                            warnings.append(msg)
                    except json.JSONDecodeError:
                        continue

            print(f"  Reconciliation events: {len(reconcile_events)}")
            print(f"  Errors: {len(errors)}")
            print(f"  Warnings: {len(warnings)}")

            # Print reconciliation summary
            if reconcile_events:
                print("  Reconciliation log:")
                for evt in reconcile_events[:10]:
                    print(f"    - {evt[:120]}")
                if len(reconcile_events) > 10:
                    print(f"    ... and {len(reconcile_events) - 10} more")

            # Print errors if any
            if errors:
                print("  Error log:")
                for err in errors[:5]:
                    print(f"    - {err[:120]}")

            # Verify reconciler actually saw the packages
            assert len(reconcile_events) > 0, \
                "No reconciliation events found — daemon may not have detected packages"

            # Also dump stderr for human inspection
            daemon_stderr_file.flush()
            stderr_content = daemon_stderr_path.read_text() if daemon_stderr_path.exists() else ""
            stderr_lines = [line for line in stderr_content.splitlines() if line.strip()]
            if stderr_lines:
                print(f"  Stderr summary ({len(stderr_lines)} lines):")
                # Show last few lines
                for line in stderr_lines[-5:]:
                    print(f"    {line[:150]}")

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
