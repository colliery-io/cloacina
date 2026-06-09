# cloacina::dal::unified::delivery_outbox <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified Delivery Outbox DAL (substrate — CLOACI-S-0012 / A-0006, task T-0625).

The delivery outbox is the **durable system of record** for the interservice
communication substrate: a row is enqueued (ideally in the same transaction
as the state change that produced it), pushed to its addressed recipient over
WebSocket, and acked. State machine: `pending → delivered → acked`, with
`delivered → pending` as the redelivery path (sweeper reclaim / reconnect
resync).
Distinct from [`super::task_outbox`], the transient competing-consumer
scheduler→executor claim queue. Rows here are addressed and retained until
acked.
State transitions are implemented as **atomic compare-and-set** updates
(filter on the expected current state; zero rows affected ⇒ the transition
was not permitted from the row's actual state ⇒ [`ValidationError::InvalidStateTransition`]).
The substrate is Postgres-only at runtime; the SQLite arms exist for unified
schema parity and test coverage.

## Structs

### `cloacina::dal::unified::delivery_outbox::DeliveryOutboxDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Data access layer for delivery-outbox operations with runtime backend selection.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



## Functions

### `cloacina::dal::unified::delivery_outbox::to_domain`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn to_domain (r : UnifiedDeliveryOutbox) -> DeliveryOutbox
```

<details>
<summary>Source</summary>

```rust
fn to_domain(r: UnifiedDeliveryOutbox) -> DeliveryOutbox {
    DeliveryOutbox {
        id: r.id,
        recipient: r.recipient,
        kind: r.kind,
        tenant_id: r.tenant_id,
        payload: r.payload.into_inner(),
        delivery_state: r.delivery_state,
        delivery_attempts: r.delivery_attempts,
        created_at: r.created_at,
        delivered_at: r.delivered_at,
        acked_at: r.acked_at,
    }
}
```

</details>



### `cloacina::dal::unified::delivery_outbox::build_insert`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn build_insert (new : NewDeliveryOutbox) -> NewUnifiedDeliveryOutbox
```

Builds the insertable row for a new `pending` outbox entry.

<details>
<summary>Source</summary>

```rust
fn build_insert(new: NewDeliveryOutbox) -> NewUnifiedDeliveryOutbox {
    NewUnifiedDeliveryOutbox {
        recipient: new.recipient,
        kind: new.kind,
        tenant_id: new.tenant_id,
        payload: UniversalBinary::from(new.payload),
        delivery_state: STATE_PENDING.to_string(),
        delivery_attempts: 0,
        created_at: UniversalTimestamp::now(),
    }
}
```

</details>



### `cloacina::dal::unified::delivery_outbox::transition_result`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn transition_result (id : i64 , from : & str , to : & str , affected : usize ,) -> Result < () , ValidationError >
```

Maps a compare-and-set affected-row count to a transition result: exactly one row affected means the transition applied; zero means the row was not in the expected `from` state (or does not exist), which we reject.

<details>
<summary>Source</summary>

```rust
fn transition_result(
    id: i64,
    from: &str,
    to: &str,
    affected: usize,
) -> Result<(), ValidationError> {
    if affected == 1 {
        Ok(())
    } else {
        Err(ValidationError::InvalidStateTransition {
            id,
            from: from.to_string(),
            to: to.to_string(),
        })
    }
}
```

</details>
