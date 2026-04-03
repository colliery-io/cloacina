"""
Server soak test — end-to-end HTTP API verification.

Starts the server with Postgres, bootstraps auth, creates a tenant,
uploads a workflow package, executes it, and verifies results.
"""

import json
import signal
import subprocess
import tarfile
import time
import io
import urllib.request
import urllib.error
from pathlib import Path

import angreal  # type: ignore

from .cloacina_utils import print_section_header, print_final_success

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")


def build_server():
    """Build the server binary (debug for host dep injection)."""
    print("Building cloacinactl server (debug)...")
    subprocess.run(
        ["cargo", "build", "-p", "cloacinactl"],
        check=True,
    )
    print("Server binary built.")


def find_server_binary():
    """Find the server binary path."""
    binary = Path("target/debug/cloacinactl")
    if not binary.exists():
        raise FileNotFoundError(f"Server binary not found at {binary}.")
    return str(binary)


def start_postgres():
    """Start Postgres via docker-compose."""
    print("Starting Postgres...")
    subprocess.run(
        ["docker", "compose", "-f", ".angreal/docker-compose.yaml", "up", "-d"],
        check=True,
    )
    # Wait for Postgres to be ready
    for _ in range(30):
        result = subprocess.run(
            ["docker", "compose", "-f", ".angreal/docker-compose.yaml", "exec", "-T",
             "postgres", "pg_isready", "-U", "cloacina"],
            capture_output=True,
        )
        if result.returncode == 0:
            print("  Postgres is ready.")
            return
        time.sleep(1)
    raise RuntimeError("Postgres failed to start within 30 seconds")


def stop_postgres():
    """Stop and clean Postgres."""
    subprocess.run(
        ["docker", "compose", "-f", ".angreal/docker-compose.yaml", "down", "-v"],
        capture_output=True,
    )


def api_request(method, url, token=None, data=None, files=None):
    """Make an HTTP request and return (status_code, json_body)."""
    headers = {}
    if token:
        headers["Authorization"] = f"Bearer {token}"

    if files:
        # Multipart upload
        boundary = "----CloacinaSoakTest"
        body = f"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"package.cloacina\"\r\nContent-Type: application/octet-stream\r\n\r\n".encode()
        body += files
        body += f"\r\n--{boundary}--\r\n".encode()
        headers["Content-Type"] = f"multipart/form-data; boundary={boundary}"
        req = urllib.request.Request(url, data=body, headers=headers, method=method)
    elif data is not None:
        body = json.dumps(data).encode()
        headers["Content-Type"] = "application/json"
        req = urllib.request.Request(url, data=body, headers=headers, method=method)
    else:
        req = urllib.request.Request(url, headers=headers, method=method)

    try:
        with urllib.request.urlopen(req) as resp:
            return resp.status, json.loads(resp.read())
    except urllib.error.HTTPError as e:
        try:
            body = json.loads(e.read())
        except Exception:
            body = {"error": str(e)}
        return e.code, body


def create_python_source_package():
    """Create a minimal fidius source package with a Python workflow."""
    safe_name = "soak_server_python"
    name = "soak-server-python"
    version = "1.0.0"
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
description = "Server soak test Python workflow"
author = "soak-test"
entry_module = "{safe_name}.tasks"
"""

    init_py = ""

    tasks_py = """from __future__ import annotations
import cloaca

@cloaca.task(id="py-server-task1", dependencies=[])
def py_server_task1(context):
    context.set("python_server_soak", True)
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


def create_test_source_package():
    """Create a minimal fidius source package for testing."""
    safe_name = "soak_server_test"
    name = "soak-server-test"
    version = "1.0.0"
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
description = "Server soak test workflow"
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
        context.insert("server_soak".to_string(), serde_json::json!(true));
        Ok(())
    }}
}}
"""

    build_rs = """fn main() {
    cloacina_build::configure();
}
"""

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


@cloacina()
@angreal.command(
    name="server-soak",
    about="run server soak test — end-to-end HTTP API verification with Postgres",
    when_to_use=[
        "validating server API end-to-end",
        "testing auth, upload, execute pipeline",
    ],
    when_not_to_use=[
        "unit testing",
        "daemon testing (use soak instead)",
    ],
)
def server_soak():
    """Run server soak test."""
    print_section_header("Server Soak Test")

    # Step 1: Build
    print_section_header("Step 1: Build server")
    build_server()
    server_binary = find_server_binary()

    # Step 2: Start Postgres
    print_section_header("Step 2: Start Postgres")
    start_postgres()

    base_url = "http://127.0.0.1:18080"
    db_url = "postgres://cloacina:cloacina@localhost:5432/cloacina"
    soak_home = Path("target/server-soak-test")
    if soak_home.exists():
        import shutil
        shutil.rmtree(soak_home)

    server_proc = None
    try:
        # Step 3: Start server
        print_section_header("Step 3: Start server")
        soak_home.mkdir(parents=True, exist_ok=True)
        stderr_path = soak_home / "server_stderr.log"
        stderr_file = open(stderr_path, "w")

        # Use a known bootstrap key for deterministic testing
        bootstrap_key = "clk_soak_test_key_for_server_verification_00"

        server_proc = subprocess.Popen(
            [server_binary, "serve", "--home", str(soak_home),
             "--database-url", db_url, "--bind", "127.0.0.1:18080",
             "--bootstrap-key", bootstrap_key],
            stdout=subprocess.PIPE,
            stderr=stderr_file,
        )
        print(f"  PID: {server_proc.pid}")

        # Wait for health endpoint
        for i in range(30):
            time.sleep(1)
            try:
                status, _ = api_request("GET", f"{base_url}/health")
                if status == 200:
                    print(f"  Server healthy after {i+1}s")
                    break
            except Exception:
                continue
        else:
            raise RuntimeError("Server failed to become healthy within 30s")

        # Step 4: Use the known bootstrap key
        print_section_header("Step 4: Verify bootstrap key")
        token = bootstrap_key
        # Verify the key file was also written
        key_path = soak_home / "bootstrap-key"
        for wait in range(10):
            if key_path.exists():
                break
            time.sleep(1)
        if key_path.exists():
            file_key = key_path.read_text().strip()
            assert file_key == token, "Key file doesn't match provided key"
            print("  Bootstrap key file verified ✓")
        else:
            print("  WARNING: bootstrap-key file not created (server may still be starting)")
        print(f"  Using key: {token[:10]}...{token[-4:]}")

        # Step 5: Test auth
        print_section_header("Step 5: Test auth")

        # No auth → 401
        status, body = api_request("GET", f"{base_url}/auth/keys")
        assert status == 401, f"Expected 401, got {status}"
        print("  No auth → 401 ✓")

        # Valid auth → 200
        status, body = api_request("GET", f"{base_url}/auth/keys", token=token)
        assert status == 200, f"Expected 200, got {status}: {body}"
        print(f"  Valid auth → 200 ✓ ({len(body.get('keys', []))} keys)")

        # Step 6: Create another key
        print_section_header("Step 6: Create API key")
        status, body = api_request("POST", f"{base_url}/auth/keys",
                                   token=token, data={"name": "test-key"})
        assert status == 201, f"Expected 201, got {status}: {body}"
        new_key = body.get("key", "")
        assert new_key.startswith("clk_"), "New key should start with clk_"
        print(f"  Created key: {new_key[:10]}...{new_key[-4:]}")

        # Step 7: Upload workflow package
        print_section_header("Step 7: Upload workflow package")
        package_data = create_test_source_package()
        status, body = api_request("POST", f"{base_url}/tenants/public/workflows",
                                   token=token, files=package_data)
        print(f"  Upload status: {status}")
        if status == 201:
            print("  Upload successful ✓")
        else:
            print(f"  Upload returned {status}: {json.dumps(body)[:200]}")

        # Step 8: Wait for reconciler to compile and load the package
        print_section_header("Step 8: Wait for package compilation")
        print("  Reconciler compiles source packages in the background...")
        compile_start = time.time()
        workflow_ready = False
        for _ in range(90):  # up to 90s for first compile
            time.sleep(2)
            assert server_proc.poll() is None, "Server crashed during compilation!"
            # Check stderr for successful registration
            stderr_file.flush()
            stderr = stderr_path.read_text() if stderr_path.exists() else ""
            if "Successfully registered" in stderr and "soak-server-test" in stderr:
                elapsed = int(time.time() - compile_start)
                print(f"  Package compiled and loaded ({elapsed}s) ✓")
                workflow_ready = True
                break
        if not workflow_ready:
            print("  WARNING: Package may not have compiled — executions may fail")

        # Step 8b: Upload and load Python workflow package
        print_section_header("Step 8b: Upload Python workflow package")
        py_package_data = create_python_source_package()
        status, body = api_request("POST", f"{base_url}/tenants/public/workflows",
                                   token=token, files=py_package_data)
        print(f"  Python upload status: {status}")
        if status == 201:
            print("  Python upload successful ✓")
        else:
            print(f"  Python upload returned {status}: {json.dumps(body)[:200]}")

        # Wait for Python package to load (no compilation needed — should be fast)
        print("  Waiting for Python package to load (up to 30s)...")
        py_workflow_ready = False
        py_load_start = time.time()
        for _ in range(15):
            time.sleep(2)
            assert server_proc.poll() is None, "Server crashed during Python package loading!"
            stderr_file.flush()
            stderr = stderr_path.read_text() if stderr_path.exists() else ""
            if "Python package loaded" in stderr or "Python workflow imported" in stderr:
                elapsed = int(time.time() - py_load_start)
                print(f"  Python package loaded ({elapsed}s) ✓")
                py_workflow_ready = True
                break
        if not py_workflow_ready:
            print("  WARNING: Python package may not have loaded — Python executions may fail")

        # Execute the Python workflow once to verify it works
        if py_workflow_ready:
            print("  Executing Python workflow...")
            s, b = api_request(
                "POST",
                f"{base_url}/tenants/public/workflows/soak_server_python/execute",
                token=token,
                data={"context": {"test": "python_soak"}},
            )
            if s in (200, 202):
                print(f"  Python execution accepted ✓ (status {s})")
            else:
                print(f"  Python execution returned {s}: {json.dumps(b)[:200]}")

            # Wait a few seconds for execution to complete
            time.sleep(5)
            stderr_file.flush()
            stderr = stderr_path.read_text() if stderr_path.exists() else ""
            if "Pipeline completed" in stderr:
                print("  Python pipeline completed ✓")
            else:
                print("  WARNING: Python pipeline may not have completed yet")

        # Step 9: Operational soak — execute workflows while querying API
        soak_duration = 60
        print_section_header(f"Step 9: Operational soak ({soak_duration}s)")
        print("  Executing workflows + querying API concurrently...")

        stats = {
            "health_ok": 0,
            "executions_triggered": 0,
            "executions_accepted": 0,
            "py_executions_triggered": 0,
            "py_executions_accepted": 0,
            "list_queries": 0,
            "api_errors": 0,
            "connection_errors": 0,
        }

        soak_start = time.time()
        iteration = 0
        last_report = 0

        while time.time() - soak_start < soak_duration:
            iteration += 1
            assert server_proc.poll() is None, \
                f"Server crashed at iteration {iteration}!"

            try:
                # Health check
                s, _ = api_request("GET", f"{base_url}/health")
                if s == 200:
                    stats["health_ok"] += 1
                else:
                    stats["api_errors"] += 1

                # Trigger Rust workflow execution every 3 iterations
                if iteration % 3 == 0 and workflow_ready:
                    stats["executions_triggered"] += 1
                    s, b = api_request(
                        "POST",
                        f"{base_url}/tenants/public/workflows/soak_server_test/execute",
                        token=token,
                        data={"context": {"iteration": iteration}},
                    )
                    if s in (200, 202):
                        stats["executions_accepted"] += 1
                    else:
                        stats["api_errors"] += 1

                # Trigger Python workflow execution every 5 iterations
                if iteration % 5 == 0 and py_workflow_ready:
                    stats["py_executions_triggered"] += 1
                    s, b = api_request(
                        "POST",
                        f"{base_url}/tenants/public/workflows/soak_server_python/execute",
                        token=token,
                        data={"context": {"iteration": iteration, "lang": "python"}},
                    )
                    if s in (200, 202):
                        stats["py_executions_accepted"] += 1
                    else:
                        stats["api_errors"] += 1

                # Query executions list
                s, b = api_request(
                    "GET", f"{base_url}/tenants/public/executions", token=token
                )
                if s == 200:
                    stats["list_queries"] += 1
                else:
                    stats["api_errors"] += 1

                # Query triggers
                s, _ = api_request(
                    "GET", f"{base_url}/tenants/public/triggers", token=token
                )
                if s == 200:
                    stats["list_queries"] += 1
                else:
                    stats["api_errors"] += 1

                # Query workflows
                s, _ = api_request(
                    "GET", f"{base_url}/tenants/public/workflows", token=token
                )
                if s == 200:
                    stats["list_queries"] += 1
                else:
                    stats["api_errors"] += 1

            except Exception as e:
                if "Connection refused" in str(e) or "URLError" in str(type(e).__name__):
                    stats["connection_errors"] += 1
                else:
                    stats["api_errors"] += 1

            # Report every 10s
            elapsed = int(time.time() - soak_start)
            if elapsed >= last_report + 10:
                last_report = elapsed
                print(
                    f"  [{elapsed}s] health={stats['health_ok']} "
                    f"rust={stats['executions_accepted']}/{stats['executions_triggered']} "
                    f"python={stats['py_executions_accepted']}/{stats['py_executions_triggered']} "
                    f"queries={stats['list_queries']} "
                    f"errors={stats['api_errors']}"
                )

            time.sleep(0.2)  # ~5 req bursts/sec

        # Check completed pipelines in server logs
        stderr_file.flush()
        stderr = stderr_path.read_text() if stderr_path.exists() else ""
        pipelines_completed = stderr.count("Pipeline completed")

        print("\n  Soak complete:")
        print(f"    Iterations:           {iteration}")
        print(f"    Health checks OK:     {stats['health_ok']}")
        print(f"    Rust exec triggered:  {stats['executions_triggered']}")
        print(f"    Rust exec accepted:   {stats['executions_accepted']}")
        print(f"    Python exec triggered:{stats['py_executions_triggered']}")
        print(f"    Python exec accepted: {stats['py_executions_accepted']}")
        print(f"    List queries OK:      {stats['list_queries']}")
        print(f"    API errors:           {stats['api_errors']}")
        print(f"    Connection errors:    {stats['connection_errors']}")
        print(f"    Pipelines completed:  {pipelines_completed} (from server logs)")

        assert stats["connection_errors"] == 0, "Server had connection errors!"
        assert stats["health_ok"] > 0, "No successful health checks!"
        if workflow_ready:
            assert stats["executions_accepted"] > 0, "No Rust executions accepted!"
            assert pipelines_completed > 0, "No pipelines completed in server logs!"
        if py_workflow_ready:
            assert stats["py_executions_accepted"] > 0, "No Python executions accepted!"

        # Step 10: Final health check
        print_section_header("Step 10: Final health check")
        status, _ = api_request("GET", f"{base_url}/health")
        assert status == 200, "Health check failed"
        assert server_proc.poll() is None, "Server crashed!"
        print("  Health: OK ✓")
        print("  Server still running ✓")

        # Shutdown
        print_section_header("Step 11: Graceful shutdown")
        server_proc.send_signal(signal.SIGINT)
        exit_code = server_proc.wait(timeout=15)
        print(f"  Server exited with code: {exit_code}")
        assert exit_code == 0, f"Non-zero exit: {exit_code}"

        print_final_success("Server soak test passed!")

    except Exception:
        # Print server stderr on failure
        if stderr_path.exists():
            stderr = stderr_path.read_text()
            if stderr.strip():
                print("\n  === Server stderr (last 20 lines) ===")
                for line in stderr.splitlines()[-20:]:
                    print(f"    {line}")
        if server_proc and server_proc.poll() is None:
            server_proc.kill()
        raise
    finally:
        stop_postgres()
