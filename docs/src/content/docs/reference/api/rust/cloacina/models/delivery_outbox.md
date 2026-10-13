---
title: "cloacina::models::delivery_outbox"
description: "Delivery Outbox Model"
editUrl: false
sidebar:
  order: 135
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Delivery Outbox Model

Domain types for the delivery outbox — the durable, ack-tracked,
recipient-addressed push-delivery outbox of the interservice communication
substrate (spec CLOACI-S-0012, decided in CLOACI-A-0006).

Unlike the historical `task_outbox` (a transient, competing-consumer claim
queue deleted on claim, removed in CLOACI-T-0914), delivery-outbox rows are
addressed to a specific recipient, carry a payload, and are retained until
acked. The substrate is Postgres-only at runtime.

## Structs

### <span id="struct.DeliveryOutbox"></span><code style="font-size: 0.85em">struct DeliveryOutbox</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

A delivery-outbox row (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `i64` | Auto-incrementing primary key (also the replay-ordering cursor). |
| `recipient` | `String` | Addressed recipient key (e.g. `agent:<uuid>`, `exec_events:<exec_id>`). |
| `kind` | `String` | Payload discriminator (e.g. `work`, `execution_event`). |
| `tenant_id` | `Option<String>` | Tenant scope, when applicable (matches the server's `Nullable<Text>` tenant id). |
| `payload` | `Vec<u8>` | Opaque payload bytes. NOTIFY never carries this — only the row id. |
| `delivery_state` | `String` | Current delivery lifecycle state (see [`DeliveryState`](../../../../rust/cloacina/models/delivery_outbox/#enum.DeliveryState)). |
| `delivery_attempts` | `i32` | Number of delivery attempts so far (incremented on each (re)delivery). |
| `created_at` | `UniversalTimestamp` | When the row was enqueued. |
| `delivered_at` | `Option<UniversalTimestamp>` | When it was last pushed to the recipient. |
| `acked_at` | `Option<UniversalTimestamp>` | When the recipient acked. |

#### Methods

##### <span id="DeliveryOutbox.state"></span><code>state</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn state(&self) -> Option<DeliveryState>
```

Typed view of the persisted `delivery_state` string.

<details>
<summary>Source</summary>

```rust
    pub fn state(&self) -> Option<DeliveryState> {
        DeliveryState::from_db(&self.delivery_state)
    }
```

</details>

### <span id="struct.NewDeliveryOutbox"></span><code style="font-size: 0.85em">struct NewDeliveryOutbox</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Structure for enqueuing a new delivery-outbox row.

`delivery_state` defaults to `pending` and counters/timestamps are set by
the DAL; callers supply only the addressing and payload.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `recipient` | `String` |  |
| `kind` | `String` |  |
| `tenant_id` | `Option<String>` |  |
| `payload` | `Vec<u8>` |  |

## Enums

### <span id="enum.DeliveryState"></span><code style="font-size: 0.85em">enum DeliveryState</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Delivery lifecycle of an outbox row.

`Pending` → `Delivered` (pushed to recipient, awaiting ack) → `Acked`
(recipient confirmed receipt). `Delivered` → `Pending` is the sweeper /
reconnect redelivery path (T-0628 / T-0627).

#### Variants

- **`Pending`**
- **`Delivered`**
- **`Acked`**

#### Methods

##### <span id="DeliveryState.as_str"></span><code>as_​str</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn as_str(&self) -> &'static str
```

String form persisted in the `delivery_state` column.

<details>
<summary>Source</summary>

```rust
    pub fn as_str(&self) -> &'static str {
        match self {
            DeliveryState::Pending => "pending",
            DeliveryState::Delivered => "delivered",
            DeliveryState::Acked => "acked",
        }
    }
```

</details>

##### <span id="DeliveryState.from_db"></span><code>from_​db</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_db(s: &str) -> Option<DeliveryState>
```

Parse from the persisted string form.

<details>
<summary>Source</summary>

```rust
    pub fn from_db(s: &str) -> Option<DeliveryState> {
        match s {
            "pending" => Some(DeliveryState::Pending),
            "delivered" => Some(DeliveryState::Delivered),
            "acked" => Some(DeliveryState::Acked),
            _ => None,
        }
    }
```

</details>

##### <span id="DeliveryState.can_transition_to"></span><code>can_​transition_​to</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn can_transition_to(self, next: DeliveryState) -> bool
```

Whether `self → next` is a permitted transition.

- `Pending → Delivered`: relay pushed the row.
- `Delivered → Acked`: recipient confirmed.
- `Delivered → Pending`: redelivery (sweeper reclaim / reconnect resync).

`Acked` is terminal. All other transitions are rejected.

<details>
<summary>Source</summary>

```rust
    pub fn can_transition_to(self, next: DeliveryState) -> bool {
        use DeliveryState::*;
        matches!(
            (self, next),
            (Pending, Delivered) | (Delivered, Acked) | (Delivered, Pending)
        )
    }
```

</details>
