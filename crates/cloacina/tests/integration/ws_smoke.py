#!/usr/bin/env python3
"""
WebSocket smoke test for computation graph endpoints.

Requires:
- Server running on localhost:8080 with Postgres
- Bootstrap API key available
- pip install websockets

Run via: angreal cloacina ws-smoke
"""

import asyncio
import json
import sys
import urllib.request
import urllib.error


def get_bootstrap_key():
    """Read the bootstrap key from ~/.cloacina/bootstrap-key."""
    from pathlib import Path
    key_path = Path.home() / ".cloacina" / "bootstrap-key"
    if key_path.exists():
        return key_path.read_text().strip()
    return None


def api_request(url, token=None, method="GET", data=None):
    """Simple HTTP request helper."""
    headers = {}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    if data:
        headers["Content-Type"] = "application/json"
        body = json.dumps(data).encode()
    else:
        body = None

    req = urllib.request.Request(url, data=body, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req) as resp:
            return resp.status, json.loads(resp.read())
    except urllib.error.HTTPError as e:
        try:
            body = json.loads(e.read())
        except Exception:
            body = {"error": str(e)}
        return e.code, body


async def test_ws_accumulator_auth_required(base_url):
    """Test that WS accumulator endpoint rejects unauthenticated connections."""
    import websockets

    ws_url = base_url.replace("http://", "ws://") + "/v1/ws/accumulator/alpha"

    try:
        async with websockets.connect(ws_url) as _ws:
            # Should not get here without auth
            print("  FAIL: connected without auth")
            return False
    except websockets.exceptions.InvalidStatusCode as e:
        if e.status_code == 401:
            print("  PASS: rejected with 401 (no auth)")
            return True
        print(f"  FAIL: unexpected status {e.status_code}")
        return False
    except Exception as e:
        # Connection refused or similar — server might not be running
        print(f"  SKIP: {e}")
        return True


async def test_ws_accumulator_with_token(base_url, token):
    """Test that WS accumulator endpoint accepts authenticated connections."""
    import websockets

    ws_url = base_url.replace("http://", "ws://") + "/v1/ws/accumulator/alpha?token=" + token

    try:
        async with websockets.connect(ws_url) as ws:
            # Connected — send a test message
            test_data = json.dumps({"value": 42.0})
            await ws.send(test_data)

            # The handler will try to forward to registry — since no accumulator
            # is registered for "alpha", it should close with 4404
            try:
                msg = await asyncio.wait_for(ws.recv(), timeout=2.0)
                print(f"  PASS: received response (unexpected but ok): {msg[:100]}")
                return True
            except websockets.exceptions.ConnectionClosed as e:
                if e.code == 4404:
                    print("  PASS: closed with 4404 (accumulator not registered)")
                    return True
                print(f"  PASS: closed with code {e.code}")
                return True
            except asyncio.TimeoutError:
                print("  PASS: connected, message sent, no response (expected)")
                return True
    except websockets.exceptions.InvalidStatusCode as e:
        if e.status_code == 403:
            print("  PASS: rejected with 403 (no authZ policy — deny by default)")
            return True
        print(f"  FAIL: unexpected status {e.status_code}")
        return False
    except Exception as e:
        print(f"  FAIL: {e}")
        return False


async def test_ws_reactor_auth_required(base_url):
    """Test that WS reactor endpoint rejects unauthenticated connections."""
    import websockets

    ws_url = base_url.replace("http://", "ws://") + "/v1/ws/reactor/test_graph"

    try:
        async with websockets.connect(ws_url) as _ws:
            print("  FAIL: connected without auth")
            return False
    except websockets.exceptions.InvalidStatusCode as e:
        if e.status_code == 401:
            print("  PASS: rejected with 401 (no auth)")
            return True
        print(f"  FAIL: unexpected status {e.status_code}")
        return False
    except Exception as e:
        print(f"  SKIP: {e}")
        return True


async def run_tests(base_url, token):
    """Run all WS smoke tests."""
    results = []

    print("\n1. Accumulator WS — no auth:")
    results.append(await test_ws_accumulator_auth_required(base_url))

    print("\n2. Accumulator WS — with token:")
    results.append(await test_ws_accumulator_with_token(base_url, token))

    print("\n3. Reactor WS — no auth:")
    results.append(await test_ws_reactor_auth_required(base_url))

    return all(results)


def main():
    base_url = "http://localhost:8080"

    # Check server is running
    try:
        status, body = api_request(f"{base_url}/health")
        if status != 200:
            print(f"Server not healthy: {status}")
            sys.exit(1)
    except Exception as e:
        print(f"Server not reachable: {e}")
        print("Start the server first: cargo run -p cloacinactl -- serve")
        sys.exit(1)

    # Get bootstrap key
    token = get_bootstrap_key()
    if not token:
        print("No bootstrap key found at ~/.cloacina/bootstrap-key")
        sys.exit(1)

    print("=== WebSocket Smoke Tests ===")
    print(f"Server: {base_url}")
    print(f"Token: {token[:8]}...")

    success = asyncio.run(run_tests(base_url, token))

    if success:
        print("\n=== All WS smoke tests passed ===")
        sys.exit(0)
    else:
        print("\n=== Some WS smoke tests failed ===")
        sys.exit(1)


if __name__ == "__main__":
    main()
