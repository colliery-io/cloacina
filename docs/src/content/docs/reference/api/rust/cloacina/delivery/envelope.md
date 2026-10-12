---
title: "cloacina::delivery::envelope"
description: "Substrate WS envelope — re-exported from `cloacina-api-types`."
sidebar:
  order: 121
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Substrate WS envelope — re-exported from `cloacina-api-types`.

The envelope types moved to `cloacina-api-types` in T-0642 (CLOACI-I-0113)
so SDK clients can consume them without linking the engine. This module
keeps the original `cloacina::delivery::envelope` paths working and holds
the one helper that needs the diesel-backed outbox model.

## Functions

### <span id="fn.push_from_row"></span>`fn push_from_row`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



