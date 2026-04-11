# cloacina::execution_planner::stale_claim_sweeper <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Stale claim sweeper — background service for expired claim recovery.

Periodically scans for tasks with stale heartbeats (crashed runners),
releases their claims, and resets them to Ready for re-execution.

## Structs

### `cloacina::execution_planner::stale_claim_sweeper::StaleClaimSweeperConfig`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Configuration for the stale claim sweeper.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `sweep_interval` | `Duration` | How often to run the sweep (default 30s). |
| `stale_threshold` | `Duration` | How old a heartbeat must be to consider the claim stale (default 60s).
Must be greater than the heartbeat interval. |



### `cloacina::execution_planner::stale_claim_sweeper::StaleClaimSweeper`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Background service that sweeps for stale task claims.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `Arc < DAL >` |  |
| `config` | `StaleClaimSweeperConfig` |  |
| `shutdown_rx` | `watch :: Receiver < bool >` |  |
| `ready_at` | `Instant` | When the sweeper became ready. Used for the startup grace period. |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (dal : Arc < DAL > , config : StaleClaimSweeperConfig , shutdown_rx : watch :: Receiver < bool > ,) -> Self
```

Create a new stale claim sweeper.

<details>
<summary>Source</summary>

```rust
    pub fn new(
        dal: Arc<DAL>,
        config: StaleClaimSweeperConfig,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Self {
        Self {
            dal,
            config,
            shutdown_rx,
            ready_at: Instant::now(),
        }
    }
```

</details>



##### `run` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn run (& mut self)
```

Run the sweep loop.

<details>
<summary>Source</summary>

```rust
    pub async fn run(&mut self) {
        info!(
            "Starting stale claim sweeper (interval: {}s, threshold: {}s, grace period: {}s)",
            self.config.sweep_interval.as_secs(),
            self.config.stale_threshold.as_secs(),
            self.config.stale_threshold.as_secs(),
        );

        let mut interval = tokio::time::interval(self.config.sweep_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.sweep().await;
                }
                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        info!("Stale claim sweeper shutting down");
                        break;
                    }
                }
            }
        }
    }
```

</details>



##### `sweep` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn sweep (& self)
```

Perform a single sweep pass.

<details>
<summary>Source</summary>

```rust
    pub async fn sweep(&self) {
        // Startup grace period: don't sweep until we've been running for
        // at least one full stale_threshold duration. This prevents false
        // positives when the scheduler restarts — tasks that were being
        // executed by healthy runners look "stale" simply because the
        // sweeper wasn't running to see their heartbeats.
        let uptime = self.ready_at.elapsed();
        if uptime < self.config.stale_threshold {
            debug!(
                "Stale claim sweeper in grace period ({:.1}s / {}s) — skipping sweep",
                uptime.as_secs_f64(),
                self.config.stale_threshold.as_secs()
            );
            return;
        }

        // Find tasks with stale heartbeats
        let stale_claims = match self
            .dal
            .task_execution()
            .find_stale_claims(self.config.stale_threshold)
            .await
        {
            Ok(claims) => claims,
            Err(e) => {
                warn!("Stale claim sweep failed: {}", e);
                return;
            }
        };

        if stale_claims.is_empty() {
            debug!("Stale claim sweep: no stale claims found");
            return;
        }

        info!(
            "Stale claim sweep found {} stale claims",
            stale_claims.len()
        );

        for claim in &stale_claims {
            let age = chrono::Utc::now() - claim.heartbeat_at;

            // Release the claim
            if let Err(e) = self
                .dal
                .task_execution()
                .release_runner_claim(claim.task_id)
                .await
            {
                warn!(
                    "Failed to release stale claim on task {}: {}",
                    claim.task_id, e
                );
                continue;
            }

            // Reset task status to Ready for re-execution
            if let Err(e) = self.dal.task_execution().mark_ready(claim.task_id).await {
                warn!(
                    "Failed to reset task {} to Ready after stale claim release: {}",
                    claim.task_id, e
                );
                continue;
            }

            info!(
                "Released stale claim: task {} (runner {}, last heartbeat {}s ago)",
                claim.task_id,
                claim.claimed_by,
                age.num_seconds()
            );
        }

        info!(
            "Stale claim sweep complete: {} claims released",
            stale_claims.len()
        );
    }
```

</details>
