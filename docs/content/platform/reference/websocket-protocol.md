---
title: "WebSocket Protocol"
description: "Reference for Cloacina's WebSocket endpoints and message formats"
weight: 36
---

# WebSocket Protocol Reference

Cloacina exposes two WebSocket endpoints for real-time interaction with computation graphs. The **accumulator** endpoint allows external producers to push events into graph accumulators. The **reactor** endpoint allows operators to send manual commands (force-fire, pause, resume) and query reactor state.

Both endpoints authenticate on the HTTP upgrade request before promoting to a WebSocket connection.

## Endpoint Overview

| Endpoint | Purpose | Frame type |
|---|---|---|
| `GET /v1/ws/accumulator/{name}` | Push events to a named accumulator | Binary or Text |
| `GET /v1/ws/reactor/{name}` | Send commands to a named reactor | Text (JSON) |

**Base URL:** `ws://host:port` (or `wss://` with TLS termination)

**Authentication** is required on both endpoints and is validated during the HTTP upgrade handshake. Two methods are supported:

| Method | Header/Param | Example |
|---|---|---|
| Bearer token | `Authorization` header | `Authorization: Bearer clk_a1b2c3...` |
| Query parameter | `?token=` | `?token=clk_a1b2c3...` |

Browsers cannot set custom headers on WebSocket upgrade requests, so the query parameter method exists for browser-based clients. Server-to-server clients should prefer the `Authorization` header.

---

## Connection Lifecycle

### 1. WebSocket Upgrade Handshake

The client sends a standard HTTP/1.1 upgrade request:

```http
GET /v1/ws/accumulator/orderbook?token=clk_abc123 HTTP/1.1
Host: localhost:8080
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Version: 13
```

### 2. Authentication Validation

Before completing the upgrade, the server:

1. Extracts the token from the `Authorization: Bearer` header (preferred) or `?token=` query parameter
2. Hashes the token with SHA-256
3. Checks the LRU cache (256 entries, 30-second TTL)
4. On cache miss, validates against the database
5. Checks per-endpoint authorization policy (tenant scoping, explicit producer/operator lists)

If authentication fails, the server returns an HTTP error response (not a WebSocket close frame) since the upgrade has not yet completed:

| HTTP Status | Cause |
|---|---|
| `401 Unauthorized` | Missing token, invalid/revoked API key |
| `403 Forbidden` | Valid key but not authorized for this endpoint |

### 3. Connection Establishment

On success, the server responds with `101 Switching Protocols` and the connection is upgraded to WebSocket.

### 4. Ping/Pong Heartbeat

Axum handles WebSocket ping/pong frames automatically at the transport layer. The server does not send application-level heartbeats. Clients may send WebSocket ping frames per RFC 6455; the server will respond with pong frames transparently.

### 5. Graceful Disconnection

Either side may initiate closure by sending a WebSocket close frame (opcode `0x8`). The server logs disconnection and cleans up internal state. Clients should send a close frame before dropping the TCP connection.

### 6. Close Codes

| Code | Meaning | Sent by |
|---|---|---|
| `1000` | Normal closure | Client or Server |
| `1001` | Going away (server shutdown) | Server |
| `4404` | Accumulator/reactor name not registered | Server |

The custom code `4404` is sent when the named endpoint exists in the URL but has no registered handler in the `EndpointRegistry` (the graph was unloaded or never loaded).

---

## Accumulator Endpoint Messages

**Endpoint:** `GET /v1/ws/accumulator/{name}`

The accumulator endpoint is a **write-only** interface for external producers. Clients send event data; the server forwards it to all graph accumulators registered under that name.

### Client to Server: Event Submission

The server accepts both **binary** (opcode `0x82`) and **text** (opcode `0x81`) frames.

**Payload format:**

- In debug builds: JSON-serialized `serde_json::Value`
- In release builds: bincode-serialized data matching the accumulator's boundary type

The payload bytes are forwarded as-is to the accumulator's deserialization layer. The accumulator attempts to deserialize the payload into its declared boundary type (e.g., `OrderBook`). Field names must exactly match the Rust struct's `serde` field names (snake_case by default).

**Example JSON payload (debug mode):**

```json
{"best_bid": 100.10, "best_ask": 100.15}
```

**Binary frame structure (RFC 6455):**

```
Byte 0:    0x82         (FIN=1, opcode=binary)
Byte 1:    0x80 | len   (MASK=1, payload length)
Bytes 2-5: mask_key     (4 random bytes)
Bytes 6+:  payload XOR masked
```

Client frames **must** be masked per RFC 6455 section 5.3.

### Server to Client: Responses

The accumulator endpoint does **not** send acknowledgment frames on success. It is a fire-and-forget interface. The server logs delivery internally (number of recipients reached).

On failure, the server sends a **close frame** with a reason:

| Close Code | Reason | Meaning |
|---|---|---|
| `4404` | `accumulator '{name}' not registered` | No accumulators registered under this name |

The server closes the connection after sending this frame. The client should not attempt to send further messages.

**Backpressure:** If an accumulator's internal channel is full (`try_send` fails with `Full`), the message is dropped silently on the server side. The client receives no indication. This is a deliberate design choice for high-throughput scenarios where occasional message loss is acceptable. Monitor the server logs for `"accumulator channel full, dropping message"` warnings.

---

## Reactor Endpoint Messages

**Endpoint:** `GET /v1/ws/reactor/{name}`

The reactor endpoint is a **bidirectional** JSON command/response interface for operators. Clients send commands as text frames; the server responds with a JSON result for each command.

### Client to Server: Commands

Commands are sent as **text frames** containing JSON. The command type is indicated by the `"command"` field (internally tagged enum, snake_case variants).

| Command | JSON | Effect |
|---|---|---|
| Force Fire | `{"command": "force_fire"}` | Execute the graph immediately, ignoring firing conditions |
| Fire With | `{"command": "fire_with", "cache": {...}}` | Execute with a specific input cache snapshot |
| Get State | `{"command": "get_state"}` | Return the current input cache state |
| Pause | `{"command": "pause"}` | Stop the reactor from firing (continues accepting boundaries) |
| Resume | `{"command": "resume"}` | Resume a paused reactor |

**FireWith cache format:**

The `cache` field is a map of source names to byte arrays (base64-encoded in JSON):

```json
{
  "command": "fire_with",
  "cache": {
    "orderbook": [123, 34, 98, 101, 115, 116, 95, 98, 105, 100, 34, 58, 49, 48, 48, 125]
  }
}
```

Each value is the serialized boundary bytes for that source, as `Vec<u8>`.

### Server to Client: Responses

Every command receives exactly one response as a text frame. Responses use the `"type"` field (internally tagged enum, snake_case variants).

| Response | JSON | Meaning |
|---|---|---|
| Fired | `{"type": "fired"}` | Graph execution was triggered |
| State | `{"type": "state", "cache": {...}}` | Current input cache (JSON string map) |
| Paused | `{"type": "paused"}` | Reactor is now paused |
| Resumed | `{"type": "resumed"}` | Reactor is now resumed |
| Error | `{"type": "error", "message": "..."}` | Command failed |

**State response cache format:**

```json
{
  "type": "state",
  "cache": {
    "orderbook": "{\"best_bid\":100.1,\"best_ask\":100.15}"
  }
}
```

Values are JSON string representations of the cached boundary data.

### Per-Command Authorization

Each command is independently authorized against the reactor's policy. A key may be permitted to `get_state` but denied `force_fire`. If a command is denied, the response is:

```json
{
  "type": "error",
  "message": "operation ForceFire not permitted on reactor 'price_signal'"
}
```

The connection remains open -- only the individual command is rejected.

---

## Error Handling

### Connection-Level Errors

These occur during the HTTP upgrade phase (before WebSocket is established):

| HTTP Status | JSON Body | Cause |
|---|---|---|
| `401` | `{"error": "missing auth token", "code": "unauthorized"}` | No token in header or query param |
| `401` | `{"error": "invalid or revoked API key", "code": "unauthorized"}` | Token not found in DB |
| `403` | `{"error": "not authorized for accumulator 'X'", "code": "endpoint_access_denied"}` | Valid key, wrong permissions |
| `403` | `{"error": "not authorized for reactor 'X'", "code": "endpoint_access_denied"}` | Valid key, wrong permissions |

### Message-Level Errors

After the WebSocket connection is established:

| Endpoint | Error | Behavior |
|---|---|---|
| Accumulator | Name not registered | Close frame `4404` + connection closed |
| Accumulator | Channel full | Message dropped silently (logged server-side) |
| Reactor | Invalid JSON | Error response: `{"type": "error", "message": "invalid command: ..."}` |
| Reactor | Unknown command variant | Error response with serde parse error |
| Reactor | Operation denied | Error response, connection stays open |
| Reactor | No reactor handle | Error response: `{"type": "error", "message": "no reactor handle for 'X'"}` |

### Reconnection Strategies

The server does not implement automatic reconnection. Clients should implement exponential backoff:

1. On close code `4404`: The accumulator/reactor is not loaded. Retry after verifying the graph is deployed via `GET /v1/health/accumulators`.
2. On close code `1001`: Server is shutting down. Reconnect with backoff.
3. On unexpected disconnection: Reconnect with exponential backoff (start 100ms, max 30s).
4. On `401`/`403` during upgrade: Do not retry with the same token. Obtain a fresh ticket or verify key validity.

---

## Authentication

### Bearer Token (Header)

Pass the API key directly in the HTTP upgrade request:

```
Authorization: Bearer clk_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6
```

Best for server-to-server communication where headers are fully controllable.

### Query Parameter

Pass the token as a URL query parameter:

```
ws://localhost:8080/v1/ws/accumulator/orderbook?token=clk_a1b2c3d4...
```

Required for browser WebSocket clients that cannot set custom headers. Note that the token will appear in server access logs and potentially in proxy logs.

### WebSocket Ticket Flow

For enhanced security, exchange a long-lived API key for a short-lived, single-use ticket:

**Step 1:** Request a ticket via REST (requires existing Bearer auth):

```http
POST /auth/ws-ticket HTTP/1.1
Authorization: Bearer clk_a1b2c3d4...
```

**Response:**

```json
{
  "ticket": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "expires_in_seconds": 60
}
```

**Step 2:** Use the ticket as the `?token=` parameter on WebSocket upgrade:

```
ws://localhost:8080/v1/ws/accumulator/orderbook?token=f47ac10b-58cc-4372-a567-0e02b2c3d479
```

**Ticket properties:**

- Single-use: consumed on first connection attempt
- TTL: 60 seconds from issuance
- Carries the same identity and permissions as the issuing key
- UUID format (not prefixed with `clk_`)

This flow avoids exposing long-lived API keys in URLs while still supporting browser clients.

### Authorization Policies

Each endpoint has an authorization policy configured at registration time:

- **allow_all_authenticated:** Any valid API key can connect (single-tenant default)
- **Tenant-scoped:** Only keys belonging to specific tenants are authorized
- **Explicit key lists:** Only specific API key IDs are authorized (accumulators use `allowed_producers`, reactors use `allowed_operators`)
- **Admin keys:** Always authorized regardless of policy (god mode)

Reactor endpoints additionally support **per-key operation restrictions** -- a key may be authorized to connect but limited to a subset of commands (e.g., `get_state` only).

---

## Rate Limiting

Rate limiting is **not currently enforced at the WebSocket layer**. The server accepts messages as fast as the client can send them, subject to:

- **Channel backpressure:** Each accumulator has a bounded channel (configured at graph registration). When the channel is full, messages are dropped. This acts as implicit rate limiting.
- **TCP flow control:** Standard TCP backpressure applies if the server cannot process frames fast enough.

Clients sending faster than the graph can execute will experience silent message loss (accumulator) or increased latency (reactor commands queue in the channel).

The `ApiError::too_many_requests` constructor exists in the error module, indicating rate limiting may be added in future versions at the HTTP upgrade layer.

---

## Example Client Code

### Rust (tokio-tungstenite)

```rust
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let token = std::env::var("TOKEN")?;
    let url = format!(
        "ws://localhost:8080/v1/ws/accumulator/orderbook?token={}",
        token
    );

    let (mut ws, _response) = connect_async(&url).await?;
    println!("Connected to accumulator");

    // Send a JSON event as a binary frame
    let event = serde_json::json!({"best_bid": 100.10, "best_ask": 100.15});
    let payload = serde_json::to_vec(&event)?;
    ws.send(Message::Binary(payload.into())).await?;
    println!("Sent event");

    // Clean close
    ws.close(None).await?;
    Ok(())
}
```

### Rust -- Reactor Commands

```rust
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let token = std::env::var("TOKEN")?;
    let url = format!(
        "ws://localhost:8080/v1/ws/reactor/price_signal?token={}",
        token
    );

    let (mut ws, _) = connect_async(&url).await?;

    // Send a GetState command
    let cmd = serde_json::json!({"command": "get_state"});
    ws.send(Message::Text(cmd.to_string())).await?;

    // Read the response
    if let Some(Ok(Message::Text(resp))) = ws.next().await {
        let parsed: serde_json::Value = serde_json::from_str(&resp)?;
        println!("State: {}", serde_json::to_string_pretty(&parsed)?);
    }

    // Force fire
    let cmd = serde_json::json!({"command": "force_fire"});
    ws.send(Message::Text(cmd.to_string())).await?;

    if let Some(Ok(Message::Text(resp))) = ws.next().await {
        println!("Response: {}", resp);
    }

    ws.close(None).await?;
    Ok(())
}
```

### Python (websockets library)

```python
import asyncio
import json
import os
import websockets

async def send_events():
    token = os.environ["TOKEN"]
    uri = f"ws://localhost:8080/v1/ws/accumulator/orderbook?token={token}"

    async with websockets.connect(uri) as ws:
        # Send events as binary frames (JSON bytes)
        for i in range(10):
            event = {"best_bid": 100.0 + i * 0.01, "best_ask": 100.05 + i * 0.01}
            await ws.send(json.dumps(event).encode("utf-8"))
            print(f"Sent event {i+1}")

        # Connection closes cleanly when context manager exits

asyncio.run(send_events())
```

### Python -- Reactor Commands

```python
import asyncio
import json
import os
import websockets

async def reactor_control():
    token = os.environ["TOKEN"]
    uri = f"ws://localhost:8080/v1/ws/reactor/price_signal?token={token}"

    async with websockets.connect(uri) as ws:
        # Get current state
        await ws.send(json.dumps({"command": "get_state"}))
        resp = json.loads(await ws.recv())
        print(f"Current state: {json.dumps(resp, indent=2)}")

        # Force fire
        await ws.send(json.dumps({"command": "force_fire"}))
        resp = json.loads(await ws.recv())
        print(f"Force fire: {resp}")

        # Pause
        await ws.send(json.dumps({"command": "pause"}))
        resp = json.loads(await ws.recv())
        print(f"Pause: {resp}")

        # Resume
        await ws.send(json.dumps({"command": "resume"}))
        resp = json.loads(await ws.recv())
        print(f"Resume: {resp}")

asyncio.run(reactor_control())
```

### websocat (CLI testing)

[websocat](https://github.com/vi/websocat) is useful for interactive testing:

```bash
# Connect to accumulator and send a JSON event
echo '{"best_bid": 100.10, "best_ask": 100.15}' | \
  websocat "ws://localhost:8080/v1/ws/accumulator/orderbook?token=${TOKEN}"

# Interactive reactor session
websocat "ws://localhost:8080/v1/ws/reactor/price_signal?token=${TOKEN}"
# Then type commands:
# {"command": "get_state"}
# {"command": "force_fire"}
# {"command": "pause"}
# {"command": "resume"}
```

---

## Wire Format Summary

| Aspect | Accumulator | Reactor |
|---|---|---|
| Direction | Client -> Server (unidirectional) | Bidirectional (command/response) |
| Client frame type | Binary (`0x82`) or Text (`0x81`) | Text (`0x81`) only |
| Client payload | Serialized boundary data (JSON or bincode) | JSON `ReactorCommand` |
| Server responses | None (close frame on error only) | JSON `ReactorResponse` per command |
| Multiplexing | Broadcast to all accumulators with same name | Single reactor per name |
| Masking | Required (RFC 6455) | Required (RFC 6455) |
