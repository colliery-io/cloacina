# cloacina::delivery::envelope <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Shared WebSocket envelope for the interservice communication substrate (CLOACI-I-0115 / S-0012, task T-0627).

Replaces the bespoke per-endpoint framing in `cloacina-server/src/routes/ws.rs`
with a single versioned envelope consumed by every substrate WS consumer
(CLI execution-events in T-0629, fleet agent protocol in T-0631 / I-0114).

## Enums

### `cloacina::delivery::envelope::ServerMessage` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Server → recipient frames.

#### Variants

- **`Welcome`** - Sent first on a new connection. Echoes the negotiated protocol version
and (informationally) the highest row id known to the server at
connect time so the recipient can size its dedup window.
- **`Push`** - A delivery-outbox row addressed to this recipient. `payload_b64` is
base64-encoded raw bytes from `delivery_outbox.payload`.



### `cloacina::delivery::envelope::ClientMessage` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Recipient → server frames.

#### Variants

- **`Hello`** - Optional connect-time advisory. `since_id` is a cursor hint the server
may use to skip already-acked rows in a future cursor-based catch-up;
v1 ignores it (the server resets `delivered` → `pending` and replays
via the relay).
- **`Ack`** - Recipient confirms it has processed row `id`. Idempotent.



### `cloacina::delivery::envelope::EnvelopeError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors decoding/encoding substrate envelope frames.

#### Variants

- **`WrongVariant`**
- **`Base64`**
- **`Json`**
- **`UnsupportedVersion`**
