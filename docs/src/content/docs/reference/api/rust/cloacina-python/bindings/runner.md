---
title: "cloacina-python::bindings::runner"
description: "API reference of the Rust module cloacina-python::bindings::runner."
editUrl: false
sidebar:
  order: 8
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

## Structs

### <span id="struct.PyWorkflowResult"></span><code style="font-size: 0.85em">class WorkflowResult</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.WorkflowResult`](../../../../cloaca/#class.WorkflowResult)

Python wrapper for WorkflowExecutionResult

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `cloacina::executor::WorkflowExecutionResult` |  |

#### Methods

##### <span id="PyWorkflowResult.status"></span><code>status</code>

```rust
fn status(&self) -> String
```

> **Python API**: [`cloaca.WorkflowResult.status`](../../../../cloaca/#WorkflowResult.status)

<details>
<summary>Source</summary>

```rust
    pub fn status(&self) -> String {
        format!("{:?}", self.inner.status)
    }
```

</details>

##### <span id="PyWorkflowResult.start_time"></span><code>start_​time</code>

```rust
fn start_time(&self) -> String
```

> **Python API**: [`cloaca.WorkflowResult.start_time`](../../../../cloaca/#WorkflowResult.start_time)

<details>
<summary>Source</summary>

```rust
    pub fn start_time(&self) -> String {
        self.inner.start_time.to_rfc3339()
    }
```

</details>

##### <span id="PyWorkflowResult.end_time"></span><code>end_​time</code>

```rust
fn end_time(&self) -> Option<String>
```

> **Python API**: [`cloaca.WorkflowResult.end_time`](../../../../cloaca/#WorkflowResult.end_time)

<details>
<summary>Source</summary>

```rust
    pub fn end_time(&self) -> Option<String> {
        self.inner.end_time.map(|t| t.to_rfc3339())
    }
```

</details>

##### <span id="PyWorkflowResult.final_context"></span><code>final_​context</code>

```rust
fn final_context(&self) -> PyContext
```

> **Python API**: [`cloaca.WorkflowResult.final_context`](../../../../cloaca/#WorkflowResult.final_context)

<details>
<summary>Source</summary>

```rust
    pub fn final_context(&self) -> PyContext {
        let new_context = self.inner.final_context.clone_data();
        PyContext::from_rust_context(new_context)
    }
```

</details>

##### <span id="PyWorkflowResult.error_message"></span><code>error_​message</code>

```rust
fn error_message(&self) -> Option<&str>
```

> **Python API**: [`cloaca.WorkflowResult.error_message`](../../../../cloaca/#WorkflowResult.error_message)

<details>
<summary>Source</summary>

```rust
    pub fn error_message(&self) -> Option<&str> {
        self.inner.error_message.as_deref()
    }
```

</details>

##### <span id="PyWorkflowResult.__repr__"></span><code>_​_​repr_​_​</code>

```rust
fn __repr__(&self) -> String
```

> **Python API**: [`cloaca.WorkflowResult.__repr__`](../../../../cloaca/#WorkflowResult.__repr__)

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

##### <span id="PyWorkflowResult.from_result"></span><code>from_​result</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_result(
    result: cloacina::executor::WorkflowExecutionResult,
) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn from_result(result: cloacina::executor::WorkflowExecutionResult) -> Self {
        PyWorkflowResult { inner: result }
    }
```

</details>

### <span id="struct.PyDefaultRunner"></span><code style="font-size: 0.85em">class DefaultRunner</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.DefaultRunner`](../../../../cloaca/#class.DefaultRunner)

Python wrapper for DefaultRunner

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `runtime_handle` | `Arc<Mutex<AsyncRuntimeHandle>>` |  |

#### Methods

##### <span id="PyDefaultRunner.new"></span><code>new</code>

```rust
fn new(py: Python<'_>, database_url: &str) -> PyResult<Self>
```

> **Python API**: [`cloaca.DefaultRunner.new`](../../../../cloaca/#DefaultRunner.new)

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

##### <span id="PyDefaultRunner.with_config"></span><code>with_​config</code>

```rust
fn with_config(
    py: Python<'_>,
    database_url: &str,
    config: &super::context::PyDefaultRunnerConfig,
) -> PyResult<PyDefaultRunner>
```

> **Python API**: [`cloaca.DefaultRunner.with_config`](../../../../cloaca/#DefaultRunner.with_config)

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

##### <span id="PyDefaultRunner.with_schema"></span><code>with_​schema</code>

```rust
fn with_schema(
    py: Python<'_>,
    database_url: &str,
    schema: &str,
) -> PyResult<PyDefaultRunner>
```

> **Python API**: [`cloaca.DefaultRunner.with_schema`](../../../../cloaca/#DefaultRunner.with_schema)

Create a new DefaultRunner with PostgreSQL schema-based multi-tenancy

Each tenant gets their own schema with independent tables, migrations,
and data. Requires a PostgreSQL database.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database_url` | `&str` | PostgreSQL connection string |
| `schema` | `&str` | Schema name for tenant isolation (alphanumeric + underscores only) |

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

##### <span id="PyDefaultRunner.execute"></span><code>execute</code>

```rust
fn execute(
    &self,
    workflow_name: &str,
    context: &PyContext,
    py: Python,
) -> PyResult<PyWorkflowResult>
```

> **Python API**: [`cloaca.DefaultRunner.execute`](../../../../cloaca/#DefaultRunner.execute)

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

##### <span id="PyDefaultRunner.shutdown"></span><code>shutdown</code>

```rust
fn shutdown(&self, py: Python) -> PyResult<()>
```

> **Python API**: [`cloaca.DefaultRunner.shutdown`](../../../../cloaca/#DefaultRunner.shutdown)

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

##### <span id="PyDefaultRunner.register_workflow_instance"></span><code>register_​workflow_​instance</code>

```rust
fn register_workflow_instance(
    &self,
    workflow_name: String,
    instance_name: String,
    cron_expression: String,
    timezone: String,
    params: &crate::context::PyContext,
    py: Python,
) -> PyResult<String>
```

> **Python API**: [`cloaca.DefaultRunner.register_workflow_instance`](../../../../cloaca/#DefaultRunner.register_workflow_instance)

Register a cron workflow for automatic execution at scheduled times

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `String` | Name of the workflow to execute |
| `cron_expression` | `String` | Standard cron expression (e.g., "0 2 * * *") |
| `timezone` | `String` | Timezone for cron interpretation (e.g., "UTC") |

**Returns:**

* Schedule ID as a string
Register a NAMED, PARAMETERIZED cron instance (CLOACI-I-0116):
`params` (a dict) is the instance's fully-resolved bound parameters,
merged into the context at every fire under flat top-level keys.

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

##### <span id="PyDefaultRunner.register_cron_workflow"></span><code>register_​cron_​workflow</code>

```rust
fn register_cron_workflow(
    &self,
    workflow_name: String,
    cron_expression: String,
    timezone: String,
    py: Python,
) -> PyResult<String>
```

> **Python API**: [`cloaca.DefaultRunner.register_cron_workflow`](../../../../cloaca/#DefaultRunner.register_cron_workflow)

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

##### <span id="PyDefaultRunner.list_cron_schedules"></span><code>list_​cron_​schedules</code>

```rust
fn list_cron_schedules(
    &self,
    enabled_only: Option<bool>,
    limit: Option<i64>,
    offset: Option<i64>,
    py: Python,
) -> PyResult<Vec<PyObject>>
```

> **Python API**: [`cloaca.DefaultRunner.list_cron_schedules`](../../../../cloaca/#DefaultRunner.list_cron_schedules)

List all cron schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `enabled_only` | `Option<bool>` | If True, only return enabled schedules |
| `limit` | `Option<i64>` | Maximum number of schedules to return (default: 100) |
| `offset` | `Option<i64>` | Number of schedules to skip (default: 0) |

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

##### <span id="PyDefaultRunner.set_cron_schedule_enabled"></span><code>set_​cron_​schedule_​enabled</code>

```rust
fn set_cron_schedule_enabled(
    &self,
    schedule_id: String,
    enabled: bool,
    py: Python,
) -> PyResult<()>
```

> **Python API**: [`cloaca.DefaultRunner.set_cron_schedule_enabled`](../../../../cloaca/#DefaultRunner.set_cron_schedule_enabled)

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

##### <span id="PyDefaultRunner.delete_cron_schedule"></span><code>delete_​cron_​schedule</code>

```rust
fn delete_cron_schedule(
    &self) -> PyResult<(,
    schedule_id: String) -> PyResult<(,
    py: Python) -> PyResult<(,
) -> PyResult<()>
```

> **Python API**: [`cloaca.DefaultRunner.delete_cron_schedule`](../../../../cloaca/#DefaultRunner.delete_cron_schedule)

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

##### <span id="PyDefaultRunner.get_cron_schedule"></span><code>get_​cron_​schedule</code>

```rust
fn get_cron_schedule(
    &self,
    schedule_id: String,
    py: Python,
) -> PyResult<PyObject>
```

> **Python API**: [`cloaca.DefaultRunner.get_cron_schedule`](../../../../cloaca/#DefaultRunner.get_cron_schedule)

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

##### <span id="PyDefaultRunner.update_cron_schedule"></span><code>update_​cron_​schedule</code>

```rust
fn update_cron_schedule(
    &self,
    schedule_id: String,
    cron_expression: String,
    timezone: String,
    py: Python,
) -> PyResult<()>
```

> **Python API**: [`cloaca.DefaultRunner.update_cron_schedule`](../../../../cloaca/#DefaultRunner.update_cron_schedule)

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

##### <span id="PyDefaultRunner.get_cron_execution_history"></span><code>get_​cron_​execution_​history</code>

```rust
fn get_cron_execution_history(
    &self,
    schedule_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
    py: Python,
) -> PyResult<Vec<PyObject>>
```

> **Python API**: [`cloaca.DefaultRunner.get_cron_execution_history`](../../../../cloaca/#DefaultRunner.get_cron_execution_history)

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

##### <span id="PyDefaultRunner.get_cron_execution_stats"></span><code>get_​cron_​execution_​stats</code>

```rust
fn get_cron_execution_stats(
    &self,
    since: String,
    py: Python,
) -> PyResult<PyObject>
```

> **Python API**: [`cloaca.DefaultRunner.get_cron_execution_stats`](../../../../cloaca/#DefaultRunner.get_cron_execution_stats)

Get execution statistics for cron schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `since` | `String` | Start time for statistics collection (ISO 8601 string) |

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

##### <span id="PyDefaultRunner.list_trigger_schedules"></span><code>list_​trigger_​schedules</code>

```rust
fn list_trigger_schedules(
    &self,
    enabled_only: Option<bool>,
    limit: Option<i64>,
    offset: Option<i64>,
    py: Python,
) -> PyResult<Vec<PyObject>>
```

> **Python API**: [`cloaca.DefaultRunner.list_trigger_schedules`](../../../../cloaca/#DefaultRunner.list_trigger_schedules)

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

##### <span id="PyDefaultRunner.get_trigger_schedule"></span><code>get_​trigger_​schedule</code>

```rust
fn get_trigger_schedule(
    &self,
    trigger_name: String,
    py: Python,
) -> PyResult<Option<PyObject>>
```

> **Python API**: [`cloaca.DefaultRunner.get_trigger_schedule`](../../../../cloaca/#DefaultRunner.get_trigger_schedule)

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

##### <span id="PyDefaultRunner.set_trigger_enabled"></span><code>set_​trigger_​enabled</code>

```rust
fn set_trigger_enabled(
    &self,
    trigger_name: String,
    enabled: bool,
    py: Python,
) -> PyResult<()>
```

> **Python API**: [`cloaca.DefaultRunner.set_trigger_enabled`](../../../../cloaca/#DefaultRunner.set_trigger_enabled)

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

##### <span id="PyDefaultRunner.get_trigger_execution_history"></span><code>get_​trigger_​execution_​history</code>

```rust
fn get_trigger_execution_history(
    &self,
    trigger_name: String,
    limit: Option<i64>,
    offset: Option<i64>,
    py: Python,
) -> PyResult<Vec<PyObject>>
```

> **Python API**: [`cloaca.DefaultRunner.get_trigger_execution_history`](../../../../cloaca/#DefaultRunner.get_trigger_execution_history)

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

##### <span id="PyDefaultRunner.subscribe_workflow_to_reactor"></span><code>subscribe_​workflow_​to_​reactor</code>

```rust
fn subscribe_workflow_to_reactor(
    &self,
    reactor: String,
    workflow: String,
    tenant: Option<String>,
    when: Option<String>,
    py: Python,
) -> PyResult<String>
```

> **Python API**: [`cloaca.DefaultRunner.subscribe_workflow_to_reactor`](../../../../cloaca/#DefaultRunner.subscribe_workflow_to_reactor)

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
| `reactor` | `String` | The reactor name. |
| `workflow` | `String` | The workflow to dispatch on each firing. |
| `tenant` | `Option<String>` | Tenant scope. Defaults to `"public"` if omitted. |
| `when` | `Option<String>` | Optional CEL filter expression (CLOACI-T-0602). When provided, the scheduler evaluates it against the firing payload and only dispatches when it evaluates to true. The watermark still advances when it evaluates to false. Variables available: `payload` (dict, keys are boundary source names), `reactor` (str), `tenant` (str). Invalid CEL is rejected immediately with a ValueError — no row written. |

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

##### <span id="PyDefaultRunner.unsubscribe_workflow_from_reactor"></span><code>unsubscribe_​workflow_​from_​reactor</code>

```rust
fn unsubscribe_workflow_from_reactor(
    &self,
    reactor: String,
    workflow: String,
    tenant: Option<String>,
    py: Python,
) -> PyResult<bool>
```

> **Python API**: [`cloaca.DefaultRunner.unsubscribe_workflow_from_reactor`](../../../../cloaca/#DefaultRunner.unsubscribe_workflow_from_reactor)

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

##### <span id="PyDefaultRunner.list_reactor_subscriptions"></span><code>list_​reactor_​subscriptions</code>

```rust
fn list_reactor_subscriptions(
    &self,
    tenant: Option<String>,
    py: Python,
) -> PyResult<Vec<PyObject>>
```

> **Python API**: [`cloaca.DefaultRunner.list_reactor_subscriptions`](../../../../cloaca/#DefaultRunner.list_reactor_subscriptions)

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

##### <span id="PyDefaultRunner.__repr__"></span><code>_​_​repr_​_​</code>

```rust
fn __repr__(&self) -> String
```

> **Python API**: [`cloaca.DefaultRunner.__repr__`](../../../../cloaca/#DefaultRunner.__repr__)

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        "DefaultRunner(thread_separated_async_runtime)".to_string()
    }
```

</details>

##### <span id="PyDefaultRunner.__enter__"></span><code>_​_​enter_​_​</code>

```rust
fn __enter__(slf: PyRef<Self>) -> PyRef<Self>
```

> **Python API**: [`cloaca.DefaultRunner.__enter__`](../../../../cloaca/#DefaultRunner.__enter__)

<details>
<summary>Source</summary>

```rust
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
```

</details>

##### <span id="PyDefaultRunner.__exit__"></span><code>_​_​exit_​_​</code>

```rust
fn __exit__(
    &self,
    py: Python,
    _exc_type: Option<&Bound<PyAny>>,
    _exc_value: Option<&Bound<PyAny>>,
    _traceback: Option<&Bound<PyAny>>,
) -> PyResult<bool>
```

> **Python API**: [`cloaca.DefaultRunner.__exit__`](../../../../cloaca/#DefaultRunner.__exit__)

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

### <span id="enum.ShutdownError"></span><code style="font-size: 0.85em">enum ShutdownError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors that can occur during async runtime shutdown

#### Variants

- **`ChannelClosed`** - Failed to send shutdown signal to runtime thread
- **`ThreadPanic`** - Runtime thread panicked during shutdown
- **`Timeout`** - Shutdown timed out waiting for thread to complete

## Functions

### <span id="fn._shutdown_all_runners"></span><code style="font-size: 0.85em">fn _​shutdown_​all_​runners</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

```rust
fn _shutdown_all_runners(py: Python) -> PyResult<()>
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
