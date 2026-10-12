---
title: "cloacina-python.bindings.context"
description: "API reference of the Python module cloacina-python.bindings.context."
sidebar:
  order: 2
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


## Classes

### <span id="class.DefaultRunnerConfig"></span>`class DefaultRunnerConfig`

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig](../../../rust/cloacina-python/bindings/context/#struct.PyDefaultRunnerConfig)

PyDefaultRunnerConfig - Python wrapper for Rust DefaultRunnerConfig

#### Methods

##### <span id="DefaultRunnerConfig.new"></span>`new`

```python
def new(
    max_concurrent_tasks: Option < usize >,
    scheduler_poll_interval_ms: Option < u64 >,
    task_timeout_seconds: Option < u64 >,
    workflow_timeout_seconds: Option < u64 >,
    db_pool_size: Option < u32 >,
    enable_recovery: Option < bool >,
    enable_cron_scheduling: Option < bool >,
    cron_poll_interval_seconds: Option < u64 >,
    cron_max_catchup_executions: Option < usize >,
    cron_enable_recovery: Option < bool >,
    cron_recovery_interval_seconds: Option < u64 >,
    cron_lost_threshold_minutes: Option < i32 >,
    cron_max_recovery_age_seconds: Option < u64 >,
    cron_max_recovery_attempts: Option < usize >,
) -> pyo3 :: PyResult < Self >
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::new](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.new)

Creates a new DefaultRunnerConfig with customizable parameters

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `max_concurrent_tasks` | `Option < usize >` |  |
| `scheduler_poll_interval_ms` | `Option < u64 >` |  |
| `task_timeout_seconds` | `Option < u64 >` |  |
| `workflow_timeout_seconds` | `Option < u64 >` |  |
| `db_pool_size` | `Option < u32 >` |  |
| `enable_recovery` | `Option < bool >` |  |
| `enable_cron_scheduling` | `Option < bool >` |  |
| `cron_poll_interval_seconds` | `Option < u64 >` |  |
| `cron_max_catchup_executions` | `Option < usize >` |  |
| `cron_enable_recovery` | `Option < bool >` |  |
| `cron_recovery_interval_seconds` | `Option < u64 >` |  |
| `cron_lost_threshold_minutes` | `Option < i32 >` |  |
| `cron_max_recovery_age_seconds` | `Option < u64 >` |  |
| `cron_max_recovery_attempts` | `Option < usize >` |  |


<details>
<summary>Source</summary>

```python
    pub fn new(
        max_concurrent_tasks: Option<usize>,
        scheduler_poll_interval_ms: Option<u64>,
        task_timeout_seconds: Option<u64>,
        workflow_timeout_seconds: Option<u64>,
        db_pool_size: Option<u32>,
        enable_recovery: Option<bool>,
        enable_cron_scheduling: Option<bool>,
        cron_poll_interval_seconds: Option<u64>,
        cron_max_catchup_executions: Option<usize>,
        cron_enable_recovery: Option<bool>,
        cron_recovery_interval_seconds: Option<u64>,
        cron_lost_threshold_minutes: Option<i32>,
        cron_max_recovery_age_seconds: Option<u64>,
        cron_max_recovery_attempts: Option<usize>,
    ) -> pyo3::PyResult<Self> {
        use std::time::Duration;

        let mut builder = cloacina::runner::DefaultRunnerConfig::builder();

        if let Some(val) = max_concurrent_tasks {
            builder = builder.max_concurrent_tasks(val);
        }
        if let Some(val) = scheduler_poll_interval_ms {
            builder = builder.scheduler_poll_interval(Duration::from_millis(val));
        }
        if let Some(val) = task_timeout_seconds {
            builder = builder.task_timeout(Duration::from_secs(val));
        }
        if let Some(val) = workflow_timeout_seconds {
            builder = builder.workflow_timeout(Some(Duration::from_secs(val)));
        }
        if let Some(val) = db_pool_size {
            builder = builder.db_pool_size(val);
        }
        if let Some(val) = enable_recovery {
            builder = builder.enable_recovery(val);
        }
        if let Some(val) = enable_cron_scheduling {
            builder = builder.enable_cron_scheduling(val);
        }
        if let Some(val) = cron_poll_interval_seconds {
            builder = builder.cron_poll_interval(Duration::from_secs(val));
        }
        if let Some(val) = cron_max_catchup_executions {
            builder = builder.cron_max_catchup_executions(val);
        }
        if let Some(val) = cron_enable_recovery {
            builder = builder.cron_enable_recovery(val);
        }
        if let Some(val) = cron_recovery_interval_seconds {
            builder = builder.cron_recovery_interval(Duration::from_secs(val));
        }
        if let Some(val) = cron_lost_threshold_minutes {
            builder = builder.cron_lost_threshold_minutes(val);
        }
        if let Some(val) = cron_max_recovery_age_seconds {
            builder = builder.cron_max_recovery_age(Duration::from_secs(val));
        }
        if let Some(val) = cron_max_recovery_attempts {
            builder = builder.cron_max_recovery_attempts(val);
        }

        Ok(PyDefaultRunnerConfig {
            inner: builder.build().map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid configuration: {}", e))
            })?,
        })
    }
```

</details>



##### <span id="DefaultRunnerConfig.default"></span>`default`

```python
def default() -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::default](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.default)

Creates a DefaultRunnerConfig with all default values

<details>
<summary>Source</summary>

```python
    pub fn default() -> Self {
        PyDefaultRunnerConfig {
            inner: cloacina::runner::DefaultRunnerConfig::default(),
        }
    }
```

</details>



##### <span id="DefaultRunnerConfig.max_concurrent_tasks"></span>`max_concurrent_tasks`

```python
def max_concurrent_tasks() -> int
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::max_concurrent_tasks](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.max_concurrent_tasks)

<details>
<summary>Source</summary>

```python
    pub fn max_concurrent_tasks(&self) -> usize {
        self.inner.max_concurrent_tasks()
    }
```

</details>



##### <span id="DefaultRunnerConfig.scheduler_poll_interval_ms"></span>`scheduler_poll_interval_ms`

```python
def scheduler_poll_interval_ms() -> int
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::scheduler_poll_interval_ms](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.scheduler_poll_interval_ms)

<details>
<summary>Source</summary>

```python
    pub fn scheduler_poll_interval_ms(&self) -> u64 {
        self.inner.scheduler_poll_interval().as_millis() as u64
    }
```

</details>



##### <span id="DefaultRunnerConfig.task_timeout_seconds"></span>`task_timeout_seconds`

```python
def task_timeout_seconds() -> int
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::task_timeout_seconds](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.task_timeout_seconds)

<details>
<summary>Source</summary>

```python
    pub fn task_timeout_seconds(&self) -> u64 {
        self.inner.task_timeout().as_secs()
    }
```

</details>



##### <span id="DefaultRunnerConfig.workflow_timeout_seconds"></span>`workflow_timeout_seconds`

```python
def workflow_timeout_seconds() -> Option < u64 >
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::workflow_timeout_seconds](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.workflow_timeout_seconds)

<details>
<summary>Source</summary>

```python
    pub fn workflow_timeout_seconds(&self) -> Option<u64> {
        self.inner.workflow_timeout().map(|d| d.as_secs())
    }
```

</details>



##### <span id="DefaultRunnerConfig.db_pool_size"></span>`db_pool_size`

```python
def db_pool_size() -> int
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::db_pool_size](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.db_pool_size)

<details>
<summary>Source</summary>

```python
    pub fn db_pool_size(&self) -> u32 {
        self.inner.db_pool_size()
    }
```

</details>



##### <span id="DefaultRunnerConfig.enable_recovery"></span>`enable_recovery`

```python
def enable_recovery() -> bool
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::enable_recovery](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.enable_recovery)

<details>
<summary>Source</summary>

```python
    pub fn enable_recovery(&self) -> bool {
        self.inner.enable_recovery()
    }
```

</details>



##### <span id="DefaultRunnerConfig.enable_cron_scheduling"></span>`enable_cron_scheduling`

```python
def enable_cron_scheduling() -> bool
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::enable_cron_scheduling](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.enable_cron_scheduling)

<details>
<summary>Source</summary>

```python
    pub fn enable_cron_scheduling(&self) -> bool {
        self.inner.enable_cron_scheduling()
    }
```

</details>



##### <span id="DefaultRunnerConfig.cron_poll_interval_seconds"></span>`cron_poll_interval_seconds`

```python
def cron_poll_interval_seconds() -> int
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::cron_poll_interval_seconds](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.cron_poll_interval_seconds)

<details>
<summary>Source</summary>

```python
    pub fn cron_poll_interval_seconds(&self) -> u64 {
        self.inner.cron_poll_interval().as_secs()
    }
```

</details>



##### <span id="DefaultRunnerConfig.cron_max_catchup_executions"></span>`cron_max_catchup_executions`

```python
def cron_max_catchup_executions() -> int
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::cron_max_catchup_executions](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.cron_max_catchup_executions)

<details>
<summary>Source</summary>

```python
    pub fn cron_max_catchup_executions(&self) -> usize {
        self.inner.cron_max_catchup_executions()
    }
```

</details>



##### <span id="DefaultRunnerConfig.cron_enable_recovery"></span>`cron_enable_recovery`

```python
def cron_enable_recovery() -> bool
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::cron_enable_recovery](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.cron_enable_recovery)

<details>
<summary>Source</summary>

```python
    pub fn cron_enable_recovery(&self) -> bool {
        self.inner.cron_enable_recovery()
    }
```

</details>



##### <span id="DefaultRunnerConfig.cron_recovery_interval_seconds"></span>`cron_recovery_interval_seconds`

```python
def cron_recovery_interval_seconds() -> int
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::cron_recovery_interval_seconds](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.cron_recovery_interval_seconds)

<details>
<summary>Source</summary>

```python
    pub fn cron_recovery_interval_seconds(&self) -> u64 {
        self.inner.cron_recovery_interval().as_secs()
    }
```

</details>



##### <span id="DefaultRunnerConfig.cron_lost_threshold_minutes"></span>`cron_lost_threshold_minutes`

```python
def cron_lost_threshold_minutes() -> int
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::cron_lost_threshold_minutes](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.cron_lost_threshold_minutes)

<details>
<summary>Source</summary>

```python
    pub fn cron_lost_threshold_minutes(&self) -> i32 {
        self.inner.cron_lost_threshold_minutes()
    }
```

</details>



##### <span id="DefaultRunnerConfig.cron_max_recovery_age_seconds"></span>`cron_max_recovery_age_seconds`

```python
def cron_max_recovery_age_seconds() -> int
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::cron_max_recovery_age_seconds](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.cron_max_recovery_age_seconds)

<details>
<summary>Source</summary>

```python
    pub fn cron_max_recovery_age_seconds(&self) -> u64 {
        self.inner.cron_max_recovery_age().as_secs()
    }
```

</details>



##### <span id="DefaultRunnerConfig.cron_max_recovery_attempts"></span>`cron_max_recovery_attempts`

```python
def cron_max_recovery_attempts() -> int
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::cron_max_recovery_attempts](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.cron_max_recovery_attempts)

<details>
<summary>Source</summary>

```python
    pub fn cron_max_recovery_attempts(&self) -> usize {
        self.inner.cron_max_recovery_attempts()
    }
```

</details>



##### <span id="DefaultRunnerConfig.set_max_concurrent_tasks"></span>`set_max_concurrent_tasks`

```python
def set_max_concurrent_tasks(value: int)
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::set_max_concurrent_tasks](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_max_concurrent_tasks)

<details>
<summary>Source</summary>

```python
    pub fn set_max_concurrent_tasks(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.max_concurrent_tasks(value));
    }
```

</details>



##### <span id="DefaultRunnerConfig.set_scheduler_poll_interval_ms"></span>`set_scheduler_poll_interval_ms`

```python
def set_scheduler_poll_interval_ms(value: int)
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::set_scheduler_poll_interval_ms](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_scheduler_poll_interval_ms)

<details>
<summary>Source</summary>

```python
    pub fn set_scheduler_poll_interval_ms(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.scheduler_poll_interval(std::time::Duration::from_millis(value)));
    }
```

</details>



##### <span id="DefaultRunnerConfig.set_task_timeout_seconds"></span>`set_task_timeout_seconds`

```python
def set_task_timeout_seconds(value: int)
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::set_task_timeout_seconds](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_task_timeout_seconds)

<details>
<summary>Source</summary>

```python
    pub fn set_task_timeout_seconds(&mut self, value: u64) {
        self.inner = self.rebuild(|b| b.task_timeout(std::time::Duration::from_secs(value)));
    }
```

</details>



##### <span id="DefaultRunnerConfig.set_workflow_timeout_seconds"></span>`set_workflow_timeout_seconds`

```python
def set_workflow_timeout_seconds(value: Option < u64 >)
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::set_workflow_timeout_seconds](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_workflow_timeout_seconds)

<details>
<summary>Source</summary>

```python
    pub fn set_workflow_timeout_seconds(&mut self, value: Option<u64>) {
        self.inner =
            self.rebuild(|b| b.workflow_timeout(value.map(std::time::Duration::from_secs)));
    }
```

</details>



##### <span id="DefaultRunnerConfig.set_db_pool_size"></span>`set_db_pool_size`

```python
def set_db_pool_size(value: int)
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::set_db_pool_size](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_db_pool_size)

<details>
<summary>Source</summary>

```python
    pub fn set_db_pool_size(&mut self, value: u32) {
        self.inner = self.rebuild(|b| b.db_pool_size(value));
    }
```

</details>



##### <span id="DefaultRunnerConfig.set_enable_recovery"></span>`set_enable_recovery`

```python
def set_enable_recovery(value: bool)
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::set_enable_recovery](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_enable_recovery)

<details>
<summary>Source</summary>

```python
    pub fn set_enable_recovery(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.enable_recovery(value));
    }
```

</details>



##### <span id="DefaultRunnerConfig.set_enable_cron_scheduling"></span>`set_enable_cron_scheduling`

```python
def set_enable_cron_scheduling(value: bool)
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::set_enable_cron_scheduling](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_enable_cron_scheduling)

<details>
<summary>Source</summary>

```python
    pub fn set_enable_cron_scheduling(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.enable_cron_scheduling(value));
    }
```

</details>



##### <span id="DefaultRunnerConfig.set_cron_poll_interval_seconds"></span>`set_cron_poll_interval_seconds`

```python
def set_cron_poll_interval_seconds(value: int)
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::set_cron_poll_interval_seconds](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_cron_poll_interval_seconds)

<details>
<summary>Source</summary>

```python
    pub fn set_cron_poll_interval_seconds(&mut self, value: u64) {
        self.inner = self.rebuild(|b| b.cron_poll_interval(std::time::Duration::from_secs(value)));
    }
```

</details>



##### <span id="DefaultRunnerConfig.set_cron_max_catchup_executions"></span>`set_cron_max_catchup_executions`

```python
def set_cron_max_catchup_executions(value: int)
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::set_cron_max_catchup_executions](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_cron_max_catchup_executions)

<details>
<summary>Source</summary>

```python
    pub fn set_cron_max_catchup_executions(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.cron_max_catchup_executions(value));
    }
```

</details>



##### <span id="DefaultRunnerConfig.set_cron_enable_recovery"></span>`set_cron_enable_recovery`

```python
def set_cron_enable_recovery(value: bool)
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::set_cron_enable_recovery](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_cron_enable_recovery)

<details>
<summary>Source</summary>

```python
    pub fn set_cron_enable_recovery(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.cron_enable_recovery(value));
    }
```

</details>



##### <span id="DefaultRunnerConfig.set_cron_recovery_interval_seconds"></span>`set_cron_recovery_interval_seconds`

```python
def set_cron_recovery_interval_seconds(value: int)
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::set_cron_recovery_interval_seconds](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_cron_recovery_interval_seconds)

<details>
<summary>Source</summary>

```python
    pub fn set_cron_recovery_interval_seconds(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.cron_recovery_interval(std::time::Duration::from_secs(value)));
    }
```

</details>



##### <span id="DefaultRunnerConfig.set_cron_lost_threshold_minutes"></span>`set_cron_lost_threshold_minutes`

```python
def set_cron_lost_threshold_minutes(value: int)
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::set_cron_lost_threshold_minutes](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_cron_lost_threshold_minutes)

<details>
<summary>Source</summary>

```python
    pub fn set_cron_lost_threshold_minutes(&mut self, value: i32) {
        self.inner = self.rebuild(|b| b.cron_lost_threshold_minutes(value));
    }
```

</details>



##### <span id="DefaultRunnerConfig.set_cron_max_recovery_age_seconds"></span>`set_cron_max_recovery_age_seconds`

```python
def set_cron_max_recovery_age_seconds(value: int)
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::set_cron_max_recovery_age_seconds](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_cron_max_recovery_age_seconds)

<details>
<summary>Source</summary>

```python
    pub fn set_cron_max_recovery_age_seconds(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.cron_max_recovery_age(std::time::Duration::from_secs(value)));
    }
```

</details>



##### <span id="DefaultRunnerConfig.set_cron_max_recovery_attempts"></span>`set_cron_max_recovery_attempts`

```python
def set_cron_max_recovery_attempts(value: int)
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::set_cron_max_recovery_attempts](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_cron_max_recovery_attempts)

<details>
<summary>Source</summary>

```python
    pub fn set_cron_max_recovery_attempts(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.cron_max_recovery_attempts(value));
    }
```

</details>



##### <span id="DefaultRunnerConfig.to_dict"></span>`to_dict`

```python
def to_dict() -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::to_dict](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.to_dict)

Returns a dictionary representation of the configuration

<details>
<summary>Source</summary>

```python
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new(py);

        dict.set_item("max_concurrent_tasks", self.inner.max_concurrent_tasks())?;
        dict.set_item(
            "scheduler_poll_interval_ms",
            self.inner.scheduler_poll_interval().as_millis() as u64,
        )?;
        dict.set_item("task_timeout_seconds", self.inner.task_timeout().as_secs())?;
        dict.set_item(
            "workflow_timeout_seconds",
            self.inner.workflow_timeout().map(|d| d.as_secs()),
        )?;
        dict.set_item("db_pool_size", self.inner.db_pool_size())?;
        dict.set_item("enable_recovery", self.inner.enable_recovery())?;
        dict.set_item(
            "enable_cron_scheduling",
            self.inner.enable_cron_scheduling(),
        )?;
        dict.set_item(
            "cron_poll_interval_seconds",
            self.inner.cron_poll_interval().as_secs(),
        )?;
        dict.set_item(
            "cron_max_catchup_executions",
            self.inner.cron_max_catchup_executions(),
        )?;
        dict.set_item("cron_enable_recovery", self.inner.cron_enable_recovery())?;
        dict.set_item(
            "cron_recovery_interval_seconds",
            self.inner.cron_recovery_interval().as_secs(),
        )?;
        dict.set_item(
            "cron_lost_threshold_minutes",
            self.inner.cron_lost_threshold_minutes(),
        )?;
        dict.set_item(
            "cron_max_recovery_age_seconds",
            self.inner.cron_max_recovery_age().as_secs(),
        )?;
        dict.set_item(
            "cron_max_recovery_attempts",
            self.inner.cron_max_recovery_attempts(),
        )?;

        Ok(dict.into())
    }
```

</details>



##### <span id="DefaultRunnerConfig.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::context::PyDefaultRunnerConfig::__repr__](../../../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.__repr__)

String representation of the configuration

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        format!(
            "DefaultRunnerConfig(max_concurrent_tasks={}, enable_cron_scheduling={}, db_pool_size={})",
            self.inner.max_concurrent_tasks(),
            self.inner.enable_cron_scheduling(),
            self.inner.db_pool_size()
        )
    }
```

</details>





