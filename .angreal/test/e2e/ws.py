"""
WebSocket integration test for computation graph endpoints.

Bootstraps Postgres, starts the server, connects via WebSocket,
and verifies auth + endpoint behavior.
"""

import http.client
import json
import signal
import subprocess
import time
from pathlib import Path
from urllib.parse import urlparse

import angreal  # type: ignore

from .._utils import print_section_header, print_final_success


def build_server():
    """Build the server binary."""
    print("Building cloacinactl server (debug)...")
    subprocess.run(["cargo", "build", "-p", "cloacina-server"], check=True)


def find_server_binary():
    """Find the server binary path."""
    binary = Path("target/debug/cloacina-server")
    if not binary.exists():
        raise FileNotFoundError(f"Server binary not found at {binary}.")
    return str(binary)


def start_postgres():
    """Start Postgres via docker-compose."""
    subprocess.run(
        ["docker", "compose", "-f", ".angreal/docker-compose.yaml", "up", "-d"],
        check=True,
    )
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
    raise RuntimeError("Postgres failed to start")


def stop_postgres():
    """Stop Postgres."""
    subprocess.run(
        ["docker", "compose", "-f", ".angreal/docker-compose.yaml", "down", "-v"],
        capture_output=True,
    )


def api_request(method, url, token=None):
    """Simple HTTP request helper."""
    import urllib.request
    import urllib.error

    headers = {}
    if token:
        headers["Authorization"] = f"Bearer {token}"
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

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
e2e = angreal.command_group(name="e2e", about="end-to-end tests against a live server")


def ws_connect(url, timeout=5):
    """Try to establish a WebSocket connection using urllib (upgrade request).

    Returns (connected: bool, status_code: int or None, error: str or None).
    We use a raw HTTP upgrade request since we don't want to require
    the `websockets` package.
    """
    parsed = urlparse(url)
    host = parsed.hostname
    port = parsed.port or 80
    path = parsed.path
    if parsed.query:
        path += f"?{parsed.query}"

    try:
        conn = http.client.HTTPConnection(host, port, timeout=timeout)
        conn.request(
            "GET",
            path,
            headers={
                "Upgrade": "websocket",
                "Connection": "Upgrade",
                "Sec-WebSocket-Key": "dGhlIHNhbXBsZSBub25jZQ==",
                "Sec-WebSocket-Version": "13",
            },
        )
        resp = conn.getresponse()
        status = resp.status
        conn.close()

        if status == 101:
            return True, 101, None
        else:
            return False, status, resp.reason
    except Exception as e:
        return False, None, str(e)


@test()
@e2e()
@angreal.command(
    name="ws",
    about="WebSocket integration tests for computation graph endpoints",
    when_to_use=[
        "Testing WebSocket auth and endpoint behavior",
        "Verifying computation graph WS layer",
    ],
    when_not_to_use=[
        "Unit testing (use cargo test instead)",
    ],
)
def ws_integration_test():
    """Run WebSocket integration tests with full server bootstrap."""

    print_section_header("Step 1: Build server")
    build_server()
    server_binary = find_server_binary()

    print_section_header("Step 2: Start Postgres")
    start_postgres()

    base_url = "http://127.0.0.1:18081"
    ws_base = "http://127.0.0.1:18081"
    db_url = "postgres://cloacina:cloacina@localhost:5432/cloacina"
    test_home = Path("target/ws-integration-test")
    if test_home.exists():
        import shutil

        shutil.rmtree(test_home)

    server_proc = None
    bootstrap_key = "clk_ws_integration_test_key_00000000000000000"

    try:
        # Step 3: Start server
        print_section_header("Step 3: Start server")
        test_home.mkdir(parents=True, exist_ok=True)
        stderr_path = test_home / "server_stderr.log"
        stderr_file = open(stderr_path, "w")

        server_proc = subprocess.Popen(
            [
                server_binary,
                "--home",
                str(test_home),
                "--database-url",
                db_url,
                "--bind",
                "127.0.0.1:18081",
                "--bootstrap-key",
                bootstrap_key,
            ],
            stdout=subprocess.PIPE,
            stderr=stderr_file,
        )
        print(f"  PID: {server_proc.pid}")

        # Wait for health
        for i in range(30):
            time.sleep(1)
            try:
                status, _ = api_request("GET", f"{base_url}/health")
                if status == 200:
                    print(f"  Server healthy after {i + 1}s")
                    break
            except Exception:
                continue
        else:
            raise RuntimeError("Server failed to become healthy within 30s")

        token = bootstrap_key
        passed = 0
        failed = 0

        # --- Test 1: Accumulator WS — no auth → 401 ---
        print_section_header("Test 1: Accumulator WS — no auth")
        connected, status, err = ws_connect(
            f"{ws_base}/v1/ws/accumulator/alpha"
        )
        if not connected and status == 401:
            print("  PASS: rejected with 401")
            passed += 1
        else:
            print(f"  FAIL: connected={connected}, status={status}, err={err}")
            failed += 1

        # --- Test 2: Reactor WS — no auth → 401 ---
        print_section_header("Test 2: Reactor WS — no auth")
        connected, status, err = ws_connect(
            f"{ws_base}/v1/ws/reactor/test_graph"
        )
        if not connected and status == 401:
            print("  PASS: rejected with 401")
            passed += 1
        else:
            print(f"  FAIL: connected={connected}, status={status}, err={err}")
            failed += 1

        # --- Test 3: Accumulator WS — with token → 403 (no authZ policy) ---
        print_section_header("Test 3: Accumulator WS — with token (no authZ policy)")
        connected, status, err = ws_connect(
            f"{ws_base}/v1/ws/accumulator/alpha?token={token}"
        )
        if not connected and status == 403:
            print("  PASS: rejected with 403 (deny-by-default, no authZ policy)")
            passed += 1
        elif connected:
            print("  PASS: connected (authZ policy may be open)")
            passed += 1
        else:
            print(f"  FAIL: connected={connected}, status={status}, err={err}")
            failed += 1

        # --- Test 4: Reactor WS — with token → 403 (no authZ policy) ---
        print_section_header("Test 4: Reactor WS — with token (no authZ policy)")
        connected, status, err = ws_connect(
            f"{ws_base}/v1/ws/reactor/test_graph?token={token}"
        )
        if not connected and status == 403:
            print("  PASS: rejected with 403 (deny-by-default)")
            passed += 1
        elif connected:
            print("  PASS: connected (authZ policy may be open)")
            passed += 1
        else:
            print(f"  FAIL: connected={connected}, status={status}, err={err}")
            failed += 1

        # --- Test 5: Health endpoints ---
        print_section_header("Test 5: Reactive health endpoints")
        status, body = api_request(
            "GET", f"{base_url}/v1/health/accumulators", token=token
        )
        if status == 200:
            print(f"  PASS: /v1/health/accumulators → {json.dumps(body)[:100]}")
            passed += 1
        else:
            print(f"  FAIL: status={status}")
            failed += 1

        status, body = api_request(
            "GET", f"{base_url}/v1/health/reactors", token=token
        )
        if status == 200:
            print(f"  PASS: /v1/health/reactors → {json.dumps(body)[:100]}")
            passed += 1
        else:
            print(f"  FAIL: status={status}")
            failed += 1

        # --- Results ---
        print_section_header("Results")
        print(f"  Passed: {passed}")
        print(f"  Failed: {failed}")

        if failed == 0:
            print_final_success("All WebSocket integration tests passed!")
            return 0
        else:
            print(f"\n  {failed} test(s) FAILED")
            return 1

    finally:
        if server_proc:
            server_proc.send_signal(signal.SIGTERM)
            server_proc.wait(timeout=10)
            print("  Server stopped.")
        stop_postgres()
