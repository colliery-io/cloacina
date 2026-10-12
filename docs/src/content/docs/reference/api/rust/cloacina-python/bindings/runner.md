---
title: "cloacina-python::bindings::runner"
description: "API reference of the Rust module cloacina-python::bindings::runner."
sidebar:
  order: 23
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


## Structs

### <span id="struct.AsyncRuntimeHandle"></span>`struct AsyncRuntimeHandle`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


Handle to the background async runtime thread

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tx` | `mpsc :: UnboundedSender < RuntimeMessage >` |  |
| `thread_handle` | `Option < thread :: JoinHandle < () > >` |  |

#### Methods

##### <span id="AsyncRuntimeHandle.shutdown"></span>`shutdown`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn shutdown (& mut self) -> Result < () , ShutdownError >
```

Shutdown the runtime thread and wait for it to complete

<details>
<summary>Source</summary>

```rust
    fn shutdown(&mut self) -> Result<(), ShutdownError> {
        let start = std::time::Instant::now();
        debug!("Initiating async runtime shutdown");

        // Send shutdown signal
        if let Err(e) = self.tx.send(RuntimeMessage::Shutdown) {
            error!("Failed to send shutdown signal to runtime thread: {:?}", e);
        }

        // Wait for thread to finish with timeout
        if let Some(handle) = self.thread_handle.take() {
            let (done_tx, done_rx) = std::sync::mpsc::channel();

            let join_thread = thread::spawn(move || {
                let result = handle.join();
                let _ = done_tx.send(result);
            });

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
                    drop(join_thread);
                    Err(ShutdownError::Timeout(SHUTDOWN_TIMEOUT.as_secs()))
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
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





### <span id="struct.PyWorkflowResult"></span>`class WorkflowResult`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.bindings.runner.WorkflowResult](../../../../cloacina-python/bindings/runner/#class.WorkflowResult)

Python wrapper for WorkflowExecutionResult

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `cloacina :: executor :: WorkflowExecutionResult` |  |

#### Methods

##### <span id="PyWorkflowResult.status"></span>`status`

```rust
fn status (& self) -> String
```

> **Python API**: [cloacina-python.bindings.runner.WorkflowResult.status](../../../../cloacina-python/bindings/runner/#WorkflowResult.status)

<details>
<summary>Source</summary>

```rust
    pub fn status(&self) -> String {
        format!("{:?}", self.inner.status)
    }
```

</details>



##### <span id="PyWorkflowResult.start_time"></span>`start_time`

```rust
fn start_time (& self) -> String
```

> **Python API**: [cloacina-python.bindings.runner.WorkflowResult.start_time](../../../../cloacina-python/bindings/runner/#WorkflowResult.start_time)

<details>
<summary>Source</summary>

```rust
    pub fn start_time(&self) -> String {
        self.inner.start_time.to_rfc3339()
    }
```

</details>



##### <span id="PyWorkflowResult.end_time"></span>`end_time`

```rust
fn end_time (& self) -> Option < String >
```

> **Python API**: [cloacina-python.bindings.runner.WorkflowResult.end_time](../../../../cloacina-python/bindings/runner/#WorkflowResult.end_time)

<details>
<summary>Source</summary>

```rust
    pub fn end_time(&self) -> Option<String> {
        self.inner.end_time.map(|t| t.to_rfc3339())
    }
```

</details>



##### <span id="PyWorkflowResult.final_context"></span>`final_context`

```rust
fn final_context (& self) -> PyContext
```

> **Python API**: [cloacina-python.bindings.runner.WorkflowResult.final_context](../../../../cloacina-python/bindings/runner/#WorkflowResult.final_context)

<details>
<summary>Source</summary>

```rust
    pub fn final_context(&self) -> PyContext {
        let new_context = self.inner.final_context.clone_data();
        PyContext::from_rust_context(new_context)
    }
```

</details>



##### <span id="PyWorkflowResult.error_message"></span>`error_message`

```rust
fn error_message (& self) -> Option < & str >
```

> **Python API**: [cloacina-python.bindings.runner.WorkflowResult.error_message](../../../../cloacina-python/bindings/runner/#WorkflowResult.error_message)

<details>
<summary>Source</summary>

```rust
    pub fn error_message(&self) -> Option<&str> {
        self.inner.error_message.as_deref()
    }
```

</details>



##### <span id="PyWorkflowResult.__repr__"></span>`__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloacina-python.bindings.runner.WorkflowResult.__repr__](../../../../cloacina-python/bindings/runner/#WorkflowResult.__repr__)

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



##### <span id="PyWorkflowResult.from_result"></span>`from_result`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn from_result (result : cloacina :: executor :: WorkflowExecutionResult) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn from_result(result: cloacina::executor::WorkflowExecutionResult) -> Self {
        PyWorkflowResult { inner: result }
    }
```

</details>





### <span id="struct.PyDefaultRunner"></span>`class DefaultRunner`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.bindings.runner.DefaultRunner](../../../../cloacina-python/bindings/runner/#class.DefaultRunner)

Python wrapper for DefaultRunner

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `runtime_handle` | `Arc < Mutex < AsyncRuntimeHandle > >` |  |

#### Methods

##### <span id="PyDefaultRunner.send_and_recv"></span>`send_and_recv`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn send_and_recv < T : Send > (& self , message : RuntimeMessage , response_rx : oneshot :: Receiver < Result < T , cloacina :: executor :: WorkflowExecutionError > > , py : Python , error_context : & str ,) -> PyResult < T >
```

Send a message to the runtime thread and block until a response arrives.

Releases the GIL while waiting so Python threads can proceed.

<details>
<summary>Source</summary>

```rust
    fn send_and_recv<T: Send>(
        &self,
        message: RuntimeMessage,
        response_rx: oneshot::Receiver<Result<T, cloacina::executor::WorkflowExecutionError>>,
        py: Python,
        error_context: &str,
    ) -> PyResult<T> {
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
            result.map_err(|e| PyValueError::new_err(format!("{}: {}", error_context, e)))
        })
    }
```

</details>



##### <span id="PyDefaultRunner.new"></span>`new`

```rust
fn new (py : Python < '_ > , database_url : & str) -> PyResult < Self >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.new](../../../../cloacina-python/bindings/runner/#DefaultRunner.new)

Create a new DefaultRunner with database connection

<details>
<summary>Source</summary>

```rust
    pub fn new(py: Python<'_>, database_url: &str) -> PyResult<Self> {
        let database_url = database_url.to_string();
        // Share the caller's ScopedRuntime (installed by the cloaca pymodule
        // init, and where `@task`/`WorkflowBuilder` register) so the runner's
        // scheduler can find workflows registered before the runner existed.
        let rt_arc = crate::runtime_scope::current_runtime();
        spawn_runtime(py, move |rt| {
            info!(
                "Creating DefaultRunner with database_url: {}",
                cloacina::logging::mask_db_url(&database_url)
            );
            let mut builder = cloacina::DefaultRunner::builder().database_url(&database_url);
            if let Some(arc) = rt_arc {
                builder = builder.runtime_arc(arc);
            }
            rt.block_on(builder.build())
        })
    }
```

</details>



##### <span id="PyDefaultRunner.with_config"></span>`with_config`

```rust
fn with_config (py : Python < '_ > , database_url : & str , config : & super :: context :: PyDefaultRunnerConfig ,) -> PyResult < PyDefaultRunner >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.with_config](../../../../cloacina-python/bindings/runner/#DefaultRunner.with_config)

Create a new DefaultRunner with custom configuration

<details>
<summary>Source</summary>

```rust
    pub fn with_config(
        py: Python<'_>,
        database_url: &str,
        config: &super::context::PyDefaultRunnerConfig,
    ) -> PyResult<PyDefaultRunner> {
        let database_url = database_url.to_string();
        let rust_config = config.to_rust_config();
        let rt_arc = crate::runtime_scope::current_runtime();
        spawn_runtime(py, move |rt| {
            let mut builder = cloacina::DefaultRunner::builder()
                .database_url(&database_url)
                .with_config(rust_config);
            if let Some(arc) = rt_arc {
                builder = builder.runtime_arc(arc);
            }
            rt.block_on(builder.build())
        })
    }
```

</details>



##### <span id="PyDefaultRunner.with_schema"></span>`with_schema`

```rust
fn with_schema (py : Python < '_ > , database_url : & str , schema : & str ,) -> PyResult < PyDefaultRunner >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.with_schema](../../../../cloacina-python/bindings/runner/#DefaultRunner.with_schema)

Create a new DefaultRunner with PostgreSQL schema-based multi-tenancy

Each tenant gets their own schema with independent tables, migrations,
and data. Requires a PostgreSQL database.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database_url` | `-` | PostgreSQL connection string |
| `schema` | `-` | Schema name for tenant isolation (alphanumeric + underscores only) |


<details>
<summary>Source</summary>

```rust
    pub fn with_schema(
        py: Python<'_>,
        database_url: &str,
        schema: &str,
    ) -> PyResult<PyDefaultRunner> {
        if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
            return Err(PyValueError::new_err(
                "Schema-based multi-tenancy requires PostgreSQL. \
                 SQLite does not support database schemas. \
                 Use a PostgreSQL URL like 'postgres://user:pass@host/db'",
            ));
        }
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
        let rt_arc = crate::runtime_scope::current_runtime();
        spawn_runtime(py, move |rt| {
            info!(
                "Creating DefaultRunner with schema: {} and database_url: {}",
                schema,
                cloacina::logging::mask_db_url(&database_url)
            );
            let mut builder = cloacina::DefaultRunner::builder()
                .database_url(&database_url)
                .schema(&schema);
            if let Some(arc) = rt_arc {
                builder = builder.runtime_arc(arc);
            }
            rt.block_on(builder.build())
        })
    }
```

</details>



##### <span id="PyDefaultRunner.execute"></span>`execute`

```rust
fn execute (& self , workflow_name : & str , context : & PyContext , py : Python ,) -> PyResult < PyWorkflowResult >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.execute](../../../../cloacina-python/bindings/runner/#DefaultRunner.execute)

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
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::Execute {
            workflow_name: workflow_name.to_string(),
            context: context.clone_inner(),
            response_tx,
        };
        let result = self.send_and_recv(message, response_rx, py, "Workflow execution failed")?;
        Ok(PyWorkflowResult::from_result(result))
    }
```

</details>



##### <span id="PyDefaultRunner.shutdown"></span>`shutdown`

```rust
fn shutdown (& self , py : Python) -> PyResult < () >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.shutdown](../../../../cloacina-python/bindings/runner/#DefaultRunner.shutdown)

Shutdown the runner and cleanup resources

<details>
<summary>Source</summary>

```rust
    pub fn shutdown(&self, py: Python) -> PyResult<()> {
        info!("Starting async runtime shutdown from Python");
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



##### <span id="PyDefaultRunner.register_workflow_instance"></span>`register_workflow_instance`

```rust
fn register_workflow_instance (& self , workflow_name : String , instance_name : String , cron_expression : String , timezone : String , params : & crate :: context :: PyContext , py : Python ,) -> PyResult < String >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.register_workflow_instance](../../../../cloacina-python/bindings/runner/#DefaultRunner.register_workflow_instance)

Register a cron workflow for automatic execution at scheduled times

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `-` | Name of the workflow to execute |
| `cron_expression` | `-` | Standard cron expression (e.g., "0 2 * * *") |
| `timezone` | `-` | Timezone for cron interpretation (e.g., "UTC") |


**Returns:**

* Schedule ID as a string Register a NAMED, PARAMETERIZED cron instance (CLOACI-I-0116): `params` (a dict) is the instance's fully-resolved bound parameters, merged into the context at every fire under flat top-level keys.

<details>
<summary>Source</summary>

```rust
    pub fn register_workflow_instance(
        &self,
        workflow_name: String,
        instance_name: String,
        cron_expression: String,
        timezone: String,
        params: &crate::context::PyContext,
        py: Python,
    ) -> PyResult<String> {
        // A cloaca Context carries the params dict; its JSON form is the
        // fully-resolved instance param object persisted on the schedule.
        let params_json = params.to_json()?;
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::RegisterWorkflowInstance {
            workflow_name,
            instance_name,
            cron_expression,
            timezone,
            params_json,
            response_tx,
        };
        self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to register workflow instance",
        )
    }
```

</details>



##### <span id="PyDefaultRunner.register_cron_workflow"></span>`register_cron_workflow`

```rust
fn register_cron_workflow (& self , workflow_name : String , cron_expression : String , timezone : String , py : Python ,) -> PyResult < String >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.register_cron_workflow](../../../../cloacina-python/bindings/runner/#DefaultRunner.register_cron_workflow)

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
        self.send_and_recv(message, response_rx, py, "Failed to register cron workflow")
    }
```

</details>



##### <span id="PyDefaultRunner.list_cron_schedules"></span>`list_cron_schedules`

```rust
fn list_cron_schedules (& self , enabled_only : Option < bool > , limit : Option < i64 > , offset : Option < i64 > , py : Python ,) -> PyResult < Vec < PyObject > >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.list_cron_schedules](../../../../cloacina-python/bindings/runner/#DefaultRunner.list_cron_schedules)

List all cron schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `enabled_only` | `-` | If True, only return enabled schedules |
| `limit` | `-` | Maximum number of schedules to return (default: 100) |
| `offset` | `-` | Number of schedules to skip (default: 0) |


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
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::ListCronSchedules {
            enabled_only: enabled_only.unwrap_or(false),
            limit: limit.unwrap_or(100),
            offset: offset.unwrap_or(0),
            response_tx,
        };
        let schedules =
            self.send_and_recv(message, response_rx, py, "Failed to list cron schedules")?;
        schedules
            .into_iter()
            .map(|s| schedule_to_cron_dict(s, py))
            .collect()
    }
```

</details>



##### <span id="PyDefaultRunner.set_cron_schedule_enabled"></span>`set_cron_schedule_enabled`

```rust
fn set_cron_schedule_enabled (& self , schedule_id : String , enabled : bool , py : Python ,) -> PyResult < () >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.set_cron_schedule_enabled](../../../../cloacina-python/bindings/runner/#DefaultRunner.set_cron_schedule_enabled)

Enable or disable a cron schedule

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
        self.send_and_recv(message, response_rx, py, "Failed to update cron schedule")
    }
```

</details>



##### <span id="PyDefaultRunner.delete_cron_schedule"></span>`delete_cron_schedule`

```rust
fn delete_cron_schedule (& self , schedule_id : String , py : Python) -> PyResult < () >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.delete_cron_schedule](../../../../cloacina-python/bindings/runner/#DefaultRunner.delete_cron_schedule)

Delete a cron schedule

<details>
<summary>Source</summary>

```rust
    pub fn delete_cron_schedule(&self, schedule_id: String, py: Python) -> PyResult<()> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::DeleteCronSchedule {
            schedule_id,
            response_tx,
        };
        self.send_and_recv(message, response_rx, py, "Failed to delete cron schedule")
    }
```

</details>



##### <span id="PyDefaultRunner.get_cron_schedule"></span>`get_cron_schedule`

```rust
fn get_cron_schedule (& self , schedule_id : String , py : Python) -> PyResult < PyObject >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.get_cron_schedule](../../../../cloacina-python/bindings/runner/#DefaultRunner.get_cron_schedule)

Get details of a specific cron schedule

<details>
<summary>Source</summary>

```rust
    pub fn get_cron_schedule(&self, schedule_id: String, py: Python) -> PyResult<PyObject> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::GetCronSchedule {
            schedule_id,
            response_tx,
        };
        let schedule =
            self.send_and_recv(message, response_rx, py, "Failed to get cron schedule")?;
        schedule_to_cron_dict(schedule, py)
    }
```

</details>



##### <span id="PyDefaultRunner.update_cron_schedule"></span>`update_cron_schedule`

```rust
fn update_cron_schedule (& self , schedule_id : String , cron_expression : String , timezone : String , py : Python ,) -> PyResult < () >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.update_cron_schedule](../../../../cloacina-python/bindings/runner/#DefaultRunner.update_cron_schedule)

Update a cron schedule's expression and timezone

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
        self.send_and_recv(message, response_rx, py, "Failed to update cron schedule")
    }
```

</details>



##### <span id="PyDefaultRunner.get_cron_execution_history"></span>`get_cron_execution_history`

```rust
fn get_cron_execution_history (& self , schedule_id : String , limit : Option < i64 > , offset : Option < i64 > , py : Python ,) -> PyResult < Vec < PyObject > >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.get_cron_execution_history](../../../../cloacina-python/bindings/runner/#DefaultRunner.get_cron_execution_history)

Get execution history for a specific cron schedule

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
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::GetCronExecutionHistory {
            schedule_id,
            limit: limit.unwrap_or(100),
            offset: offset.unwrap_or(0),
            response_tx,
        };
        let executions = self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to get cron execution history",
        )?;
        executions
            .into_iter()
            .map(|e| cron_execution_to_dict(e, py))
            .collect()
    }
```

</details>



##### <span id="PyDefaultRunner.get_cron_execution_stats"></span>`get_cron_execution_stats`

```rust
fn get_cron_execution_stats (& self , since : String , py : Python) -> PyResult < PyObject >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.get_cron_execution_stats](../../../../cloacina-python/bindings/runner/#DefaultRunner.get_cron_execution_stats)

Get execution statistics for cron schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `since` | `-` | Start time for statistics collection (ISO 8601 string) |


<details>
<summary>Source</summary>

```rust
    pub fn get_cron_execution_stats(&self, since: String, py: Python) -> PyResult<PyObject> {
        let since_dt = chrono::DateTime::parse_from_rfc3339(&since)
            .map_err(|e| PyValueError::new_err(format!("Invalid datetime format: {}", e)))?
            .with_timezone(&chrono::Utc);

        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::GetCronExecutionStats {
            since: since_dt,
            response_tx,
        };
        let stats = self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to get cron execution stats",
        )?;

        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("total_executions", stats.total_executions)?;
        dict.set_item("successful_executions", stats.successful_executions)?;
        dict.set_item("lost_executions", stats.lost_executions)?;
        dict.set_item("success_rate", stats.success_rate)?;
        Ok(dict.into())
    }
```

</details>



##### <span id="PyDefaultRunner.list_trigger_schedules"></span>`list_trigger_schedules`

```rust
fn list_trigger_schedules (& self , enabled_only : Option < bool > , limit : Option < i64 > , offset : Option < i64 > , py : Python ,) -> PyResult < Vec < PyObject > >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.list_trigger_schedules](../../../../cloacina-python/bindings/runner/#DefaultRunner.list_trigger_schedules)

List all trigger schedules

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
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::ListTriggerSchedules {
            enabled_only: enabled_only.unwrap_or(false),
            limit: limit.unwrap_or(100),
            offset: offset.unwrap_or(0),
            response_tx,
        };
        let schedules =
            self.send_and_recv(message, response_rx, py, "Failed to list trigger schedules")?;
        schedules
            .into_iter()
            .map(|s| schedule_to_trigger_dict(s, py))
            .collect()
    }
```

</details>



##### <span id="PyDefaultRunner.get_trigger_schedule"></span>`get_trigger_schedule`

```rust
fn get_trigger_schedule (& self , trigger_name : String , py : Python ,) -> PyResult < Option < PyObject > >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.get_trigger_schedule](../../../../cloacina-python/bindings/runner/#DefaultRunner.get_trigger_schedule)

Get details of a specific trigger schedule

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
        let schedule_opt =
            self.send_and_recv(message, response_rx, py, "Failed to get trigger schedule")?;
        match schedule_opt {
            Some(schedule) => Ok(Some(schedule_to_trigger_dict(schedule, py)?)),
            None => Ok(None),
        }
    }
```

</details>



##### <span id="PyDefaultRunner.set_trigger_enabled"></span>`set_trigger_enabled`

```rust
fn set_trigger_enabled (& self , trigger_name : String , enabled : bool , py : Python ,) -> PyResult < () >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.set_trigger_enabled](../../../../cloacina-python/bindings/runner/#DefaultRunner.set_trigger_enabled)

Enable or disable a trigger

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
        self.send_and_recv(message, response_rx, py, "Failed to update trigger")
    }
```

</details>



##### <span id="PyDefaultRunner.get_trigger_execution_history"></span>`get_trigger_execution_history`

```rust
fn get_trigger_execution_history (& self , trigger_name : String , limit : Option < i64 > , offset : Option < i64 > , py : Python ,) -> PyResult < Vec < PyObject > >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.get_trigger_execution_history](../../../../cloacina-python/bindings/runner/#DefaultRunner.get_trigger_execution_history)

Get execution history for a specific trigger

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
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::GetTriggerExecutionHistory {
            trigger_name,
            limit: limit.unwrap_or(100),
            offset: offset.unwrap_or(0),
            response_tx,
        };
        let executions = self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to get trigger execution history",
        )?;
        executions
            .into_iter()
            .map(|e| trigger_execution_to_dict(e, py))
            .collect()
    }
```

</details>



##### <span id="PyDefaultRunner.subscribe_workflow_to_reactor"></span>`subscribe_workflow_to_reactor`

```rust
fn subscribe_workflow_to_reactor (& self , reactor : String , workflow : String , tenant : Option < String > , when : Option < String > , py : Python ,) -> PyResult < String >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.subscribe_workflow_to_reactor](../../../../cloacina-python/bindings/runner/#DefaultRunner.subscribe_workflow_to_reactor)

Subscribe a workflow to a reactor's firings.

Each future fire of `reactor` (within `tenant`) will dispatch
`workflow` via the unified scheduler's reactor poll tick. The
dispatched workflow's input context carries the boundary cache
from the firing.
Idempotent — calling twice with the same `(reactor, workflow,
tenant)` upserts; a later call's `when` predicate replaces the
earlier one's.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `reactor` | `-` | The reactor name. |
| `workflow` | `-` | The workflow to dispatch on each firing. |
| `tenant` | `-` | Tenant scope. Defaults to `"public"` if omitted. |
| `when` | `-` | Optional CEL filter expression (CLOACI-T-0602). When provided, the scheduler evaluates it against the firing payload and only dispatches when it evaluates to true. The watermark still advances when it evaluates to false. Variables available: `payload` (dict, keys are boundary source names), `reactor` (str), `tenant` (str). Invalid CEL is rejected immediately with a ValueError — no row written. |


**Examples:**

```python
# Unfiltered — fires on every firing.
runner.subscribe_workflow_to_reactor("pricing", "alert")

# Filtered — fires only when the boundary's `quote` source
# carries a price above $100 in us-east.
runner.subscribe_workflow_to_reactor(
    "pricing", "alert",
    when="payload.quote.price > 100 && payload.quote.region == 'us-east'",
)
```

<details>
<summary>Source</summary>

```rust
    pub fn subscribe_workflow_to_reactor(
        &self,
        reactor: String,
        workflow: String,
        tenant: Option<String>,
        when: Option<String>,
        py: Python,
    ) -> PyResult<String> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::SubscribeWorkflowToReactor {
            reactor,
            workflow,
            tenant,
            predicate: when,
            response_tx,
        };
        self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to subscribe workflow to reactor",
        )
    }
```

</details>



##### <span id="PyDefaultRunner.unsubscribe_workflow_from_reactor"></span>`unsubscribe_workflow_from_reactor`

```rust
fn unsubscribe_workflow_from_reactor (& self , reactor : String , workflow : String , tenant : Option < String > , py : Python ,) -> PyResult < bool >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.unsubscribe_workflow_from_reactor](../../../../cloacina-python/bindings/runner/#DefaultRunner.unsubscribe_workflow_from_reactor)

Remove a workflow-to-reactor subscription.

Returns True if a row was deleted, False if no subscription
matched `(reactor, workflow, tenant)`.

<details>
<summary>Source</summary>

```rust
    pub fn unsubscribe_workflow_from_reactor(
        &self,
        reactor: String,
        workflow: String,
        tenant: Option<String>,
        py: Python,
    ) -> PyResult<bool> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::UnsubscribeWorkflowFromReactor {
            reactor,
            workflow,
            tenant,
            response_tx,
        };
        self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to unsubscribe workflow from reactor",
        )
    }
```

</details>



##### <span id="PyDefaultRunner.list_reactor_subscriptions"></span>`list_reactor_subscriptions`

```rust
fn list_reactor_subscriptions (& self , tenant : Option < String > , py : Python ,) -> PyResult < Vec < PyObject > >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.list_reactor_subscriptions](../../../../cloacina-python/bindings/runner/#DefaultRunner.list_reactor_subscriptions)

List enabled reactor subscriptions for a tenant.

<details>
<summary>Source</summary>

```rust
    pub fn list_reactor_subscriptions(
        &self,
        tenant: Option<String>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::ListReactorSubscriptions {
            tenant,
            response_tx,
        };
        let subs = self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to list reactor subscriptions",
        )?;
        subs.into_iter()
            .map(|s| reactor_subscription_to_dict(s, py))
            .collect()
    }
```

</details>



##### <span id="PyDefaultRunner.__repr__"></span>`__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.__repr__](../../../../cloacina-python/bindings/runner/#DefaultRunner.__repr__)

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        "DefaultRunner(thread_separated_async_runtime)".to_string()
    }
```

</details>



##### <span id="PyDefaultRunner.__enter__"></span>`__enter__`

```rust
fn __enter__ (slf : PyRef < Self >) -> PyRef < Self >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.__enter__](../../../../cloacina-python/bindings/runner/#DefaultRunner.__enter__)

<details>
<summary>Source</summary>

```rust
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
```

</details>



##### <span id="PyDefaultRunner.__exit__"></span>`__exit__`

```rust
fn __exit__ (& self , py : Python , _exc_type : Option < & Bound < PyAny > > , _exc_value : Option < & Bound < PyAny > > , _traceback : Option < & Bound < PyAny > > ,) -> PyResult < bool >
```

> **Python API**: [cloacina-python.bindings.runner.DefaultRunner.__exit__](../../../../cloacina-python/bindings/runner/#DefaultRunner.__exit__)

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

### <span id="enum.ShutdownError"></span>`enum ShutdownError`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Errors that can occur during async runtime shutdown

#### Variants

- **`ChannelClosed`** - Failed to send shutdown signal to runtime thread
- **`ThreadPanic`** - Runtime thread panicked during shutdown
- **`Timeout`** - Shutdown timed out waiting for thread to complete



### <span id="enum.RuntimeMessage"></span>`enum RuntimeMessage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


Message types for communication with the async runtime thread

#### Variants

- **`Execute`**
- **`RegisterCronWorkflow`**
- **`RegisterWorkflowInstance`**
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
- **`SubscribeWorkflowToReactor`**
- **`UnsubscribeWorkflowFromReactor`**
- **`ListReactorSubscriptions`**
- **`Shutdown`**



## Functions

### <span id="fn._shutdown_all_runners"></span>`fn _shutdown_all_runners`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.bindings.runner._shutdown_all_runners](../../../../cloacina-python/bindings/runner/#fn._shutdown_all_runners)

```rust
fn _shutdown_all_runners (py : Python) -> PyResult < () >
```

Shut down every still-live runner. Registered with Python's `atexit` by the wheel's `#[pymodule]` init; also callable directly (idempotent — shutdown of an already-joined handle is a no-op).

<details>
<summary>Source</summary>

```rust
pub fn _shutdown_all_runners(py: Python) -> PyResult<()> {
    let handles: Vec<_> = {
        let mut reg = LIVE_RUNNERS.lock().unwrap();
        reg.drain(..).filter_map(|w| w.upgrade()).collect()
    };
    py.allow_threads(|| {
        for h in &handles {
            if let Err(e) = h.lock().unwrap().shutdown() {
                warn!("atexit runner shutdown reported: {}", e);
            }
        }
    });
    ATEXIT_FIRED.store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}
```

</details>



### <span id="fn.parse_schedule_id"></span>`fn parse_schedule_id`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn parse_schedule_id (schedule_id : & str ,) -> Result < cloacina :: database :: universal_types :: UniversalUuid , cloacina :: executor :: WorkflowExecutionError , >
```

Parse a schedule ID string into a UniversalUuid.

<details>
<summary>Source</summary>

```rust
fn parse_schedule_id(
    schedule_id: &str,
) -> Result<
    cloacina::database::universal_types::UniversalUuid,
    cloacina::executor::WorkflowExecutionError,
> {
    schedule_id
        .parse::<uuid::Uuid>()
        .map(cloacina::UniversalUuid::from)
        .map_err(
            |e| cloacina::executor::WorkflowExecutionError::Configuration {
                message: format!("Invalid schedule ID: {}", e),
            },
        )
}
```

</details>



### <span id="fn.schedule_to_cron_dict"></span>`fn schedule_to_cron_dict`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn schedule_to_cron_dict (schedule : cloacina :: models :: schedule :: Schedule , py : Python ,) -> PyResult < PyObject >
```

Convert a cron Schedule to a Python dict.

<details>
<summary>Source</summary>

```rust
fn schedule_to_cron_dict(
    schedule: cloacina::models::schedule::Schedule,
    py: Python,
) -> PyResult<PyObject> {
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
}
```

</details>



### <span id="fn.schedule_to_trigger_dict"></span>`fn schedule_to_trigger_dict`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn schedule_to_trigger_dict (schedule : cloacina :: models :: schedule :: Schedule , py : Python ,) -> PyResult < PyObject >
```

Convert a trigger Schedule to a Python dict.

<details>
<summary>Source</summary>

```rust
fn schedule_to_trigger_dict(
    schedule: cloacina::models::schedule::Schedule,
    py: Python,
) -> PyResult<PyObject> {
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
    Ok(dict.into())
}
```

</details>



### <span id="fn.reactor_subscription_to_dict"></span>`fn reactor_subscription_to_dict`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn reactor_subscription_to_dict (sub : cloacina :: dal :: unified :: ReactorSubscription , py : Python ,) -> PyResult < PyObject >
```

Convert a ReactorSubscription to a Python dict (CLOACI-I-0100 / T-0600).

<details>
<summary>Source</summary>

```rust
fn reactor_subscription_to_dict(
    sub: cloacina::dal::unified::ReactorSubscription,
    py: Python,
) -> PyResult<PyObject> {
    let dict = pyo3::types::PyDict::new(py);
    dict.set_item("id", sub.id.to_string())?;
    dict.set_item("reactor_name", &sub.reactor_name)?;
    dict.set_item("workflow_name", &sub.workflow_name)?;
    dict.set_item("tenant_id", &sub.tenant_id)?;
    dict.set_item("enabled", sub.enabled.is_true())?;
    dict.set_item(
        "last_seen_fired_at",
        sub.last_seen_fired_at.map(|t| t.to_string()),
    )?;
    dict.set_item("created_at", sub.created_at.to_string())?;
    dict.set_item("updated_at", sub.updated_at.to_string())?;
    dict.set_item("predicate_expression", sub.predicate_expression.clone())?;
    // CLOACI-T-0922 — predicate health. `list_reactor_subscriptions` is the
    // only listing surface a user has for reactor subscriptions (there is no
    // HTTP API), so the degraded/dead-letter state has to show up here or an
    // operator can only find it by reading the row or the logs.
    dict.set_item("predicate_degraded", sub.predicate_degraded.is_true())?;
    dict.set_item("predicate_error_count", sub.predicate_error_count)?;
    dict.set_item("last_predicate_error", sub.last_predicate_error.clone())?;
    dict.set_item(
        "last_predicate_error_at",
        sub.last_predicate_error_at.map(|t| t.to_string()),
    )?;
    dict.set_item(
        "predicate_error_firing_id",
        sub.predicate_error_firing_id.map(|id| id.to_string()),
    )?;
    Ok(dict.into())
}
```

</details>



### <span id="fn.cron_execution_to_dict"></span>`fn cron_execution_to_dict`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn cron_execution_to_dict (execution : cloacina :: models :: schedule :: ScheduleExecution , py : Python ,) -> PyResult < PyObject >
```

Convert a cron ScheduleExecution to a Python dict.

<details>
<summary>Source</summary>

```rust
fn cron_execution_to_dict(
    execution: cloacina::models::schedule::ScheduleExecution,
    py: Python,
) -> PyResult<PyObject> {
    let dict = pyo3::types::PyDict::new(py);
    dict.set_item("id", execution.id.to_string())?;
    dict.set_item("schedule_id", execution.schedule_id.to_string())?;
    dict.set_item(
        "scheduled_time",
        execution.scheduled_time.map(|t| t.to_string()),
    )?;
    dict.set_item("claimed_at", execution.claimed_at.map(|t| t.to_string()))?;
    dict.set_item(
        "workflow_execution_id",
        execution.workflow_execution_id.map(|id| id.to_string()),
    )?;
    dict.set_item("created_at", execution.created_at.to_string())?;
    dict.set_item("updated_at", execution.updated_at.to_string())?;
    Ok(dict.into())
}
```

</details>



### <span id="fn.trigger_execution_to_dict"></span>`fn trigger_execution_to_dict`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn trigger_execution_to_dict (execution : cloacina :: models :: schedule :: ScheduleExecution , py : Python ,) -> PyResult < PyObject >
```

Convert a trigger ScheduleExecution to a Python dict.

<details>
<summary>Source</summary>

```rust
fn trigger_execution_to_dict(
    execution: cloacina::models::schedule::ScheduleExecution,
    py: Python,
) -> PyResult<PyObject> {
    let dict = pyo3::types::PyDict::new(py);
    dict.set_item("id", execution.id.to_string())?;
    dict.set_item("schedule_id", execution.schedule_id.to_string())?;
    dict.set_item("context_hash", execution.context_hash.as_deref())?;
    dict.set_item(
        "workflow_execution_id",
        execution.workflow_execution_id.map(|id| id.to_string()),
    )?;
    dict.set_item("started_at", execution.started_at.to_string())?;
    dict.set_item(
        "completed_at",
        execution.completed_at.map(|t| t.to_string()),
    )?;
    dict.set_item("created_at", execution.created_at.to_string())?;
    Ok(dict.into())
}
```

</details>



### <span id="fn.run_event_loop"></span>`fn run_event_loop`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
async fn run_event_loop (runner : Arc < cloacina :: DefaultRunner > , mut rx : mpsc :: UnboundedReceiver < RuntimeMessage > ,)
```

The single event loop that dispatches RuntimeMessages to the DefaultRunner.

Previously this ~300-line match block was copy-pasted into each constructor.

<details>
<summary>Source</summary>

```rust
async fn run_event_loop(
    runner: Arc<cloacina::DefaultRunner>,
    mut rx: mpsc::UnboundedReceiver<RuntimeMessage>,
) {
    use cloacina::executor::WorkflowExecutor;

    while let Some(message) = rx.recv().await {
        match message {
            RuntimeMessage::Execute {
                workflow_name,
                context,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = runner.execute(&workflow_name, context).await;
                    let _ = response_tx.send(result);
                });
            }
            RuntimeMessage::RegisterCronWorkflow {
                workflow_name,
                cron_expression,
                timezone,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = runner
                        .register_cron_workflow(&workflow_name, &cron_expression, &timezone)
                        .await
                        .map(|uuid| uuid.to_string());
                    let _ = response_tx.send(result);
                });
            }
            RuntimeMessage::RegisterWorkflowInstance {
                workflow_name,
                instance_name,
                cron_expression,
                timezone,
                params_json,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = match serde_json::from_str::<
                        serde_json::Map<String, serde_json::Value>,
                    >(&params_json)
                    {
                        Ok(map) => {
                            let instance =
                                cloacina::workflow_instance::WorkflowInstance::from_resolved(
                                    workflow_name,
                                    map,
                                );
                            runner
                                .register_cron_workflow_instance(
                                    &instance,
                                    &instance_name,
                                    &cron_expression,
                                    &timezone,
                                )
                                .await
                                .map(|uuid| uuid.to_string())
                        }
                        Err(e) => Err(cloacina::executor::WorkflowExecutionError::Configuration {
                            message: format!("params must be a JSON object: {}", e),
                        }),
                    };
                    let _ = response_tx.send(result);
                });
            }
            RuntimeMessage::ListCronSchedules {
                enabled_only,
                limit,
                offset,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = runner
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
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = match parse_schedule_id(&schedule_id) {
                        Ok(id) => runner.set_cron_schedule_enabled(id, enabled).await,
                        Err(e) => Err(e),
                    };
                    let _ = response_tx.send(result);
                });
            }
            RuntimeMessage::DeleteCronSchedule {
                schedule_id,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = match parse_schedule_id(&schedule_id) {
                        Ok(id) => runner.delete_cron_schedule(id).await,
                        Err(e) => Err(e),
                    };
                    let _ = response_tx.send(result);
                });
            }
            RuntimeMessage::GetCronSchedule {
                schedule_id,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = match parse_schedule_id(&schedule_id) {
                        Ok(id) => runner.get_cron_schedule(id).await,
                        Err(e) => Err(e),
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
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = match parse_schedule_id(&schedule_id) {
                        Ok(id) => {
                            runner
                                .update_cron_schedule(id, Some(&cron_expression), Some(&timezone))
                                .await
                        }
                        Err(e) => Err(e),
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
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = match parse_schedule_id(&schedule_id) {
                        Ok(id) => runner.get_cron_execution_history(id, limit, offset).await,
                        Err(e) => Err(e),
                    };
                    let _ = response_tx.send(result);
                });
            }
            RuntimeMessage::GetCronExecutionStats { since, response_tx } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = runner.get_cron_execution_stats(since).await;
                    let _ = response_tx.send(result);
                });
            }
            RuntimeMessage::ListTriggerSchedules {
                enabled_only,
                limit,
                offset,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let dal = runner.dal();
                    let result = if enabled_only {
                        dal.schedule().get_enabled_triggers().await
                    } else {
                        dal.schedule()
                            .list(Some("trigger"), false, limit, offset)
                            .await
                    };
                    let _ = response_tx.send(result.map_err(|e| {
                        cloacina::executor::WorkflowExecutionError::Configuration {
                            message: e.to_string(),
                        }
                    }));
                });
            }
            RuntimeMessage::GetTriggerSchedule {
                trigger_name,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let dal = runner.dal();
                    let result = dal.schedule().get_by_trigger_name(&trigger_name).await;
                    let _ = response_tx.send(result.map_err(|e| {
                        cloacina::executor::WorkflowExecutionError::Configuration {
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
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = async {
                        if let Some(scheduler) = runner.unified_scheduler().await {
                            if enabled {
                                scheduler.enable_trigger(&trigger_name).await
                            } else {
                                scheduler.disable_trigger(&trigger_name).await
                            }
                        } else {
                            let dal = runner.dal();
                            if let Some(schedule) =
                                dal.schedule().get_by_trigger_name(&trigger_name).await?
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
                        cloacina::executor::WorkflowExecutionError::Configuration {
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
                let runner = runner.clone();
                tokio::spawn(async move {
                    let dal = runner.dal();
                    let result = async {
                        let schedule_opt = dal
                            .schedule()
                            .get_by_trigger_name(&trigger_name)
                            .await
                            .map_err(|e| {
                                cloacina::executor::WorkflowExecutionError::Configuration {
                                    message: e.to_string(),
                                }
                            })?;
                        if let Some(schedule) = schedule_opt {
                            dal.schedule_execution()
                                .list_by_schedule(schedule.id, limit, offset)
                                .await
                                .map_err(|e| {
                                    cloacina::executor::WorkflowExecutionError::Configuration {
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
            RuntimeMessage::SubscribeWorkflowToReactor {
                reactor,
                workflow,
                tenant,
                predicate,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = runner
                        .subscribe_workflow_to_reactor(
                            &reactor,
                            &workflow,
                            tenant.as_deref(),
                            predicate.as_deref(),
                        )
                        .await
                        .map(|uuid| uuid.to_string());
                    let _ = response_tx.send(result);
                });
            }
            RuntimeMessage::UnsubscribeWorkflowFromReactor {
                reactor,
                workflow,
                tenant,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = runner
                        .unsubscribe_workflow_from_reactor(&reactor, &workflow, tenant.as_deref())
                        .await;
                    let _ = response_tx.send(result);
                });
            }
            RuntimeMessage::ListReactorSubscriptions {
                tenant,
                response_tx,
            } => {
                let runner = runner.clone();
                tokio::spawn(async move {
                    let result = runner.list_reactor_subscriptions(tenant.as_deref()).await;
                    let _ = response_tx.send(result);
                });
            }
            RuntimeMessage::Shutdown => {
                info!("Received shutdown signal");
                break;
            }
        }
    }
}
```

</details>



### <span id="fn.spawn_runtime"></span>`fn spawn_runtime`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn spawn_runtime < F > (py : Python < '_ > , create_runner : F) -> PyResult < PyDefaultRunner > where F : FnOnce (& Runtime ,) -> Result < cloacina :: DefaultRunner , cloacina :: executor :: WorkflowExecutionError > + Send + 'static ,
```

Spawn a background thread running a Tokio runtime with a DefaultRunner and wire it to an mpsc event loop.

`create_runner` is called inside the thread with a reference to the Tokio
runtime so it can block_on async DefaultRunner constructors.
CLOACI-I-0140: takes `py` so the init wait can RELEASE THE GIL. This was
the reproduced scenario-flake deadlock (1-in-~25 locally): the constructor
pymethod held the GIL through `init_rx.blocking_recv()` while
`create_runner`'s background services raced to their first `with_gil`
(task-factory clone_refs at materialization, or flushing the previous
test's deferred decrefs) BEFORE sending init — classic AB-BA. The blocked
C call on the main thread also silenced pytest-timeout's SIGALRM, turning
the deadlock into the CI "hang" class (T-0622).

<details>
<summary>Source</summary>

```rust
fn spawn_runtime<F>(py: Python<'_>, create_runner: F) -> PyResult<PyDefaultRunner>
where
    F: FnOnce(
            &Runtime,
        )
            -> Result<cloacina::DefaultRunner, cloacina::executor::WorkflowExecutionError>
        + Send
        + 'static,
{
    let (tx, rx) = mpsc::unbounded_channel::<RuntimeMessage>();
    let (init_tx, init_rx) = oneshot::channel::<Result<(), String>>();

    let thread_handle = thread::spawn(move || {
        // Idempotent — only the first call in the process succeeds
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .try_init();

        let rt = Runtime::new().expect("Failed to create tokio runtime");

        let runner = match create_runner(&rt) {
            Ok(r) => {
                info!("DefaultRunner created successfully, background services running");
                let _ = init_tx.send(Ok(()));
                r
            }
            Err(e) => {
                let _ = init_tx.send(Err(format!("Failed to create DefaultRunner: {}", e)));
                return;
            }
        };

        let runner = Arc::new(runner);
        // CLOACI-I-0136: deliberately NOT crate::gil::py_block_on — this is a
        // long-running actor loop on this dedicated OS thread that does not hold
        // the GIL, so the "release GIL then block" bridge does not apply here.
        rt.block_on(run_event_loop(runner, rx));
    });

    // Wait for init — propagate errors instead of panicking. MUST release the
    // GIL: the runtime thread's startup path may acquire it (see fn docs).
    match py.allow_threads(move || init_rx.blocking_recv()) {
        Ok(Ok(())) => {}
        Ok(Err(msg)) => return Err(PyRuntimeError::new_err(msg)),
        Err(_) => {
            return Err(PyRuntimeError::new_err(
                "Runtime thread died during initialization",
            ))
        }
    }

    let handle = Arc::new(Mutex::new(AsyncRuntimeHandle {
        tx,
        thread_handle: Some(thread_handle),
    }));
    // CLOACI-I-0140: register for the atexit drain (prune dead entries while
    // holding the lock so the registry can't grow unboundedly).
    {
        let mut reg = LIVE_RUNNERS.lock().unwrap();
        reg.retain(|w| w.strong_count() > 0);
        reg.push(Arc::downgrade(&handle));
    }
    Ok(PyDefaultRunner {
        runtime_handle: handle,
    })
}
```

</details>



