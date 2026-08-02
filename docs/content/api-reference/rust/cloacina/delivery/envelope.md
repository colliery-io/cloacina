# cloacina::delivery::envelope <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Substrate WS envelope — re-exported from `cloacina-api-types`.

The envelope types moved to `cloacina-api-types` in T-0642 (CLOACI-I-0113)
so SDK clients can consume them without linking the engine. This module
keeps the original `cloacina::delivery::envelope` paths working and holds
the one helper that needs the diesel-backed outbox model.

## Functions

### `cloacina::delivery::envelope::push_from_row`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn push_from_row (row : & DeliveryOutbox) -> ServerMessage
```

Build a `push` frame from an outbox row, base64-encoding the payload.

(Was `ServerMessage::push_from_row` before the type moved crates — an
inherent impl is no longer possible on the foreign type.)

<details>
<summary>Source</summary>

```rust
pub fn push_from_row(row: &DeliveryOutbox) -> ServerMessage {
    ServerMessage::push(
        row.id,
        &row.kind,
        &row.recipient,
        row.tenant_id.clone(),
        &row.payload,
    )
}
```

</details>
