# cloacina::delivery::sweeper <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Safety-net sweeper for the delivery substrate (CLOACI-I-0115 / S-0012 / A-0006, task T-0628).

The sweeper is **what makes the substrate at-least-once.** The relay
(T-0626) drains and pushes on every wake, and the WS handler (T-0627)
resets stuck rows back to `pending` on reconnect — but a recipient that
never reconnects, a NOTIFY lost during a LISTEN reconnect, or a replica
that crashed between commit and ack would otherwise leave rows stranded.
The sweeper periodically scans the outbox for rows that have been open
past a threshold and pushes them back through the relay path.

## Structs

### `cloacina::delivery::sweeper::SweeperConfig`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Tunables for the sweeper.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `sweep_interval` | `Duration` | How often the sweeper wakes to scan. |
| `stuck_threshold` | `Duration` | Rows whose `created_at` is older than `now - stuck_threshold` are
considered stuck and eligible for redelivery. |
| `batch_limit` | `i64` | Maximum rows examined per sweep, keeping a single tick bounded. |



### `cloacina::delivery::sweeper::DeliverySweeper`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Periodic backstop scan over the delivery outbox.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `DAL` |  |
| `wake` | `WakeHandle` |  |
| `config` | `SweeperConfig` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (dal : DAL , wake : WakeHandle) -> Self
```

Construct with [`SweeperConfig::default`].

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: DAL, wake: WakeHandle) -> Self {
        Self::with_config(dal, wake, SweeperConfig::default())
    }
```

</details>



##### `with_config` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_config (dal : DAL , wake : WakeHandle , config : SweeperConfig) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn with_config(dal: DAL, wake: WakeHandle, config: SweeperConfig) -> Self {
        Self { dal, wake, config }
    }
```

</details>



##### `run` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn run (self , mut shutdown : watch :: Receiver < bool >)
```

Run until `shutdown` flips to `true` (or the sender drops).

<details>
<summary>Source</summary>

```rust
    pub async fn run(self, mut shutdown: watch::Receiver<bool>) {
        let mut ticker = tokio::time::interval(self.config.sweep_interval);
        // The first tick fires immediately; we want to wait one interval so
        // startup doesn't race the relay's own catch-up drain.
        ticker.tick().await;
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match self.sweep_once().await {
                        Ok(reset) => debug!(reset, "delivery sweep complete"),
                        Err(e) => warn!(error = %e, "delivery sweep failed"),
                    }
                }
                res = shutdown.changed() => {
                    if res.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
            }
        }
        debug!("delivery sweeper stopped");
    }
```

</details>



##### `sweep_once` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn sweep_once (& self) -> Result < usize , ValidationError >
```

One sweep pass. Returns the number of rows reset (`delivered → pending`). Pending-past-threshold rows are left alone — a wake at the end of the sweep tells the relay to retry them via its normal drain path.

<details>
<summary>Source</summary>

```rust
    pub async fn sweep_once(&self) -> Result<usize, ValidationError> {
        let cutoff_chrono = chrono::Utc::now()
            - chrono::Duration::from_std(self.config.stuck_threshold)
                .unwrap_or_else(|_| chrono::Duration::seconds(60));
        let cutoff = UniversalTimestamp(cutoff_chrono);

        let stuck = self
            .dal
            .delivery_outbox()
            .list_stuck(cutoff, self.config.batch_limit)
            .await?;

        let mut reset = 0usize;
        for row in &stuck {
            // Delivered-past-threshold rows = pushed but not acked. Reset to
            // pending so the relay re-pushes (idempotent for the recipient per
            // the envelope contract). Pending-past-threshold rows are left as-is
            // — the wake below tells the relay to re-drain.
            if row.state() == Some(DeliveryState::Delivered) {
                match self.dal.delivery_outbox().reset_to_pending(row.id).await {
                    Ok(()) => reset += 1,
                    // Race with another sweeper or with an ack: row already
                    // moved out of `delivered`. Safe to ignore.
                    Err(e) => debug!(
                        id = row.id,
                        error = %e,
                        "sweep reset skipped (state changed concurrently)"
                    ),
                }
            }
        }

        // One wake per non-empty sweep coalesces multiple redeliveries into a
        // single drain pass (relay's Notify holds at most one permit).
        if !stuck.is_empty() {
            self.wake.wake();
        }

        metrics::counter!("cloacina_delivery_outbox_sweep_runs_total").increment(1);
        if reset > 0 {
            metrics::counter!("cloacina_delivery_outbox_sweep_redeliveries_total")
                .increment(reset as u64);
        }
        match self.dal.delivery_outbox().count_open().await {
            Ok(n) => metrics::gauge!("cloacina_delivery_outbox_open").set(n as f64),
            Err(e) => debug!(error = %e, "count_open failed during sweep"),
        }

        Ok(reset)
    }
```

</details>
