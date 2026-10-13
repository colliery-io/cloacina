---
title: "cloacina::dal::unified::delivery_outbox"
description: "Unified Delivery Outbox DAL (substrate — CLOACI-S-0012 / A-0006, task T-0625)."
editUrl: false
sidebar:
  order: 72
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Unified Delivery Outbox DAL (substrate — CLOACI-S-0012 / A-0006, task T-0625).

The delivery outbox is the **durable system of record** for the interservice
communication substrate: a row is enqueued (ideally in the same transaction
as the state change that produced it), pushed to its addressed recipient over
WebSocket, and acked. State machine: `pending → delivered → acked`, with
`delivered → pending` as the redelivery path (sweeper reclaim / reconnect
resync).

Distinct from the historical `task_outbox` (a transient competing-consumer
scheduler→executor claim queue, removed in CLOACI-T-0914). Rows here are
addressed and retained until acked.

State transitions are implemented as **atomic compare-and-set** updates
(filter on the expected current state; zero rows affected ⇒ the transition
was not permitted from the row's actual state ⇒ [`ValidationError::InvalidStateTransition`](../../../../../rust/cloacina/error/#enum.ValidationError)).
The substrate is Postgres-only at runtime; the SQLite arms exist for unified
schema parity and test coverage.

## Structs

### <span id="struct.DeliveryOutboxDAL"></span><code style="font-size: 0.85em">struct DeliveryOutboxDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Data access layer for delivery-outbox operations with runtime backend selection.

#### Methods

##### <span id="DeliveryOutboxDAL.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
```

</details>

##### <span id="DeliveryOutboxDAL.enqueue"></span><code>enqueue</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn enqueue(
    &self,
    new: NewDeliveryOutbox,
) -> Result<DeliveryOutbox, ValidationError>
```

Enqueues a new outbox row in the `pending` state.

For the atomicity guarantee (REQ-1.1.1), producing code should perform
the equivalent insert inside its own transaction; this convenience
method opens its own connection and is used standalone and in tests.

<details>
<summary>Source</summary>

```rust
    pub async fn enqueue(&self, new: NewDeliveryOutbox) -> Result<DeliveryOutbox, ValidationError> {
        let row = build_insert(new);
        let result: UnifiedDeliveryOutbox = crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(delivery_outbox::table)
                .values(&row)
                .get_result(conn)
        })?;
        Ok(to_domain(result))
    }
```

</details>

##### <span id="DeliveryOutboxDAL.mark_delivered"></span><code>mark_​delivered</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn mark_delivered(&self, id: i64) -> Result<(), ValidationError>
```

`pending → delivered`: record that the row was pushed to its recipient. Increments `delivery_attempts` and stamps `delivered_at`.

<details>
<summary>Source</summary>

```rust
    pub async fn mark_delivered(&self, id: i64) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        let affected = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                delivery_outbox::table
                    .filter(delivery_outbox::id.eq(id))
                    .filter(delivery_outbox::delivery_state.eq(STATE_PENDING)),
            )
            .set((
                delivery_outbox::delivery_state.eq(STATE_DELIVERED),
                delivery_outbox::delivered_at.eq(Some(now)),
                delivery_outbox::delivery_attempts.eq(delivery_outbox::delivery_attempts + 1),
            ))
            .execute(conn)
        })
        .map_err(ValidationError::from)?;
        transition_result(id, STATE_PENDING, STATE_DELIVERED, affected)
    }
```

</details>

##### <span id="DeliveryOutboxDAL.mark_acked"></span><code>mark_​acked</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn mark_acked(&self, id: i64) -> Result<(), ValidationError>
```

`pending|delivered → acked`: recipient confirmed receipt.

Accepts BOTH `pending` and `delivered` as valid source states to handle
the relay-vs-recipient race: on localhost the recipient can process a
push frame and ack it before the relay's own `mark_delivered` CAS
lands (the sink's `try_send` returns before the relay's UPDATE runs).
Restricting to `delivered → acked` only would cause those races to
silently drop the ack and leave the row stuck in `delivered` until the
sweeper reclaimed it. Either source state is fine semantically: the
recipient saw the row, whichever bookkeeping order the server reached.

<details>
<summary>Source</summary>

```rust
    pub async fn mark_acked(&self, id: i64) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        let affected = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                delivery_outbox::table
                    .filter(delivery_outbox::id.eq(id))
                    .filter(delivery_outbox::delivery_state.ne(STATE_ACKED)),
            )
            .set((
                delivery_outbox::delivery_state.eq(STATE_ACKED),
                delivery_outbox::acked_at.eq(Some(now)),
            ))
            .execute(conn)
        })
        .map_err(ValidationError::from)?;
        transition_result(id, "pending|delivered", STATE_ACKED, affected)
    }
```

</details>

##### <span id="DeliveryOutboxDAL.reset_to_pending"></span><code>reset_​to_​pending</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn reset_to_pending(&self, id: i64) -> Result<(), ValidationError>
```

`delivered → pending`: redelivery path (sweeper reclaim / reconnect resync).

<details>
<summary>Source</summary>

```rust
    pub async fn reset_to_pending(&self, id: i64) -> Result<(), ValidationError> {
        let affected = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                delivery_outbox::table
                    .filter(delivery_outbox::id.eq(id))
                    .filter(delivery_outbox::delivery_state.eq(STATE_DELIVERED)),
            )
            .set(delivery_outbox::delivery_state.eq(STATE_PENDING))
            .execute(conn)
        })
        .map_err(ValidationError::from)?;
        transition_result(id, STATE_DELIVERED, STATE_PENDING, affected)
    }
```

</details>

##### <span id="DeliveryOutboxDAL.reset_delivered_to_pending_for_recipient"></span><code>reset_​delivered_​to_​pending_​for_​recipient</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn reset_delivered_to_pending_for_recipient(
    &self,
    recipient: &str,
    tenant_id: Option<&str>,
) -> Result<usize, ValidationError>
```

Reset every `delivered`-state row addressed to `(recipient, tenant_id)` back to `pending`. Called on WS reconnect (T-0627) so the relay re-pushes the stuck-delivered set through its normal sink path, avoiding a separate handler-side resync that would race the relay. Returns the number of rows reset.

`tenant_id = None` matches rows where the column is NULL.

<details>
<summary>Source</summary>

```rust
    pub async fn reset_delivered_to_pending_for_recipient(
        &self,
        recipient: &str,
        tenant_id: Option<&str>,
    ) -> Result<usize, ValidationError> {
        let recipient = recipient.to_string();
        let tenant_id = tenant_id.map(|s| s.to_string());
        crate::interact_on_backend!(self.dal, |conn| {
            // diesel: branch on the tenant filter because Option doesn't auto-map to IS NULL.
            let base = delivery_outbox::table
                .filter(delivery_outbox::recipient.eq(recipient))
                .filter(delivery_outbox::delivery_state.eq(STATE_DELIVERED));
            match tenant_id {
                Some(t) => diesel::update(base.filter(delivery_outbox::tenant_id.eq(t)))
                    .set(delivery_outbox::delivery_state.eq(STATE_PENDING))
                    .execute(conn),
                None => diesel::update(base.filter(delivery_outbox::tenant_id.is_null()))
                    .set(delivery_outbox::delivery_state.eq(STATE_PENDING))
                    .execute(conn),
            }
        })
        .map_err(ValidationError::from)
    }
```

</details>

##### <span id="DeliveryOutboxDAL.reassign_open_rows"></span><code>reassign_​open_​rows</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn reassign_open_rows(
    &self,
    from_recipient: &str,
    to_recipient: &str,
) -> Result<usize, ValidationError>
```

Reassign every non-`acked` row from `from_recipient` to `to_recipient`, resetting them to `pending` (and clearing `delivered_at`) so the relay re-pushes them to the new recipient (CLOACI-T-0634 dead-agent reclaim).

Called by the fleet heartbeat sweeper when an agent is evicted: its
in-flight work packets (addressed to `agent:<deadId>`) are re-targeted
to a live agent rather than left pinned to a recipient whose connection
is gone (which would otherwise spin on `NoRoute` forever). The work
keeps its `task_execution_id`, so the `FleetExecutor`'s awaiting
rendezvous receives the new agent's result without any change.

Returns the number of rows reassigned. OQ-2: if the "dead" agent is
actually partitioned-but-alive, the re-addressed work may double-execute
— this matches the at-least-once posture the thread executor has on
heartbeat-loss (deliberate; see initiative OQ-2).

<details>
<summary>Source</summary>

```rust
    pub async fn reassign_open_rows(
        &self,
        from_recipient: &str,
        to_recipient: &str,
    ) -> Result<usize, ValidationError> {
        let from_recipient = from_recipient.to_string();
        let to_recipient = to_recipient.to_string();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                delivery_outbox::table
                    .filter(delivery_outbox::recipient.eq(from_recipient))
                    .filter(delivery_outbox::delivery_state.ne(STATE_ACKED)),
            )
            .set((
                delivery_outbox::recipient.eq(to_recipient),
                delivery_outbox::delivery_state.eq(STATE_PENDING),
                delivery_outbox::delivered_at
                    .eq(None::<crate::database::universal_types::UniversalTimestamp>),
            ))
            .execute(conn)
        })
        .map_err(ValidationError::from)
    }
```

</details>

##### <span id="DeliveryOutboxDAL.list_open_for_recipient"></span><code>list_​open_​for_​recipient</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_open_for_recipient(
    &self,
    recipient: &str,
    limit: i64,
) -> Result<Vec<DeliveryOutbox>, ValidationError>
```

Open (un-acked) rows addressed to `recipient`, ordered by id for deterministic replay (relay drain / reconnect resync).

<details>
<summary>Source</summary>

```rust
    pub async fn list_open_for_recipient(
        &self,
        recipient: &str,
        limit: i64,
    ) -> Result<Vec<DeliveryOutbox>, ValidationError> {
        let recipient = recipient.to_string();
        let rows: Vec<UnifiedDeliveryOutbox> = crate::interact_on_backend!(self.dal, |conn| {
            delivery_outbox::table
                .filter(delivery_outbox::recipient.eq(recipient))
                .filter(delivery_outbox::delivery_state.ne(STATE_ACKED))
                .order(delivery_outbox::id.asc())
                .limit(limit)
                .load(conn)
        })?;
        Ok(rows.into_iter().map(to_domain).collect())
    }
```

</details>

##### <span id="DeliveryOutboxDAL.list_pending"></span><code>list_​pending</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_pending(
    &self,
    limit: i64,
) -> Result<Vec<DeliveryOutbox>, ValidationError>
```

Rows in the `pending` state across all recipients, oldest first — the relay's drain query (T-0626). Delivered-but-unacked rows are excluded (they await ack or sweeper reclaim, not (re)delivery on a fresh wake).

<details>
<summary>Source</summary>

```rust
    pub async fn list_pending(&self, limit: i64) -> Result<Vec<DeliveryOutbox>, ValidationError> {
        let rows: Vec<UnifiedDeliveryOutbox> = crate::interact_on_backend!(self.dal, |conn| {
            delivery_outbox::table
                .filter(delivery_outbox::delivery_state.eq(STATE_PENDING))
                .order(delivery_outbox::id.asc())
                .limit(limit)
                .load(conn)
        })?;
        Ok(rows.into_iter().map(to_domain).collect())
    }
```

</details>

##### <span id="DeliveryOutboxDAL.list_stuck"></span><code>list_​stuck</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_stuck(
    &self,
    cutoff: UniversalTimestamp,
    limit: i64,
) -> Result<Vec<DeliveryOutbox>, ValidationError>
```

Open rows enqueued before `cutoff`, oldest first — the safety-net sweeper's stuck-row scan (T-0628).

<details>
<summary>Source</summary>

```rust
    pub async fn list_stuck(
        &self,
        cutoff: UniversalTimestamp,
        limit: i64,
    ) -> Result<Vec<DeliveryOutbox>, ValidationError> {
        let rows: Vec<UnifiedDeliveryOutbox> = crate::interact_on_backend!(self.dal, |conn| {
            delivery_outbox::table
                .filter(delivery_outbox::delivery_state.ne(STATE_ACKED))
                .filter(delivery_outbox::created_at.lt(cutoff))
                .order(delivery_outbox::created_at.asc())
                .limit(limit)
                .load(conn)
        })?;
        Ok(rows.into_iter().map(to_domain).collect())
    }
```

</details>

##### <span id="DeliveryOutboxDAL.count_open"></span><code>count_​open</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn count_open(&self) -> Result<i64, ValidationError>
```

Count of open (un-acked) rows — outbox-depth signal for monitoring (T-0628).

<details>
<summary>Source</summary>

```rust
    pub async fn count_open(&self) -> Result<i64, ValidationError> {
        let count: i64 = crate::interact_on_backend!(self.dal, |conn| {
            delivery_outbox::table
                .filter(delivery_outbox::delivery_state.ne(STATE_ACKED))
                .count()
                .get_result(conn)
        })?;
        Ok(count)
    }
```

</details>
