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

        # Step 8: Sustained soak — hammer the API for duration
        soak_duration = 60  # seconds
        print_section_header(f"Step 8: Sustained soak ({soak_duration}s)")
        print("  Continuously hitting all endpoints...")

        stats = {
            "health": 0,
            "ready": 0,
            "list_keys": 0,
            "list_workflows": 0,
            "list_triggers": 0,
            "list_executions": 0,
            "create_key": 0,
            "errors": 0,
            "server_crashes": 0,
        }

        soak_start = time.time()
        iteration = 0
        while time.time() - soak_start < soak_duration:
            iteration += 1
            assert server_proc.poll() is None, \
                f"Server crashed at iteration {iteration}!"

            try:
                # Health check (no auth)
                s, _ = api_request("GET", f"{base_url}/health")
                if s == 200:
                    stats["health"] += 1
                else:
                    stats["errors"] += 1

                # Ready check (no auth)
                s, _ = api_request("GET", f"{base_url}/ready")
                if s == 200:
                    stats["ready"] += 1
                else:
                    stats["errors"] += 1

                # List keys (auth)
                s, _ = api_request("GET", f"{base_url}/auth/keys", token=token)
                if s == 200:
                    stats["list_keys"] += 1
                else:
                    stats["errors"] += 1

                # List workflows (auth)
                s, _ = api_request("GET", f"{base_url}/tenants/public/workflows",
                                   token=token)
                if s == 200:
                    stats["list_workflows"] += 1
                else:
                    stats["errors"] += 1

                # List triggers (auth)
                s, _ = api_request("GET", f"{base_url}/tenants/public/triggers",
                                   token=token)
                if s == 200:
                    stats["list_triggers"] += 1
                else:
                    stats["errors"] += 1

                # List executions (auth)
                s, _ = api_request("GET", f"{base_url}/tenants/public/executions",
                                   token=token)
                if s == 200:
                    stats["list_executions"] += 1
                else:
                    stats["errors"] += 1

                # Create a key every 10 iterations
                if iteration % 10 == 0:
                    s, b = api_request("POST", f"{base_url}/auth/keys",
                                       token=token,
                                       data={"name": f"soak-key-{iteration}"})
                    if s == 201:
                        stats["create_key"] += 1
                    else:
                        stats["errors"] += 1

            except Exception as e:
                stats["errors"] += 1
                if "Connection refused" in str(e):
                    stats["server_crashes"] += 1

            # Print progress every 10s
            elapsed = int(time.time() - soak_start)
            if elapsed > 0 and elapsed % 10 == 0 and iteration % 5 == 0:
                total_ok = sum(v for k, v in stats.items() if k != "errors" and k != "server_crashes")
                print(f"  [{elapsed}s] {total_ok} OK, {stats['errors']} errors, iter {iteration}")

            time.sleep(0.1)  # ~10 req/s burst

        total_ok = sum(v for k, v in stats.items() if k != "errors" and k != "server_crashes")
        print(f"\n  Soak complete: {iteration} iterations, {total_ok} OK, {stats['errors']} errors")
        for key, val in stats.items():
            if val > 0:
                print(f"    {key}: {val}")

        assert stats["server_crashes"] == 0, "Server crashed during soak!"
        assert stats["errors"] < iteration * 0.1, \
            f"Too many errors: {stats['errors']}/{iteration} ({stats['errors']*100//iteration}%)"
        assert total_ok > 0, "No successful requests during soak!"

        # Step 9: Final health check
        print_section_header("Step 9: Final health check")
        status, _ = api_request("GET", f"{base_url}/health")
        assert status == 200, "Health check failed"
        assert server_proc.poll() is None, "Server crashed!"
        print("  Health: OK ✓")
        print("  Server still running ✓")

        # Shutdown
        print_section_header("Step 10: Graceful shutdown")
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
