---
title: "cloacina::execution_planner::scheduler_loop"
description: "Scheduler loop and workflow execution processing."
editUrl: false
sidebar:
  order: 113
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Scheduler loop and workflow execution processing.

This module contains the main scheduling loop that continuously processes
active workflow executions and manages task readiness.

## Structs

### <span id="struct.SchedulerLoop"></span><code style="font-size: 0.85em">struct SchedulerLoop&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Scheduler loop operations.

#### Methods

##### <span id="SchedulerLoop.with_dispatcher"></span><code>with_​dispatcher</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_dispatcher(
    dal: &'a DAL,
    runtime: Arc<Runtime>,
    instance_id: Uuid,
    poll_interval: Duration,
    dispatcher: Option<Arc<dyn Dispatcher>>,
) -> Self
```

Creates a new SchedulerLoop with an optional dispatcher.

<details>
<summary>Source</summary>

```rust
    pub fn with_dispatcher(
        dal: &'a DAL,
        runtime: Arc<Runtime>,
        instance_id: Uuid,
        poll_interval: Duration,
        dispatcher: Option<Arc<dyn Dispatcher>>,
    ) -> Self {
        Self {
            dal,
            runtime,
            instance_id,
            poll_interval,
            dispatcher,
            shutdown_rx: None,
            consecutive_errors: 0,
        }
    }
```

</details>

##### <span id="SchedulerLoop.with_shutdown"></span><code>with_​shutdown</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_shutdown(
    mut self,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> Self
```

Set the shutdown receiver for graceful termination.

<details>
<summary>Source</summary>

```rust
    pub fn with_shutdown(mut self, shutdown_rx: tokio::sync::watch::Receiver<bool>) -> Self {
        self.shutdown_rx = Some(shutdown_rx);
        self
    }
```

</details>

##### <span id="SchedulerLoop.run"></span><code>run</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn run(&mut self) -> Result<(), ValidationError>
```

Runs the main scheduling loop that continuously processes active workflow executions.

This loop:
1. Checks for active workflow executions
2. Updates task readiness based on dependencies and trigger rules
3. Marks completed workflow executions
4. Repeats at the configured poll interval

<details>
<summary>Source</summary>

```rust
    pub async fn run(&mut self) -> Result<(), ValidationError> {
        info!(
            "Starting task scheduler loop (instance: {}, poll_interval: {:?})",
            self.instance_id, self.poll_interval
        );
        let mut interval = time::interval(self.poll_interval);

        loop {
            if let Some(ref mut shutdown_rx) = self.shutdown_rx {
                tokio::select! {
                    _ = interval.tick() => {}
                    _ = shutdown_rx.changed() => {
                        info!("SchedulerLoop shutting down");
                        break;
                    }
                }
            } else {
                interval.tick().await;
            }

            match self.process_active_executions().await {
                Ok(_) => {
                    if self.consecutive_errors > 0 {
                        info!(
                            "Scheduler loop recovered after {} consecutive errors",
                            self.consecutive_errors
                        );
                        self.consecutive_errors = 0;
                    }
                    debug!("Scheduling loop completed successfully");
                }
                Err(e) => {
                    self.consecutive_errors += 1;

                    if self.consecutive_errors == CIRCUIT_OPEN_THRESHOLD {
                        warn!(
                            "Scheduler loop circuit open: {} consecutive errors — backing off (latest: {})",
                            self.consecutive_errors, e
                        );
                    } else if self.consecutive_errors.is_multiple_of(10) {
                        // Rate-limited logging: every 10th error after circuit opens
                        warn!(
                            "Scheduler loop still failing: {} consecutive errors (latest: {})",
                            self.consecutive_errors, e
                        );
                    } else if self.consecutive_errors < CIRCUIT_OPEN_THRESHOLD {
                        error!("Scheduling loop error: {}", e);
                    }

                    // Exponential backoff: poll_interval * 2^min(errors, 8) capped at MAX_BACKOFF
                    let backoff_exp = self.consecutive_errors.min(8);
                    let backoff = self
                        .poll_interval
                        .saturating_mul(1u32 << backoff_exp)
                        .min(MAX_BACKOFF);
                    if backoff > self.poll_interval {
                        time::sleep(backoff - self.poll_interval).await;
                    }
                }
            }
        }

        Ok(())
    }
```

</details>

##### <span id="SchedulerLoop.process_active_executions"></span><code>process_​active_​executions</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn process_active_executions(&self) -> Result<(), ValidationError>
```

Processes all active workflow executions to update task readiness.

<details>
<summary>Source</summary>

```rust
    pub async fn process_active_executions(&self) -> Result<(), ValidationError> {
        let active_executions = self
            .dal
            .workflow_execution()
            .get_active_executions()
            .await?;

        // SQL-derived gauges — re-seeded every tick so they cannot drift on
        // crash, claim loss, or any path that skips
        // finalize_workflow_execution (workflows) /
        // `ThreadTaskExecutor::execute_task` (tasks). See T-0534 + T-0589.
        metrics::gauge!("cloacina_active_workflows").set(active_executions.len() as f64);
        match self.dal.task_execution().count_running_tasks().await {
            Ok(n) => {
                metrics::gauge!("cloacina_active_tasks").set(n as f64);
            }
            Err(e) => {
                // Best-effort — leave the previous tick's value rather than
                // zeroing the gauge on a transient DB hiccup.
                warn!("failed to count running tasks for gauge re-seed: {}", e);
            }
        }

        if active_executions.is_empty() {
            // Even with no active workflow executions, dispatch any Ready tasks (e.g., retries)
            if self.dispatcher.is_some() {
                self.dispatch_ready_tasks().await?;
            }
            return Ok(());
        }

        // Batch process all active workflow executions (evaluates pending tasks, marks them Ready)
        self.process_executions_batch(active_executions).await?;

        // Dispatch all Ready tasks (including newly marked and retry tasks)
        if self.dispatcher.is_some() {
            self.dispatch_ready_tasks().await?;
        }

        Ok(())
    }
```

</details>
