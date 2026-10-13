---
title: "cloacina-python::bindings::context"
description: "API reference of the Rust module cloacina-python::bindings::context."
editUrl: false
sidebar:
  order: 7
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

## Re-exports

- [`PyContext`](../../../../rust/cloacina-python/context/#struct.PyContext)

## Structs

### <span id="struct.PyDefaultRunnerConfig"></span><code style="font-size: 0.85em">class DefaultRunnerConfig</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.DefaultRunnerConfig`](../../../../cloaca/#class.DefaultRunnerConfig)

PyDefaultRunnerConfig - Python wrapper for Rust DefaultRunnerConfig

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `cloacina::runner::DefaultRunnerConfig` |  |

#### Methods

##### <span id="PyDefaultRunnerConfig.new"></span><code>new</code>

```rust
fn new(
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
) -> pyo3::PyResult<Self>
```

> **Python API**: [`cloaca.DefaultRunnerConfig.new`](../../../../cloaca/#DefaultRunnerConfig.new)

Creates a new DefaultRunnerConfig with customizable parameters

<details>
<summary>Source</summary>

```rust
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

##### <span id="PyDefaultRunnerConfig.default"></span><code>default</code>

```rust
fn default() -> Self
```

> **Python API**: [`cloaca.DefaultRunnerConfig.default`](../../../../cloaca/#DefaultRunnerConfig.default)

Creates a DefaultRunnerConfig with all default values

<details>
<summary>Source</summary>

```rust
    pub fn default() -> Self {
        PyDefaultRunnerConfig {
            inner: cloacina::runner::DefaultRunnerConfig::default(),
        }
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.max_concurrent_tasks"></span><code>max_​concurrent_​tasks</code>

```rust
fn max_concurrent_tasks(&self) -> usize
```

> **Python API**: [`cloaca.DefaultRunnerConfig.max_concurrent_tasks`](../../../../cloaca/#DefaultRunnerConfig.max_concurrent_tasks)

<details>
<summary>Source</summary>

```rust
    pub fn max_concurrent_tasks(&self) -> usize {
        self.inner.max_concurrent_tasks()
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.scheduler_poll_interval_ms"></span><code>scheduler_​poll_​interval_​ms</code>

```rust
fn scheduler_poll_interval_ms(&self) -> u64
```

> **Python API**: [`cloaca.DefaultRunnerConfig.scheduler_poll_interval_ms`](../../../../cloaca/#DefaultRunnerConfig.scheduler_poll_interval_ms)

<details>
<summary>Source</summary>

```rust
    pub fn scheduler_poll_interval_ms(&self) -> u64 {
        self.inner.scheduler_poll_interval().as_millis() as u64
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.task_timeout_seconds"></span><code>task_​timeout_​seconds</code>

```rust
fn task_timeout_seconds(&self) -> u64
```

> **Python API**: [`cloaca.DefaultRunnerConfig.task_timeout_seconds`](../../../../cloaca/#DefaultRunnerConfig.task_timeout_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn task_timeout_seconds(&self) -> u64 {
        self.inner.task_timeout().as_secs()
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.workflow_timeout_seconds"></span><code>workflow_​timeout_​seconds</code>

```rust
fn workflow_timeout_seconds(&self) -> Option<u64>
```

> **Python API**: [`cloaca.DefaultRunnerConfig.workflow_timeout_seconds`](../../../../cloaca/#DefaultRunnerConfig.workflow_timeout_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn workflow_timeout_seconds(&self) -> Option<u64> {
        self.inner.workflow_timeout().map(|d| d.as_secs())
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.db_pool_size"></span><code>db_​pool_​size</code>

```rust
fn db_pool_size(&self) -> u32
```

> **Python API**: [`cloaca.DefaultRunnerConfig.db_pool_size`](../../../../cloaca/#DefaultRunnerConfig.db_pool_size)

<details>
<summary>Source</summary>

```rust
    pub fn db_pool_size(&self) -> u32 {
        self.inner.db_pool_size()
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.enable_recovery"></span><code>enable_​recovery</code>

```rust
fn enable_recovery(&self) -> bool
```

> **Python API**: [`cloaca.DefaultRunnerConfig.enable_recovery`](../../../../cloaca/#DefaultRunnerConfig.enable_recovery)

<details>
<summary>Source</summary>

```rust
    pub fn enable_recovery(&self) -> bool {
        self.inner.enable_recovery()
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.enable_cron_scheduling"></span><code>enable_​cron_​scheduling</code>

```rust
fn enable_cron_scheduling(&self) -> bool
```

> **Python API**: [`cloaca.DefaultRunnerConfig.enable_cron_scheduling`](../../../../cloaca/#DefaultRunnerConfig.enable_cron_scheduling)

<details>
<summary>Source</summary>

```rust
    pub fn enable_cron_scheduling(&self) -> bool {
        self.inner.enable_cron_scheduling()
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.cron_poll_interval_seconds"></span><code>cron_​poll_​interval_​seconds</code>

```rust
fn cron_poll_interval_seconds(&self) -> u64
```

> **Python API**: [`cloaca.DefaultRunnerConfig.cron_poll_interval_seconds`](../../../../cloaca/#DefaultRunnerConfig.cron_poll_interval_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn cron_poll_interval_seconds(&self) -> u64 {
        self.inner.cron_poll_interval().as_secs()
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.cron_max_catchup_executions"></span><code>cron_​max_​catchup_​executions</code>

```rust
fn cron_max_catchup_executions(&self) -> usize
```

> **Python API**: [`cloaca.DefaultRunnerConfig.cron_max_catchup_executions`](../../../../cloaca/#DefaultRunnerConfig.cron_max_catchup_executions)

<details>
<summary>Source</summary>

```rust
    pub fn cron_max_catchup_executions(&self) -> usize {
        self.inner.cron_max_catchup_executions()
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.cron_enable_recovery"></span><code>cron_​enable_​recovery</code>

```rust
fn cron_enable_recovery(&self) -> bool
```

> **Python API**: [`cloaca.DefaultRunnerConfig.cron_enable_recovery`](../../../../cloaca/#DefaultRunnerConfig.cron_enable_recovery)

<details>
<summary>Source</summary>

```rust
    pub fn cron_enable_recovery(&self) -> bool {
        self.inner.cron_enable_recovery()
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.cron_recovery_interval_seconds"></span><code>cron_​recovery_​interval_​seconds</code>

```rust
fn cron_recovery_interval_seconds(&self) -> u64
```

> **Python API**: [`cloaca.DefaultRunnerConfig.cron_recovery_interval_seconds`](../../../../cloaca/#DefaultRunnerConfig.cron_recovery_interval_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn cron_recovery_interval_seconds(&self) -> u64 {
        self.inner.cron_recovery_interval().as_secs()
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.cron_lost_threshold_minutes"></span><code>cron_​lost_​threshold_​minutes</code>

```rust
fn cron_lost_threshold_minutes(&self) -> i32
```

> **Python API**: [`cloaca.DefaultRunnerConfig.cron_lost_threshold_minutes`](../../../../cloaca/#DefaultRunnerConfig.cron_lost_threshold_minutes)

<details>
<summary>Source</summary>

```rust
    pub fn cron_lost_threshold_minutes(&self) -> i32 {
        self.inner.cron_lost_threshold_minutes()
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.cron_max_recovery_age_seconds"></span><code>cron_​max_​recovery_​age_​seconds</code>

```rust
fn cron_max_recovery_age_seconds(&self) -> u64
```

> **Python API**: [`cloaca.DefaultRunnerConfig.cron_max_recovery_age_seconds`](../../../../cloaca/#DefaultRunnerConfig.cron_max_recovery_age_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn cron_max_recovery_age_seconds(&self) -> u64 {
        self.inner.cron_max_recovery_age().as_secs()
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.cron_max_recovery_attempts"></span><code>cron_​max_​recovery_​attempts</code>

```rust
fn cron_max_recovery_attempts(&self) -> usize
```

> **Python API**: [`cloaca.DefaultRunnerConfig.cron_max_recovery_attempts`](../../../../cloaca/#DefaultRunnerConfig.cron_max_recovery_attempts)

<details>
<summary>Source</summary>

```rust
    pub fn cron_max_recovery_attempts(&self) -> usize {
        self.inner.cron_max_recovery_attempts()
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.set_max_concurrent_tasks"></span><code>set_​max_​concurrent_​tasks</code>

```rust
fn set_max_concurrent_tasks(&mut self, value: usize)
```

> **Python API**: [`cloaca.DefaultRunnerConfig.set_max_concurrent_tasks`](../../../../cloaca/#DefaultRunnerConfig.set_max_concurrent_tasks)

<details>
<summary>Source</summary>

```rust
    pub fn set_max_concurrent_tasks(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.max_concurrent_tasks(value));
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.set_scheduler_poll_interval_ms"></span><code>set_​scheduler_​poll_​interval_​ms</code>

```rust
fn set_scheduler_poll_interval_ms(&mut self, value: u64)
```

> **Python API**: [`cloaca.DefaultRunnerConfig.set_scheduler_poll_interval_ms`](../../../../cloaca/#DefaultRunnerConfig.set_scheduler_poll_interval_ms)

<details>
<summary>Source</summary>

```rust
    pub fn set_scheduler_poll_interval_ms(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.scheduler_poll_interval(std::time::Duration::from_millis(value)));
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.set_task_timeout_seconds"></span><code>set_​task_​timeout_​seconds</code>

```rust
fn set_task_timeout_seconds(&mut self, value: u64)
```

> **Python API**: [`cloaca.DefaultRunnerConfig.set_task_timeout_seconds`](../../../../cloaca/#DefaultRunnerConfig.set_task_timeout_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn set_task_timeout_seconds(&mut self, value: u64) {
        self.inner = self.rebuild(|b| b.task_timeout(std::time::Duration::from_secs(value)));
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.set_workflow_timeout_seconds"></span><code>set_​workflow_​timeout_​seconds</code>

```rust
fn set_workflow_timeout_seconds(&mut self, value: Option<u64>)
```

> **Python API**: [`cloaca.DefaultRunnerConfig.set_workflow_timeout_seconds`](../../../../cloaca/#DefaultRunnerConfig.set_workflow_timeout_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn set_workflow_timeout_seconds(&mut self, value: Option<u64>) {
        self.inner =
            self.rebuild(|b| b.workflow_timeout(value.map(std::time::Duration::from_secs)));
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.set_db_pool_size"></span><code>set_​db_​pool_​size</code>

```rust
fn set_db_pool_size(&mut self, value: u32)
```

> **Python API**: [`cloaca.DefaultRunnerConfig.set_db_pool_size`](../../../../cloaca/#DefaultRunnerConfig.set_db_pool_size)

<details>
<summary>Source</summary>

```rust
    pub fn set_db_pool_size(&mut self, value: u32) {
        self.inner = self.rebuild(|b| b.db_pool_size(value));
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.set_enable_recovery"></span><code>set_​enable_​recovery</code>

```rust
fn set_enable_recovery(&mut self, value: bool)
```

> **Python API**: [`cloaca.DefaultRunnerConfig.set_enable_recovery`](../../../../cloaca/#DefaultRunnerConfig.set_enable_recovery)

<details>
<summary>Source</summary>

```rust
    pub fn set_enable_recovery(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.enable_recovery(value));
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.set_enable_cron_scheduling"></span><code>set_​enable_​cron_​scheduling</code>

```rust
fn set_enable_cron_scheduling(&mut self, value: bool)
```

> **Python API**: [`cloaca.DefaultRunnerConfig.set_enable_cron_scheduling`](../../../../cloaca/#DefaultRunnerConfig.set_enable_cron_scheduling)

<details>
<summary>Source</summary>

```rust
    pub fn set_enable_cron_scheduling(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.enable_cron_scheduling(value));
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.set_cron_poll_interval_seconds"></span><code>set_​cron_​poll_​interval_​seconds</code>

```rust
fn set_cron_poll_interval_seconds(&mut self, value: u64)
```

> **Python API**: [`cloaca.DefaultRunnerConfig.set_cron_poll_interval_seconds`](../../../../cloaca/#DefaultRunnerConfig.set_cron_poll_interval_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_poll_interval_seconds(&mut self, value: u64) {
        self.inner = self.rebuild(|b| b.cron_poll_interval(std::time::Duration::from_secs(value)));
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.set_cron_max_catchup_executions"></span><code>set_​cron_​max_​catchup_​executions</code>

```rust
fn set_cron_max_catchup_executions(&mut self, value: usize)
```

> **Python API**: [`cloaca.DefaultRunnerConfig.set_cron_max_catchup_executions`](../../../../cloaca/#DefaultRunnerConfig.set_cron_max_catchup_executions)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_max_catchup_executions(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.cron_max_catchup_executions(value));
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.set_cron_enable_recovery"></span><code>set_​cron_​enable_​recovery</code>

```rust
fn set_cron_enable_recovery(&mut self, value: bool)
```

> **Python API**: [`cloaca.DefaultRunnerConfig.set_cron_enable_recovery`](../../../../cloaca/#DefaultRunnerConfig.set_cron_enable_recovery)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_enable_recovery(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.cron_enable_recovery(value));
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.set_cron_recovery_interval_seconds"></span><code>set_​cron_​recovery_​interval_​seconds</code>

```rust
fn set_cron_recovery_interval_seconds(&mut self, value: u64)
```

> **Python API**: [`cloaca.DefaultRunnerConfig.set_cron_recovery_interval_seconds`](../../../../cloaca/#DefaultRunnerConfig.set_cron_recovery_interval_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_recovery_interval_seconds(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.cron_recovery_interval(std::time::Duration::from_secs(value)));
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.set_cron_lost_threshold_minutes"></span><code>set_​cron_​lost_​threshold_​minutes</code>

```rust
fn set_cron_lost_threshold_minutes(&mut self, value: i32)
```

> **Python API**: [`cloaca.DefaultRunnerConfig.set_cron_lost_threshold_minutes`](../../../../cloaca/#DefaultRunnerConfig.set_cron_lost_threshold_minutes)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_lost_threshold_minutes(&mut self, value: i32) {
        self.inner = self.rebuild(|b| b.cron_lost_threshold_minutes(value));
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.set_cron_max_recovery_age_seconds"></span><code>set_​cron_​max_​recovery_​age_​seconds</code>

```rust
fn set_cron_max_recovery_age_seconds(&mut self, value: u64)
```

> **Python API**: [`cloaca.DefaultRunnerConfig.set_cron_max_recovery_age_seconds`](../../../../cloaca/#DefaultRunnerConfig.set_cron_max_recovery_age_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_max_recovery_age_seconds(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.cron_max_recovery_age(std::time::Duration::from_secs(value)));
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.set_cron_max_recovery_attempts"></span><code>set_​cron_​max_​recovery_​attempts</code>

```rust
fn set_cron_max_recovery_attempts(&mut self, value: usize)
```

> **Python API**: [`cloaca.DefaultRunnerConfig.set_cron_max_recovery_attempts`](../../../../cloaca/#DefaultRunnerConfig.set_cron_max_recovery_attempts)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_max_recovery_attempts(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.cron_max_recovery_attempts(value));
    }
```

</details>

##### <span id="PyDefaultRunnerConfig.to_dict"></span><code>to_​dict</code>

```rust
fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject>
```

> **Python API**: [`cloaca.DefaultRunnerConfig.to_dict`](../../../../cloaca/#DefaultRunnerConfig.to_dict)

Returns a dictionary representation of the configuration

<details>
<summary>Source</summary>

```rust
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

##### <span id="PyDefaultRunnerConfig.__repr__"></span><code>_​_​repr_​_​</code>

```rust
fn __repr__(&self) -> String
```

> **Python API**: [`cloaca.DefaultRunnerConfig.__repr__`](../../../../cloaca/#DefaultRunnerConfig.__repr__)

String representation of the configuration

<details>
<summary>Source</summary>

```rust
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
