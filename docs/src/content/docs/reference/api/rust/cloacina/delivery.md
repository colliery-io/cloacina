---
title: "cloacina::delivery"
description: "Delivery relay for the interservice communication substrate (spec CLOACI-S-0012, ADR CLOACI-A-0006, task T-0626)."
editUrl: false
sidebar:
  order: 103
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Delivery relay for the interservice communication substrate (spec CLOACI-S-0012, ADR CLOACI-A-0006, task T-0626).

The relay turns the durable [`crate::dal::unified::delivery_outbox`](../../../rust/cloacina/dal/unified/delivery_outbox/) into an
event-driven push: when woken, it drains `pending` rows, hands each to a
[`DeliverySink`](../../../rust/cloacina/delivery/#trait.DeliverySink), and marks delivered ones. Two wake sources feed it:

- **In-process** ([`WakeHandle`](../../../rust/cloacina/delivery/#struct.WakeHandle)): the producer signals its own replica's
  relay immediately after enqueue — no DB round-trip.
- **Cross-replica** (`LISTEN`/`NOTIFY`, Postgres): a `tokio-postgres`
  connection LISTENing on the `delivery_outbox` channel forwards each
  notification to a [`WakeHandle`](../../../rust/cloacina/delivery/#struct.WakeHandle). Wired in increment 2 of T-0626; the
  NOTIFY side already fires via the `delivery_outbox_notify` trigger.

There is **no steady-state polling**: the relay blocks on its wake signal.
The safety-net sweeper (T-0628) is the only periodic scan, and exists purely
to backstop a missed NOTIFY or a crash — not as the delivery path.

Postgres-only at runtime; the drain loop is backend-agnostic so it is
unit-tested on SQLite.

## Re-exports

- `ClientMessage` (`envelope::ClientMessage`)
- `DELIVERY_PROTOCOL_VERSION` (`envelope::DELIVERY_PROTOCOL_VERSION`)
- [`EnvelopeError`](../../../rust/cloacina/crypto/envelope/#enum.EnvelopeError)
- `ServerMessage` (`envelope::ServerMessage`)
- [`DeliverySweeper`](../../../rust/cloacina/delivery/sweeper/#struct.DeliverySweeper)
- [`SweeperConfig`](../../../rust/cloacina/delivery/sweeper/#struct.SweeperConfig)

## Structs

### <span id="struct.CompositeDeliverySink"></span><code style="font-size: 0.85em">struct CompositeDeliverySink</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Fans one outbox row out across several sinks, first successful delivery wins.

The relay takes a single sink, but a replica now has more than one kind of
recipient to serve: WebSocket subscribers and (CLOACI-T-0851) reactive
events addressed to locally-owned accumulators.

This is safe precisely because [`DeliveryOutcome::NoRoute`](../../../rust/cloacina/delivery/#enum.DeliveryOutcome) already means
"not mine, right now" rather than "failure" — every sink is expected to see
rows belonging to other subsystems and decline them. Composition therefore
needs no recipient-prefix dispatch table, which would be a second place for
addressing knowledge to live and drift out of step with the sinks.

#### Methods

##### <span id="CompositeDeliverySink.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(sinks: Vec<Arc<dyn DeliverySink>>) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(sinks: Vec<Arc<dyn DeliverySink>>) -> Self {
        Self { sinks }
    }
```

</details>

### <span id="struct.WakeHandle"></span><code style="font-size: 0.85em">struct WakeHandle</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

A cloneable handle producers (and the LISTEN task) use to wake the relay.

#### Methods

##### <span id="WakeHandle.wake"></span><code>wake</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn wake(&self)
```

Wake the relay to drain. Coalesces: many wakes between drains collapse into a single drain (which reads all pending rows anyway). A wake that arrives with no waiter is retained as one permit, so a signal racing the tail of a drain is not lost.

<details>
<summary>Source</summary>

```rust
    pub fn wake(&self) {
        self.notify.notify_one();
    }
```

</details>

### <span id="struct.DeliveryRelay"></span><code style="font-size: 0.85em">struct DeliveryRelay</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Drains the delivery outbox on demand and pushes rows to a [`DeliverySink`](../../../rust/cloacina/delivery/#trait.DeliverySink).

#### Methods

##### <span id="DeliveryRelay.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: DAL, sink: Arc<dyn DeliverySink>) -> Self
```

Creates a relay over the given DAL and sink.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: DAL, sink: Arc<dyn DeliverySink>) -> Self {
        Self {
            dal,
            sink,
            notify: Arc::new(Notify::new()),
            drain_batch: DEFAULT_DRAIN_BATCH,
        }
    }
```

</details>

##### <span id="DeliveryRelay.with_drain_batch"></span><code>with_​drain_​batch</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_drain_batch(mut self, batch: i64) -> Self
```

Overrides the per-wake drain batch size (defaults to `DEFAULT_DRAIN_BATCH`).

<details>
<summary>Source</summary>

```rust
    pub fn with_drain_batch(mut self, batch: i64) -> Self {
        self.drain_batch = batch;
        self
    }
```

</details>

##### <span id="DeliveryRelay.wake_handle"></span><code>wake_​handle</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn wake_handle(&self) -> WakeHandle
```

Returns a wake handle for producers / the LISTEN task.

<details>
<summary>Source</summary>

```rust
    pub fn wake_handle(&self) -> WakeHandle {
        WakeHandle {
            notify: self.notify.clone(),
        }
    }
```

</details>

##### <span id="DeliveryRelay.drain_once"></span><code>drain_​once</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn drain_once(
    &self,
) -> Result<usize, crate::error::ValidationError>
```

Drains one batch of `pending` rows: deliver each via the sink, mark the delivered ones. Rows the sink can't route (or that error) stay `pending`. Returns the count marked `delivered`.

<details>
<summary>Source</summary>

```rust
    pub async fn drain_once(&self) -> Result<usize, crate::error::ValidationError> {
        let rows = self
            .dal
            .delivery_outbox()
            .list_pending(self.drain_batch)
            .await?;

        let mut delivered = 0usize;
        for row in rows {
            match self.sink.deliver(&row).await {
                Ok(DeliveryOutcome::Delivered) => {
                    match self.dal.delivery_outbox().mark_delivered(row.id).await {
                        Ok(()) => delivered += 1,
                        Err(crate::error::ValidationError::InvalidStateTransition { .. }) => {
                            // Benign race: the recipient acked the row (now
                            // `acked`), a sweeper reset it, or another replica
                            // handled it between our drain read and this CAS.
                            // Either way the row has already advanced; nothing
                            // to do here.
                            debug!(id = row.id, "mark_delivered skipped — row already advanced");
                        }
                        Err(e) => warn!(
                            id = row.id,
                            error = %e,
                            "delivery_outbox: mark_delivered failed; row stays pending for retry"
                        ),
                    }
                }
                Ok(DeliveryOutcome::NoRoute) => debug!(
                    id = row.id,
                    recipient = %row.recipient,
                    "delivery_outbox: no local route; leaving pending"
                ),
                Err(e) => warn!(
                    id = row.id,
                    error = %e,
                    "delivery_outbox: sink delivery failed; leaving pending"
                ),
            }
        }
        Ok(delivered)
    }
```

</details>

##### <span id="DeliveryRelay.run"></span><code>run</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn run(self, mut shutdown: watch::Receiver<bool>)
```

Runs the relay until `shutdown` flips to `true`. Drains once on startup (catch-up for anything enqueued before the relay was listening), then drains on every wake. No periodic timer — purely event-driven.

<details>
<summary>Source</summary>

```rust
    pub async fn run(self, mut shutdown: watch::Receiver<bool>) {
        if let Err(e) = self.drain_once().await {
            error!(error = %e, "delivery_outbox: initial catch-up drain failed");
        }
        loop {
            tokio::select! {
                _ = self.notify.notified() => {
                    if let Err(e) = self.drain_once().await {
                        error!(error = %e, "delivery_outbox: drain failed");
                    }
                }
                res = shutdown.changed() => {
                    // Sender dropped or shutdown requested.
                    if res.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
            }
        }
        debug!("delivery_outbox: relay shut down");
    }
```

</details>

## Enums

### <span id="enum.DeliveryError"></span><code style="font-size: 0.85em">enum DeliveryError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors a [`DeliverySink`](../../../rust/cloacina/delivery/#trait.DeliverySink) can report. Transient by contract: a failed delivery leaves the row `pending` for the next wake or the sweeper.

#### Variants

- **`Sink`**

### <span id="enum.DeliveryOutcome"></span><code style="font-size: 0.85em">enum DeliveryOutcome</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Outcome of handing a row to a sink.

#### Variants

- **`Delivered`** - Pushed to the recipient; the relay marks the row `delivered`.
- **`NoRoute`** - The recipient is not reachable from this replica right now (no owned
  connection). The row stays `pending`; another replica's relay (woken by
  NOTIFY) or the sweeper will pick it up. This is how connection-ownership
  routing falls out without the relay needing a roster.

## Traits

### <span id="trait.DeliverySink"></span><code style="font-size: 0.85em">trait DeliverySink</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Transport that actually delivers an outbox row to its addressed recipient.

T-0626 ships only a test sink; the WebSocket sink (push over the substrate
envelope, await ack) lands in T-0627.

#### Methods

##### <span id="DeliverySink.deliver"></span><code>deliver</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn deliver(
    &self,
    row: &DeliveryOutbox,
) -> Result<DeliveryOutcome, DeliveryError>
```

<details>
<summary>Source</summary>

```rust
    async fn deliver(&self, row: &DeliveryOutbox) -> Result<DeliveryOutcome, DeliveryError>;
```

</details>

## Functions

### <span id="fn.run_pg_listener"></span><code style="font-size: 0.85em">fn run_​pg_​listener</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn run_pg_listener(
    conn_str: String,
    channel: String,
    wake: WakeHandle,
    mut shutdown: watch::Receiver<bool>,
)
```

Runs a Postgres `LISTEN` loop on `channel`, waking `wake` on every notification — the cross-replica wake of [[CLOACI-A-0006]]. Also wakes once on each successful (re)connect, so anything enqueued while disconnected is caught up. Reconnects with a fixed backoff until `shutdown` flips to `true`.

`conn_str` is a libpq-style URL plumbed from server config — `Database` does
not retain it. NoTls only in v1: the substrate targets the server's
local/in-cluster Postgres; TLS is a follow-up.

<details>
<summary>Source</summary>

```rust
pub async fn run_pg_listener(
    conn_str: String,
    channel: String,
    wake: WakeHandle,
    mut shutdown: watch::Receiver<bool>,
) {
    use std::time::Duration;
    const BACKOFF: Duration = Duration::from_secs(1);

    loop {
        if *shutdown.borrow() {
            break;
        }
        match listen_once(&conn_str, &channel, &wake, &mut shutdown).await {
            Ok(()) => break, // clean shutdown
            Err(e) => {
                warn!(error = %e, "delivery_outbox: LISTEN connection lost; reconnecting after backoff");
                tokio::select! {
                    _ = tokio::time::sleep(BACKOFF) => {}
                    _ = shutdown.changed() => {}
                }
            }
        }
    }
    debug!("delivery_outbox: LISTEN loop stopped");
}
```

</details>
