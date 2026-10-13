---
title: "cloacina::cron_recovery"
description: "Cron execution recovery service for handling lost executions."
editUrl: false
sidebar:
  order: 56
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Cron execution recovery service for handling lost executions.

This module provides a recovery mechanism that detects and retries cron executions
that were claimed but never successfully handed off to the workflow executor.
It implements the recovery side of the guaranteed execution pattern.

**Architecture**

The recovery service runs as a background task that periodically:
1. Queries for lost executions (claimed but no workflow execution)
2. Determines if recovery is appropriate
3. Retries the execution through the workflow executor
4. Updates audit records to reflect recovery attempts

**Recovery Policy**

Executions are considered "lost" if:
- They have a schedule_executions record (were claimed)
- They are NOT linked to a live workflow execution (the scheduler links
  the audit row to the workflow execution at handoff — CLOACI-T-0914)
- They were claimed more than X minutes ago (configurable)

A row linked to a workflow execution in a non-terminal state is a
legitimately running workflow, not a lost handoff — it is skipped. A row
linked to a terminal execution that was never marked complete only has its
completion accounting backfilled; it is never re-fired.

Recovery is skipped if:
- The schedule is disabled
- The schedule has been deleted
- Too many recovery attempts have been made
- The execution is too old (beyond recovery window)
- Another recovery service already owns the handoff

**Ownership and accounting (CLOACI-T-0926)**

Both live on the `schedule_executions` row, not in this process:

* `recovery_attempts` is the attempt cap. It used to be an in-process
  `HashMap`, so a schedule that reliably failed recovery got a fresh budget
  on every restart.
* `recovery_claimed_by` / `recovery_heartbeat_at` are a compare-and-set
  claim. `find_lost_executions` is a plain SELECT, so every replica sees
  the same lost rows; the claim is what makes exactly one of them re-fire.
  Because a re-fire blocks for the workflow's whole (unbounded) duration,
  the owner beats the claim while it works instead of relying on a fixed
  expiry window — a stale beat is then a true death signal, so a crashed
  recovery service's claim is taken over rather than held forever.

## Structs

### <span id="struct.CronRecoveryConfig"></span><code style="font-size: 0.85em">struct CronRecoveryConfig</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Configuration for the cron recovery service.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `check_interval` | `Duration` | How often to check for lost executions |
| `lost_threshold_minutes` | `i32` | Consider executions lost if claimed more than this many minutes ago |
| `max_recovery_age` | `Duration` | Maximum age of executions to recover (older ones are abandoned) |
| `max_recovery_attempts` | `usize` | Maximum number of recovery attempts per execution |
| `recover_disabled_schedules` | `bool` | Whether to recover executions for disabled schedules |
| `claim_heartbeat_interval` | `Duration` | How often the owner of a recovery claim refreshes its heartbeat while the re-fire is in flight (CLOACI-T-0926). Must be comfortably smaller than `claim_stale_after`. |
| `claim_stale_after` | `Duration` | How long a recovery claim may go without a heartbeat before another recovery service may take it over (CLOACI-T-0926). |

### <span id="struct.CronRecoveryService"></span><code style="font-size: 0.85em">struct CronRecoveryService</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Recovery service for lost cron executions.

This service implements the recovery side of the guaranteed execution pattern,
detecting executions that were claimed but never handed off and retrying them.

#### Methods

##### <span id="CronRecoveryService.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    dal: Arc<DAL>,
    executor: Arc<dyn WorkflowExecutor>,
    config: CronRecoveryConfig,
    shutdown: watch::Receiver<bool>,
) -> Self
```

Creates a new cron recovery service.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `dal` | `Arc<DAL>` | Data access layer for database operations |
| `executor` | `Arc<dyn WorkflowExecutor>` | Workflow executor for retrying executions |
| `config` | `CronRecoveryConfig` | Recovery service configuration |
| `shutdown` | `watch::Receiver<bool>` | Shutdown signal receiver |

<details>
<summary>Source</summary>

```rust
    pub fn new(
        dal: Arc<DAL>,
        executor: Arc<dyn WorkflowExecutor>,
        config: CronRecoveryConfig,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            dal,
            executor,
            config,
            shutdown,
            owner_id: UniversalUuid::new_v4(),
        }
    }
```

</details>

##### <span id="CronRecoveryService.with_defaults"></span><code>with_​defaults</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_defaults(
    dal: Arc<DAL>,
    executor: Arc<dyn WorkflowExecutor>,
    shutdown: watch::Receiver<bool>,
) -> Self
```

Creates a new recovery service with default configuration.

<details>
<summary>Source</summary>

```rust
    pub fn with_defaults(
        dal: Arc<DAL>,
        executor: Arc<dyn WorkflowExecutor>,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self::new(dal, executor, CronRecoveryConfig::default(), shutdown)
    }
```

</details>

##### <span id="CronRecoveryService.run_recovery_loop"></span><code>run_​recovery_​loop</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn run_recovery_loop(
    &mut self) -> Result<(,
) -> Result<(), WorkflowExecutionError>
```

Runs the recovery service loop.

This method starts an infinite loop that periodically checks for and
recovers lost executions until a shutdown signal is received.

<details>
<summary>Source</summary>

```rust
    pub async fn run_recovery_loop(&mut self) -> Result<(), WorkflowExecutionError> {
        info!(
            "Starting cron recovery service (interval: {:?}, threshold: {} minutes)",
            self.config.check_interval, self.config.lost_threshold_minutes
        );

        let mut interval = tokio::time::interval(self.config.check_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.check_and_recover_lost_executions().await {
                        error!("Error in cron recovery service: {}", e);
                        // Continue running despite errors
                    }
                }
                _ = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        info!("Cron recovery service received shutdown signal");
                        break;
                    }
                }
            }
        }

        info!("Cron recovery service stopped");
        Ok(())
    }
```

</details>

##### <span id="CronRecoveryService.recover_lost_executions_once"></span><code>recover_​lost_​executions_​once</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn recover_lost_executions_once(
    &self) -> Result<(,
) -> Result<(), WorkflowExecutionError>
```

Runs one recovery pass over currently-lost executions.

Exposed so integration tests (and ad-hoc operator scripts) can drive
the recovery logic deterministically without waiting on the background
`check_interval` tick (CLOACI-T-0914).

<details>
<summary>Source</summary>

```rust
    pub async fn recover_lost_executions_once(&self) -> Result<(), WorkflowExecutionError> {
        self.check_and_recover_lost_executions().await
    }
```

</details>

##### <span id="CronRecoveryService.clear_recovery_attempts"></span><code>clear_​recovery_​attempts</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn clear_recovery_attempts(
    &self,
    execution_id: UniversalUuid,
) -> Result<(), crate::error::ValidationError>
```

Clears the durable recovery attempt count for one execution.

Useful for testing, or for an operator who wants to give a previously
abandoned handoff a fresh budget. CLOACI-T-0926 moved the count from
process memory onto the audit row, so this now takes the execution to
reset rather than wiping a process-local cache.

<details>
<summary>Source</summary>

```rust
    pub async fn clear_recovery_attempts(
        &self,
        execution_id: UniversalUuid,
    ) -> Result<(), crate::error::ValidationError> {
        self.dal
            .schedule_execution()
            .reset_recovery_attempts(execution_id)
            .await?;
        info!("Cleared recovery attempts for execution {}", execution_id);
        Ok(())
    }
```

</details>

##### <span id="CronRecoveryService.get_recovery_attempts"></span><code>get_​recovery_​attempts</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_recovery_attempts(
    &self,
    execution_id: UniversalUuid,
) -> usize
```

Gets the durable recovery attempt count for an execution.

Returns 0 if the row cannot be read; callers use this for reporting,
never as the cap check (the cap reads the value inside the same
claimed critical section that increments it).

<details>
<summary>Source</summary>

```rust
    pub async fn get_recovery_attempts(&self, execution_id: UniversalUuid) -> usize {
        match self.dal.schedule_execution().get_by_id(execution_id).await {
            Ok(row) => row.recovery_attempts.max(0) as usize,
            Err(e) => {
                warn!(
                    "Failed to read recovery attempts for execution {}: {}",
                    execution_id, e
                );
                0
            }
        }
    }
```

</details>
