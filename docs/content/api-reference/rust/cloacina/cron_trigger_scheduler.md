# cloacina::cron_trigger_scheduler <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified scheduler for both cron and trigger-based workflow execution.

This module provides a single `Scheduler` that replaces the separate
`CronScheduler` and `TriggerScheduler`, driving both cron and trigger
schedules from one run loop backed by the unified `schedules` and
`schedule_executions` tables.

## Structs

### `cloacina::cron_trigger_scheduler::SchedulerConfig`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Configuration for the unified scheduler.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `cron_poll_interval` | `Duration` | How often to check for due cron schedules. |
| `max_catchup_executions` | `usize` | Maximum number of missed executions to run in catchup mode. |
| `max_acceptable_delay` | `Duration` | Maximum acceptable delay for cron (used for observability / alerting). |
| `trigger_base_poll_interval` | `Duration` | Base poll interval — the tick rate of the run loop. |
| `trigger_poll_timeout` | `Duration` | Maximum time to wait for a single trigger poll operation. |



### `cloacina::cron_trigger_scheduler::Scheduler`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Unified scheduler for both cron and trigger-based workflow execution.

The scheduler runs a single polling loop that:
1. Ticks at `trigger_base_poll_interval` (default 1 s)
2. Every `cron_poll_interval`, queries due cron schedules and processes them
3. Every tick, checks enabled triggers respecting per-trigger poll intervals
4. Records audit trail for every handoff via `schedule_executions`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `Arc < DAL >` |  |
| `executor` | `Arc < dyn WorkflowExecutor >` |  |
| `config` | `SchedulerConfig` |  |
| `shutdown` | `watch :: Receiver < bool >` |  |
| `last_poll_times` | `HashMap < String , Instant >` | Tracks when each trigger was last polled (by trigger name). |
| `last_cron_check` | `Option < Instant >` | Tracks when cron schedules were last checked. |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (dal : Arc < DAL > , executor : Arc < dyn WorkflowExecutor > , config : SchedulerConfig , shutdown : watch :: Receiver < bool > ,) -> Self
```

Creates a new unified scheduler.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `dal` | `-` | Data access layer for database operations |
| `executor` | `-` | Pipeline executor for workflow execution |
| `config` | `-` | Scheduler configuration |
| `shutdown` | `-` | Shutdown signal receiver |


<details>
<summary>Source</summary>

```rust
    pub fn new(
        dal: Arc<DAL>,
        executor: Arc<dyn WorkflowExecutor>,
        config: SchedulerConfig,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            dal,
            executor,
            config,
            shutdown,
            last_poll_times: HashMap::new(),
            last_cron_check: None,
        }
    }
```

</details>



##### `with_defaults` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_defaults (dal : Arc < DAL > , executor : Arc < dyn WorkflowExecutor > , shutdown : watch :: Receiver < bool > ,) -> Self
```

Creates a new unified scheduler with default configuration.

<details>
<summary>Source</summary>

```rust
    pub fn with_defaults(
        dal: Arc<DAL>,
        executor: Arc<dyn WorkflowExecutor>,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self::new(dal, executor, SchedulerConfig::default(), shutdown)
    }
```

</details>



##### `run_polling_loop` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn run_polling_loop (& mut self) -> Result < () , WorkflowExecutionError >
```

Runs the main polling loop.

Ticks at `trigger_base_poll_interval`. On each tick it:
- Checks cron schedules if `cron_poll_interval` has elapsed since the
last cron check.
- Checks all enabled triggers, respecting per-trigger poll intervals.
The loop continues until a shutdown signal is received.

<details>
<summary>Source</summary>

```rust
    pub async fn run_polling_loop(&mut self) -> Result<(), WorkflowExecutionError> {
        info!(
            "Starting unified scheduler (cron interval: {:?}, trigger base interval: {:?})",
            self.config.cron_poll_interval, self.config.trigger_base_poll_interval,
        );

        let mut interval = tokio::time::interval(self.config.trigger_base_poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // --- Cron ---
                    let now = Instant::now();
                    let should_check_cron = match self.last_cron_check {
                        Some(last) => now.duration_since(last) >= self.config.cron_poll_interval,
                        None => true,
                    };

                    if should_check_cron {
                        self.last_cron_check = Some(now);
                        if let Err(e) = self.check_and_execute_cron_schedules().await {
                            error!("Error processing cron schedules: {}", e);
                        }
                    }

                    // --- Triggers ---
                    if let Err(e) = self.check_and_process_triggers().await {
                        error!("Error processing triggers: {}", e);
                    }
                }
                _ = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        info!("Unified scheduler received shutdown signal");
                        break;
                    }
                }
            }
        }

        info!("Unified scheduler polling loop stopped");
        Ok(())
    }
```

</details>



##### `check_and_execute_cron_schedules` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn check_and_execute_cron_schedules (& self) -> Result < () , WorkflowExecutionError >
```

Checks for due cron schedules and executes them.

<details>
<summary>Source</summary>

```rust
    async fn check_and_execute_cron_schedules(&self) -> Result<(), WorkflowExecutionError> {
        let now = Utc::now();
        debug!("Checking for due cron schedules at {}", now);

        let due_schedules = self
            .dal
            .schedule()
            .get_due_cron_schedules(now)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: e.to_string(),
            })?;

        if due_schedules.is_empty() {
            debug!("No due cron schedules found");
            return Ok(());
        }

        info!("Found {} due cron schedule(s)", due_schedules.len());

        for schedule in due_schedules {
            if let Err(e) = self.process_cron_schedule(&schedule, now).await {
                error!("Failed to process cron schedule {}: {}", schedule.id, e);
            }
        }

        Ok(())
    }
```

</details>



##### `process_cron_schedule` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn process_cron_schedule (& self , schedule : & Schedule , now : DateTime < Utc > ,) -> Result < () , WorkflowExecutionError >
```

Processes a single cron schedule using the saga pattern.

<details>
<summary>Source</summary>

```rust
    async fn process_cron_schedule(
        &self,
        schedule: &Schedule,
        now: DateTime<Utc>,
    ) -> Result<(), WorkflowExecutionError> {
        debug!(
            "Processing cron schedule: {} (workflow: {})",
            schedule.id, schedule.workflow_name
        );

        // Check active time window
        if !self.is_cron_schedule_active(schedule, now) {
            debug!(
                "Cron schedule {} is outside its active time window, skipping",
                schedule.id
            );
            return Ok(());
        }

        // Calculate execution times based on catchup policy
        let execution_times = self.calculate_execution_times(schedule, now)?;
        if execution_times.is_empty() {
            debug!(
                "No execution times calculated for cron schedule {}",
                schedule.id
            );
            return Ok(());
        }

        // Calculate next run time
        let next_run = self.calculate_next_run(schedule, now)?;

        // Atomically claim the schedule
        let claimed = self
            .dal
            .schedule()
            .claim_and_update_cron(schedule.id, now, now, next_run)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: e.to_string(),
            })?;

        if !claimed {
            debug!(
                "Cron schedule {} was already claimed by another instance",
                schedule.id
            );
            return Ok(());
        }

        info!(
            "Successfully claimed cron schedule {} for {} execution(s)",
            schedule.id,
            execution_times.len()
        );

        // Execute all scheduled times
        for scheduled_time in execution_times {
            // Step 1: Create audit record BEFORE handoff
            let audit_record_id = match self
                .create_cron_execution_audit(schedule.id, scheduled_time)
                .await
            {
                Ok(id) => id,
                Err(e) => {
                    error!(
                        "Failed to create execution audit for cron schedule {} at {}: {}",
                        schedule.id, scheduled_time, e
                    );
                    continue;
                }
            };

            // Step 2: Hand off to pipeline executor
            match self.execute_cron_workflow(schedule, scheduled_time).await {
                Ok(pipeline_execution_id) => {
                    // Step 3: Link audit record
                    if let Err(e) = self
                        .dal
                        .schedule_execution()
                        .update_pipeline_execution_id(audit_record_id, pipeline_execution_id)
                        .await
                    {
                        error!(
                            "Failed to complete audit trail for cron schedule {} execution: {}",
                            schedule.id, e
                        );
                    }

                    info!(
                        "Successfully executed and audited workflow {} for cron schedule {} (scheduled: {})",
                        schedule.workflow_name, schedule.id, scheduled_time
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to execute workflow {} for cron schedule {} (scheduled: {}): {}",
                        schedule.workflow_name, schedule.id, scheduled_time, e
                    );
                    error!(
                        "Execution lost: audit record {} exists but pipeline execution failed",
                        audit_record_id
                    );
                }
            }
        }

        Ok(())
    }
```

</details>



##### `is_cron_schedule_active` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn is_cron_schedule_active (& self , schedule : & Schedule , now : DateTime < Utc >) -> bool
```

Checks if a cron schedule is within its active time window.

<details>
<summary>Source</summary>

```rust
    fn is_cron_schedule_active(&self, schedule: &Schedule, now: DateTime<Utc>) -> bool {
        if let Some(start) = &schedule.start_date {
            if now < start.0 {
                return false;
            }
        }
        if let Some(end) = &schedule.end_date {
            if now > end.0 {
                return false;
            }
        }
        true
    }
```

</details>



##### `calculate_execution_times` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn calculate_execution_times (& self , schedule : & Schedule , now : DateTime < Utc > ,) -> Result < Vec < DateTime < Utc > > , WorkflowExecutionError >
```

Calculates execution times based on the schedule's catchup policy.

<details>
<summary>Source</summary>

```rust
    fn calculate_execution_times(
        &self,
        schedule: &Schedule,
        now: DateTime<Utc>,
    ) -> Result<Vec<DateTime<Utc>>, WorkflowExecutionError> {
        let policy_str = schedule.catchup_policy.as_deref().unwrap_or("skip");
        let policy = CatchupPolicy::from(policy_str.to_string());

        match policy {
            CatchupPolicy::Skip => {
                // Just return the current next_run_at
                let next_run = schedule.next_run_at.map(|t| t.0).unwrap_or(now);
                Ok(vec![next_run])
            }
            CatchupPolicy::RunAll => {
                let cron_expr = schedule.cron_expression.as_deref().unwrap_or("* * * * *");
                let tz = schedule.timezone.as_deref().unwrap_or("UTC");

                let evaluator = CronEvaluator::new(cron_expr, tz).map_err(|e| {
                    WorkflowExecutionError::ExecutionFailed {
                        message: format!("Cron evaluation error: {}", e),
                    }
                })?;

                let start_time = schedule
                    .last_run_at
                    .map(|t| t.0)
                    .unwrap_or(schedule.created_at.0);

                let missed_executions = evaluator
                    .executions_between(start_time, now, self.config.max_catchup_executions)
                    .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                        message: format!("Cron evaluation error: {}", e),
                    })?;

                if missed_executions.len() >= self.config.max_catchup_executions {
                    warn!(
                        "Limited catchup executions to {} for cron schedule {} (policy: RunAll)",
                        self.config.max_catchup_executions, schedule.id
                    );
                }

                Ok(missed_executions)
            }
        }
    }
```

</details>



##### `calculate_next_run` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn calculate_next_run (& self , schedule : & Schedule , after : DateTime < Utc > ,) -> Result < DateTime < Utc > , WorkflowExecutionError >
```

Calculates the next run time for a cron schedule.

<details>
<summary>Source</summary>

```rust
    fn calculate_next_run(
        &self,
        schedule: &Schedule,
        after: DateTime<Utc>,
    ) -> Result<DateTime<Utc>, WorkflowExecutionError> {
        let cron_expr = schedule.cron_expression.as_deref().unwrap_or("* * * * *");
        let tz = schedule.timezone.as_deref().unwrap_or("UTC");

        let evaluator = CronEvaluator::new(cron_expr, tz).map_err(|e| {
            WorkflowExecutionError::ExecutionFailed {
                message: format!("Cron evaluation error: {}", e),
            }
        })?;

        evaluator
            .next_execution(after)
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Cron evaluation error: {}", e),
            })
    }
```

</details>



##### `execute_cron_workflow` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn execute_cron_workflow (& self , schedule : & Schedule , scheduled_time : DateTime < Utc > ,) -> Result < UniversalUuid , WorkflowExecutionError >
```

Executes a cron workflow by handing it off to the pipeline executor.

<details>
<summary>Source</summary>

```rust
    async fn execute_cron_workflow(
        &self,
        schedule: &Schedule,
        scheduled_time: DateTime<Utc>,
    ) -> Result<UniversalUuid, WorkflowExecutionError> {
        let mut context = Context::new();
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

        info!(
            "Executing workflow '{}' for cron schedule {} (scheduled time: {})",
            schedule.workflow_name, schedule.id, scheduled_time
        );

        let pipeline_result = self
            .executor
            .execute(&schedule.workflow_name, context)
            .await?;

        debug!(
            "Successfully handed off workflow '{}' to executor (execution_id: {})",
            schedule.workflow_name, pipeline_result.execution_id
        );

        Ok(UniversalUuid(pipeline_result.execution_id))
    }
```

</details>



##### `create_cron_execution_audit` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn create_cron_execution_audit (& self , schedule_id : UniversalUuid , scheduled_time : DateTime < Utc > ,) -> Result < UniversalUuid , ValidationError >
```

Creates an audit record for a cron execution.

<details>
<summary>Source</summary>

```rust
    async fn create_cron_execution_audit(
        &self,
        schedule_id: UniversalUuid,
        scheduled_time: DateTime<Utc>,
    ) -> Result<UniversalUuid, ValidationError> {
        let new_execution = NewScheduleExecution {
            schedule_id,
            pipeline_execution_id: None,
            scheduled_time: Some(UniversalTimestamp(scheduled_time)),
            claimed_at: Some(UniversalTimestamp(Utc::now())),
            context_hash: None,
        };

        let audit_record = self.dal.schedule_execution().create(new_execution).await?;

        debug!(
            "Created cron execution audit record {} for schedule {} (scheduled: {})",
            audit_record.id, schedule_id, scheduled_time
        );

        Ok(audit_record.id)
    }
```

</details>



##### `check_and_process_triggers` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn check_and_process_triggers (& mut self) -> Result < () , WorkflowExecutionError >
```

Checks all enabled triggers and processes those that are due.

<details>
<summary>Source</summary>

```rust
    async fn check_and_process_triggers(&mut self) -> Result<(), WorkflowExecutionError> {
        debug!("Checking trigger schedules");

        let schedules = self
            .dal
            .schedule()
            .get_enabled_triggers()
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to get trigger schedules: {}", e),
            })?;

        if schedules.is_empty() {
            debug!("No enabled trigger schedules found");
            return Ok(());
        }

        let now = Instant::now();

        for schedule in schedules {
            let trigger_name = schedule
                .trigger_name
                .as_deref()
                .unwrap_or("unknown")
                .to_string();

            // Check if this trigger is due for polling
            let poll_interval = schedule
                .poll_interval()
                .unwrap_or(self.config.trigger_base_poll_interval);
            let last_poll = self.last_poll_times.get(&trigger_name);

            let should_poll = match last_poll {
                Some(last) => now.duration_since(*last) >= poll_interval,
                None => true,
            };

            if !should_poll {
                continue;
            }

            // Process this trigger
            if let Err(e) = self.process_trigger(&schedule).await {
                error!("Failed to process trigger '{}': {}", trigger_name, e);
            }

            // Update last poll time
            self.last_poll_times.insert(trigger_name, now);
        }

        Ok(())
    }
```

</details>



##### `process_trigger` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn process_trigger (& self , schedule : & Schedule) -> Result < () , TriggerError >
```

Processes a single trigger schedule.

<details>
<summary>Source</summary>

```rust
    async fn process_trigger(&self, schedule: &Schedule) -> Result<(), TriggerError> {
        let trigger_name = schedule.trigger_name.as_deref().unwrap_or("unknown");

        debug!(
            "Processing trigger '{}' (workflow: {})",
            trigger_name, schedule.workflow_name
        );

        // Get the trigger instance from registry
        let trigger = get_trigger(trigger_name).ok_or_else(|| TriggerError::TriggerNotFound {
            name: trigger_name.to_string(),
        })?;

        // Poll the trigger with timeout
        let poll_result = tokio::time::timeout(self.config.trigger_poll_timeout, trigger.poll())
            .await
            .map_err(|_| TriggerError::PollError {
                message: format!(
                    "Trigger '{}' poll timed out after {:?}",
                    trigger_name, self.config.trigger_poll_timeout
                ),
            })?
            .map_err(|e| {
                error!("Trigger '{}' poll error: {}", trigger_name, e);
                e
            })?;

        // Update last poll time in database
        let now = Utc::now();
        if let Err(e) = self.dal.schedule().update_last_poll(schedule.id, now).await {
            warn!(
                "Failed to update last_poll_at for trigger '{}': {}",
                trigger_name, e
            );
        }

        // Check if trigger should fire
        if !poll_result.should_fire() {
            debug!("Trigger '{}' returned Skip", trigger_name);
            return Ok(());
        }

        // Compute context hash for deduplication
        let context_hash = poll_result.context_hash();

        // Check for duplicate active execution (unless allow_concurrent)
        if !schedule.allows_concurrent() {
            let has_active = self
                .dal
                .schedule_execution()
                .has_active_execution(schedule.id, &context_hash)
                .await
                .map_err(|e| TriggerError::ConnectionPool(e.to_string()))?;

            if has_active {
                debug!(
                    "Trigger '{}' has active execution with same context hash, skipping",
                    trigger_name
                );
                return Ok(());
            }
        }

        info!(
            "Trigger '{}' fired, scheduling workflow '{}'",
            trigger_name, schedule.workflow_name
        );

        // Create execution audit record before handoff
        let execution = self
            .create_trigger_execution_audit(schedule.id, &context_hash)
            .await?;

        // Extract context from result
        let context = poll_result.into_context().unwrap_or_else(Context::new);

        // Hand off to pipeline executor
        match self.execute_trigger_workflow(schedule, context).await {
            Ok(pipeline_execution_id) => {
                // Link the execution to the pipeline execution
                if let Err(e) = self
                    .dal
                    .schedule_execution()
                    .update_pipeline_execution_id(execution.id, pipeline_execution_id)
                    .await
                {
                    warn!(
                        "Failed to link schedule execution to pipeline execution: {}",
                        e
                    );
                }

                info!(
                    "Successfully scheduled workflow '{}' for trigger '{}' (execution: {})",
                    schedule.workflow_name, trigger_name, pipeline_execution_id
                );
            }
            Err(e) => {
                error!(
                    "Failed to execute workflow '{}' for trigger '{}': {}",
                    schedule.workflow_name, trigger_name, e
                );
                // Mark execution as completed (failed)
                if let Err(e) = self
                    .dal
                    .schedule_execution()
                    .complete(execution.id, Utc::now())
                    .await
                {
                    warn!(
                        "Failed to mark schedule execution as completed after failure: {}",
                        e
                    );
                }
                return Err(TriggerError::WorkflowSchedulingFailed {
                    workflow: schedule.workflow_name.clone(),
                    message: e.to_string(),
                });
            }
        }

        Ok(())
    }
```

</details>



##### `create_trigger_execution_audit` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn create_trigger_execution_audit (& self , schedule_id : UniversalUuid , context_hash : & str ,) -> Result < crate :: models :: schedule :: ScheduleExecution , TriggerError >
```

Creates an audit record for a trigger execution.

<details>
<summary>Source</summary>

```rust
    async fn create_trigger_execution_audit(
        &self,
        schedule_id: UniversalUuid,
        context_hash: &str,
    ) -> Result<crate::models::schedule::ScheduleExecution, TriggerError> {
        let new_execution = NewScheduleExecution {
            schedule_id,
            pipeline_execution_id: None,
            scheduled_time: None,
            claimed_at: None,
            context_hash: Some(context_hash.to_string()),
        };

        let execution = self
            .dal
            .schedule_execution()
            .create(new_execution)
            .await
            .map_err(|e| TriggerError::ConnectionPool(e.to_string()))?;

        debug!(
            "Created trigger execution audit record {} for schedule {}",
            execution.id, schedule_id
        );

        Ok(execution)
    }
```

</details>



##### `execute_trigger_workflow` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn execute_trigger_workflow (& self , schedule : & Schedule , mut context : Context < serde_json :: Value > ,) -> Result < UniversalUuid , WorkflowExecutionError >
```

Executes a trigger workflow by handing it off to the pipeline executor.

<details>
<summary>Source</summary>

```rust
    async fn execute_trigger_workflow(
        &self,
        schedule: &Schedule,
        mut context: Context<serde_json::Value>,
    ) -> Result<UniversalUuid, WorkflowExecutionError> {
        let trigger_name = schedule.trigger_name.as_deref().unwrap_or("unknown");

        context
            .insert("trigger_name", serde_json::json!(trigger_name))
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;
        context
            .insert("triggered_at", serde_json::json!(Utc::now().to_rfc3339()))
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Context error: {}", e),
            })?;

        let result = self
            .executor
            .execute(&schedule.workflow_name, context)
            .await?;

        debug!(
            "Successfully handed off workflow '{}' to executor (execution_id: {})",
            schedule.workflow_name, result.execution_id
        );

        Ok(UniversalUuid(result.execution_id))
    }
```

</details>



##### `register_trigger` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn register_trigger (& self , trigger : & dyn Trigger , workflow_name : & str ,) -> Result < Schedule , ValidationError >
```

Registers a trigger with the scheduler.

Persists the trigger configuration to the database for recovery across
restarts. The trigger must also be registered in the global trigger
registry for the actual polling function.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `trigger` | `-` | The trigger instance to register |
| `workflow_name` | `-` | Name of the workflow to fire when trigger activates |


<details>
<summary>Source</summary>

```rust
    pub async fn register_trigger(
        &self,
        trigger: &dyn Trigger,
        workflow_name: &str,
    ) -> Result<Schedule, ValidationError> {
        let mut new_schedule =
            NewSchedule::trigger(trigger.name(), workflow_name, trigger.poll_interval());
        new_schedule.allow_concurrent = Some(crate::database::universal_types::UniversalBool::new(
            trigger.allow_concurrent(),
        ));

        // Upsert to handle re-registration
        self.dal.schedule().upsert_trigger(new_schedule).await
    }
```

</details>



##### `disable_trigger` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn disable_trigger (& self , trigger_name : & str) -> Result < () , ValidationError >
```

Disables a trigger by name.

<details>
<summary>Source</summary>

```rust
    pub async fn disable_trigger(&self, trigger_name: &str) -> Result<(), ValidationError> {
        if let Some(schedule) = self
            .dal
            .schedule()
            .get_by_trigger_name(trigger_name)
            .await?
        {
            self.dal.schedule().disable(schedule.id).await?;
            info!("Disabled trigger '{}'", trigger_name);
        }
        Ok(())
    }
```

</details>



##### `enable_trigger` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn enable_trigger (& self , trigger_name : & str) -> Result < () , ValidationError >
```

Enables a trigger by name.

<details>
<summary>Source</summary>

```rust
    pub async fn enable_trigger(&self, trigger_name: &str) -> Result<(), ValidationError> {
        if let Some(schedule) = self
            .dal
            .schedule()
            .get_by_trigger_name(trigger_name)
            .await?
        {
            self.dal.schedule().enable(schedule.id).await?;
            info!("Enabled trigger '{}'", trigger_name);
        }
        Ok(())
    }
```

</details>
