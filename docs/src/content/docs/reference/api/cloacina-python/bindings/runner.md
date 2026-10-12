---
title: "cloacina-python.bindings.runner"
description: "API reference of the Python module cloacina-python.bindings.runner."
sidebar:
  order: 3
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


## Classes

### <span id="class.WorkflowResult"></span>`class WorkflowResult`

> **Rust Implementation**: [cloacina-python::bindings::runner::PyWorkflowResult](../../../rust/cloacina-python/bindings/runner/#struct.PyWorkflowResult)

Python wrapper for WorkflowExecutionResult

#### Methods

##### <span id="WorkflowResult.status"></span>`status`

```python
def status() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyWorkflowResult::status](../../../rust/cloacina-python/bindings/runner/#PyWorkflowResult.status)

<details>
<summary>Source</summary>

```python
    pub fn status(&self) -> String {
        format!("{:?}", self.inner.status)
    }
```

</details>



##### <span id="WorkflowResult.start_time"></span>`start_time`

```python
def start_time() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyWorkflowResult::start_time](../../../rust/cloacina-python/bindings/runner/#PyWorkflowResult.start_time)

<details>
<summary>Source</summary>

```python
    pub fn start_time(&self) -> String {
        self.inner.start_time.to_rfc3339()
    }
```

</details>



##### <span id="WorkflowResult.end_time"></span>`end_time`

```python
def end_time() -> Option < String >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyWorkflowResult::end_time](../../../rust/cloacina-python/bindings/runner/#PyWorkflowResult.end_time)

<details>
<summary>Source</summary>

```python
    pub fn end_time(&self) -> Option<String> {
        self.inner.end_time.map(|t| t.to_rfc3339())
    }
```

</details>



##### <span id="WorkflowResult.final_context"></span>`final_context`

```python
def final_context() -> PyContext
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyWorkflowResult::final_context](../../../rust/cloacina-python/bindings/runner/#PyWorkflowResult.final_context)

<details>
<summary>Source</summary>

```python
    pub fn final_context(&self) -> PyContext {
        let new_context = self.inner.final_context.clone_data();
        PyContext::from_rust_context(new_context)
    }
```

</details>



##### <span id="WorkflowResult.error_message"></span>`error_message`

```python
def error_message() -> Option < & str >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyWorkflowResult::error_message](../../../rust/cloacina-python/bindings/runner/#PyWorkflowResult.error_message)

<details>
<summary>Source</summary>

```python
    pub fn error_message(&self) -> Option<&str> {
        self.inner.error_message.as_deref()
    }
```

</details>



##### <span id="WorkflowResult.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyWorkflowResult::__repr__](../../../rust/cloacina-python/bindings/runner/#PyWorkflowResult.__repr__)

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





### <span id="class.DefaultRunner"></span>`class DefaultRunner`

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner](../../../rust/cloacina-python/bindings/runner/#struct.PyDefaultRunner)

Python wrapper for DefaultRunner

#### Methods

##### <span id="DefaultRunner.new"></span>`new`

```python
def new(database_url:  str) -> PyResult < Self >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::new](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.new)

Create a new DefaultRunner with database connection

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database_url` | ` str` |  |


<details>
<summary>Source</summary>

```python
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



##### <span id="DefaultRunner.with_config"></span>`with_config`

```python
def with_config(
    database_url:  str,
    config:  super :: context :: PyDefaultRunnerConfig,
) -> PyResult < PyDefaultRunner >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::with_config](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.with_config)

Create a new DefaultRunner with custom configuration

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database_url` | ` str` |  |
| `config` | ` super :: context :: PyDefaultRunnerConfig` |  |


<details>
<summary>Source</summary>

```python
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



##### <span id="DefaultRunner.with_schema"></span>`with_schema`

```python
def with_schema(database_url:  str, schema:  str) -> PyResult < PyDefaultRunner >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::with_schema](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.with_schema)

Create a new DefaultRunner with PostgreSQL schema-based multi-tenancy

Each tenant gets their own schema with independent tables, migrations,
and data. Requires a PostgreSQL database.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database_url` | ` str` | PostgreSQL connection string |
| `schema` | ` str` | Schema name for tenant isolation (alphanumeric + underscores only) |


<details>
<summary>Source</summary>

```python
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



##### <span id="DefaultRunner.execute"></span>`execute`

```python
def execute(workflow_name:  str, context:  PyContext) -> PyResult < PyWorkflowResult >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::execute](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.execute)

Execute a workflow by name with context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | ` str` |  |
| `context` | ` PyContext` |  |


<details>
<summary>Source</summary>

```python
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



##### <span id="DefaultRunner.shutdown"></span>`shutdown`

```python
def shutdown() -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::shutdown](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.shutdown)

Shutdown the runner and cleanup resources

<details>
<summary>Source</summary>

```python
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



##### <span id="DefaultRunner.register_workflow_instance"></span>`register_workflow_instance`

```python
def register_workflow_instance(
    workflow_name: str,
    instance_name: str,
    cron_expression: str,
    timezone: str,
    params:  crate :: context :: PyContext,
) -> PyResult < String >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::register_workflow_instance](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.register_workflow_instance)

Register a cron workflow for automatic execution at scheduled times

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `str` | Name of the workflow to execute |
| `instance_name` | `str` |  |
| `cron_expression` | `str` | Standard cron expression (e.g., "0 2 * * *") |
| `timezone` | `str` | Timezone for cron interpretation (e.g., "UTC") |
| `params` | ` crate :: context :: PyContext` |  |


**Returns:**

* Schedule ID as a string Register a NAMED, PARAMETERIZED cron instance (CLOACI-I-0116): `params` (a dict) is the instance's fully-resolved bound parameters, merged into the context at every fire under flat top-level keys.

<details>
<summary>Source</summary>

```python
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



##### <span id="DefaultRunner.register_cron_workflow"></span>`register_cron_workflow`

```python
def register_cron_workflow(
    workflow_name: str,
    cron_expression: str,
    timezone: str,
) -> PyResult < String >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::register_cron_workflow](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.register_cron_workflow)

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
        self.send_and_recv(message, response_rx, py, "Failed to register cron workflow")
    }
```

</details>



##### <span id="DefaultRunner.list_cron_schedules"></span>`list_cron_schedules`

```python
def list_cron_schedules(
    enabled_only: Option < bool >,
    limit: Option < i64 >,
    offset: Option < i64 >,
) -> PyResult < Vec < PyObject > >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::list_cron_schedules](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.list_cron_schedules)

List all cron schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `enabled_only` | `Option < bool >` | If True, only return enabled schedules |
| `limit` | `Option < i64 >` | Maximum number of schedules to return (default: 100) |
| `offset` | `Option < i64 >` | Number of schedules to skip (default: 0) |


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



##### <span id="DefaultRunner.set_cron_schedule_enabled"></span>`set_cron_schedule_enabled`

```python
def set_cron_schedule_enabled(schedule_id: str, enabled: bool) -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::set_cron_schedule_enabled](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.set_cron_schedule_enabled)

Enable or disable a cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `str` |  |
| `enabled` | `bool` |  |


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
        self.send_and_recv(message, response_rx, py, "Failed to update cron schedule")
    }
```

</details>



##### <span id="DefaultRunner.delete_cron_schedule"></span>`delete_cron_schedule`

```python
def delete_cron_schedule(schedule_id: str) -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::delete_cron_schedule](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.delete_cron_schedule)

Delete a cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `str` |  |


<details>
<summary>Source</summary>

```python
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



##### <span id="DefaultRunner.get_cron_schedule"></span>`get_cron_schedule`

```python
def get_cron_schedule(schedule_id: str) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::get_cron_schedule](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.get_cron_schedule)

Get details of a specific cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `str` |  |


<details>
<summary>Source</summary>

```python
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



##### <span id="DefaultRunner.update_cron_schedule"></span>`update_cron_schedule`

```python
def update_cron_schedule(
    schedule_id: str,
    cron_expression: str,
    timezone: str,
) -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::update_cron_schedule](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.update_cron_schedule)

Update a cron schedule's expression and timezone

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `str` |  |
| `cron_expression` | `str` |  |
| `timezone` | `str` |  |


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
        self.send_and_recv(message, response_rx, py, "Failed to update cron schedule")
    }
```

</details>



##### <span id="DefaultRunner.get_cron_execution_history"></span>`get_cron_execution_history`

```python
def get_cron_execution_history(
    schedule_id: str,
    limit: Option < i64 >,
    offset: Option < i64 >,
) -> PyResult < Vec < PyObject > >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::get_cron_execution_history](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.get_cron_execution_history)

Get execution history for a specific cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `str` |  |
| `limit` | `Option < i64 >` |  |
| `offset` | `Option < i64 >` |  |


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



##### <span id="DefaultRunner.get_cron_execution_stats"></span>`get_cron_execution_stats`

```python
def get_cron_execution_stats(since: str) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::get_cron_execution_stats](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.get_cron_execution_stats)

Get execution statistics for cron schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `since` | `str` | Start time for statistics collection (ISO 8601 string) |


<details>
<summary>Source</summary>

```python
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



##### <span id="DefaultRunner.list_trigger_schedules"></span>`list_trigger_schedules`

```python
def list_trigger_schedules(
    enabled_only: Option < bool >,
    limit: Option < i64 >,
    offset: Option < i64 >,
) -> PyResult < Vec < PyObject > >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::list_trigger_schedules](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.list_trigger_schedules)

List all trigger schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `enabled_only` | `Option < bool >` |  |
| `limit` | `Option < i64 >` |  |
| `offset` | `Option < i64 >` |  |


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



##### <span id="DefaultRunner.get_trigger_schedule"></span>`get_trigger_schedule`

```python
def get_trigger_schedule(trigger_name: str) -> PyResult < Option < PyObject > >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::get_trigger_schedule](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.get_trigger_schedule)

Get details of a specific trigger schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `trigger_name` | `str` |  |


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
        let schedule_opt =
            self.send_and_recv(message, response_rx, py, "Failed to get trigger schedule")?;
        match schedule_opt {
            Some(schedule) => Ok(Some(schedule_to_trigger_dict(schedule, py)?)),
            None => Ok(None),
        }
    }
```

</details>



##### <span id="DefaultRunner.set_trigger_enabled"></span>`set_trigger_enabled`

```python
def set_trigger_enabled(trigger_name: str, enabled: bool) -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::set_trigger_enabled](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.set_trigger_enabled)

Enable or disable a trigger

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `trigger_name` | `str` |  |
| `enabled` | `bool` |  |


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
        self.send_and_recv(message, response_rx, py, "Failed to update trigger")
    }
```

</details>



##### <span id="DefaultRunner.get_trigger_execution_history"></span>`get_trigger_execution_history`

```python
def get_trigger_execution_history(
    trigger_name: str,
    limit: Option < i64 >,
    offset: Option < i64 >,
) -> PyResult < Vec < PyObject > >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::get_trigger_execution_history](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.get_trigger_execution_history)

Get execution history for a specific trigger

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `trigger_name` | `str` |  |
| `limit` | `Option < i64 >` |  |
| `offset` | `Option < i64 >` |  |


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



##### <span id="DefaultRunner.subscribe_workflow_to_reactor"></span>`subscribe_workflow_to_reactor`

```python
def subscribe_workflow_to_reactor(
    reactor: str,
    workflow: str,
    tenant: Option < String >,
    when: Option < String >,
) -> PyResult < String >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::subscribe_workflow_to_reactor](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.subscribe_workflow_to_reactor)

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
| `reactor` | `str` | The reactor name. |
| `workflow` | `str` | The workflow to dispatch on each firing. |
| `tenant` | `Option < String >` | Tenant scope. Defaults to `"public"` if omitted. |
| `when` | `Option < String >` | Optional CEL filter expression (CLOACI-T-0602). When provided, the scheduler evaluates it against the firing payload and only dispatches when it evaluates to true. The watermark still advances when it evaluates to false. Variables available: `payload` (dict, keys are boundary source names), `reactor` (str), `tenant` (str). Invalid CEL is rejected immediately with a ValueError — no row written. |


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

```python
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



##### <span id="DefaultRunner.unsubscribe_workflow_from_reactor"></span>`unsubscribe_workflow_from_reactor`

```python
def unsubscribe_workflow_from_reactor(
    reactor: str,
    workflow: str,
    tenant: Option < String >,
) -> PyResult < bool >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::unsubscribe_workflow_from_reactor](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.unsubscribe_workflow_from_reactor)

Remove a workflow-to-reactor subscription.

Returns True if a row was deleted, False if no subscription
matched `(reactor, workflow, tenant)`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `reactor` | `str` |  |
| `workflow` | `str` |  |
| `tenant` | `Option < String >` |  |


<details>
<summary>Source</summary>

```python
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



##### <span id="DefaultRunner.list_reactor_subscriptions"></span>`list_reactor_subscriptions`

```python
def list_reactor_subscriptions(
    tenant: Option < String >,
) -> PyResult < Vec < PyObject > >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::list_reactor_subscriptions](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.list_reactor_subscriptions)

List enabled reactor subscriptions for a tenant.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `tenant` | `Option < String >` |  |


<details>
<summary>Source</summary>

```python
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



##### <span id="DefaultRunner.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::__repr__](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.__repr__)

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        "DefaultRunner(thread_separated_async_runtime)".to_string()
    }
```

</details>



##### <span id="DefaultRunner.__enter__"></span>`__enter__`

```python
def __enter__(slf: PyRef < Self >) -> PyRef < Self >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::__enter__](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.__enter__)

<details>
<summary>Source</summary>

```python
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
```

</details>



##### <span id="DefaultRunner.__exit__"></span>`__exit__`

```python
def __exit__(
    _exc_type: Option < & Bound < PyAny > >,
    _exc_value: Option < & Bound < PyAny > >,
    _traceback: Option < & Bound < PyAny > >,
) -> PyResult < bool >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::PyDefaultRunner::__exit__](../../../rust/cloacina-python/bindings/runner/#PyDefaultRunner.__exit__)

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





## Functions

### <span id="fn._shutdown_all_runners"></span>`_shutdown_all_runners`

```python
def _shutdown_all_runners() -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::bindings::runner::_shutdown_all_runners](../../../rust/cloacina-python/bindings/runner/#fn._shutdown_all_runners)

Shut down every still-live runner. Registered with Python's `atexit` by the wheel's `#[pymodule]` init; also callable directly (idempotent — shutdown of an already-joined handle is a no-op).

<details>
<summary>Source</summary>

```python
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



