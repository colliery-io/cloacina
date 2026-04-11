# cloacina::python::bindings::runner <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


## Structs

### `cloacina::python::bindings::runner::AsyncRuntimeHandle`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


Handle to the background async runtime thread

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tx` | `mpsc :: UnboundedSender < RuntimeMessage >` |  |
| `thread_handle` | `Option < thread :: JoinHandle < () > >` |  |

#### Methods

##### `shutdown` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn shutdown (& mut self) -> Result < () , ShutdownError >
```

Shutdown the runtime thread and wait for it to complete

This method sends a shutdown signal to the runtime thread and waits
for it to complete with a timeout. Errors are logged and returned.

<details>
<summary>Source</summary>

```rust
    fn shutdown(&mut self) -> Result<(), ShutdownError> {
        let start = std::time::Instant::now();
        debug!("Initiating async runtime shutdown");

        // Send shutdown signal
        if let Err(e) = self.tx.send(RuntimeMessage::Shutdown) {
            error!("Failed to send shutdown signal to runtime thread: {:?}", e);
            // Continue anyway - thread might already be dead
        }

        // Wait for thread to finish with timeout
        if let Some(handle) = self.thread_handle.take() {
            // Use a channel to implement timeout on join
            let (done_tx, done_rx) = std::sync::mpsc::channel();

            // Spawn a helper thread to do the blocking join
            let join_thread = thread::spawn(move || {
                let result = handle.join();
                let _ = done_tx.send(result);
            });

            // Wait for completion with timeout
            match done_rx.recv_timeout(SHUTDOWN_TIMEOUT) {
                Ok(Ok(())) => {
                    debug!(
                        duration_ms = start.elapsed().as_millis() as u64,
                        "Async runtime shutdown completed successfully"
                    );
                    Ok(())
                }
                Ok(Err(_panic_payload)) => {
                    error!("Async runtime thread panicked during shutdown");
                    Err(ShutdownError::ThreadPanic)
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    error!(
                        timeout_secs = SHUTDOWN_TIMEOUT.as_secs(),
                        "Async runtime shutdown timed out - thread may be stuck"
                    );
                    // Detach the join thread - we can't wait forever
                    drop(join_thread);
                    Err(ShutdownError::Timeout(SHUTDOWN_TIMEOUT.as_secs()))
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    // Join thread finished but channel was dropped - treat as panic
                    error!("Join thread disconnected unexpectedly");
                    Err(ShutdownError::ThreadPanic)
                }
            }
        } else {
            debug!("Async runtime already shut down");
            Ok(())
        }
    }
```

</details>





### `cloacina::python::bindings::runner::WorkflowResult`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.bindings.runner.WorkflowResult](../../../../cloaca/python/bindings/runner.md#class-workflowresult)

Python wrapper for WorkflowExecutionResult

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `crate :: executor :: WorkflowExecutionResult` |  |

#### Methods

##### `status`

```rust
fn status (& self) -> String
```

> **Python API**: [cloaca.python.bindings.runner.WorkflowResult.status](../../../../cloaca/python/bindings/runner.md#status)

Get the execution status

<details>
<summary>Source</summary>

```rust
    pub fn status(&self) -> String {
        format!("{:?}", self.inner.status)
    }
```

</details>



##### `start_time`

```rust
fn start_time (& self) -> String
```

> **Python API**: [cloaca.python.bindings.runner.WorkflowResult.start_time](../../../../cloaca/python/bindings/runner.md#start_time)

Get execution start time as ISO string

<details>
<summary>Source</summary>

```rust
    pub fn start_time(&self) -> String {
        self.inner.start_time.to_rfc3339()
    }
```

</details>



##### `end_time`

```rust
fn end_time (& self) -> Option < String >
```

> **Python API**: [cloaca.python.bindings.runner.WorkflowResult.end_time](../../../../cloaca/python/bindings/runner.md#end_time)

Get execution end time as ISO string

<details>
<summary>Source</summary>

```rust
    pub fn end_time(&self) -> Option<String> {
        self.inner.end_time.map(|t| t.to_rfc3339())
    }
```

</details>



##### `final_context`

```rust
fn final_context (& self) -> PyContext
```

> **Python API**: [cloaca.python.bindings.runner.WorkflowResult.final_context](../../../../cloaca/python/bindings/runner.md#final_context)

Get the final context

<details>
<summary>Source</summary>

```rust
    pub fn final_context(&self) -> PyContext {
        // Create a new context by cloning the data without the dependency loader
        let new_context = self.inner.final_context.clone_data();
        PyContext::from_rust_context(new_context)
    }
```

</details>



##### `error_message`

```rust
fn error_message (& self) -> Option < & str >
```

> **Python API**: [cloaca.python.bindings.runner.WorkflowResult.error_message](../../../../cloaca/python/bindings/runner.md#error_message)

Get error message if execution failed

<details>
<summary>Source</summary>

```rust
    pub fn error_message(&self) -> Option<&str> {
        self.inner.error_message.as_deref()
    }
```

</details>



##### `__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.bindings.runner.WorkflowResult.__repr__](../../../../cloaca/python/bindings/runner.md#__repr__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!(
            "WorkflowResult(status={}, error={})",
            self.status(),
            self.error_message().unwrap_or("None")
        )
    }
```

</details>



##### `from_result` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_result (result : crate :: executor :: WorkflowExecutionResult) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn from_result(result: crate::executor::WorkflowExecutionResult) -> Self {
        PyWorkflowResult { inner: result }
    }
```

</details>





### `cloacina::python::bindings::runner::DefaultRunner`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.bindings.runner.DefaultRunner](../../../../cloaca/python/bindings/runner.md#class-defaultrunner)

Python wrapper for DefaultRunner

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `runtime_handle` | `Mutex < AsyncRuntimeHandle >` |  |

#### Methods

##### `new`

```rust
fn new (database_url : & str) -> PyResult < Self >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.new](../../../../cloaca/python/bindings/runner.md#new)

Create a new DefaultRunner with database connection

<details>
<summary>Source</summary>

```rust
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

```rust
fn with_config (database_url : & str , config : & super :: context :: PyDefaultRunnerConfig ,) -> PyResult < PyDefaultRunner >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.with_config](../../../../cloaca/python/bindings/runner.md#with_config)

Create a new DefaultRunner with custom configuration

<details>
<summary>Source</summary>

```rust
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

```rust
fn with_schema (database_url : & str , schema : & str) -> PyResult < PyDefaultRunner >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.with_schema](../../../../cloaca/python/bindings/runner.md#with_schema)

Create a new DefaultRunner with PostgreSQL schema-based multi-tenancy

This method enables multi-tenant deployments by using PostgreSQL schemas
for complete data isolation between tenants. Each tenant gets their own
schema with independent tables, migrations, and data.
Note: This method requires a PostgreSQL database. SQLite does not support
database schemas.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database_url` | `-` | PostgreSQL connection string |
| `schema` | `-` | Schema name for tenant isolation (alphanumeric + underscores only) |


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

```rust
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

```rust
fn execute (& self , workflow_name : & str , context : & PyContext , py : Python ,) -> PyResult < PyWorkflowResult >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.execute](../../../../cloaca/python/bindings/runner.md#execute)

Execute a workflow by name with context

<details>
<summary>Source</summary>

```rust
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

```rust
fn start (& self) -> PyResult < () >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.start](../../../../cloaca/python/bindings/runner.md#start)

Start the runner (task scheduler and executor)

<details>
<summary>Source</summary>

```rust
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

```rust
fn stop (& self) -> PyResult < () >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.stop](../../../../cloaca/python/bindings/runner.md#stop)

Stop the runner

<details>
<summary>Source</summary>

```rust
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

```rust
fn shutdown (& self , py : Python) -> PyResult < () >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.shutdown](../../../../cloaca/python/bindings/runner.md#shutdown)

Shutdown the runner and cleanup resources

This method sends a shutdown signal to the async runtime thread and waits
for it to complete (with a 5-second timeout). Errors during shutdown are
raised as Python exceptions.

<details>
<summary>Source</summary>

```rust
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

```rust
fn register_cron_workflow (& self , workflow_name : String , cron_expression : String , timezone : String , py : Python ,) -> PyResult < String >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.register_cron_workflow](../../../../cloaca/python/bindings/runner.md#register_cron_workflow)

Register a cron workflow for automatic execution at scheduled times

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `-` | Name of the workflow to execute |
| `cron_expression` | `-` | Standard cron expression (e.g., "0 2 * * *" for daily at 2 AM) |
| `timezone` | `-` | Timezone for cron interpretation (e.g., "UTC", "America/New_York") |


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

```rust
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

```rust
fn list_cron_schedules (& self , enabled_only : Option < bool > , limit : Option < i64 > , offset : Option < i64 > , py : Python ,) -> PyResult < Vec < PyObject > >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.list_cron_schedules](../../../../cloaca/python/bindings/runner.md#list_cron_schedules)

List all cron schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `enabled_only` | `-` | If True, only return enabled schedules |
| `limit` | `-` | Maximum number of schedules to return (default: 100) |
| `offset` | `-` | Number of schedules to skip (default: 0) |


**Returns:**

* List of dictionaries containing schedule information

<details>
<summary>Source</summary>

```rust
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

```rust
fn set_cron_schedule_enabled (& self , schedule_id : String , enabled : bool , py : Python ,) -> PyResult < () >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.set_cron_schedule_enabled](../../../../cloaca/python/bindings/runner.md#set_cron_schedule_enabled)

Enable or disable a cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `-` | Schedule ID to modify |
| `enabled` | `-` | True to enable, False to disable |


<details>
<summary>Source</summary>

```rust
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

```rust
fn delete_cron_schedule (& self , schedule_id : String , py : Python) -> PyResult < () >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.delete_cron_schedule](../../../../cloaca/python/bindings/runner.md#delete_cron_schedule)

Delete a cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `-` | Schedule ID to delete |


<details>
<summary>Source</summary>

```rust
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

```rust
fn get_cron_schedule (& self , schedule_id : String , py : Python) -> PyResult < PyObject >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.get_cron_schedule](../../../../cloaca/python/bindings/runner.md#get_cron_schedule)

Get details of a specific cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `-` | Schedule ID to retrieve |


**Returns:**

* Dictionary containing schedule information

<details>
<summary>Source</summary>

```rust
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

```rust
fn update_cron_schedule (& self , schedule_id : String , cron_expression : String , timezone : String , py : Python ,) -> PyResult < () >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.update_cron_schedule](../../../../cloaca/python/bindings/runner.md#update_cron_schedule)

Update a cron schedule's expression and timezone

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `-` | Schedule ID to update |
| `cron_expression` | `-` | New cron expression |
| `timezone` | `-` | New timezone |


<details>
<summary>Source</summary>

```rust
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

```rust
fn get_cron_execution_history (& self , schedule_id : String , limit : Option < i64 > , offset : Option < i64 > , py : Python ,) -> PyResult < Vec < PyObject > >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.get_cron_execution_history](../../../../cloaca/python/bindings/runner.md#get_cron_execution_history)

Get execution history for a specific cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `-` | Schedule ID to get history for |
| `limit` | `-` | Maximum number of executions to return (default: 100) |
| `offset` | `-` | Number of executions to skip (default: 0) |


**Returns:**

* List of dictionaries containing execution information

<details>
<summary>Source</summary>

```rust
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

```rust
fn get_cron_execution_stats (& self , since : String , py : Python) -> PyResult < PyObject >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.get_cron_execution_stats](../../../../cloaca/python/bindings/runner.md#get_cron_execution_stats)

Get execution statistics for cron schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `since` | `-` | Start time for statistics collection (ISO 8601 string) |


**Returns:**

* Dictionary containing execution statistics

<details>
<summary>Source</summary>

```rust
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

```rust
fn list_trigger_schedules (& self , enabled_only : Option < bool > , limit : Option < i64 > , offset : Option < i64 > , py : Python ,) -> PyResult < Vec < PyObject > >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.list_trigger_schedules](../../../../cloaca/python/bindings/runner.md#list_trigger_schedules)

List all trigger schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `enabled_only` | `-` | If True, only return enabled triggers |
| `limit` | `-` | Maximum number of triggers to return (default: 100) |
| `offset` | `-` | Number of triggers to skip (default: 0) |


**Returns:**

* List of dictionaries containing trigger schedule information

<details>
<summary>Source</summary>

```rust
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

```rust
fn get_trigger_schedule (& self , trigger_name : String , py : Python ,) -> PyResult < Option < PyObject > >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.get_trigger_schedule](../../../../cloaca/python/bindings/runner.md#get_trigger_schedule)

Get details of a specific trigger schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `trigger_name` | `-` | Name of the trigger to retrieve |


**Returns:**

* Dictionary containing trigger schedule information, or None if not found

<details>
<summary>Source</summary>

```rust
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

```rust
fn set_trigger_enabled (& self , trigger_name : String , enabled : bool , py : Python ,) -> PyResult < () >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.set_trigger_enabled](../../../../cloaca/python/bindings/runner.md#set_trigger_enabled)

Enable or disable a trigger

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `trigger_name` | `-` | Name of the trigger to modify |
| `enabled` | `-` | True to enable, False to disable |


<details>
<summary>Source</summary>

```rust
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

```rust
fn get_trigger_execution_history (& self , trigger_name : String , limit : Option < i64 > , offset : Option < i64 > , py : Python ,) -> PyResult < Vec < PyObject > >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.get_trigger_execution_history](../../../../cloaca/python/bindings/runner.md#get_trigger_execution_history)

Get execution history for a specific trigger

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `trigger_name` | `-` | Name of the trigger to get history for |
| `limit` | `-` | Maximum number of executions to return (default: 100) |
| `offset` | `-` | Number of executions to skip (default: 0) |


**Returns:**

* List of dictionaries containing execution information

<details>
<summary>Source</summary>

```rust
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

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.__repr__](../../../../cloaca/python/bindings/runner.md#__repr__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        "DefaultRunner(thread_separated_async_runtime)".to_string()
    }
```

</details>



##### `__enter__`

```rust
fn __enter__ (slf : PyRef < Self >) -> PyRef < Self >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.__enter__](../../../../cloaca/python/bindings/runner.md#__enter__)

Context manager entry

<details>
<summary>Source</summary>

```rust
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
```

</details>



##### `__exit__`

```rust
fn __exit__ (& self , py : Python , _exc_type : Option < & Bound < PyAny > > , _exc_value : Option < & Bound < PyAny > > , _traceback : Option < & Bound < PyAny > > ,) -> PyResult < bool >
```

> **Python API**: [cloaca.python.bindings.runner.DefaultRunner.__exit__](../../../../cloaca/python/bindings/runner.md#__exit__)

Context manager exit - automatically shutdown

<details>
<summary>Source</summary>

```rust
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





## Enums

### `cloacina::python::bindings::runner::ShutdownError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during async runtime shutdown

#### Variants

- **`ChannelClosed`** - Failed to send shutdown signal to runtime thread
- **`ThreadPanic`** - Runtime thread panicked during shutdown
- **`Timeout`** - Shutdown timed out waiting for thread to complete



### `cloacina::python::bindings::runner::RuntimeMessage` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


Message types for communication with the async runtime thread

#### Variants

- **`Execute`**
- **`RegisterCronWorkflow`**
- **`ListCronSchedules`**
- **`SetCronScheduleEnabled`**
- **`DeleteCronSchedule`**
- **`GetCronSchedule`**
- **`UpdateCronSchedule`**
- **`GetCronExecutionHistory`**
- **`GetCronExecutionStats`**
- **`ListTriggerSchedules`**
- **`GetTriggerSchedule`**
- **`SetTriggerEnabled`**
- **`GetTriggerExecutionHistory`**
- **`Shutdown`**
