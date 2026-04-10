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


def create_cg_source_package():
    """Create a fidius source package with a computation graph (market maker)."""
    name = "soak-cg-package"
    version = "1.0.0"
    prefix = f"{name}-{version}"

    package_toml = f"""[package]
name = "{name}"
version = "{version}"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
package_type = ["computation_graph"]
graph_name = "soak_graph"
language = "rust"
description = "CG soak test — market maker"
reaction_mode = "when_any"
input_strategy = "latest"
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
cloacina-computation-graph = {{ path = "../../../crates/cloacina-computation-graph" }}
cloacina-macros = {{ path = "../../../crates/cloacina-macros" }}
cloacina-workflow-plugin = {{ path = "../../../crates/cloacina-workflow-plugin" }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
async-trait = "0.1"
tokio = {{ version = "1.0", features = ["full"] }}

[build-dependencies]
cloacina-build = {{ path = "../../../crates/cloacina-build" }}
"""

    lib_rs = """use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlphaData { pub value: f64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputData { pub result: f64 }

#[cloacina_macros::computation_graph(
    react = when_any(alpha),
    graph = {
        process(alpha) -> output,
    }
)]
pub mod soak_graph {
    use super::*;

    pub async fn process(alpha: Option<&AlphaData>) -> f64 {
        alpha.map(|a| a.value * 2.0).unwrap_or(0.0)
    }

    pub async fn output(value: &f64) -> OutputData {
        OutputData { result: *value }
    }
}
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


def create_python_cg_source_package():
    """Create a fidius source package with a Python computation graph."""
    name = "soak-python-cg"
    version = "1.0.0"
    prefix = f"{name}-{version}"
    graph_name = "py_soak_graph"

    package_toml = f"""[package]
name = "{name}"
version = "{version}"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
package_type = ["computation_graph"]
graph_name = "{graph_name}"
language = "python"
description = "Python CG soak test"
entry_module = "{graph_name}.graph"
reaction_mode = "when_any"
input_strategy = "latest"
"""

    init_py = ""

    graph_py = f"""import cloaca

@cloaca.passthrough_accumulator
def alpha(event):
    return event

with cloaca.ComputationGraphBuilder(
    "{graph_name}",
    react={{"mode": "when_any", "accumulators": ["alpha"]}},
    graph={{
        "process": {{"inputs": ["alpha"]}},
        "output": {{"inputs": ["process"]}},
    }},
) as builder:

    @cloaca.node
    def process(alpha):
        if alpha is None:
            return {{"result": 0.0}}
        return {{"result": alpha.get("value", 0.0) * 2.0}}

    @cloaca.node
    def output(process):
        return process
"""

    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:bz2") as tar:
        for rel_path, content in [
            ("package.toml", package_toml),
            (f"workflow/{graph_name}/__init__.py", init_py),
            (f"workflow/{graph_name}/graph.py", graph_py),
        ]:
            data = content.encode()
            archive_path = f"{prefix}/{rel_path}"
            entry = tarfile.TarInfo(name=archive_path)
            entry.size = len(data)
            entry.mode = 0o644
            tar.addfile(entry, io.BytesIO(data))

    return buf.getvalue()


def kafka_create_topic(topic_name):
    """Create a Kafka topic using the CLI tools inside the container."""
    try:
        subprocess.run(
            [
                "docker", "exec", "cloacina-kafka",
                "/opt/kafka/bin/kafka-topics.sh",
                "--bootstrap-server", "localhost:9092",
                "--create", "--topic", topic_name,
                "--partitions", "1",
                "--replication-factor", "1",
                "--if-not-exists",
            ],
            check=True,
            capture_output=True,
            timeout=15,
        )
        return True
    except Exception as e:
        print(f"  WARNING: Failed to create topic '{topic_name}': {e}")
        return False


class KafkaProducer:
    """Persistent Kafka producer using a long-running console-producer process."""

    def __init__(self, topic_name):
        self.topic = topic_name
        self.proc = subprocess.Popen(
            [
                "docker", "exec", "-i", "cloacina-kafka",
                "/opt/kafka/bin/kafka-console-producer.sh",
                "--bootstrap-server", "localhost:9092",
                "--topic", topic_name,
            ],
            stdin=subprocess.PIPE,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

    def send(self, messages):
        """Send messages (list of strings). Returns True on success."""
        try:
            data = "\n".join(messages) + "\n"
            self.proc.stdin.write(data.encode())
            self.proc.stdin.flush()
            return True
        except Exception:
            return False

    def close(self):
        try:
            self.proc.stdin.close()
            self.proc.wait(timeout=5)
        except Exception:
            self.proc.kill()


def kafka_produce(topic_name, messages):
    """One-shot produce (for backward compat). Use KafkaProducer for loops."""
    try:
        input_data = "\n".join(messages) + "\n"
        subprocess.run(
            [
                "docker", "exec", "-i", "cloacina-kafka",
                "/opt/kafka/bin/kafka-console-producer.sh",
                "--bootstrap-server", "localhost:9092",
                "--topic", topic_name,
            ],
            input=input_data,
            text=True,
            check=True,
            capture_output=True,
            timeout=10,
        )
        return True
    except Exception as e:
        print(f"  WARNING: Failed to produce to '{topic_name}': {e}")
        return False


def create_kafka_cg_source_package(pkg_name, graph_name, acc_name, topic, acc_type="stream"):
    """Create a CG source package with a Kafka-sourced accumulator.

    The accumulator type and topic are declared in package.toml metadata,
    which the reconciler merges into the FFI accumulator declarations.
    """
    version = "1.0.0"
    prefix = f"{pkg_name}-{version}"

    acc_config = f"""
[[metadata.accumulators]]
name = "{acc_name}"
accumulator_type = "{acc_type}"

[metadata.accumulators.config]
broker = "KAFKA_BROKER"
topic = "{topic}"
group = "{pkg_name}-group"
"""

    package_toml = f"""[package]
name = "{pkg_name}"
version = "{version}"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
package_type = ["computation_graph"]
graph_name = "{graph_name}"
language = "rust"
description = "Kafka soak test — {acc_type}"
reaction_mode = "when_any"
input_strategy = "latest"
{acc_config}
"""

    cargo_toml = f"""[package]
name = "{pkg_name}"
version = "{version}"
edition = "2021"

[workspace]

[features]
default = ["packaged"]
packaged = []

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
cloacina-computation-graph = {{ path = "../../../crates/cloacina-computation-graph" }}
cloacina-macros = {{ path = "../../../crates/cloacina-macros" }}
cloacina-workflow-plugin = {{ path = "../../../crates/cloacina-workflow-plugin" }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
async-trait = "0.1"
tokio = {{ version = "1.0", features = ["full"] }}

[build-dependencies]
cloacina-build = {{ path = "../../../crates/cloacina-build" }}
"""

    lib_rs = f"""use serde::{{Deserialize, Serialize}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {{ pub value: f64 }}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputData {{ pub result: f64 }}

#[cloacina_macros::computation_graph(
    react = when_any({acc_name}),
    graph = {{
        process({acc_name}) -> output,
    }}
)]
pub mod {graph_name} {{
    use super::*;

    pub async fn process({acc_name}: Option<&EventData>) -> f64 {{
        {acc_name}.map(|e| e.value * 2.0).unwrap_or(0.0)
    }}

    pub async fn output(value: &f64) -> OutputData {{
        OutputData {{ result: *value }}
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


def ws_send_event(host, port, path, token, event_json, timeout=5):
    """Send a single WebSocket binary frame with event data.

    Implements a minimal WebSocket client using stdlib:
    1. HTTP upgrade handshake
    2. Send one masked binary frame
    3. Close connection

    Returns True on success, False on failure.
    """
    import socket
    import struct
    import os
    import base64

    ws_key = base64.b64encode(os.urandom(16)).decode()

    try:
        sock = socket.create_connection((host, port), timeout=timeout)

        # HTTP upgrade request
        request = (
            f"GET {path}?token={token} HTTP/1.1\r\n"
            f"Host: {host}:{port}\r\n"
            f"Upgrade: websocket\r\n"
            f"Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {ws_key}\r\n"
            f"Sec-WebSocket-Version: 13\r\n"
            f"\r\n"
        )
        sock.sendall(request.encode())

        # Read response
        response = b""
        while b"\r\n\r\n" not in response:
            chunk = sock.recv(4096)
            if not chunk:
                sock.close()
                return False
            response += chunk

        status_line = response.split(b"\r\n")[0].decode()
        if "101" not in status_line:
            sock.close()
            return False

        # Send binary frame with event data
        payload = event_json.encode() if isinstance(event_json, str) else event_json
        mask_key = os.urandom(4)

        # Frame: FIN + binary opcode (0x82), masked, length
        frame = bytearray()
        frame.append(0x82)  # FIN + binary

        length = len(payload)
        if length < 126:
            frame.append(0x80 | length)  # mask bit set
        elif length < 65536:
            frame.append(0x80 | 126)
            frame.extend(struct.pack(">H", length))
        else:
            frame.append(0x80 | 127)
            frame.extend(struct.pack(">Q", length))

        frame.extend(mask_key)

        # Mask payload
        masked = bytearray(len(payload))
        for i in range(len(payload)):
            masked[i] = payload[i] ^ mask_key[i % 4]
        frame.extend(masked)

        sock.sendall(frame)

        # Send close frame
        close_frame = bytearray([0x88, 0x80]) + os.urandom(4)
        sock.sendall(close_frame)

        sock.close()
        return True
    except Exception:
        return False


class PersistentWebSocket:
    """Persistent WebSocket connection for high-throughput event injection."""

    def __init__(self, host, port, path, token, timeout=5):
        import socket as _socket
        import base64 as _base64
        import os as _os

        self.sock = _socket.create_connection((host, port), timeout=timeout)
        ws_key = _base64.b64encode(_os.urandom(16)).decode()

        request = (
            f"GET {path}?token={token} HTTP/1.1\r\n"
            f"Host: {host}:{port}\r\n"
            f"Upgrade: websocket\r\n"
            f"Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {ws_key}\r\n"
            f"Sec-WebSocket-Version: 13\r\n"
            f"\r\n"
        )
        self.sock.sendall(request.encode())

        response = b""
        while b"\r\n\r\n" not in response:
            chunk = self.sock.recv(4096)
            if not chunk:
                raise ConnectionError("WebSocket upgrade failed")
            response += chunk

        if b"101" not in response.split(b"\r\n")[0]:
            self.sock.close()
            raise ConnectionError(f"WebSocket upgrade rejected: {response[:100]}")

    def send(self, payload_str):
        """Send a masked binary frame. Returns True on success."""
        import struct
        import os

        try:
            payload = payload_str.encode() if isinstance(payload_str, str) else payload_str
            mask_key = os.urandom(4)

            frame = bytearray()
            frame.append(0x82)  # FIN + binary
            length = len(payload)
            if length < 126:
                frame.append(0x80 | length)
            elif length < 65536:
                frame.append(0x80 | 126)
                frame.extend(struct.pack(">H", length))
            else:
                frame.append(0x80 | 127)
                frame.extend(struct.pack(">Q", length))
            frame.extend(mask_key)

            masked = bytearray(len(payload))
            for i in range(len(payload)):
                masked[i] = payload[i] ^ mask_key[i % 4]
            frame.extend(masked)

            self.sock.sendall(frame)
            return True
        except Exception:
            return False

    def close(self):
        import os
        try:
            close_frame = bytearray([0x88, 0x80]) + os.urandom(4)
            self.sock.sendall(close_frame)
            self.sock.close()
        except Exception:
            pass


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

        import os
        server_env = os.environ.copy()
        server_env["CLOACINA_VAR_KAFKA_BROKER"] = "localhost:9092"

        server_proc = subprocess.Popen(
            [server_binary, "serve", "--home", str(soak_home),
             "--database-url", db_url, "--bind", "127.0.0.1:18080",
             "--bootstrap-key", bootstrap_key],
            stdout=subprocess.PIPE,
            stderr=stderr_file,
            env=server_env,
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
        status, body = api_request("GET", f"{base_url}/v1/auth/keys")
        assert status == 401, f"Expected 401, got {status}"
        print("  No auth → 401 ✓")

        # Valid auth → 200
        status, body = api_request("GET", f"{base_url}/v1/auth/keys", token=token)
        assert status == 200, f"Expected 200, got {status}: {body}"
        print(f"  Valid auth → 200 ✓ ({len(body.get('keys', []))} keys)")

        # Step 6: Create another key
        print_section_header("Step 6: Create API key")
        status, body = api_request("POST", f"{base_url}/v1/auth/keys",
                                   token=token, data={"name": "test-key"})
        assert status == 201, f"Expected 201, got {status}: {body}"
        new_key = body.get("key", "")
        assert new_key.startswith("clk_"), "New key should start with clk_"
        print(f"  Created key: {new_key[:10]}...{new_key[-4:]}")

        # Step 7: Upload workflow package
        print_section_header("Step 7: Upload workflow package")
        package_data = create_test_source_package()
        status, body = api_request("POST", f"{base_url}/v1/tenants/public/workflows",
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
        status, body = api_request("POST", f"{base_url}/v1/tenants/public/workflows",
                                   token=token, files=py_package_data)
        print(f"  Python upload status: {status}")
        if status == 201:
            print("  Python upload successful ✓")
        else:
            print(f"  Python upload returned {status}: {json.dumps(body)[:200]}")

        # Wait for Python package to load — reconciler may be busy with Rust compilation
        print("  Waiting for Python package to load (up to 120s)...")
        py_workflow_ready = False
        py_load_start = time.time()
        for _ in range(60):
            time.sleep(2)
            assert server_proc.poll() is None, "Server crashed during Python package loading!"
            stderr_file.flush()
            stderr = stderr_path.read_text() if stderr_path.exists() else ""
            if "soak-server-python" in stderr and ("Python package loaded" in stderr or "Python workflow imported" in stderr):
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
                f"{base_url}/v1/tenants/public/workflows/soak_server_python/execute",
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

        # Step 8d: Upload and load computation graph package
        print_section_header("Step 8d: Upload computation graph package")
        cg_package_data = create_cg_source_package()
        status, body = api_request("POST", f"{base_url}/v1/tenants/public/workflows",
                                   token=token, files=cg_package_data)
        print(f"  CG upload status: {status}")
        cg_loaded = False
        if status == 201:
            print("  CG upload successful ✓")

            # Wait for CG package compilation (Rust — may take 60-90s)
            print("  Waiting for CG package compilation (up to 120s)...")
            cg_compile_start = time.time()
            for _ in range(60):
                time.sleep(2)
                assert server_proc.poll() is None, "Server crashed during CG compilation!"
                stderr_file.flush()
                stderr = stderr_path.read_text() if stderr_path.exists() else ""
                if "computation graph loaded" in stderr and "soak_graph" in stderr:
                    elapsed = int(time.time() - cg_compile_start)
                    print(f"  CG package compiled and loaded ({elapsed}s) ✓")
                    cg_loaded = True
                    break
                if "Successfully registered" in stderr and "soak-cg-package" in stderr:
                    elapsed = int(time.time() - cg_compile_start)
                    print(f"  CG package registered ({elapsed}s) ✓")
                    cg_loaded = True
                    break
            if not cg_loaded:
                print("  WARNING: CG package may not have compiled — CG soak will be skipped")
        else:
            print(f"  CG upload returned {status}: {json.dumps(body)[:200]}")

        # Verify CG health after loading
        if cg_loaded:
            s, b = api_request("GET", f"{base_url}/v1/health/accumulators", token=token)
            if s == 200:
                print(f"  CG accumulators health: {json.dumps(b)[:150]} ✓")
            s, b = api_request("GET", f"{base_url}/v1/health/reactors", token=token)
            if s == 200:
                print(f"  CG reactors health: {json.dumps(b)[:150]} ✓")

        # Step 8d2: Upload and load Python computation graph package
        print_section_header("Step 8d2: Upload Python computation graph package")
        py_cg_package_data = create_python_cg_source_package()
        status, body = api_request("POST", f"{base_url}/v1/tenants/public/workflows",
                                   token=token, files=py_cg_package_data)
        print(f"  Python CG upload status: {status}")
        py_cg_loaded = False
        if status == 201:
            print("  Python CG upload successful ✓")

            # Wait for Python CG package to load — reconciler may be busy
            print("  Waiting for Python CG package to load (up to 120s)...")
            py_cg_load_start = time.time()
            for _ in range(60):
                time.sleep(2)
                assert server_proc.poll() is None, "Server crashed during Python CG loading!"
                stderr_file.flush()
                stderr = stderr_path.read_text() if stderr_path.exists() else ""
                if "py_soak_graph" in stderr and "loaded" in stderr.lower():
                    elapsed = int(time.time() - py_cg_load_start)
                    print(f"  Python CG package loaded ({elapsed}s) ✓")
                    py_cg_loaded = True
                    break
            if not py_cg_loaded:
                print("  WARNING: Python CG package may not have loaded — Python CG soak will be skipped")
        else:
            print(f"  Python CG upload returned {status}: {json.dumps(body)[:200]}")

        # Step 8e: Kafka-sourced computation graph packages
        print_section_header("Step 8e: Kafka stream accumulator packages")
        kafka_ready = False
        kafka_topics = []

        # Check if Kafka is available
        try:
            subprocess.run(
                ["docker", "exec", "cloacina-kafka",
                 "/opt/kafka/bin/kafka-broker-api-versions.sh",
                 "--bootstrap-server", "localhost:9092"],
                capture_output=True, timeout=10, check=True,
            )
            kafka_ready = True
            print("  Kafka broker ready ✓")
        except Exception:
            print("  WARNING: Kafka not available — skipping Kafka soak steps")

        kafka_stream_loaded = False
        kafka_batch_loaded = False

        if kafka_ready:
            # Create topics
            for topic in ["soak.stream", "soak.batch"]:
                if kafka_create_topic(topic):
                    kafka_topics.append(topic)
                    print(f"  Topic '{topic}' created ✓")

            # Upload stream CG package
            stream_pkg = create_kafka_cg_source_package(
                "soak-kafka-stream", "kafka_stream_graph", "stream_source", "soak.stream", "stream"
            )
            s, b = api_request("POST", f"{base_url}/v1/tenants/public/workflows",
                               token=token, files=stream_pkg)
            if s == 201:
                print("  Kafka stream CG package uploaded ✓")
            else:
                print(f"  Kafka stream upload: {s}")

            # Upload batch CG package
            batch_pkg = create_kafka_cg_source_package(
                "soak-kafka-batch", "kafka_batch_graph", "batch_source", "soak.batch", "stream"
            )
            s, b = api_request("POST", f"{base_url}/v1/tenants/public/workflows",
                               token=token, files=batch_pkg)
            if s == 201:
                print("  Kafka batch CG package uploaded ✓")
            else:
                print(f"  Kafka batch upload: {s}")

            # Wait for Kafka CG packages to compile and load
            print("  Waiting for Kafka CG packages (up to 120s)...")
            kafka_compile_start = time.time()
            for _ in range(60):
                time.sleep(2)
                assert server_proc.poll() is None, "Server crashed during Kafka CG compilation!"
                stderr_file.flush()
                stderr = stderr_path.read_text() if stderr_path.exists() else ""
                stream_ok = "kafka_stream_graph" in stderr and "loaded into ReactiveScheduler" in stderr
                batch_ok = "kafka_batch_graph" in stderr and "loaded into ReactiveScheduler" in stderr
                if not kafka_stream_loaded and stream_ok:
                    print(f"  Kafka stream graph loaded ({int(time.time() - kafka_compile_start)}s) ✓")
                    kafka_stream_loaded = True
                if not kafka_batch_loaded and batch_ok:
                    print(f"  Kafka batch graph loaded ({int(time.time() - kafka_compile_start)}s) ✓")
                    kafka_batch_loaded = True
                if kafka_stream_loaded and kafka_batch_loaded:
                    break
            if not kafka_stream_loaded:
                print("  WARNING: Kafka stream graph not loaded")
            if not kafka_batch_loaded:
                print("  WARNING: Kafka batch graph not loaded")

        # Step 9: Operational soak — execute workflows while querying API
        soak_duration = 60
        print_section_header(f"Step 9: Operational soak ({soak_duration}s)")
        print("  Executing workflows + querying API concurrently...")

        # Step 8c: Verify computation graph health endpoints exist
        print_section_header("Step 8c: Verify computation graph health endpoints")
        s, b = api_request("GET", f"{base_url}/v1/health/accumulators", token=token)
        if s == 200:
            print(f"  /v1/health/accumulators → {json.dumps(b)[:100]} ✓")
        else:
            print(f"  /v1/health/accumulators → {s} (expected 200)")

        s, b = api_request("GET", f"{base_url}/v1/health/reactors", token=token)
        if s == 200:
            print(f"  /v1/health/reactors → {json.dumps(b)[:100]} ✓")
        else:
            print(f"  /v1/health/reactors → {s} (expected 200)")

        stats = {
            "health_ok": 0,
            "executions_triggered": 0,
            "executions_accepted": 0,
            "py_executions_triggered": 0,
            "py_executions_accepted": 0,
            "cg_events_sent": 0,
            "cg_events_failed": 0,
            "kafka_stream_produced": 0,
            "kafka_batch_produced": 0,
            "cg_health_ok": 0,
            "list_queries": 0,
            "api_errors": 0,
            "connection_errors": 0,
        }

        # Start independent Kafka producer threads at high rate
        import threading

        kafka_stop_event = threading.Event()
        kafka_stream_count = {"sent": 0}
        kafka_batch_count = {"sent": 0}
        ws_event_count = {"sent": 0, "failed": 0}
        py_cg_event_count = {"sent": 0, "failed": 0}

        def ws_event_worker():
            """Push events via persistent WebSocket at ~200 msg/sec."""
            try:
                ws = PersistentWebSocket(
                    "127.0.0.1", 18080,
                    "/v1/ws/accumulator/alpha", token
                )
                import math
                seq = 0
                while not kafka_stop_event.is_set():
                    msg = json.dumps({"value": 42.0 + math.sin(seq * 0.1)})
                    if ws.send(msg):
                        ws_event_count["sent"] += 1
                    else:
                        ws_event_count["failed"] += 1
                    seq += 1
                    time.sleep(0.005)  # 200 msg/sec
                ws.close()
            except Exception as e:
                print(f"  WS worker error: {e}")

        def kafka_stream_worker():
            """Produce to stream topic at ~100 msg/sec independently."""
            producer = KafkaProducer("soak.stream")
            seq = 0
            while not kafka_stop_event.is_set():
                msg = json.dumps({"value": float(seq)})
                if producer.send([msg]):
                    kafka_stream_count["sent"] += 1
                seq += 1
                time.sleep(0.01)  # 100 msg/sec
            producer.close()

        def kafka_batch_worker():
            """Produce to batch topic at ~50 msg/sec independently."""
            producer = KafkaProducer("soak.batch")
            seq = 0
            while not kafka_stop_event.is_set():
                msgs = [json.dumps({"value": float(seq + i)}) for i in range(5)]
                if producer.send(msgs):
                    kafka_batch_count["sent"] += 5
                seq += 5
                time.sleep(0.1)  # 50 msg/sec (5 per 100ms)
            producer.close()

        def py_cg_event_worker():
            """Push events to Python CG accumulator via WebSocket at ~100 msg/sec."""
            try:
                ws = PersistentWebSocket(
                    "127.0.0.1", 18080,
                    "/v1/ws/accumulator/alpha", token
                )
                import math
                seq = 0
                while not kafka_stop_event.is_set():
                    msg = json.dumps({"value": 10.0 + math.cos(seq * 0.1)})
                    if ws.send(msg):
                        py_cg_event_count["sent"] += 1
                    else:
                        py_cg_event_count["failed"] += 1
                    seq += 1
                    time.sleep(0.01)  # 100 msg/sec
                ws.close()
            except Exception as e:
                print(f"  Python CG WS worker error: {e}")

        producer_threads = []
        if cg_loaded:
            t = threading.Thread(target=ws_event_worker, daemon=True)
            t.start()
            producer_threads.append(t)
        if py_cg_loaded:
            t = threading.Thread(target=py_cg_event_worker, daemon=True)
            t.start()
            producer_threads.append(t)
        if kafka_stream_loaded:
            t = threading.Thread(target=kafka_stream_worker, daemon=True)
            t.start()
            producer_threads.append(t)
        if kafka_batch_loaded:
            t = threading.Thread(target=kafka_batch_worker, daemon=True)
            t.start()
            producer_threads.append(t)

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
                        f"{base_url}/v1/tenants/public/workflows/soak_server_test/execute",
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
                        f"{base_url}/v1/tenants/public/workflows/soak_server_python/execute",
                        token=token,
                        data={"context": {"iteration": iteration, "lang": "python"}},
                    )
                    if s in (200, 202):
                        stats["py_executions_accepted"] += 1
                    else:
                        stats["api_errors"] += 1

                # Query executions list
                s, b = api_request(
                    "GET", f"{base_url}/v1/tenants/public/executions", token=token
                )
                if s == 200:
                    stats["list_queries"] += 1
                else:
                    stats["api_errors"] += 1

                # Query triggers
                s, _ = api_request(
                    "GET", f"{base_url}/v1/tenants/public/triggers", token=token
                )
                if s == 200:
                    stats["list_queries"] += 1
                else:
                    stats["api_errors"] += 1

                # Query workflows
                s, _ = api_request(
                    "GET", f"{base_url}/v1/tenants/public/workflows", token=token
                )
                if s == 200:
                    stats["list_queries"] += 1
                else:
                    stats["api_errors"] += 1

                # Query computation graph health endpoints every 4 iterations
                if iteration % 4 == 0:
                    s, _ = api_request(
                        "GET", f"{base_url}/v1/health/reactors", token=token
                    )
                    if s == 200:
                        stats["cg_health_ok"] += 1
                    else:
                        stats["api_errors"] += 1

                    s, _ = api_request(
                        "GET", f"{base_url}/v1/health/accumulators", token=token
                    )
                    if s == 200:
                        stats["cg_health_ok"] += 1
                    else:
                        stats["api_errors"] += 1

                # All event production handled by independent threads
                # (WS, Kafka stream, Kafka batch)

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
                    f"cg_events={stats['cg_events_sent']}/{stats['cg_events_sent']+stats['cg_events_failed']} "
                    f"cg_health={stats['cg_health_ok']} "
                    f"queries={stats['list_queries']} "
                    f"errors={stats['api_errors']}"
                )

            time.sleep(0.2)  # ~5 req bursts/sec

        # Stop all producer threads
        kafka_stop_event.set()
        for t in producer_threads:
            t.join(timeout=5)
        stats["cg_events_sent"] = ws_event_count["sent"]
        stats["cg_events_failed"] = ws_event_count["failed"]
        stats["py_cg_events_sent"] = py_cg_event_count["sent"]
        stats["py_cg_events_failed"] = py_cg_event_count["failed"]
        stats["kafka_stream_produced"] = kafka_stream_count["sent"]
        stats["kafka_batch_produced"] = kafka_batch_count["sent"]

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
        print(f"    CG events sent:       {stats['cg_events_sent']}")
        print(f"    CG events failed:     {stats['cg_events_failed']}")
        print(f"    Py CG events sent:    {stats['py_cg_events_sent']}")
        print(f"    Py CG events failed:  {stats['py_cg_events_failed']}")
        print(f"    Kafka stream produced:{stats['kafka_stream_produced']}")
        print(f"    Kafka batch produced: {stats['kafka_batch_produced']}")

        # Count graph fires from server logs (INFO level: "graph execution completed")
        # Strip ANSI escape codes for reliable matching
        import re
        clean_stderr = re.sub(r'\x1b\[[0-9;]*m', '', stderr)

        def count_graph_fires(log_text, graph_name):
            return log_text.count(f"graph execution completed graph={graph_name}")

        ws_graph_fires = count_graph_fires(clean_stderr, "soak_graph")
        py_cg_fires = count_graph_fires(clean_stderr, "py_soak_graph")
        kafka_stream_fires = count_graph_fires(clean_stderr, "kafka_stream_graph")
        kafka_batch_fires = count_graph_fires(clean_stderr, "kafka_batch_graph")
        print(f"    WS graph fires:       {ws_graph_fires}")
        print(f"    Py CG graph fires:    {py_cg_fires}")
        print(f"    Kafka stream fires:   {kafka_stream_fires}")
        print(f"    Kafka batch fires:    {kafka_batch_fires}")
        print(f"    CG health checks OK:  {stats['cg_health_ok']}")
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
        if cg_loaded:
            assert stats["cg_events_sent"] > 0, "No CG events sent via WebSocket!"
            assert ws_graph_fires > 0, "WS graph (soak_graph) never fired!"
        if py_cg_loaded:
            assert stats["py_cg_events_sent"] > 0, "No Python CG events sent via WebSocket!"
            assert py_cg_fires > 0, "Python CG graph (py_soak_graph) never fired!"
        if kafka_stream_loaded:
            assert kafka_stream_fires > 0, f"Kafka stream graph never fired! ({stats['kafka_stream_produced']} messages produced)"
        if kafka_batch_loaded:
            assert kafka_batch_fires > 0, f"Kafka batch graph never fired! ({stats['kafka_batch_produced']} messages produced)"

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
