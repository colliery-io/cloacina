---
title: "WebSocket Protocol"
description: "Reference for Cloacina's WebSocket endpoints and message formats"
weight: 36
aliases:
  - "/platform/reference/websocket-protocol/"

---

# WebSocket Protocol Reference

Cloacina exposes three WebSocket endpoints. The **accumulator** endpoint allows external producers to push events into graph accumulators. The **reactor** endpoint allows operators to send manual commands (force-fire, pause, resume) and query reactor state. The **substrate delivery** endpoint streams at-least-once push deliveries from the server's transactional outbox to a named recipient — this is how execution events reach `cloacinactl execution follow` and SDK subscribers.

All endpoints authenticate on the HTTP upgrade request before promoting to a WebSocket connection.

Machine-readable JSON Schemas for every message variant are published alongside this document — see [Message Schemas](#message-schemas).

## Endpoint Overview

| Endpoint | Purpose | Frame type |
|---|---|---|
| `GET /v1/ws/accumulator/{name}` | Push events to a named accumulator | Binary or Text |
| `GET /v1/ws/reactor/{name}` | Send commands to a named reactor | Text (JSON) |
| `GET /v1/ws/delivery/{recipient}` | Subscribe to outbox deliveries addressed to `{recipient}` | Text (JSON, versioned envelope) |

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
| `4400` | Invalid client frame (delivery endpoint — unparsable JSON) | Server |
| `4404` | Accumulator/reactor name not registered | Server |
| `4426` | Unsupported `protocol_version` in `hello` (delivery endpoint) | Server |

The custom code `4404` is sent when the named endpoint exists in the URL but has no registered handler in the `EndpointRegistry` (the graph was unloaded or never loaded). `4400` and `4426` apply to the substrate delivery endpoint only.

---

## Accumulator Endpoint Messages

**Endpoint:** `GET /v1/ws/accumulator/{name}`

The accumulator endpoint is a **write-only** interface for external producers. Clients send event data; the server forwards it to all graph accumulators registered under that name.

### Client to Server: Event Submission

The server accepts both **binary** (opcode `0x82`) and **text** (opcode `0x81`) frames.

**Payload format:**

- In debug builds: JSON-serialized `serde_json::Value`
- In release builds: bincode-serialized data matching the accumulator's boundary type

Production servers use release builds (bincode). When testing against a debug server, send JSON.

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

## Substrate Delivery Endpoint

**Endpoint:** `GET /v1/ws/delivery/{recipient}`

The delivery endpoint is the client side of the interservice communication substrate (CLOACI-S-0012): a transactional outbox in the server database, drained by a relay, pushed over WebSocket to whichever connection has registered for `{recipient}`. It carries a **versioned envelope** — every frame in both directions includes a `protocol_version` field (currently `1`).

Authentication matches the other endpoints (Bearer header or `?token=` — typically a [single-use ticket](#websocket-ticket-flow)). The tenant scope is inferred from the authenticated key and enforced against each outbox row's `tenant_id`.

### Protocol Versioning

- Every frame carries `protocol_version`. The current version is **1**; it is bumped on backwards-incompatible changes.
- The server's `welcome` frame announces the version it speaks.
- A client *should* send `hello` declaring its version after connecting. The server validates it: an unsupported version closes the connection with code **`4426`** (`unsupported protocol_version`), so a version-mismatched SDK fails loudly instead of silently misreading frames.
- Unparsable client frames close the connection with code **`4400`**.

### Server to Client: `welcome` and `push`

The first frame on every connection is `welcome`:

```json
{"type": "welcome", "protocol_version": 1, "max_known_id": 0}
```

`max_known_id` is advisory (a dedup-window sizing hint); v1 servers send `0`.

Each subsequent frame is a `push` — one outbox row addressed to this recipient:

```json
{
  "type": "push",
  "protocol_version": 1,
  "id": 42,
  "kind": "execution_event",
  "recipient": "exec_events:f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "tenant_id": null,
  "payload_b64": "eyJldmVudF90eXBlIjoi..."
}
```

`payload_b64` is the base64-encoded raw payload bytes. The inner format is producer-defined and discriminated by `kind` — execution events are JSON.

### Client to Server: `hello` and `ack`

```json
{"type": "hello", "protocol_version": 1, "since_id": null}
```

`since_id` is an advisory cursor for future cursor-based catch-up; v1 servers ignore it (resync is handled server-side, below).

After processing a `push`, the client acknowledges it by row id:

```json
{"type": "ack", "protocol_version": 1, "id": 42}
```

Acks are **idempotent** — re-acking an acked or unknown row is a no-op.

### Delivery Semantics: At-Least-Once + Resync

Delivery is **at-least-once**. A recipient may see the same `push` more than once across disconnect/reconnect cycles and must deduplicate on `id`.

On every (re)connect, the server resets all `delivered`-but-unacked rows for the authenticated `(recipient, tenant)` back to `pending` and wakes the relay, which re-pushes them through the normal path. There are no separate resync frames — after `welcome`, the client simply sees a stream of `push` frames that includes any unacked backlog. A safety-net sweeper additionally redelivers rows stuck in `delivered` past a staleness threshold.

**Backpressure:** each connection has a bounded push channel. When it fills, rows simply remain `pending` in the outbox and are delivered when the client catches up — nothing is dropped (unlike the accumulator endpoint).

**Single subscriber:** one connection per `(recipient, tenant)` — a new connection for the same recipient takes over and the previous connection's channel closes.

### Execution-Events Subscription

Workflow execution events are delivered through this endpoint using the recipient convention:

```
exec_events:<execution_id>
```

Flow (this is exactly what `cloacinactl execution follow <id>` does):

1. `POST /v1/auth/ws-ticket` to mint a single-use ticket.
2. Connect to `wss://host/v1/ws/delivery/exec_events%3A<execution_id>?token=<ticket>` (the `:` may be percent-encoded as `%3A`; both forms are accepted).
3. Read `push` frames; base64-decode `payload_b64` into a JSON execution event.
4. `ack` each frame by `id`.

The connection stays open after the execution completes; close it client-side when a terminal event is observed.

---

## Message Schemas

JSON Schema (draft 2020-12) for every message variant, served by this documentation site and checked into the repository under `docs/static/schemas/ws/`:

| Schema | Covers |
|---|---|
| [`delivery-server-message.schema.json`](/schemas/ws/delivery-server-message.schema.json) | `welcome`, `push` |
| [`delivery-client-message.schema.json`](/schemas/ws/delivery-client-message.schema.json) | `hello`, `ack` |
| [`reactor-command.schema.json`](/schemas/ws/reactor-command.schema.json) | `force_fire`, `fire_with`, `get_state`, `pause`, `resume` |
| [`reactor-response.schema.json`](/schemas/ws/reactor-response.schema.json) | `fired`, `state`, `paused`, `resumed`, `error` |

Accumulator frames have no fixed schema — the payload is the registered boundary type's serialization (JSON in debug builds, bincode in release builds).

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
| Delivery | Unparsable client frame | Close frame `4400` + connection closed |
| Delivery | Unsupported `protocol_version` in `hello` | Close frame `4426` + connection closed |

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
POST /v1/auth/ws-ticket HTTP/1.1
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

Until native rate limiting is implemented, use a reverse proxy (nginx, envoy) to enforce connection and message rate limits on WebSocket endpoints.

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
