# cloacina::cron_recovery <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Cron execution recovery service for handling lost executions.

This module provides a recovery mechanism that detects and retries cron executions
that were claimed but never successfully handed off to the pipeline executor.
It implements the recovery side of the guaranteed execution pattern.

## Structs

### `cloacina::cron_recovery::CronRecoveryConfig`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::cron_recovery::CronRecoveryService`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Recovery service for lost cron executions.

This service implements the recovery side of the guaranteed execution pattern,
detecting executions that were claimed but never handed off and retrying them.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `Arc < DAL >` |  |
| `executor` | `Arc < dyn WorkflowExecutor >` |  |
| `config` | `CronRecoveryConfig` |  |
| `shutdown` | `watch :: Receiver < bool >` |  |
| `recovery_attempts` | `Arc < tokio :: sync :: Mutex < HashMap < crate :: database :: UniversalUuid , usize > > >` | Tracks recovery attempts per execution ID |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (dal : Arc < DAL > , executor : Arc < dyn WorkflowExecutor > , config : CronRecoveryConfig , shutdown : watch :: Receiver < bool > ,) -> Self
```

Creates a new cron recovery service.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `dal` | `-` | Data access layer for database operations |
| `executor` | `-` | Pipeline executor for retrying executions |
| `config` | `-` | Recovery service configuration |
| `shutdown` | `-` | Shutdown signal receiver |


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
            recovery_attempts: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }
```

</details>



##### `with_defaults` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_defaults (dal : Arc < DAL > , executor : Arc < dyn WorkflowExecutor > , shutdown : watch :: Receiver < bool > ,) -> Self
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



##### `run_recovery_loop` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn run_recovery_loop (& mut self) -> Result < () , WorkflowExecutionError >
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



##### `check_and_recover_lost_executions` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn check_and_recover_lost_executions (& self) -> Result < () , WorkflowExecutionError >
```

Checks for lost executions and attempts to recover them.

<details>
<summary>Source</summary>

```rust
    async fn check_and_recover_lost_executions(&self) -> Result<(), WorkflowExecutionError> {
        debug!("Checking for lost cron executions");

        // Find lost executions
        let lost_executions = self
            .dal
            .schedule_execution()
            .find_lost_executions(self.config.lost_threshold_minutes)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to find lost executions: {}", e),
            })?;

        if lost_executions.is_empty() {
            debug!("No lost executions found");
            return Ok(());
        }

        info!("Found {} lost cron execution(s)", lost_executions.len());

        // Attempt to recover each lost execution
        for execution in lost_executions {
            if let Err(e) = self.recover_execution(&execution).await {
                error!(
                    "Failed to recover execution {} for schedule {}: {}",
                    execution.id, execution.schedule_id, e
                );
                // Continue with other executions
            }
        }

        Ok(())
    }
```

</details>



##### `recover_execution` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn recover_execution (& self , execution : & ScheduleExecution ,) -> Result < () , WorkflowExecutionError >
```

Attempts to recover a single lost execution.

<details>
<summary>Source</summary>

```rust
    async fn recover_execution(
        &self,
        execution: &ScheduleExecution,
    ) -> Result<(), WorkflowExecutionError> {
        // Use scheduled_time if available; fall back to created_at
        let scheduled_time = execution
            .scheduled_time
            .as_ref()
            .map(|t| t.0)
            .unwrap_or(execution.created_at.0);

        let execution_age = Utc::now() - scheduled_time;

        // Check if execution is too old to recover
        if execution_age > chrono::Duration::from_std(self.config.max_recovery_age).unwrap() {
            warn!(
                "Execution {} is too old to recover (age: {:?}), abandoning",
                execution.id, execution_age
            );
            return Ok(());
        }

        // Check recovery attempts
        let mut attempts = self.recovery_attempts.lock().await;
        let attempt_count = attempts.entry(execution.id).or_insert(0);
        *attempt_count += 1;

        if *attempt_count > self.config.max_recovery_attempts {
            error!(
                "Execution {} has exceeded max recovery attempts ({}), abandoning",
                execution.id, self.config.max_recovery_attempts
            );
            return Ok(());
        }

        info!(
            "Attempting recovery of execution {} (schedule: {}, attempt: {}/{})",
            execution.id, execution.schedule_id, attempt_count, self.config.max_recovery_attempts
        );

        // Get the schedule to check if it's still active
        let schedule = match self.dal.schedule().get_by_id(execution.schedule_id).await {
            Ok(sched) => sched,
            Err(e) => {
                warn!(
                    "Schedule {} not found for execution {}, skipping recovery: {}",
                    execution.schedule_id, execution.id, e
                );
                return Ok(());
            }
        };

        // Check if schedule is enabled (unless configured to recover disabled schedules)
        if !self.config.recover_disabled_schedules && !schedule.enabled.is_true() {
            info!(
                "Schedule {} is disabled, skipping recovery of execution {}",
                schedule.id, execution.id
            );
            return Ok(());
        }

        // Create recovery context
        let mut context = Context::new();

        // Add recovery metadata
        context
            .insert("is_recovery", serde_json::json!(true))
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert("recovery_attempt", serde_json::json!(attempt_count))
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert(
                "original_execution_id",
                serde_json::json!(execution.id.to_string()),
            )
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;

        // Add original scheduling metadata
        context
            .insert(
                "scheduled_time",
                serde_json::json!(scheduled_time.to_rfc3339()),
            )
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert("schedule_id", serde_json::json!(schedule.id.to_string()))
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert(
                "schedule_timezone",
                serde_json::json!(schedule.timezone.as_deref().unwrap_or("UTC")),
            )
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert(
                "schedule_expression",
                serde_json::json!(schedule.cron_expression.as_deref().unwrap_or("")),
            )
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;

        // Execute the workflow
        info!(
            "Executing recovery for workflow '{}' (execution: {}, schedule: {})",
            schedule.workflow_name, execution.id, schedule.id
        );

        match self
            .executor
            .execute(&schedule.workflow_name, context)
            .await
        {
            Ok(pipeline_result) => {
                // Update the audit record with the new pipeline execution ID
                if let Err(e) = self
                    .dal
                    .schedule_execution()
                    .update_pipeline_execution_id(
                        execution.id,
                        crate::database::UniversalUuid(pipeline_result.execution_id),
                    )
                    .await
                {
                    error!(
                        "Failed to update audit record for recovered execution {}: {}",
                        execution.id, e
                    );
                    // Continue - the recovery succeeded, just audit update failed
                }

                info!(
                    "Successfully recovered execution {} (new pipeline: {})",
                    execution.id, pipeline_result.execution_id
                );

                // Clear recovery attempts on success
                attempts.remove(&execution.id);

                Ok(())
            }
            Err(e) => {
                error!(
                    "Failed to recover execution {} for workflow '{}': {}",
                    execution.id, schedule.workflow_name, e
                );
                Err(e)
            }
        }
    }
```

</details>



##### `clear_recovery_attempts` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn clear_recovery_attempts (& self)
```

Clears the recovery attempts cache.

This can be useful for testing or when you want to retry
previously abandoned executions.

<details>
<summary>Source</summary>

```rust
    pub async fn clear_recovery_attempts(&self) {
        let mut attempts = self.recovery_attempts.lock().await;
        attempts.clear();
        info!("Cleared recovery attempts cache");
    }
```

</details>



##### `get_recovery_attempts` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn get_recovery_attempts (& self , execution_id : crate :: database :: UniversalUuid ,) -> usize
```

Gets the current recovery attempts for an execution.

<details>
<summary>Source</summary>

```rust
    pub async fn get_recovery_attempts(
        &self,
        execution_id: crate::database::UniversalUuid,
    ) -> usize {
        let attempts = self.recovery_attempts.lock().await;
        attempts.get(&execution_id).copied().unwrap_or(0)
    }
```

</details>
