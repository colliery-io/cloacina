---
title: "08 - WebSocket Event Injection"
description: "Push events into a running computation graph accumulator over a WebSocket connection"
weight: 20
---

In this tutorial you'll push events into the `orderbook` accumulator of the `price_signal` graph deployed in [Tutorial 07]({{< ref "/tutorials/computation-graphs/service/07-packaging/" >}}). Events travel over a WebSocket connection to the accumulator endpoint at `/v1/ws/accumulator/{name}`. When enough events arrive to satisfy the reactor's firing condition, the computation graph executes automatically.

## What you'll learn

- The accumulator WebSocket endpoint URL and query-parameter auth
- The wire format: text or binary frames containing JSON (debug mode) or raw bytes (release mode)
- How to open a connection, send events, and close cleanly using Python's standard library
- How to verify the graph fired by polling `/v1/health/reactors`
- The reactor WebSocket endpoint for manual commands (`ForceFire`, `GetState`, `Pause`, `Resume`)

## Prerequisites

- Tutorial 07 complete — the `price_signal` graph must be loaded and accumulators healthy
- Your PAK token exported as `TOKEN`
- Python 3.9+ available (no third-party packages needed)
- `curl` available for quick health checks

## Time estimate

10–15 minutes

---

## Background: the accumulator WebSocket endpoint

Each accumulator registered with the `ReactiveScheduler` is reachable at:

```
GET /v1/ws/accumulator/{name}
```

The server upgrades the connection to WebSocket after validating the auth token. Once upgraded, the server forwards every incoming frame to all accumulators registered under that name (there is one per graph that declared an accumulator with that name).

**Authentication** is checked on the HTTP upgrade request — before the WebSocket handshake completes. You can supply the PAK token in two ways:

| Method | Example |
|---|---|
| Query parameter | `?token=clk_...` |
| Authorization header | `Authorization: Bearer clk_...` |

Browsers must use the query parameter because they cannot set custom headers on WebSocket upgrade requests. Server-to-server clients should prefer the header.

**Frame format**: the server accepts both binary (`0x82`) and text (`0x81`) frames. The accumulator deserializes the payload as `serde_json::Value` — in debug builds this is plain JSON; in release builds the payload is expected to be bincode. For this tutorial the server is assumed to be running in debug mode, so all payloads are JSON.

---

## Step 1: Confirm the graph is ready

```bash
BASE_URL="http://localhost:8080"
TOKEN="clk_your_token_here"

curl -s "${BASE_URL}/v1/health/accumulators" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -m json.tool
```

You should see:

```json
{
  "accumulators": [
    {
      "name": "orderbook",
      "status": "healthy"
    }
  ]
}
```

If the accumulator is not listed or shows `"unhealthy"`, revisit Tutorial 07 before continuing.

---

## Step 2: Send a single event with curl's `--unix-socket` workaround

`curl` does not support WebSocket upgrades in versions before 7.86. A reliable, dependency-free approach is a small Python script. We'll build up to a full client; start by understanding the wire protocol.

A WebSocket client frame for a JSON payload looks like this:

```
Byte 0: 0x82  (FIN bit set, opcode = 0x2 binary)
Byte 1: 0x80 | <length>  (mask bit set, length if < 126 bytes)
Bytes 2-5: 4-byte masking key (random)
Bytes 6+:   payload XOR'd with the masking key
```

Client frames **must** be masked (RFC 6455 §5.3). Server frames sent back to the client are never masked.

---

## Step 3: Python one-shot event sender

Save this as `send_event.py`. It implements a minimal WebSocket client using only the Python standard library — no `websockets`, `aiohttp`, or other dependencies.

```python
#!/usr/bin/env python3
"""Send a single JSON event to a Cloacina accumulator over WebSocket."""

import socket
import struct
import base64
import os
import sys
import json


def send_accumulator_event(host, port, accumulator_name, token, event):
    """Open a WebSocket connection, send one binary frame, and close."""
    path = f"/v1/ws/accumulator/{accumulator_name}"
    ws_key = base64.b64encode(os.urandom(16)).decode()

    sock = socket.create_connection((host, port), timeout=10)

    # HTTP/1.1 upgrade request — token in query string
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

    # Read until we have the full HTTP response header
    response = b""
    while b"\r\n\r\n" not in response:
        chunk = sock.recv(4096)
        if not chunk:
            raise ConnectionError("Server closed connection during upgrade")
        response += chunk

    status_line = response.split(b"\r\n")[0].decode()
    if "101" not in status_line:
        raise ConnectionError(f"WebSocket upgrade rejected: {status_line}")

    print(f"  WebSocket connected ({status_line.strip()})")

    # Serialize the event to JSON bytes
    payload = json.dumps(event).encode("utf-8")
    mask_key = os.urandom(4)

    # Build the frame header
    frame = bytearray()
    frame.append(0x82)  # FIN + binary opcode

    length = len(payload)
    if length < 126:
        frame.append(0x80 | length)         # mask bit set
    elif length < 65536:
        frame.append(0x80 | 126)
        frame.extend(struct.pack(">H", length))
    else:
        frame.append(0x80 | 127)
        frame.extend(struct.pack(">Q", length))

    frame.extend(mask_key)

    # Mask the payload
    masked = bytearray(length)
    for i in range(length):
        masked[i] = payload[i] ^ mask_key[i % 4]
    frame.extend(masked)

    sock.sendall(frame)
    print(f"  Sent {length} bytes: {json.dumps(event)}")

    # Send a clean close frame (opcode 0x8, masked, no body)
    close_frame = bytearray([0x88, 0x80]) + os.urandom(4)
    sock.sendall(close_frame)

    sock.close()
    print("  Connection closed.")


if __name__ == "__main__":
    host = sys.argv[1] if len(sys.argv) > 1 else "localhost"
    port = int(sys.argv[2]) if len(sys.argv) > 2 else 8080
    token = sys.argv[3] if len(sys.argv) > 3 else os.environ.get("TOKEN", "")

    if not token:
        print("Usage: python3 send_event.py <host> <port> <token>", file=sys.stderr)
        sys.exit(1)

    event = {
        "best_bid": 100.10,
        "best_ask": 100.15,
    }

    print(f"Sending event to orderbook accumulator on {host}:{port}...")
    send_accumulator_event(host, port, "orderbook", token, event)
```

Run it:

```bash
python3 send_event.py localhost 8080 "${TOKEN}"
```

Expected output:

```
Sending event to orderbook accumulator on localhost:8080...
  WebSocket connected (HTTP/1.1 101 Switching Protocols)
  Sent 37 bytes: {"best_bid": 100.1, "best_ask": 100.15}
  Connection closed.
```

---

## Step 4: Verify the graph fired

After the event is delivered, the `price_signal` reactor evaluates its `when_any` firing condition. Because `orderbook` is the only declared source and it just received a value, the reactor fires immediately.

```bash
curl -s "${BASE_URL}/v1/health/reactors/price_signal" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -m json.tool
```

The response includes the reactor's live state:

```json
{
  "name": "price_signal",
  "health": {
    "state": "running",
    "last_fired_at": "2026-04-06T12:34:56.789Z",
    "fire_count": 1
  },
  "accumulators": ["orderbook"],
  "paused": false
}
```

The `fire_count` and `last_fired_at` fields confirm the graph executed. If they are missing or `fire_count` is 0, the event did not reach the reactor — check the server logs for deserialization errors.

{{< hint type=warning title="Type mismatch errors" >}}
The accumulator deserializes the payload into the boundary type declared in your graph (`OrderBook` in Tutorial 07). If the JSON keys don't match the struct fields exactly, deserialization fails silently and the reactor does not fire. Double-check field names: `best_bid` and `best_ask`.
{{< /hint >}}

---

## Step 5: High-throughput persistent connection

Opening a new TCP connection per event is expensive. For continuous event feeds, keep the WebSocket open and send multiple frames on the same connection.

```python
#!/usr/bin/env python3
"""Persistent WebSocket producer — sends events at a configurable rate."""

import socket
import struct
import base64
import os
import json
import time
import math


class AccumulatorClient:
    """Persistent WebSocket client for a single accumulator."""

    def __init__(self, host, port, accumulator_name, token, timeout=10):
        path = f"/v1/ws/accumulator/{accumulator_name}"
        ws_key = base64.b64encode(os.urandom(16)).decode()

        self.sock = socket.create_connection((host, port), timeout=timeout)

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
                raise ConnectionError("Server closed connection during upgrade")
            response += chunk

        if b"101" not in response.split(b"\r\n")[0]:
            self.sock.close()
            raise ConnectionError("WebSocket upgrade rejected")

    def send(self, event: dict) -> bool:
        """Send a single masked binary frame. Returns True on success."""
        try:
            payload = json.dumps(event).encode("utf-8")
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
            masked = bytearray(length)
            for i in range(length):
                masked[i] = payload[i] ^ mask_key[i % 4]
            frame.extend(masked)

            self.sock.sendall(frame)
            return True
        except OSError:
            return False

    def close(self):
        try:
            close_frame = bytearray([0x88, 0x80]) + os.urandom(4)
            self.sock.sendall(close_frame)
        except OSError:
            pass
        self.sock.close()


if __name__ == "__main__":
    import sys

    token = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("TOKEN", "")
    events_per_second = 10
    duration_seconds = 30

    client = AccumulatorClient("localhost", 8080, "orderbook", token)
    print(f"Sending {events_per_second} events/sec for {duration_seconds}s...")

    sent = 0
    interval = 1.0 / events_per_second
    start = time.monotonic()
    seq = 0

    while time.monotonic() - start < duration_seconds:
        # Simulate a slowly varying mid-price
        mid = 100.0 + math.sin(seq * 0.05) * 0.5
        spread = 0.05 + abs(math.cos(seq * 0.1)) * 0.10

        event = {
            "best_bid": round(mid - spread / 2, 4),
            "best_ask": round(mid + spread / 2, 4),
        }
        if client.send(event):
            sent += 1
        seq += 1
        time.sleep(interval)

    client.close()
    print(f"Done. Sent {sent} events.")
```

Run it with your token:

```bash
python3 persistent_producer.py "${TOKEN}"
```

---

## Step 6: The reactor WebSocket endpoint

Operators can also send commands directly to a reactor via the reactor WebSocket:

```
GET /v1/ws/reactor/{name}?token=...
```

Unlike the accumulator endpoint (which accepts arbitrary payloads), the reactor endpoint expects JSON `ReactorCommand` messages and responds with `ReactorResponse` JSON. Commands are text frames.

Supported commands:

| Command | Effect |
|---|---|
| `{"type":"ForceFire"}` | Execute the graph immediately, ignoring the firing condition |
| `{"type":"FireWith","cache":{...}}` | Execute with a specific `InputCache` snapshot |
| `{"type":"GetState"}` | Return the current `InputCache` state |
| `{"type":"Pause"}` | Stop the reactor from firing |
| `{"type":"Resume"}` | Resume a paused reactor |

Example — force a manual fire from Python:

```python
import socket, base64, os, json

def reactor_command(host, port, reactor_name, token, command):
    path = f"/v1/ws/reactor/{reactor_name}"
    ws_key = base64.b64encode(os.urandom(16)).decode()
    sock = socket.create_connection((host, port), timeout=10)

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

    response = b""
    while b"\r\n\r\n" not in response:
        chunk = sock.recv(4096)
        if not chunk:
            break
        response += chunk

    # Send command as a text frame (opcode 0x81)
    payload = json.dumps(command).encode()
    mask_key = os.urandom(4)
    frame = bytearray([0x81, 0x80 | len(payload)])
    frame.extend(mask_key)
    masked = bytearray(b ^ mask_key[i % 4] for i, b in enumerate(payload))
    frame.extend(masked)
    sock.sendall(frame)

    # Read one response frame (simplified — skips frame header parsing)
    resp = sock.recv(4096)
    sock.close()
    # The JSON starts after the 2-byte frame header
    print(json.loads(resp[2:]))

reactor_command("localhost", 8080, "price_signal", token, {"type": "ForceFire"})
```

Expected response:

```json
{"type": "Fired"}
```

---

## Troubleshooting

**HTTP 401 on upgrade**: The token is missing or invalid. Make sure you're passing it as `?token=...` in the URL, not as a header, if using a browser WebSocket client. Server-side clients should use `Authorization: Bearer ...`.

**HTTP 403 on upgrade**: The PAK is valid but not authorized for this accumulator. Admin keys have access to all accumulators. Tenant-scoped keys can only push to accumulators registered under their tenant.

**Close frame `4404` after connecting**: The accumulator name in the URL does not match any registered accumulator. Check the exact name (case-sensitive) against `/v1/health/accumulators`.

**`fire_count` stays at 0 after sending events**: The payload deserialization is failing. The accumulator forwards the raw bytes to the reactor, which attempts to deserialize them as the boundary type. Make sure the JSON keys exactly match the Rust struct field names as seen by `serde` (by default, the snake_case field names).

---

## Next steps

- [**Tutorial 09: Kafka-Sourced Computation Graphs**]({{< ref "/tutorials/computation-graphs/service/09-kafka-stream/" >}}) — declare a `stream` accumulator in `package.toml` so the server reads events from a Kafka topic automatically, without an external producer
