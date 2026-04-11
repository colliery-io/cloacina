# cloaca.python.bindings.runner <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


## Classes

### `cloaca.python.bindings.runner.WorkflowResult`

> **Rust Implementation**: [cloacina::python::bindings::runner::PyWorkflowResult](../../../rust/cloacina/python/bindings/runner.md#class-workflowresult)

Python wrapper for WorkflowExecutionResult

#### Methods

##### `status`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">status</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyWorkflowResult::status](../../../rust/cloacina/python/bindings/runner.md#status)

Get the execution status

<details>
<summary>Source</summary>

```python
    pub fn status(&self) -> String {
        format!("{:?}", self.inner.status)
    }
```

</details>



##### `start_time`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">start_time</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyWorkflowResult::start_time](../../../rust/cloacina/python/bindings/runner.md#start_time)

Get execution start time as ISO string

<details>
<summary>Source</summary>

```python
    pub fn start_time(&self) -> String {
        self.inner.start_time.to_rfc3339()
    }
```

</details>



##### `end_time`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">end_time</span>() -> <span style="color: var(--md-default-fg-color--light);">Optional[str]</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyWorkflowResult::end_time](../../../rust/cloacina/python/bindings/runner.md#end_time)

Get execution end time as ISO string

<details>
<summary>Source</summary>

```python
    pub fn end_time(&self) -> Option<String> {
        self.inner.end_time.map(|t| t.to_rfc3339())
    }
```

</details>



##### `final_context`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">final_context</span>() -> <span style="color: var(--md-default-fg-color--light);">PyContext</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyWorkflowResult::final_context](../../../rust/cloacina/python/bindings/runner.md#final_context)

Get the final context

<details>
<summary>Source</summary>

```python
    pub fn final_context(&self) -> PyContext {
        // Create a new context by cloning the data without the dependency loader
        let new_context = self.inner.final_context.clone_data();
        PyContext::from_rust_context(new_context)
    }
```

</details>



##### `error_message`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">error_message</span>() -> <span style="color: var(--md-default-fg-color--light);">Optional[str]</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyWorkflowResult::error_message](../../../rust/cloacina/python/bindings/runner.md#error_message)

Get error message if execution failed

<details>
<summary>Source</summary>

```python
    pub fn error_message(&self) -> Option<&str> {
        self.inner.error_message.as_deref()
    }
```

</details>



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyWorkflowResult::__repr__](../../../rust/cloacina/python/bindings/runner.md#__repr__)

String representation

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        format!(
            "WorkflowResult(status={}, error={})",
            self.status(),
            self.error_message().unwrap_or("None")
        )
    }
```

</details>





### `cloaca.python.bindings.runner.DefaultRunner`

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner](../../../rust/cloacina/python/bindings/runner.md#class-defaultrunner)

Python wrapper for DefaultRunner

#### Methods

##### `new`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">new</span>(database_url: str) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::new](../../../rust/cloacina/python/bindings/runner.md#new)

Create a new DefaultRunner with database connection

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database_url` | `str` |  |


<details>
<summary>Source</summary>

```python
    pub fn new(database_url: &str) -> PyResult<Self> {
        let database_url = database_url.to_string();

        // Create a channel for communicating with the async runtime thread
        let (tx, mut rx) = mpsc::unbounded_channel::<RuntimeMessage>();

        // Spawn a dedicated thread for the async runtime
        let thread_handle = thread::spawn(move || {
            // Initialize logging in this thread

            // Try to initialize tracing
            use tracing::{debug, info};
            let _guard = tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
                )
                .try_init();

            info!("Background thread started with tracing");

            // Create the tokio runtime in the dedicated thread
            debug!("Creating tokio runtime");
            let rt = Runtime::new().expect("Failed to create tokio runtime");
            info!("Tokio runtime created successfully");

            // Create the DefaultRunner within the async context
            let runner = rt.block_on(async {
                info!(
                    "Creating DefaultRunner with database_url: {}",
                    crate::logging::mask_db_url(&database_url)
                );
                debug!("About to call crate::DefaultRunner::new()");
                let runner = crate::DefaultRunner::new(&database_url)
                    .await
                    .expect("Failed to create DefaultRunner");
                info!("DefaultRunner created successfully, background services running");
                runner
            });
            info!("DefaultRunner creation completed");

            let runner = Arc::new(runner);

            // Event loop for processing messages - spawn tasks instead of blocking
            rt.block_on(async {
                while let Some(message) = rx.recv().await {
                    match message {
                        RuntimeMessage::Execute {
                            workflow_name,
                            context,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            // Spawn the execution as a separate task to avoid blocking the message loop
                            tokio::spawn(async move {
                                // Execute the workflow in the async runtime
                                use crate::executor::WorkflowExecutor;
                                let result = runner_clone.execute(&workflow_name, context).await;

                                // Send response back to the calling thread
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::RegisterCronWorkflow {
                            workflow_name,
                            cron_expression,
                            timezone,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone
                                    .register_cron_workflow(
                                        &workflow_name,
                                        &cron_expression,
                                        &timezone,
                                    )
                                    .await
                                    .map(|uuid| uuid.to_string());

                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::ListCronSchedules {
                            enabled_only,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone
                                    .list_cron_schedules(enabled_only, limit, offset)
                                    .await;
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::SetCronScheduleEnabled {
                            schedule_id,
                            enabled,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .set_cron_schedule_enabled(universal_uuid, enabled)
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::WorkflowExecutionError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::DeleteCronSchedule {
                            schedule_id,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone.delete_cron_schedule(universal_uuid).await
                                    }
                                    Err(e) => Err(crate::executor::WorkflowExecutionError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronSchedule {
                            schedule_id,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone.get_cron_schedule(universal_uuid).await
                                    }
                                    Err(e) => Err(crate::executor::WorkflowExecutionError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::UpdateCronSchedule {
                            schedule_id,
                            cron_expression,
                            timezone,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .update_cron_schedule(
                                                universal_uuid,
                                                Some(&cron_expression),
                                                Some(&timezone),
                                            )
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::WorkflowExecutionError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronExecutionHistory {
                            schedule_id,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .get_cron_execution_history(
                                                universal_uuid,
                                                limit,
                                                offset,
                                            )
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::WorkflowExecutionError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronExecutionStats { since, response_tx } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone.get_cron_execution_stats(since).await;
                                let _ = response_tx.send(result);
                            });
                        }
                        // Trigger management messages
                        RuntimeMessage::ListTriggerSchedules {
                            enabled_only,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result = if enabled_only {
                                    dal.schedule().get_enabled_triggers().await
                                } else {
                                    dal.schedule()
                                        .list(Some("trigger"), false, limit, offset)
                                        .await
                                };
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::WorkflowExecutionError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::GetTriggerSchedule {
                            trigger_name,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result =
                                    dal.schedule().get_by_trigger_name(&trigger_name).await;
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::WorkflowExecutionError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::SetTriggerEnabled {
                            trigger_name,
                            enabled,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = async {
                                    if let Some(scheduler) = runner_clone.unified_scheduler().await
                                    {
                                        if enabled {
                                            scheduler.enable_trigger(&trigger_name).await
                                        } else {
                                            scheduler.disable_trigger(&trigger_name).await
                                        }
                                    } else {
                                        // No unified scheduler, use DAL directly
                                        let dal = runner_clone.dal();
                                        if let Some(schedule) = dal
                                            .schedule()
                                            .get_by_trigger_name(&trigger_name)
                                            .await?
                                        {
                                            if enabled {
                                                dal.schedule().enable(schedule.id).await
                                            } else {
                                                dal.schedule().disable(schedule.id).await
                                            }
                                        } else {
                                            Ok(())
                                        }
                                    }
                                }
                                .await;
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::WorkflowExecutionError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::GetTriggerExecutionHistory {
                            trigger_name,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result = async {
                                    // Look up schedule by trigger name, then list executions
                                    let schedule_opt = dal
                                        .schedule()
                                        .get_by_trigger_name(&trigger_name)
                                        .await
                                        .map_err(|e| {
                                            crate::executor::WorkflowExecutionError::Configuration {
                                                message: e.to_string(),
                                            }
                                        })?;
                                    if let Some(schedule) = schedule_opt {
                                        dal.schedule_execution()
                                            .list_by_schedule(schedule.id, limit, offset)
                                            .await
                                            .map_err(|e| {
                                                crate::executor::WorkflowExecutionError::Configuration {
                                                    message: e.to_string(),
                                                }
                                            })
                                    } else {
                                        Ok(vec![])
                                    }
                                }
                                .await;
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::Shutdown => {
                            break;
                        }
                    }
                }
            });
        });

        Ok(PyDefaultRunner {
            runtime_handle: Mutex::new(AsyncRuntimeHandle {
                tx,
                thread_handle: Some(thread_handle),
            }),
        })
    }
```

</details>



##### `with_config`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">with_config</span>(database_url: str, config: PyDefaultRunnerConfig) -> <span style="color: var(--md-default-fg-color--light);">PyDefaultRunner</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::with_config](../../../rust/cloacina/python/bindings/runner.md#with_config)

Create a new DefaultRunner with custom configuration

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database_url` | `str` |  |
| `config` | `PyDefaultRunnerConfig` |  |


<details>
<summary>Source</summary>

```python
    pub fn with_config(
        database_url: &str,
        config: &super::context::PyDefaultRunnerConfig,
    ) -> PyResult<PyDefaultRunner> {
        let database_url = database_url.to_string();
        let rust_config = config.to_rust_config();

        // Create a channel for communicating with the async runtime thread
        let (tx, mut rx) = mpsc::unbounded_channel::<RuntimeMessage>();

        // Spawn a dedicated thread for the async runtime
        let thread_handle = thread::spawn(move || {
            // Initialize logging in this thread
            if std::env::var("RUST_LOG").is_ok() {
                // Try to initialize tracing in this thread
                let _ = tracing_subscriber::fmt()
                    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                    .try_init();
            } else {
            }

            // Create the tokio runtime in the dedicated thread
            let rt = Runtime::new().expect("Failed to create tokio runtime");

            // Create the DefaultRunner within the async context
            let runner = rt.block_on(async {
                crate::DefaultRunner::with_config(&database_url, rust_config)
                    .await
                    .expect("Failed to create DefaultRunner")
            });

            let runner = Arc::new(runner);

            // Event loop for processing messages - spawn tasks instead of blocking
            rt.block_on(async {
                while let Some(message) = rx.recv().await {
                    match message {
                        RuntimeMessage::Execute {
                            workflow_name,
                            context,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            // Spawn the execution as a separate task to avoid blocking the message loop
                            tokio::spawn(async move {
                                // Execute the workflow in the async runtime
                                use crate::executor::WorkflowExecutor;
                                let result = runner_clone.execute(&workflow_name, context).await;

                                // Send response back to the calling thread
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::RegisterCronWorkflow {
                            workflow_name,
                            cron_expression,
                            timezone,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone
                                    .register_cron_workflow(
                                        &workflow_name,
                                        &cron_expression,
                                        &timezone,
                                    )
                                    .await
                                    .map(|uuid| uuid.to_string());

                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::ListCronSchedules {
                            enabled_only,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone
                                    .list_cron_schedules(enabled_only, limit, offset)
                                    .await;
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::SetCronScheduleEnabled {
                            schedule_id,
                            enabled,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .set_cron_schedule_enabled(universal_uuid, enabled)
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::WorkflowExecutionError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::DeleteCronSchedule {
                            schedule_id,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone.delete_cron_schedule(universal_uuid).await
                                    }
                                    Err(e) => Err(crate::executor::WorkflowExecutionError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronSchedule {
                            schedule_id,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone.get_cron_schedule(universal_uuid).await
                                    }
                                    Err(e) => Err(crate::executor::WorkflowExecutionError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::UpdateCronSchedule {
                            schedule_id,
                            cron_expression,
                            timezone,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .update_cron_schedule(
                                                universal_uuid,
                                                Some(&cron_expression),
                                                Some(&timezone),
                                            )
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::WorkflowExecutionError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronExecutionHistory {
                            schedule_id,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .get_cron_execution_history(
                                                universal_uuid,
                                                limit,
                                                offset,
                                            )
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::WorkflowExecutionError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronExecutionStats { since, response_tx } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone.get_cron_execution_stats(since).await;
                                let _ = response_tx.send(result);
                            });
                        }
                        // Trigger management messages
                        RuntimeMessage::ListTriggerSchedules {
                            enabled_only,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result = if enabled_only {
                                    dal.schedule().get_enabled_triggers().await
                                } else {
                                    dal.schedule()
                                        .list(Some("trigger"), false, limit, offset)
                                        .await
                                };
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::WorkflowExecutionError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::GetTriggerSchedule {
                            trigger_name,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result =
                                    dal.schedule().get_by_trigger_name(&trigger_name).await;
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::WorkflowExecutionError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::SetTriggerEnabled {
                            trigger_name,
                            enabled,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = async {
                                    if let Some(scheduler) = runner_clone.unified_scheduler().await
                                    {
                                        if enabled {
                                            scheduler.enable_trigger(&trigger_name).await
                                        } else {
                                            scheduler.disable_trigger(&trigger_name).await
                                        }
                                    } else {
                                        // No unified scheduler, use DAL directly
                                        let dal = runner_clone.dal();
                                        if let Some(schedule) = dal
                                            .schedule()
                                            .get_by_trigger_name(&trigger_name)
                                            .await?
                                        {
                                            if enabled {
                                                dal.schedule().enable(schedule.id).await
                                            } else {
                                                dal.schedule().disable(schedule.id).await
                                            }
                                        } else {
                                            Ok(())
                                        }
                                    }
                                }
                                .await;
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::WorkflowExecutionError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::GetTriggerExecutionHistory {
                            trigger_name,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result = async {
                                    // Look up schedule by trigger name, then list executions
                                    let schedule_opt = dal
                                        .schedule()
                                        .get_by_trigger_name(&trigger_name)
                                        .await
                                        .map_err(|e| {
                                            crate::executor::WorkflowExecutionError::Configuration {
                                                message: e.to_string(),
                                            }
                                        })?;
                                    if let Some(schedule) = schedule_opt {
                                        dal.schedule_execution()
                                            .list_by_schedule(schedule.id, limit, offset)
                                            .await
                                            .map_err(|e| {
                                                crate::executor::WorkflowExecutionError::Configuration {
                                                    message: e.to_string(),
                                                }
                                            })
                                    } else {
                                        Ok(vec![])
                                    }
                                }
                                .await;
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::Shutdown => {
                            break;
                        }
                    }
                }
            });
        });

        Ok(PyDefaultRunner {
            runtime_handle: Mutex::new(AsyncRuntimeHandle {
                tx,
                thread_handle: Some(thread_handle),
            }),
        })
    }
```

</details>



##### `with_schema`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">with_schema</span>(database_url: str, schema: str) -> <span style="color: var(--md-default-fg-color--light);">PyDefaultRunner</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::with_schema](../../../rust/cloacina/python/bindings/runner.md#with_schema)

Create a new DefaultRunner with PostgreSQL schema-based multi-tenancy

This method enables multi-tenant deployments by using PostgreSQL schemas
for complete data isolation between tenants. Each tenant gets their own
schema with independent tables, migrations, and data.
Note: This method requires a PostgreSQL database. SQLite does not support
database schemas.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database_url` | `str` | PostgreSQL connection string |
| `schema` | `str` | Schema name for tenant isolation (alphanumeric + underscores only) |


**Returns:**

A new DefaultRunner instance configured for the specified tenant schema

**Examples:**

```python
# Create tenant-specific runners
tenant_a = DefaultRunner.with_schema(
    "postgresql://user:pass@localhost/db",
    "tenant_acme"
)
tenant_b = DefaultRunner.with_schema(
    "postgresql://user:pass@localhost/db",
    "tenant_globex"
)
```

<details>
<summary>Source</summary>

```python
    pub fn with_schema(database_url: &str, schema: &str) -> PyResult<PyDefaultRunner> {
        // Runtime check for PostgreSQL - schema-based multi-tenancy requires PostgreSQL
        if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
            return Err(PyValueError::new_err(
                "Schema-based multi-tenancy requires PostgreSQL. \
                 SQLite does not support database schemas. \
                 Use a PostgreSQL URL like 'postgres://user:pass@host/db'",
            ));
        }

        info!("Creating DefaultRunner with PostgreSQL schema: {}", schema);

        // Validate schema name format
        if schema.is_empty() {
            return Err(PyValueError::new_err("Schema name cannot be empty"));
        }

        if !schema.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(PyValueError::new_err(
                "Schema name must contain only alphanumeric characters and underscores",
            ));
        }

        let database_url = database_url.to_string();
        let schema = schema.to_string();

        // Create channel for communication with the async thread
        let (tx, mut rx) = mpsc::unbounded_channel::<RuntimeMessage>();

        // Try to create the DefaultRunner first to catch errors early
        let database_url_clone = database_url.clone();
        let schema_clone = schema.clone();

        // Test the connection and schema creation in a temporary runtime
        let rt = Runtime::new()
            .map_err(|e| PyValueError::new_err(format!("Failed to create Tokio runtime: {}", e)))?;
        let _runner = rt
            .block_on(async {
                crate::DefaultRunner::with_schema(&database_url_clone, &schema_clone).await
            })
            .map_err(|e| {
                PyValueError::new_err(format!("Failed to create DefaultRunner with schema: {}", e))
            })?;

        // If we got here, the creation succeeded, so spawn the background thread
        let thread_handle = thread::spawn(move || {
            info!("Starting async runtime thread for schema: {}", schema);

            // Create a new Tokio runtime
            let rt = Runtime::new().expect("Failed to create Tokio runtime");
            info!("Tokio runtime created successfully for schema: {}", schema);

            // Create the DefaultRunner with schema within the async context
            let runner = rt.block_on(async {
                info!("Creating DefaultRunner with schema: {} and database_url: {}", schema, crate::logging::mask_db_url(&database_url));
                debug!("About to call crate::DefaultRunner::with_schema()");
                let runner = crate::DefaultRunner::with_schema(&database_url, &schema).await
                    .expect("Failed to create DefaultRunner with schema - this should not fail since we tested it above");
                info!("DefaultRunner with schema created successfully, background services running");
                runner
            });
            info!("DefaultRunner with schema creation completed");

            let runner = Arc::new(runner);

            // Event loop for processing messages - identical to standard runner
            rt.block_on(async {
                while let Some(message) = rx.recv().await {
                    match message {
                        RuntimeMessage::Execute {
                            workflow_name,
                            context,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                use crate::executor::WorkflowExecutor;
                                let result = runner_clone.execute(&workflow_name, context).await;

                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::RegisterCronWorkflow {
                            workflow_name,
                            cron_expression,
                            timezone,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone
                                    .register_cron_workflow(
                                        &workflow_name,
                                        &cron_expression,
                                        &timezone,
                                    )
                                    .await
                                    .map(|uuid| uuid.to_string());

                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::ListCronSchedules {
                            enabled_only,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone
                                    .list_cron_schedules(enabled_only, limit, offset)
                                    .await;
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::SetCronScheduleEnabled {
                            schedule_id,
                            enabled,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .set_cron_schedule_enabled(universal_uuid, enabled)
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::WorkflowExecutionError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::DeleteCronSchedule {
                            schedule_id,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone.delete_cron_schedule(universal_uuid).await
                                    }
                                    Err(e) => Err(crate::executor::WorkflowExecutionError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronSchedule {
                            schedule_id,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone.get_cron_schedule(universal_uuid).await
                                    }
                                    Err(e) => Err(crate::executor::WorkflowExecutionError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::UpdateCronSchedule {
                            schedule_id,
                            cron_expression,
                            timezone,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .update_cron_schedule(
                                                universal_uuid,
                                                Some(&cron_expression),
                                                Some(&timezone),
                                            )
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::WorkflowExecutionError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronExecutionHistory {
                            schedule_id,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = match schedule_id.parse::<uuid::Uuid>() {
                                    Ok(uuid) => {
                                        let universal_uuid = crate::UniversalUuid::from(uuid);
                                        runner_clone
                                            .get_cron_execution_history(
                                                universal_uuid,
                                                limit,
                                                offset,
                                            )
                                            .await
                                    }
                                    Err(e) => Err(crate::executor::WorkflowExecutionError::Configuration {
                                        message: format!("Invalid schedule ID: {}", e),
                                    }),
                                };
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::GetCronExecutionStats { since, response_tx } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = runner_clone.get_cron_execution_stats(since).await;
                                let _ = response_tx.send(result);
                            });
                        }
                        // Trigger management messages
                        RuntimeMessage::ListTriggerSchedules {
                            enabled_only,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result = if enabled_only {
                                    dal.schedule().get_enabled_triggers().await
                                } else {
                                    dal.schedule()
                                        .list(Some("trigger"), false, limit, offset)
                                        .await
                                };
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::WorkflowExecutionError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::GetTriggerSchedule {
                            trigger_name,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result =
                                    dal.schedule().get_by_trigger_name(&trigger_name).await;
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::WorkflowExecutionError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::SetTriggerEnabled {
                            trigger_name,
                            enabled,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let result = async {
                                    if let Some(scheduler) = runner_clone.unified_scheduler().await
                                    {
                                        if enabled {
                                            scheduler.enable_trigger(&trigger_name).await
                                        } else {
                                            scheduler.disable_trigger(&trigger_name).await
                                        }
                                    } else {
                                        // No unified scheduler, use DAL directly
                                        let dal = runner_clone.dal();
                                        if let Some(schedule) = dal
                                            .schedule()
                                            .get_by_trigger_name(&trigger_name)
                                            .await?
                                        {
                                            if enabled {
                                                dal.schedule().enable(schedule.id).await
                                            } else {
                                                dal.schedule().disable(schedule.id).await
                                            }
                                        } else {
                                            Ok(())
                                        }
                                    }
                                }
                                .await;
                                let _ = response_tx.send(result.map_err(|e| {
                                    crate::executor::WorkflowExecutionError::Configuration {
                                        message: e.to_string(),
                                    }
                                }));
                            });
                        }
                        RuntimeMessage::GetTriggerExecutionHistory {
                            trigger_name,
                            limit,
                            offset,
                            response_tx,
                        } => {
                            let runner_clone = runner.clone();
                            tokio::spawn(async move {
                                let dal = runner_clone.dal();
                                let result = async {
                                    // Look up schedule by trigger name, then list executions
                                    let schedule_opt = dal
                                        .schedule()
                                        .get_by_trigger_name(&trigger_name)
                                        .await
                                        .map_err(|e| {
                                            crate::executor::WorkflowExecutionError::Configuration {
                                                message: e.to_string(),
                                            }
                                        })?;
                                    if let Some(schedule) = schedule_opt {
                                        dal.schedule_execution()
                                            .list_by_schedule(schedule.id, limit, offset)
                                            .await
                                            .map_err(|e| {
                                                crate::executor::WorkflowExecutionError::Configuration {
                                                    message: e.to_string(),
                                                }
                                            })
                                    } else {
                                        Ok(vec![])
                                    }
                                }
                                .await;
                                let _ = response_tx.send(result);
                            });
                        }
                        RuntimeMessage::Shutdown => {
                            info!("Received shutdown message, breaking from event loop");
                            break;
                        }
                    }
                }
            });

            info!("Event loop finished, thread ending");
        });

        // Return the Python wrapper
        Ok(PyDefaultRunner {
            runtime_handle: Mutex::new(AsyncRuntimeHandle {
                tx,
                thread_handle: Some(thread_handle),
            }),
        })
    }
```

</details>



##### `execute`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">execute</span>(workflow_name: str, context: PyContext) -> <span style="color: var(--md-default-fg-color--light);">PyWorkflowResult</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::execute](../../../rust/cloacina/python/bindings/runner.md#execute)

Execute a workflow by name with context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `str` |  |
| `context` | `PyContext` |  |


<details>
<summary>Source</summary>

```python
    pub fn execute(
        &self,
        workflow_name: &str,
        context: &PyContext,
        py: Python,
    ) -> PyResult<PyWorkflowResult> {
        let rust_context = context.clone_inner();
        let workflow_name = workflow_name.to_string();

        // Create a oneshot channel for the response
        let (response_tx, response_rx) = oneshot::channel();

        // Send the execute message to the async runtime thread
        let message = RuntimeMessage::Execute {
            workflow_name: workflow_name.clone(),
            context: rust_context,
            response_tx,
        };

        // Send message without holding the GIL
        let result = py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            // Wait for the response
            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            result.map_err(|e| PyValueError::new_err(format!("Workflow execution failed: {}", e)))
        })?;

        Ok(PyWorkflowResult::from_result(result))
    }
```

</details>



##### `start`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">start</span>() -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::start](../../../rust/cloacina/python/bindings/runner.md#start)

Start the runner (task scheduler and executor)

<details>
<summary>Source</summary>

```python
    pub fn start(&self) -> PyResult<()> {
        // Start the runner in background
        // For now, return an error indicating this limitation
        Err(PyValueError::new_err(
            "Runner startup requires async runtime support. \
             This will be implemented in a future update.",
        ))
    }
```

</details>



##### `stop`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">stop</span>() -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::stop](../../../rust/cloacina/python/bindings/runner.md#stop)

Stop the runner

<details>
<summary>Source</summary>

```python
    pub fn stop(&self) -> PyResult<()> {
        // Stop the runner
        // For now, return an error indicating this limitation
        Err(PyValueError::new_err(
            "Runner shutdown requires async runtime support. \
             This will be implemented in a future update.",
        ))
    }
```

</details>



##### `shutdown`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">shutdown</span>() -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::shutdown](../../../rust/cloacina/python/bindings/runner.md#shutdown)

Shutdown the runner and cleanup resources

This method sends a shutdown signal to the async runtime thread and waits
for it to complete (with a 5-second timeout). Errors during shutdown are
raised as Python exceptions.

<details>
<summary>Source</summary>

```python
    pub fn shutdown(&self, py: Python) -> PyResult<()> {
        info!("Starting async runtime shutdown from Python");

        // Release the GIL while waiting for the thread to complete
        let result = py.allow_threads(|| self.runtime_handle.lock().unwrap().shutdown());

        match result {
            Ok(()) => {
                info!("Async runtime shutdown completed successfully");
                Ok(())
            }
            Err(e) => {
                error!("Async runtime shutdown failed: {}", e);
                Err(PyValueError::new_err(format!(
                    "Failed to shutdown async runtime: {}",
                    e
                )))
            }
        }
    }
```

</details>



##### `register_cron_workflow`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">register_cron_workflow</span>(workflow_name: str, cron_expression: str, timezone: str) -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::register_cron_workflow](../../../rust/cloacina/python/bindings/runner.md#register_cron_workflow)

Register a cron workflow for automatic execution at scheduled times

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `str` | Name of the workflow to execute |
| `cron_expression` | `str` | Standard cron expression (e.g., "0 2 * * *" for daily at 2 AM) |
| `timezone` | `str` | Timezone for cron interpretation (e.g., "UTC", "America/New_York") |


**Returns:**

* Schedule ID as a string

**Examples:**

```python
# Daily backup at 2 AM UTC
schedule_id = runner.register_cron_workflow("backup_workflow", "0 2 * * *", "UTC")

# Business hours processing (9 AM - 5 PM, weekdays, Eastern Time)
schedule_id = runner.register_cron_workflow("business_workflow", "0 9-17 * * 1-5", "America/New_York")
```

<details>
<summary>Source</summary>

```python
    pub fn register_cron_workflow(
        &self,
        workflow_name: String,
        cron_expression: String,
        timezone: String,
        py: Python,
    ) -> PyResult<String> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::RegisterCronWorkflow {
            workflow_name,
            cron_expression,
            timezone,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            result.map_err(|e| {
                PyValueError::new_err(format!("Failed to register cron workflow: {}", e))
            })
        })
    }
```

</details>



##### `list_cron_schedules`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">list_cron_schedules</span>(enabled_only: Optional[bool], limit: Optional[int], offset: Optional[int]) -> <span style="color: var(--md-default-fg-color--light);">List[Any]</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::list_cron_schedules](../../../rust/cloacina/python/bindings/runner.md#list_cron_schedules)

List all cron schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `enabled_only` | `Optional[bool]` | If True, only return enabled schedules |
| `limit` | `Optional[int]` | Maximum number of schedules to return (default: 100) |
| `offset` | `Optional[int]` | Number of schedules to skip (default: 0) |


**Returns:**

* List of dictionaries containing schedule information

<details>
<summary>Source</summary>

```python
    pub fn list_cron_schedules(
        &self,
        enabled_only: Option<bool>,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let enabled_only = enabled_only.unwrap_or(false);
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::ListCronSchedules {
            enabled_only,
            limit,
            offset,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            let schedules = result.map_err(|e| {
                PyValueError::new_err(format!("Failed to list cron schedules: {}", e))
            })?;

            // Convert schedules to Python dictionaries
            let py_schedules: Result<Vec<PyObject>, PyErr> = schedules
                .into_iter()
                .map(|schedule| {
                    Python::with_gil(|py| {
                        let dict = pyo3::types::PyDict::new(py);
                        dict.set_item("id", schedule.id.to_string())?;
                        dict.set_item("workflow_name", &schedule.workflow_name)?;
                        dict.set_item(
                            "cron_expression",
                            schedule.cron_expression.as_deref().unwrap_or(""),
                        )?;
                        dict.set_item("timezone", schedule.timezone.as_deref().unwrap_or("UTC"))?;
                        dict.set_item("enabled", schedule.enabled.is_true())?;
                        dict.set_item(
                            "catchup_policy",
                            schedule.catchup_policy.as_deref().unwrap_or("skip"),
                        )?;
                        dict.set_item("next_run_at", schedule.next_run_at.map(|t| t.to_string()))?;
                        dict.set_item("last_run_at", schedule.last_run_at.map(|t| t.to_string()))?;
                        dict.set_item("created_at", schedule.created_at.to_string())?;
                        dict.set_item("updated_at", schedule.updated_at.to_string())?;
                        Ok(dict.into())
                    })
                })
                .collect();

            py_schedules
        })
    }
```

</details>



##### `set_cron_schedule_enabled`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_cron_schedule_enabled</span>(schedule_id: str, enabled: bool) -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::set_cron_schedule_enabled](../../../rust/cloacina/python/bindings/runner.md#set_cron_schedule_enabled)

Enable or disable a cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `str` | Schedule ID to modify |
| `enabled` | `bool` | True to enable, False to disable |


<details>
<summary>Source</summary>

```python
    pub fn set_cron_schedule_enabled(
        &self,
        schedule_id: String,
        enabled: bool,
        py: Python,
    ) -> PyResult<()> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::SetCronScheduleEnabled {
            schedule_id,
            enabled,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            result.map_err(|e| {
                PyValueError::new_err(format!("Failed to update cron schedule: {}", e))
            })
        })
    }
```

</details>



##### `delete_cron_schedule`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">delete_cron_schedule</span>(schedule_id: str) -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::delete_cron_schedule](../../../rust/cloacina/python/bindings/runner.md#delete_cron_schedule)

Delete a cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `str` | Schedule ID to delete |


<details>
<summary>Source</summary>

```python
    pub fn delete_cron_schedule(&self, schedule_id: String, py: Python) -> PyResult<()> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::DeleteCronSchedule {
            schedule_id,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            result.map_err(|e| {
                PyValueError::new_err(format!("Failed to delete cron schedule: {}", e))
            })
        })
    }
```

</details>



##### `get_cron_schedule`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">get_cron_schedule</span>(schedule_id: str) -> <span style="color: var(--md-default-fg-color--light);">Any</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::get_cron_schedule](../../../rust/cloacina/python/bindings/runner.md#get_cron_schedule)

Get details of a specific cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `str` | Schedule ID to retrieve |


**Returns:**

* Dictionary containing schedule information

<details>
<summary>Source</summary>

```python
    pub fn get_cron_schedule(&self, schedule_id: String, py: Python) -> PyResult<PyObject> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::GetCronSchedule {
            schedule_id,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            let schedule = result.map_err(|e| {
                PyValueError::new_err(format!("Failed to get cron schedule: {}", e))
            })?;

            // Convert schedule to Python dictionary
            Python::with_gil(|py| {
                let dict = pyo3::types::PyDict::new(py);
                dict.set_item("id", schedule.id.to_string())?;
                dict.set_item("workflow_name", &schedule.workflow_name)?;
                dict.set_item(
                    "cron_expression",
                    schedule.cron_expression.as_deref().unwrap_or(""),
                )?;
                dict.set_item("timezone", schedule.timezone.as_deref().unwrap_or("UTC"))?;
                dict.set_item("enabled", schedule.enabled.is_true())?;
                dict.set_item(
                    "catchup_policy",
                    schedule.catchup_policy.as_deref().unwrap_or("skip"),
                )?;
                dict.set_item("next_run_at", schedule.next_run_at.map(|t| t.to_string()))?;
                dict.set_item("last_run_at", schedule.last_run_at.map(|t| t.to_string()))?;
                dict.set_item("created_at", schedule.created_at.to_string())?;
                dict.set_item("updated_at", schedule.updated_at.to_string())?;
                Ok(dict.into())
            })
        })
    }
```

</details>



##### `update_cron_schedule`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">update_cron_schedule</span>(schedule_id: str, cron_expression: str, timezone: str) -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::update_cron_schedule](../../../rust/cloacina/python/bindings/runner.md#update_cron_schedule)

Update a cron schedule's expression and timezone

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `str` | Schedule ID to update |
| `cron_expression` | `str` | New cron expression |
| `timezone` | `str` | New timezone |


<details>
<summary>Source</summary>

```python
    pub fn update_cron_schedule(
        &self,
        schedule_id: String,
        cron_expression: String,
        timezone: String,
        py: Python,
    ) -> PyResult<()> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::UpdateCronSchedule {
            schedule_id,
            cron_expression,
            timezone,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            result.map_err(|e| {
                PyValueError::new_err(format!("Failed to update cron schedule: {}", e))
            })
        })
    }
```

</details>



##### `get_cron_execution_history`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">get_cron_execution_history</span>(schedule_id: str, limit: Optional[int], offset: Optional[int]) -> <span style="color: var(--md-default-fg-color--light);">List[Any]</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::get_cron_execution_history](../../../rust/cloacina/python/bindings/runner.md#get_cron_execution_history)

Get execution history for a specific cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `str` | Schedule ID to get history for |
| `limit` | `Optional[int]` | Maximum number of executions to return (default: 100) |
| `offset` | `Optional[int]` | Number of executions to skip (default: 0) |


**Returns:**

* List of dictionaries containing execution information

<details>
<summary>Source</summary>

```python
    pub fn get_cron_execution_history(
        &self,
        schedule_id: String,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::GetCronExecutionHistory {
            schedule_id,
            limit,
            offset,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            let executions = result.map_err(|e| {
                PyValueError::new_err(format!("Failed to get cron execution history: {}", e))
            })?;

            // Convert executions to Python dictionaries
            let py_executions: Result<Vec<PyObject>, PyErr> = executions
                .into_iter()
                .map(|execution| {
                    Python::with_gil(|py| {
                        let dict = pyo3::types::PyDict::new(py);
                        dict.set_item("id", execution.id.to_string())?;
                        dict.set_item("schedule_id", execution.schedule_id.to_string())?;
                        dict.set_item(
                            "scheduled_time",
                            execution.scheduled_time.map(|t| t.to_string()),
                        )?;
                        dict.set_item("claimed_at", execution.claimed_at.map(|t| t.to_string()))?;
                        dict.set_item(
                            "pipeline_execution_id",
                            execution.pipeline_execution_id.map(|id| id.to_string()),
                        )?;
                        dict.set_item("created_at", execution.created_at.to_string())?;
                        dict.set_item("updated_at", execution.updated_at.to_string())?;
                        Ok(dict.into())
                    })
                })
                .collect();

            py_executions
        })
    }
```

</details>



##### `get_cron_execution_stats`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">get_cron_execution_stats</span>(since: str) -> <span style="color: var(--md-default-fg-color--light);">Any</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::get_cron_execution_stats](../../../rust/cloacina/python/bindings/runner.md#get_cron_execution_stats)

Get execution statistics for cron schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `since` | `str` | Start time for statistics collection (ISO 8601 string) |


**Returns:**

* Dictionary containing execution statistics

<details>
<summary>Source</summary>

```python
    pub fn get_cron_execution_stats(&self, since: String, py: Python) -> PyResult<PyObject> {
        // Parse the since string as ISO 8601 datetime
        let since_dt = chrono::DateTime::parse_from_rfc3339(&since)
            .map_err(|e| PyValueError::new_err(format!("Invalid datetime format: {}", e)))?
            .with_timezone(&chrono::Utc);

        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::GetCronExecutionStats {
            since: since_dt,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            let stats = result.map_err(|e| {
                PyValueError::new_err(format!("Failed to get cron execution stats: {}", e))
            })?;

            // Convert stats to Python dictionary
            Python::with_gil(|py| {
                let dict = pyo3::types::PyDict::new(py);
                dict.set_item("total_executions", stats.total_executions)?;
                dict.set_item("successful_executions", stats.successful_executions)?;
                dict.set_item("lost_executions", stats.lost_executions)?;
                dict.set_item("success_rate", stats.success_rate)?;
                Ok(dict.into())
            })
        })
    }
```

</details>



##### `list_trigger_schedules`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">list_trigger_schedules</span>(enabled_only: Optional[bool], limit: Optional[int], offset: Optional[int]) -> <span style="color: var(--md-default-fg-color--light);">List[Any]</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::list_trigger_schedules](../../../rust/cloacina/python/bindings/runner.md#list_trigger_schedules)

List all trigger schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `enabled_only` | `Optional[bool]` | If True, only return enabled triggers |
| `limit` | `Optional[int]` | Maximum number of triggers to return (default: 100) |
| `offset` | `Optional[int]` | Number of triggers to skip (default: 0) |


**Returns:**

* List of dictionaries containing trigger schedule information

<details>
<summary>Source</summary>

```python
    pub fn list_trigger_schedules(
        &self,
        enabled_only: Option<bool>,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let enabled_only = enabled_only.unwrap_or(false);
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::ListTriggerSchedules {
            enabled_only,
            limit,
            offset,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            let schedules = result.map_err(|e| {
                PyValueError::new_err(format!("Failed to list trigger schedules: {}", e))
            })?;

            // Convert schedules to Python dictionaries
            let py_schedules: Result<Vec<PyObject>, PyErr> = schedules
                .into_iter()
                .map(|schedule| {
                    Python::with_gil(|py| {
                        let dict = pyo3::types::PyDict::new(py);
                        dict.set_item("id", schedule.id.to_string())?;
                        dict.set_item(
                            "trigger_name",
                            schedule.trigger_name.as_deref().unwrap_or(""),
                        )?;
                        dict.set_item("workflow_name", &schedule.workflow_name)?;
                        dict.set_item("poll_interval_ms", schedule.poll_interval_ms.unwrap_or(0))?;
                        dict.set_item("allow_concurrent", schedule.allows_concurrent())?;
                        dict.set_item("enabled", schedule.enabled.is_true())?;
                        dict.set_item(
                            "last_poll_at",
                            schedule.last_poll_at.map(|t| t.to_string()),
                        )?;
                        dict.set_item("created_at", schedule.created_at.to_string())?;
                        dict.set_item("updated_at", schedule.updated_at.to_string())?;
                        Ok(dict.into())
                    })
                })
                .collect();

            py_schedules
        })
    }
```

</details>



##### `get_trigger_schedule`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">get_trigger_schedule</span>(trigger_name: str) -> <span style="color: var(--md-default-fg-color--light);">Optional[Any]</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::get_trigger_schedule](../../../rust/cloacina/python/bindings/runner.md#get_trigger_schedule)

Get details of a specific trigger schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `trigger_name` | `str` | Name of the trigger to retrieve |


**Returns:**

* Dictionary containing trigger schedule information, or None if not found

<details>
<summary>Source</summary>

```python
    pub fn get_trigger_schedule(
        &self,
        trigger_name: String,
        py: Python,
    ) -> PyResult<Option<PyObject>> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::GetTriggerSchedule {
            trigger_name,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            let schedule_opt = result.map_err(|e| {
                PyValueError::new_err(format!("Failed to get trigger schedule: {}", e))
            })?;

            // Convert schedule to Python dictionary if present
            match schedule_opt {
                Some(schedule) => Python::with_gil(|py| {
                    let dict = pyo3::types::PyDict::new(py);
                    dict.set_item("id", schedule.id.to_string())?;
                    dict.set_item(
                        "trigger_name",
                        schedule.trigger_name.as_deref().unwrap_or(""),
                    )?;
                    dict.set_item("workflow_name", &schedule.workflow_name)?;
                    dict.set_item("poll_interval_ms", schedule.poll_interval_ms.unwrap_or(0))?;
                    dict.set_item("allow_concurrent", schedule.allows_concurrent())?;
                    dict.set_item("enabled", schedule.enabled.is_true())?;
                    dict.set_item("last_poll_at", schedule.last_poll_at.map(|t| t.to_string()))?;
                    dict.set_item("created_at", schedule.created_at.to_string())?;
                    dict.set_item("updated_at", schedule.updated_at.to_string())?;
                    Ok(Some(dict.into()))
                }),
                None => Ok(None),
            }
        })
    }
```

</details>



##### `set_trigger_enabled`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_trigger_enabled</span>(trigger_name: str, enabled: bool) -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::set_trigger_enabled](../../../rust/cloacina/python/bindings/runner.md#set_trigger_enabled)

Enable or disable a trigger

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `trigger_name` | `str` | Name of the trigger to modify |
| `enabled` | `bool` | True to enable, False to disable |


<details>
<summary>Source</summary>

```python
    pub fn set_trigger_enabled(
        &self,
        trigger_name: String,
        enabled: bool,
        py: Python,
    ) -> PyResult<()> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::SetTriggerEnabled {
            trigger_name,
            enabled,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            result.map_err(|e| PyValueError::new_err(format!("Failed to update trigger: {}", e)))
        })
    }
```

</details>



##### `get_trigger_execution_history`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">get_trigger_execution_history</span>(trigger_name: str, limit: Optional[int], offset: Optional[int]) -> <span style="color: var(--md-default-fg-color--light);">List[Any]</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::get_trigger_execution_history](../../../rust/cloacina/python/bindings/runner.md#get_trigger_execution_history)

Get execution history for a specific trigger

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `trigger_name` | `str` | Name of the trigger to get history for |
| `limit` | `Optional[int]` | Maximum number of executions to return (default: 100) |
| `offset` | `Optional[int]` | Number of executions to skip (default: 0) |


**Returns:**

* List of dictionaries containing execution information

<details>
<summary>Source</summary>

```python
    pub fn get_trigger_execution_history(
        &self,
        trigger_name: String,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let (response_tx, response_rx) = oneshot::channel();

        let message = RuntimeMessage::GetTriggerExecutionHistory {
            trigger_name,
            limit,
            offset,
            response_tx,
        };

        py.allow_threads(|| {
            self.runtime_handle
                .lock()
                .unwrap()
                .tx
                .send(message)
                .map_err(|_| PyValueError::new_err("Failed to send message to runtime thread"))?;

            let result = response_rx.blocking_recv().map_err(|_| {
                PyValueError::new_err("Failed to receive response from runtime thread")
            })?;

            let executions = result.map_err(|e| {
                PyValueError::new_err(format!("Failed to get trigger execution history: {}", e))
            })?;

            // Convert executions to Python dictionaries
            let py_executions: Result<Vec<PyObject>, PyErr> = executions
                .into_iter()
                .map(|execution| {
                    Python::with_gil(|py| {
                        let dict = pyo3::types::PyDict::new(py);
                        dict.set_item("id", execution.id.to_string())?;
                        dict.set_item("schedule_id", execution.schedule_id.to_string())?;
                        dict.set_item("context_hash", execution.context_hash.as_deref())?;
                        dict.set_item(
                            "pipeline_execution_id",
                            execution.pipeline_execution_id.map(|id| id.to_string()),
                        )?;
                        dict.set_item("started_at", execution.started_at.to_string())?;
                        dict.set_item(
                            "completed_at",
                            execution.completed_at.map(|t| t.to_string()),
                        )?;
                        dict.set_item("created_at", execution.created_at.to_string())?;
                        Ok(dict.into())
                    })
                })
                .collect();

            py_executions
        })
    }
```

</details>



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::__repr__](../../../rust/cloacina/python/bindings/runner.md#__repr__)

String representation

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        "DefaultRunner(thread_separated_async_runtime)".to_string()
    }
```

</details>



##### `__enter__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__enter__</span>(slf: PyRef&lt;Self&gt;) -> <span style="color: var(--md-default-fg-color--light);">PyRef&lt;Self&gt;</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::__enter__](../../../rust/cloacina/python/bindings/runner.md#__enter__)

Context manager entry

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `slf` | `PyRef<Self>` |  |


<details>
<summary>Source</summary>

```python
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
```

</details>



##### `__exit__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__exit__</span>(_exc_type: Optional[Any], _exc_value: Optional[Any], _traceback: Optional[Any]) -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::runner::PyDefaultRunner::__exit__](../../../rust/cloacina/python/bindings/runner.md#__exit__)

Context manager exit - automatically shutdown

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `_exc_type` | `Optional[Any]` |  |
| `_exc_value` | `Optional[Any]` |  |
| `_traceback` | `Optional[Any]` |  |


<details>
<summary>Source</summary>

```python
    pub fn __exit__(
        &self,
        py: Python,
        _exc_type: Option<&Bound<PyAny>>,
        _exc_value: Option<&Bound<PyAny>>,
        _traceback: Option<&Bound<PyAny>>,
    ) -> PyResult<bool> {
        self.shutdown(py)?;
        Ok(false) // Don't suppress exceptions
    }
```

</details>
