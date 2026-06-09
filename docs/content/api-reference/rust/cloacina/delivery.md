# cloacina::delivery <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Delivery relay for the interservice communication substrate (spec CLOACI-S-0012, ADR CLOACI-A-0006, task T-0626).

The relay turns the durable [`crate::dal::unified::delivery_outbox`] into an
event-driven push: when woken, it drains `pending` rows, hands each to a
[`DeliverySink`], and marks delivered ones. Two wake sources feed it:
- **In-process** ([`WakeHandle`]): the producer signals its own replica's
relay immediately after enqueue — no DB round-trip.
- **Cross-replica** (`LISTEN`/`NOTIFY`, Postgres): a `tokio-postgres`
connection LISTENing on the `delivery_outbox` channel forwards each
notification to a [`WakeHandle`]. Wired in increment 2 of T-0626; the
NOTIFY side already fires via the `delivery_outbox_notify` trigger.
There is **no steady-state polling**: the relay blocks on its wake signal.
The safety-net sweeper (T-0628) is the only periodic scan, and exists purely
to backstop a missed NOTIFY or a crash — not as the delivery path.
Postgres-only at runtime; the drain loop is backend-agnostic so it is
unit-tested on SQLite.

## Structs

### `cloacina::delivery::WakeHandle`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

A cloneable handle producers (and the LISTEN task) use to wake the relay.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `notify` | `Arc < Notify >` |  |

#### Methods

##### `wake` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn wake (& self)
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





### `cloacina::delivery::DeliveryRelay`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Drains the delivery outbox on demand and pushes rows to a [`DeliverySink`].

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `DAL` |  |
| `sink` | `Arc < dyn DeliverySink >` |  |
| `notify` | `Arc < Notify >` |  |
| `drain_batch` | `i64` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (dal : DAL , sink : Arc < dyn DeliverySink >) -> Self
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



##### `with_drain_batch` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_drain_batch (mut self , batch : i64) -> Self
```

Overrides the per-wake drain batch size (defaults to [`DEFAULT_DRAIN_BATCH`]).

<details>
<summary>Source</summary>

```rust
    pub fn with_drain_batch(mut self, batch: i64) -> Self {
        self.drain_batch = batch;
        self
    }
```

</details>



##### `wake_handle` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn wake_handle (& self) -> WakeHandle
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



##### `drain_once` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn drain_once (& self) -> Result < usize , crate :: error :: ValidationError >
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



##### `run` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn run (self , mut shutdown : watch :: Receiver < bool >)
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

### `cloacina::delivery::DeliveryError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors a [`DeliverySink`] can report. Transient by contract: a failed delivery leaves the row `pending` for the next wake or the sweeper.

#### Variants

- **`Sink`**



### `cloacina::delivery::DeliveryOutcome` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Outcome of handing a row to a sink.

#### Variants

- **`Delivered`** - Pushed to the recipient; the relay marks the row `delivered`.
- **`NoRoute`** - The recipient is not reachable from this replica right now (no owned
connection). The row stays `pending`; another replica's relay (woken by
NOTIFY) or the sweeper will pick it up. This is how connection-ownership
routing falls out without the relay needing a roster.



## Functions

### `cloacina::delivery::run_pg_listener`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn run_pg_listener (conn_str : String , channel : String , wake : WakeHandle , mut shutdown : watch :: Receiver < bool > ,)
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



### `cloacina::delivery::listen_once`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
async fn listen_once (conn_str : & str , channel : & str , wake : & WakeHandle , shutdown : & mut watch :: Receiver < bool > ,) -> Result < () , DeliveryError >
```

One LISTEN session. Returns `Ok(())` only on requested shutdown; any connection loss returns `Err` so the caller reconnects.

<details>
<summary>Source</summary>

```rust
async fn listen_once(
    conn_str: &str,
    channel: &str,
    wake: &WakeHandle,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<(), DeliveryError> {
    use futures::StreamExt;
    use std::pin::Pin;

    let (client, mut connection) = tokio_postgres::connect(conn_str, tokio_postgres::NoTls)
        .await
        .map_err(|e| DeliveryError::Sink(format!("connect: {e}")))?;

    // The connection's async-message stream surfaces notifications; forward each
    // as a unit wake-signal over a channel so the main loop can also watch shutdown.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let driver = tokio::spawn(async move {
        let mut stream =
            futures::stream::poll_fn(move |cx| Pin::new(&mut connection).poll_message(cx));
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(tokio_postgres::AsyncMessage::Notification(_)) => {
                    if tx.send(()).is_err() {
                        break;
                    }
                }
                Ok(_) => {}      // notices, etc. — ignored
                Err(_) => break, // connection error → end driver → main loop reconnects
            }
        }
    });

    client
        .batch_execute(&format!("LISTEN {channel}"))
        .await
        .map_err(|e| DeliveryError::Sink(format!("LISTEN: {e}")))?;

    // Catch-up wake for anything enqueued before this session was listening.
    wake.wake();

    let outcome = loop {
        tokio::select! {
            maybe = rx.recv() => match maybe {
                Some(()) => wake.wake(),
                // Driver ended → connection dropped. Surface as error to reconnect.
                None => break Err(DeliveryError::Sink("LISTEN connection closed".to_string())),
            },
            res = shutdown.changed() => {
                // Either an explicit shutdown signal or the sender was dropped
                // (process exit). Both mean "stop the LISTEN cleanly".
                if res.is_err() || *shutdown.borrow() {
                    break Ok(());
                }
            }
        }
    };

    driver.abort();
    drop(client); // hold the client (and thus the LISTEN) until here
    outcome
}
```

</details>
