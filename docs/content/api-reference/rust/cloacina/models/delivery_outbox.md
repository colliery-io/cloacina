# cloacina::models::delivery_outbox <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Delivery Outbox Model

Domain types for the delivery outbox — the durable, ack-tracked,
recipient-addressed push-delivery outbox of the interservice communication
substrate (spec CLOACI-S-0012, decided in CLOACI-A-0006).
Unlike [`crate::models::task_outbox`] (a transient, competing-consumer
claim queue deleted on claim), delivery-outbox rows are addressed to a
specific recipient, carry a payload, and are retained until acked. The
substrate is Postgres-only at runtime.

## Structs

### `cloacina::models::delivery_outbox::DeliveryOutbox`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

A delivery-outbox row (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `i64` | Auto-incrementing primary key (also the replay-ordering cursor). |
| `recipient` | `String` | Addressed recipient key (e.g. `agent:<uuid>`, `exec_events:<exec_id>`). |
| `kind` | `String` | Payload discriminator (e.g. `work`, `execution_event`). |
| `tenant_id` | `Option < String >` | Tenant scope, when applicable (matches the server's `Nullable<Text>` tenant id). |
| `payload` | `Vec < u8 >` | Opaque payload bytes. NOTIFY never carries this — only the row id. |
| `delivery_state` | `String` | Current delivery lifecycle state (see [`DeliveryState`]). |
| `delivery_attempts` | `i32` | Number of delivery attempts so far (incremented on each (re)delivery). |
| `created_at` | `UniversalTimestamp` | When the row was enqueued. |
| `delivered_at` | `Option < UniversalTimestamp >` | When it was last pushed to the recipient. |
| `acked_at` | `Option < UniversalTimestamp >` | When the recipient acked. |

#### Methods

##### `state` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn state (& self) -> Option < DeliveryState >
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





### `cloacina::models::delivery_outbox::NewDeliveryOutbox`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Structure for enqueuing a new delivery-outbox row.

`delivery_state` defaults to `pending` and counters/timestamps are set by
the DAL; callers supply only the addressing and payload.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `recipient` | `String` |  |
| `kind` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `payload` | `Vec < u8 >` |  |



## Enums

### `cloacina::models::delivery_outbox::DeliveryState` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Delivery lifecycle of an outbox row.

`Pending` → `Delivered` (pushed to recipient, awaiting ack) → `Acked`
(recipient confirmed receipt). `Delivered` → `Pending` is the sweeper /
reconnect redelivery path (T-0628 / T-0627).

#### Variants

- **`Pending`**
- **`Delivered`**
- **`Acked`**
