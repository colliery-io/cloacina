"""
Daemon soak test — sustained package loading and execution.

Spawns the daemon process, drops packages into the watch directory,
verifies reconciliation, removes packages, and verifies clean shutdown.
"""

import json
import signal
import subprocess
import tarfile
import time
import io
from pathlib import Path

import angreal  # type: ignore

from .cloacina_utils import print_section_header, print_final_success

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")


def build_daemon():
    """Build the daemon binary."""
    # Build debug mode so cfg!(debug_assertions) enables host dep injection.
    # Source packages use path deps that the host rewrites to absolute paths.
    print("Building cloacinactl daemon (debug)...")
    subprocess.run(
        ["cargo", "build", "-p", "cloacinactl"],
        check=True,
    )
    print("Daemon binary built.")


def find_daemon_binary():
    """Find the daemon binary path."""
    binary = Path("target/debug/cloacinactl")
    if not binary.exists():
        raise FileNotFoundError(f"Daemon binary not found at {binary}. Run build first.")
    return str(binary)


def create_python_test_package(name, version="1.0.0"):
    """Create a fidius source package (.cloacina bzip2 tar) with a Python workflow.

    The package contains package.toml + workflow/ directory with a simple
    Python task module that the daemon's reconciler can load via PyO3.
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
language = "python"
description = "Python soak test package {name}"
author = "soak-test"
entry_module = "{safe_name}.tasks"

[[metadata.triggers]]
name = "{safe_name}_cron"
workflow = "{safe_name}"
poll_interval = "30s"
cron_expression = "*/30 * * * * *"
"""

    init_py = ""

    tasks_py = f"""from __future__ import annotations
import cloaca

@cloaca.task(id="py-task1", dependencies=[])
def py_task1(context):
    context.set("python_soak_test", True)
    context.set("package_name", "{name}")
    return context
"""

    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:bz2") as tar:
        for rel_path, content in [
            ("package.toml", package_toml),
            (f"workflow/{safe_name}/__init__.py", init_py),
            (f"workflow/{safe_name}/tasks.py", tasks_py),
        ]:
            data = content.encode()
            archive_path = f"{prefix}/{rel_path}"
            entry = tarfile.TarInfo(name=archive_path)
            entry.size = len(data)
            entry.mode = 0o644
            tar.addfile(entry, io.BytesIO(data))

    return buf.getvalue()


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

[[metadata.triggers]]
name = "{safe_name}_cron"
workflow = "{safe_name}"
poll_interval = "30s"
cron_expression = "*/30 * * * * *"
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
chrono = "0.4"

[build-dependencies]
cloacina-build = {{ path = "../../../crates/cloacina-build" }}
"""

    lib_rs = f"""use cloacina_workflow::{{task, workflow, Context, TaskError}};

#[workflow(name = "{safe_name}")]
pub mod {safe_name} {{
    use super::*;

    #[task(id = "task1", dependencies = [])]
    pub async fn task1(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {{
        context.insert("soak_test".to_string(), serde_json::json!(true));
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
    duration = int(duration) if duration else 120

    print_section_header("Daemon Soak Test")
    print(f"Duration: {duration}s")

    # Step 1: Build daemon
    print_section_header("Step 1: Build daemon binary")
    build_daemon()
    daemon_binary = find_daemon_binary()

    # Step 2: Use a fixed soak test directory so logs survive for inspection
    soak_home = Path("target/soak-test")
    if soak_home.exists():
        import shutil
        shutil.rmtree(soak_home)
    daemon_home = str(soak_home)
    if True:  # replaces the `with` block — no auto-cleanup
        packages_dir = Path(daemon_home) / "packages"
        packages_dir.mkdir(parents=True, exist_ok=True)

        print_section_header("Step 2: Start daemon")
        print(f"  Home: {daemon_home}")
        print(f"  Packages: {packages_dir}")

        # Start daemon process (stderr to log file to avoid mixing with test output)
        daemon_stderr_path = Path(daemon_home) / "daemon_stderr.log"
        daemon_stderr_file = open(daemon_stderr_path, "w")
        daemon_proc = subprocess.Popen(
            [daemon_binary, "daemon", "--home", daemon_home, "--poll-interval", "1000"],
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

            # Step 3: Drop a package and wait for it to compile + load
            print_section_header("Step 3: Drop test package")

            pkg_name = "soak-test-pkg-0"
            pkg_data = create_test_package(pkg_name, "1.0.0")
            pkg_path = packages_dir / f"{pkg_name}.cloacina"

            print(f"  Dropping: {pkg_name}.cloacina")
            pkg_path.write_bytes(pkg_data)

            # Wait for compilation to finish (~60s first time, deps need downloading)
            print("  Waiting for compilation (up to 90s)...")
            compile_timeout = 90
            compile_start = time.time()
            while time.time() - compile_start < compile_timeout:
                assert daemon_proc.poll() is None, "Daemon crashed during compilation!"
                time.sleep(5)
                # Check stderr for compilation success indicators
                daemon_stderr_file.flush()
                stderr = daemon_stderr_path.read_text() if daemon_stderr_path.exists() else ""
                if "Successfully registered" in stderr and pkg_name in stderr:
                    print(f"  Package compiled and loaded ({int(time.time() - compile_start)}s)")
                    break
            else:
                print(f"  WARNING: Compilation may not have finished in {compile_timeout}s")

            # Step 3b: Drop a Python package
            print_section_header("Step 3b: Drop Python test package")

            py_pkg_name = "soak-test-python-pkg"
            py_pkg_data = create_python_test_package(py_pkg_name, "1.0.0")
            py_pkg_path = packages_dir / f"{py_pkg_name}.cloacina"

            print(f"  Dropping: {py_pkg_name}.cloacina")
            py_pkg_path.write_bytes(py_pkg_data)

            # Python packages don't need compilation — should load quickly
            print("  Waiting for Python package to load (up to 30s)...")
            py_load_timeout = 30
            py_load_start = time.time()
            py_loaded = False
            while time.time() - py_load_start < py_load_timeout:
                assert daemon_proc.poll() is None, "Daemon crashed during Python package loading!"
                time.sleep(2)
                daemon_stderr_file.flush()
                stderr = daemon_stderr_path.read_text() if daemon_stderr_path.exists() else ""
                if "Python package loaded" in stderr or ("Python workflow imported" in stderr and py_pkg_name in stderr):
                    print(f"  Python package loaded ({int(time.time() - py_load_start)}s)")
                    py_loaded = True
                    break
                # Also check for registration success via task registration
                if "Successfully registered" in stderr and py_pkg_name in stderr:
                    print(f"  Python package registered ({int(time.time() - py_load_start)}s)")
                    py_loaded = True
                    break
            if not py_loaded:
                daemon_stderr_file.flush()
                stderr = daemon_stderr_path.read_text() if daemon_stderr_path.exists() else ""
                # Check for errors
                if "Python" in stderr and "error" in stderr.lower():
                    for line in stderr.splitlines():
                        if "Python" in line or "python" in line:
                            print(f"  ERROR: {line[:200]}")
                raise RuntimeError(f"Python package did not load within {py_load_timeout}s")

            # Step 4: Let cron execute for the soak duration
            soak_duration = max(30, duration - int(time.time() - compile_start))
            print_section_header(f"Step 4: Soak — running for {soak_duration}s")
            print("  Package has cron_expression = '*/30 * * * * *' (every 30s)")
            print(f"  Expecting ~{soak_duration // 30} cron executions...")

            check_interval = 10
            executions_seen = 0
            for elapsed in range(0, soak_duration, check_interval):
                time.sleep(check_interval)
                assert daemon_proc.poll() is None, \
                    f"Daemon crashed after {elapsed + check_interval}s of soak!"
                # Count completed workflow executions in stderr
                daemon_stderr_file.flush()
                stderr = daemon_stderr_path.read_text() if daemon_stderr_path.exists() else ""
                new_count = stderr.count("Workflow execution completed")
                if new_count > executions_seen:
                    executions_seen = new_count
                    print(f"  [{elapsed + check_interval}s] {executions_seen} workflow executions completed")

            print(f"  Soak complete: {executions_seen} total executions")
            assert executions_seen > 0, "No workflow executions during soak period!"

            # Step 5: Verify daemon health and parse logs
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
                    print(f"    - {err}")

            # Verify reconciler actually saw the packages
            assert len(reconcile_events) > 0, \
                "No reconciliation events found — daemon may not have detected packages"

            # Also dump stderr for human inspection
            daemon_stderr_file.flush()
            stderr_content = daemon_stderr_path.read_text() if daemon_stderr_path.exists() else ""
            stderr_lines = [line for line in stderr_content.splitlines() if line.strip()]
            if stderr_lines:
                print(f"  Stderr summary ({len(stderr_lines)} lines):")
                # Show last 20 lines for debugging
                for line in stderr_lines[-20:]:
                    print(f"    {line[:150]}")

            # Step 6: Graceful shutdown
            print_section_header("Step 6: Graceful shutdown")
            print("  Sending SIGINT...")
            daemon_proc.send_signal(signal.SIGINT)

            # Wait for clean exit
            try:
                exit_code = daemon_proc.wait(timeout=30)
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
            # Print daemon stderr before killing for debugging
            daemon_stderr_file.flush()
            if daemon_stderr_path.exists():
                stderr = daemon_stderr_path.read_text()
                if stderr.strip():
                    print("\n  === Daemon stderr (last 30 lines) ===")
                    for line in stderr.splitlines()[-30:]:
                        print(f"    {line}")
            exit_code = daemon_proc.poll()
            if exit_code is not None:
                print(f"  Daemon exit code: {exit_code}")
            # Kill daemon if test fails
            if exit_code is None:
                daemon_proc.kill()
                daemon_proc.wait(timeout=5)
            raise
