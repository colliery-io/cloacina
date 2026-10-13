---
title: "cloacina::execution_planner::stale_claim_sweeper"
description: "Stale claim sweeper — background service for expired claim recovery."
editUrl: false
sidebar:
  order: 114
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Stale claim sweeper — background service for expired claim recovery.

Periodically scans for tasks with stale heartbeats (crashed runners),
releases their claims, and resets them to Ready for re-execution.

**Startup Grace Period**

In a distributed system, the scheduler may restart while runners are
still executing tasks. The sweeper records when it became ready and
ignores all claims during its first `stale_threshold` duration after
startup. This prevents false positives where tasks look stale simply
because the sweeper wasn't running to observe their heartbeats.

## Structs

### <span id="struct.StaleClaimSweeperConfig"></span><code style="font-size: 0.85em">struct StaleClaimSweeperConfig</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Configuration for the stale claim sweeper.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `sweep_interval` | `Duration` | How often to run the sweep (default 30s). |
| `stale_threshold` | `Duration` | How old a heartbeat must be to consider the claim stale (default 60s). Must be greater than the heartbeat interval. |
| `max_recovery_attempts` | `i32` | How many stale-claim recoveries a single task gets before it is abandoned (marked `Failed` with an `ABANDONED:` error) instead of re-Readied (default 3, mirroring `cron_max_recovery_attempts`). Without a cap, a task that reliably kills its runner recycles forever (CLOACI-T-0914). |

### <span id="struct.StaleClaimSweeper"></span><code style="font-size: 0.85em">struct StaleClaimSweeper</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Background service that sweeps for stale task claims.

#### Methods

##### <span id="StaleClaimSweeper.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    dal: Arc<DAL>,
    config: StaleClaimSweeperConfig,
    shutdown_rx: watch::Receiver<bool>,
) -> Self
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

##### <span id="StaleClaimSweeper.run"></span><code>run</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn run(&mut self)
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

##### <span id="StaleClaimSweeper.sweep"></span><code>sweep</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn sweep(&self)
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

            // Recovery cap (CLOACI-T-0914): a task that has already burned
            // its recovery budget is abandoned — `mark_abandoned` writes the
            // terminal `Failed` status with an `ABANDONED:` error prefix and
            // a TaskAbandoned audit event in one transaction — instead of
            // being re-Readied forever. Failures here warn-and-continue: the
            // next sweep retries naturally.
            if claim.recovery_attempts >= self.config.max_recovery_attempts {
                let reason = format!(
                    "{} stale-claim recoveries exhausted (cap {}); last runner {}, heartbeat {}s stale",
                    claim.recovery_attempts,
                    self.config.max_recovery_attempts,
                    claim.claimed_by,
                    age.num_seconds()
                );
                if let Err(e) = self
                    .dal
                    .task_execution()
                    .mark_abandoned(claim.task_id, &reason)
                    .await
                {
                    warn!("Failed to abandon task {}: {}", claim.task_id, e);
                    continue;
                }
                metrics::counter!("cloacina_scheduler_tasks_abandoned_total").increment(1);
                warn!("Abandoned task {}: {}", claim.task_id, reason);
                continue;
            }

            // Reset to Ready for re-execution. `reset_task_for_recovery`
            // (not `mark_ready`) so the attempt is counted against the cap:
            // it re-Readies, clears started_at, increments recovery_attempts,
            // and stamps last_recovery_at.
            if let Err(e) = self
                .dal
                .task_execution()
                .reset_task_for_recovery(claim.task_id)
                .await
            {
                warn!(
                    "Failed to reset task {} to Ready after stale claim release: {}",
                    claim.task_id, e
                );
                continue;
            }

            metrics::counter!("cloacina_scheduler_stale_claims_swept_total").increment(1);

            info!(
                "Released stale claim: task {} (runner {}, last heartbeat {}s ago, recovery {}/{})",
                claim.task_id,
                claim.claimed_by,
                age.num_seconds(),
                claim.recovery_attempts + 1,
                self.config.max_recovery_attempts
            );
        }

        info!(
            "Stale claim sweep complete: {} claims released",
            stale_claims.len()
        );
    }
```

</details>
