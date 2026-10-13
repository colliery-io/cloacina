---
title: "cloacina::executor::result_handler"
description: "Shared post-execution result handler (CLOACI-I-0114 / task T-0630)."
editUrl: false
sidebar:
  order: 121
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Shared post-execution result handler (CLOACI-I-0114 / task T-0630).

Extracted from [`super::ThreadTaskExecutor`](../../../../rust/cloacina/executor/thread_task_executor/#struct.ThreadTaskExecutor)'s post-closure block so the
upcoming `FleetExecutor` (T-0633) can reconcile agent-reported results
through the **same** state-write sequence the thread executor uses.
Without this seam the two executors would inevitably drift on status
transitions, retry decisions, context persistence, and metric increments —
and "did this task get the right state?" would depend on *where* it ran.

The handler is intentionally independent of *where* the task closure ran:
it takes a `Result<Context, ExecutorError>` (the outcome) plus identity +
retry policy + elapsed duration, and applies all persistent-state effects.

## Structs

### <span id="struct.TaskResultHandler"></span><code style="font-size: 0.85em">struct TaskResultHandler</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Post-execution handler — applies all state writes / retry decisions / metric increments for one task. Cloneable so executors can hold one per instance and pass clones into spawned work where needed.

#### Methods

##### <span id="TaskResultHandler.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    dal: DAL,
    total_executed: Arc<AtomicU64>,
    total_failed: Arc<AtomicU64>,
    runner_id: Option<UniversalUuid>,
) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(
        dal: DAL,
        total_executed: Arc<AtomicU64>,
        total_failed: Arc<AtomicU64>,
        runner_id: Option<UniversalUuid>,
    ) -> Self {
        Self {
            dal,
            total_executed,
            total_failed,
            runner_id,
        }
    }
```

</details>

##### <span id="TaskResultHandler.handle_outcome"></span><code>handle_​outcome</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn handle_outcome(
    &self,
    event: &TaskReadyEvent,
    claimed_task: &ClaimedTask,
    outcome: Result<Context<serde_json::Value>, ExecutorError>,
    retry_policy: &RetryPolicy,
    duration: Duration,
) -> ExecutionResult
```

Apply the post-execution state machine for one task and return the dispatcher-shaped outcome.

- `event` is the original [`TaskReadyEvent`](../../../../rust/cloacina/dispatcher/types/#struct.TaskReadyEvent) (used for log/metric ids).
- `claimed_task` is the identity the helpers below operate on.
- `outcome` is `Ok(produced_context)` on success or `Err(executor_error)`
  on failure.
- `retry_policy` is consulted only on the error branch.
- `duration` is the wall-clock time the task closure consumed; used
  for logs and embedded into the returned `ExecutionResult`.

<details>
<summary>Source</summary>

```rust
    pub async fn handle_outcome(
        &self,
        event: &TaskReadyEvent,
        claimed_task: &ClaimedTask,
        outcome: Result<Context<serde_json::Value>, ExecutorError>,
        retry_policy: &RetryPolicy,
        duration: Duration,
    ) -> ExecutionResult {
        match outcome {
            Ok(result_context) => {
                // CLOACI-I-0140: retry transient DB contention — a sqlite-busy
                // context save was failing tasks that SUCCEEDED (the reproduced
                // 'Failed' == 'Completed' assertion flake class).
                match retry_transient(5, Duration::from_millis(100), || {
                    self.complete_task_transaction(claimed_task, result_context.clone_data())
                })
                .await
                {
                    Ok(()) => {
                        self.total_executed.fetch_add(1, Ordering::SeqCst);
                        info!(
                            task_id = %event.task_execution_id,
                            task_name = %event.task_name,
                            duration_ms = duration.as_millis(),
                            "Task executed successfully via dispatcher"
                        );
                        ExecutionResult::success(event.task_execution_id, duration)
                    }
                    Err(e) => {
                        self.total_failed.fetch_add(1, Ordering::SeqCst);
                        let error_msg = format!("Failed to save context: {}", e);
                        // Mark failed in DB — executor owns all state transitions.
                        // CLOACI-I-0140: retried + NEVER silently swallowed — a
                        // dropped mark_failed leaves the task Running forever and
                        // hangs the workflow (the reproduced hang class).
                        if let Err(mark_err) =
                            retry_transient(5, Duration::from_millis(100), || async {
                                self.dal
                                    .task_execution()
                                    .mark_failed(
                                        event.task_execution_id,
                                        &error_msg,
                                        self.runner_id,
                                    )
                                    .await
                            })
                            .await
                        {
                            error!(
                                task_id = %event.task_execution_id,
                                error = %mark_err,
                                "mark_failed FAILED after retries — task row stays Running until \
                                 the stale-claim sweeper recovers it; workflow completion is delayed"
                            );
                        }
                        ExecutionResult::failure(event.task_execution_id, error_msg, duration)
                    }
                }
            }
            Err(error) => {
                let should_retry = self
                    .should_retry_task(claimed_task, &error, retry_policy)
                    .await
                    .unwrap_or(false);

                if should_retry {
                    // CLOACI-I-0140: schedule_retry is a terminal state write too —
                    // dropping it on sqlite-busy left the task Running forever (the
                    // scenario-33 hang the first fix round missed). Retry it, and if
                    // it STILL fails, fail the task rather than leave it in limbo.
                    match retry_transient(5, Duration::from_millis(100), || {
                        self.schedule_task_retry(claimed_task, retry_policy)
                    })
                    .await
                    {
                        Ok(()) => {
                            self.total_executed.fetch_add(1, Ordering::SeqCst);
                            ExecutionResult::retry(
                                event.task_execution_id,
                                error.to_string(),
                                duration,
                            )
                        }
                        Err(sched_err) => {
                            warn!(
                                task_id = %event.task_execution_id,
                                error = %sched_err,
                                "Failed to schedule retry after retries — failing task \
                                 instead of leaving it Running"
                            );
                            self.total_failed.fetch_add(1, Ordering::SeqCst);
                            let error_str =
                                format!("{} (retry scheduling failed: {})", error, sched_err);
                            if let Err(mark_err) =
                                retry_transient(5, Duration::from_millis(100), || async {
                                    self.dal
                                        .task_execution()
                                        .mark_failed(
                                            event.task_execution_id,
                                            &error_str,
                                            self.runner_id,
                                        )
                                        .await
                                })
                                .await
                            {
                                error!(
                                    task_id = %event.task_execution_id,
                                    error = %mark_err,
                                    "mark_failed FAILED after retries — task row stays Running \
                                     until the stale-claim sweeper recovers it; workflow \
                                     completion is delayed"
                                );
                            }
                            ExecutionResult::failure(event.task_execution_id, error_str, duration)
                        }
                    }
                } else {
                    self.total_failed.fetch_add(1, Ordering::SeqCst);
                    // Mark failed in DB — executor owns all state transitions.
                    // CLOACI-I-0140: retried + never swallowed (see above) — THIS
                    // was the primary hang site: the callback scenarios' tasks fail
                    // by design, and a sqlite-busy here left them Running forever.
                    let error_str = error.to_string();
                    if let Err(mark_err) =
                        retry_transient(5, Duration::from_millis(100), || async {
                            self.dal
                                .task_execution()
                                .mark_failed(event.task_execution_id, &error_str, self.runner_id)
                                .await
                        })
                        .await
                    {
                        error!(
                            task_id = %event.task_execution_id,
                            error = %mark_err,
                            "mark_failed FAILED after retries — task row stays Running until \
                             the stale-claim sweeper recovers it; workflow completion is delayed"
                        );
                    }
                    ExecutionResult::failure(event.task_execution_id, error_str, duration)
                }
            }
        }
    }
```

</details>

##### <span id="TaskResultHandler.is_transient_error"></span><code>is_​transient_​error</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_transient_error(&self, error: &ExecutorError) -> bool
```

Classifies whether an error looks transient enough to retry under `RetryCondition::TransientOnly`. Public so the unit tests that pin the classification (moved here from ThreadTaskExecutor in T-0630) can exercise it directly.

<details>
<summary>Source</summary>

```rust
    pub fn is_transient_error(&self, error: &ExecutorError) -> bool {
        match error {
            ExecutorError::TaskTimeout => true,
            ExecutorError::Database(_) => true,
            ExecutorError::ConnectionPool(_) => true,
            ExecutorError::TaskNotFound(_) => false,
            ExecutorError::TaskExecution(task_error) => {
                let error_msg = task_error.to_string().to_lowercase();
                error_msg.contains("timeout")
                    || error_msg.contains("connection")
                    || error_msg.contains("network")
                    || error_msg.contains("temporary")
                    || error_msg.contains("unavailable")
            }
            _ => false,
        }
    }
```

</details>
