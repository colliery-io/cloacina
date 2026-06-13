"""
Server soak test — sustained load against the real demo stack.

Unified with the demo (CLOACI-T-0675): instead of host-built binaries +
hand-rolled packages + its own Postgres, the soak now drives
`docker/docker-compose.demo.yml` — the same stack, fixtures, compiler, and
cloaca-in-image the demo uses. Setup brings the stack up and waits for the
seeded `examples/fixtures/*` packages to build + load; Step 9 then drives
sustained concurrent load (workflow + Python + CG executions, WebSocket market
data into the CG accumulators, and API polling) for `--minutes` minutes, with
the CLOACI-T-0674 regression assertions checked once up front.

Kafka load is intentionally not run here — the demo compose has no broker;
adding a Kafka soak compose-profile is the tracked follow-up.
"""

import json
import subprocess
import time
import urllib.request
import urllib.error
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_section_header, print_final_success

test = angreal.command_group(name="test", about="Cloacina test suites (unit, integration, e2e, soak)")
soak = angreal.command_group(name="soak", about="sustained-load soak tests")

REPO_ROOT = Path(__file__).resolve().parents[3]
COMPOSE_FILE = REPO_ROOT / "docker" / "docker-compose.demo.yml"

# The demo stack's published surface (docker/docker-compose.demo.yml).
BASE_URL = "http://localhost:8080"
WS_HOST = "localhost"
WS_PORT = 8080
BOOTSTRAP_KEY = "clk_demo_bootstrap_key_0001"
TENANT = "public"

# Demo fixtures the load loop targets (examples/fixtures/*). Package name →
# executable workflow name (they differ by convention — CLOACI-T-0671).
RUST_WORKFLOW = "demo_slow_workflow"      # pkg demo-slow-rust
PYTHON_WORKFLOW = "demo_py_workflow"      # pkg demo-py-workflow
# Computation graphs + the accumulator each is fed via WebSocket.
RUST_CG = ("mixed_graph", "alpha")        # pkg mixed-rust
PYTHON_CG = ("demo_py_graph", "py_alpha")  # pkg demo-py-graph
# Packages we require to reach build_status=success before the soak starts.
REQUIRED_PACKAGES = ["demo-slow-rust", "demo-py-workflow", "mixed-rust"]


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


# ---------------------------------------------------------------------------
# Demo-stack lifecycle (shared infra — CLOACI-T-0675)
# ---------------------------------------------------------------------------

def _compose(*args, check=True, capture=False):
    cmd = ["docker", "compose", "-f", str(COMPOSE_FILE), *args]
    return subprocess.run(cmd, cwd=REPO_ROOT, check=check, capture_output=capture, text=True)


def _wait_health(timeout_s=180):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        try:
            s, _ = api_request("GET", f"{BASE_URL}/health")
            if s == 200:
                return True
        except Exception:
            pass
        time.sleep(2)
    raise RuntimeError(f"server /health not ready within {timeout_s}s")


def _server_alive():
    """True while the demo server container is up (soak crash-guard)."""
    try:
        r = _compose("ps", "--status", "running", "--format", "{{.Service}}",
                     check=False, capture=True)
        return "server" in (r.stdout or "")
    except Exception:
        return True  # don't false-positive a crash on a transient docker hiccup


def _wait_for_fixtures(timeout_s=600):
    """Poll until the required fixtures reach build_status=success (the harness
    seeds them; the compiler builds the Rust ones)."""
    deadline = time.time() + timeout_s
    last = {}
    while time.time() < deadline:
        s, body = api_request("GET", f"{BASE_URL}/v1/tenants/{TENANT}/workflows",
                              token=BOOTSTRAP_KEY)
        if s == 200:
            present = {w["package_name"] for w in body.get("items", [])}
            last = present
            if all(p in present for p in REQUIRED_PACKAGES):
                return sorted(present)
        time.sleep(3)
    raise RuntimeError(
        f"required fixtures {REQUIRED_PACKAGES} not all loaded within {timeout_s}s; "
        f"saw: {sorted(last)}"
    )


def _wait_for_graphs(expected_names, timeout_s=300):
    """Poll until the expected computation graphs are registered in the scheduler
    (CG registration into /health/graphs lags package build by a few reconcile
    ticks, especially right after a container recreate)."""
    deadline = time.time() + timeout_s
    last = []
    while time.time() < deadline:
        s, body = api_request("GET", f"{BASE_URL}/v1/health/graphs", token=BOOTSTRAP_KEY)
        if s == 200:
            present = [g["name"] for g in body.get("items", [])]
            last = present
            if all(n in present for n in expected_names):
                return present
        time.sleep(3)
    raise RuntimeError(
        f"computation graphs {expected_names} not all registered within {timeout_s}s; "
        f"saw: {last}"
    )


@test()
@soak()
@angreal.command(
    name="server",
    about="server soak — sustained load against the demo stack with Postgres",
    when_to_use=[
        "validating server stability + the workflow/CG/Python engine under load",
        "release soak (run with --minutes 30+)",
    ],
    when_not_to_use=[
        "unit testing",
        "daemon testing (use soak daemon instead)",
    ],
)
@angreal.argument(
    name="minutes",
    long="minutes",
    required=False,
    help="operational-soak (Step 4) duration in minutes; accepts fractions "
         "(e.g. 0.5). Default 1. Use 30+ for a real soak.",
)
def server(minutes=None):
    """Run the server soak against the shared demo stack (CLOACI-T-0675).

    Brings up `docker/docker-compose.demo.yml` (build + seed), waits for the
    seeded fixtures to load, runs the CLOACI-T-0674 regression assertions, then
    drives sustained concurrent load for `--minutes` minutes.
    """
    try:
        soak_duration = int(float(minutes) * 60) if minutes else 60
    except (TypeError, ValueError):
        raise SystemExit(f"--minutes must be a number, got {minutes!r}")
    if soak_duration < 1:
        raise SystemExit("--minutes must be > 0")

    print_section_header("Server Soak Test (demo stack)")
    print(f"  Compose: {COMPOSE_FILE}")
    print(f"  Operational-soak duration: {soak_duration}s ({soak_duration / 60:.1f} min)")

    token = BOOTSTRAP_KEY

    # Step 1: bring up the shared demo stack (server + compiler + ui + postgres
    # + fixtures + seed harness). cloaca + the real fixtures come baked in.
    print_section_header("Step 1: Bring up demo stack (build + seed)")
    _compose("up", "-d", "--build")
    _wait_health()
    print("  Server healthy ✓")

    # Step 2: wait for the seeded fixtures to build + load.
    print_section_header("Step 2: Wait for fixtures to load")
    loaded = _wait_for_fixtures()
    print(f"  Loaded packages ✓: {loaded}")
    graphs = _wait_for_graphs([RUST_CG[0], PYTHON_CG[0]])
    print(f"  Registered graphs ✓: {graphs}")

    # Step 3: regression assertions (CLOACI-T-0674) against the shared artifacts.
    print_section_header("Step 3: Regression assertions (T-0672/0673)")
    s, detail = api_request(
        "GET", f"{BASE_URL}/v1/tenants/{TENANT}/workflows/demo-py-workflow", token=token)
    assert s == 200, f"Python workflow detail GET failed: {s} {detail}"
    assert detail.get("workflow_name") == PYTHON_WORKFLOW, detail
    assert detail.get("tasks"), f"expected non-empty Python tasks: {detail}"
    tg = detail.get("task_graph") or []
    assert tg and {n["id"] for n in tg} == set(detail["tasks"]), detail
    print(f"  Python tasks/task_graph persisted ✓ ({len(detail['tasks'])} tasks, T-0672)")

    for graph_name, _acc in (RUST_CG, PYTHON_CG):
        s, gd = api_request("GET", f"{BASE_URL}/v1/health/graphs/{graph_name}", token=token)
        assert s == 200, f"CG {graph_name} GET failed: {s} {gd}"
        topo = gd.get("topology") or {}
        assert topo.get("nodes"), f"expected {graph_name} topology nodes: {gd}"
        assert topo.get("edges"), f"expected {graph_name} topology edges: {gd}"
        print(f"  CG {graph_name} topology surfaced ✓ "
              f"({len(topo['nodes'])} nodes, {len(topo['edges'])} edges, T-0673)")

    # Step 4: operational soak — sustained concurrent load.
    print_section_header(
        f"Step 4: Operational soak ({soak_duration}s / {soak_duration / 60:.1f} min)")
    print("  Executing workflows + WS market-data + API polling concurrently...")

    import threading
    import math

    stop_event = threading.Event()
    stats = {
        "health_ok": 0, "rust_triggered": 0, "rust_accepted": 0,
        "py_triggered": 0, "py_accepted": 0, "ws_alpha": 0, "ws_py_alpha": 0,
        "cg_health_ok": 0, "list_queries": 0, "api_errors": 0, "conn_errors": 0,
    }

    def ws_ticket():
        s, b = api_request("POST", f"{BASE_URL}/v1/auth/ws-ticket", token=token)
        return b.get("ticket", "") if s == 200 else ""

    def ws_worker(accumulator, counter_key, period):
        """Push market data to a CG accumulator over a persistent WS."""
        try:
            ticket = ws_ticket()
            if not ticket:
                print(f"  WS worker {accumulator}: no ticket")
                return
            ws = PersistentWebSocket(
                WS_HOST, WS_PORT, f"/v1/ws/accumulator/{accumulator}", ticket)
            seq = 0
            while not stop_event.is_set():
                mid = 100.0 + math.sin(seq * 0.1)
                spread = 0.5 if seq % 3 != 0 else 2.0  # exercise both routing branches
                msg = json.dumps({"bid": mid - spread / 2, "ask": mid + spread / 2})
                if ws.send(msg):
                    stats[counter_key] += 1
                seq += 1
                time.sleep(period)
            ws.close()
        except Exception as e:
            print(f"  WS worker {accumulator} error: {e}")

    workers = [
        threading.Thread(target=ws_worker, args=(RUST_CG[1], "ws_alpha", 0.005), daemon=True),
        threading.Thread(target=ws_worker, args=(PYTHON_CG[1], "ws_py_alpha", 0.01), daemon=True),
    ]
    for w in workers:
        w.start()

    soak_start = time.time()
    iteration = 0
    last_report = 0
    consecutive_health_fail = 0

    while time.time() - soak_start < soak_duration:
        iteration += 1
        try:
            s, _ = api_request("GET", f"{BASE_URL}/health")
            if s == 200:
                stats["health_ok"] += 1
                consecutive_health_fail = 0
            else:
                stats["api_errors"] += 1
                consecutive_health_fail += 1

            if iteration % 3 == 0:
                stats["rust_triggered"] += 1
                s, _ = api_request(
                    "POST", f"{BASE_URL}/v1/tenants/{TENANT}/workflows/{RUST_WORKFLOW}/execute",
                    token=token, data={"context": {"iteration": iteration}})
                if s in (200, 202):
                    stats["rust_accepted"] += 1
                else:
                    stats["api_errors"] += 1

            if iteration % 5 == 0:
                stats["py_triggered"] += 1
                s, _ = api_request(
                    "POST", f"{BASE_URL}/v1/tenants/{TENANT}/workflows/{PYTHON_WORKFLOW}/execute",
                    token=token, data={"context": {"iteration": iteration, "lang": "python"}})
                if s in (200, 202):
                    stats["py_accepted"] += 1
                else:
                    stats["api_errors"] += 1

            if iteration % 4 == 0:
                s, _ = api_request(
                    "GET", f"{BASE_URL}/v1/tenants/{TENANT}/workflows", token=token)
                if s == 200:
                    stats["list_queries"] += 1
                else:
                    stats["api_errors"] += 1

            if iteration % 7 == 0:
                s, _ = api_request("GET", f"{BASE_URL}/v1/health/graphs", token=token)
                if s == 200:
                    stats["cg_health_ok"] += 1
                else:
                    stats["api_errors"] += 1

        except Exception as e:
            if "Connection refused" in str(e) or "URLError" in type(e).__name__:
                stats["conn_errors"] += 1
            else:
                stats["api_errors"] += 1

        # Crash guard: if the server is unreachable for a sustained stretch and
        # the container is gone, fail loudly.
        if consecutive_health_fail >= 25 and not _server_alive():
            raise AssertionError(
                f"server container down at iteration {iteration} "
                f"({consecutive_health_fail} consecutive health failures)")

        elapsed = int(time.time() - soak_start)
        if elapsed >= last_report + 10:
            last_report = elapsed
            print(
                f"  [{elapsed}s] health={stats['health_ok']} "
                f"rust={stats['rust_accepted']}/{stats['rust_triggered']} "
                f"python={stats['py_accepted']}/{stats['py_triggered']} "
                f"ws(alpha={stats['ws_alpha']},py_alpha={stats['ws_py_alpha']}) "
                f"cg_health={stats['cg_health_ok']} queries={stats['list_queries']} "
                f"errors={stats['api_errors']} conn_err={stats['conn_errors']}")

        time.sleep(0.2)  # ~5 req bursts/sec

    stop_event.set()
    for w in workers:
        w.join(timeout=5)

    # Step 5: final health + summary.
    print_section_header("Step 5: Final health check")
    s, _ = api_request("GET", f"{BASE_URL}/health")
    assert s == 200, f"server unhealthy at end: {s}"
    print("  Server healthy ✓")
    print_section_header("Soak summary")
    for k, v in stats.items():
        print(f"  {k}: {v}")
    assert stats["health_ok"] > 0, "no successful health checks — soak did nothing"
    assert stats["api_errors"] == 0, f"API errors during soak: {stats['api_errors']}"
    assert stats["conn_errors"] == 0, f"connection errors during soak: {stats['conn_errors']}"

    print_section_header("Teardown")
    print("  Leaving the demo stack UP (shared with the demo). Tear down with:")
    print(f"    docker compose -f {COMPOSE_FILE} down")

    print_final_success(f"Server soak passed ({soak_duration / 60:.1f} min)")
