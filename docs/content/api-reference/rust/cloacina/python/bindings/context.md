# cloacina::python::bindings::context <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


## Structs

### `cloacina::python::bindings::context::DefaultRunnerConfig`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig](../../../../cloaca/python/bindings/context.md#class-defaultrunnerconfig)

PyDefaultRunnerConfig - Python wrapper for Rust DefaultRunnerConfig

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `crate :: runner :: DefaultRunnerConfig` |  |

#### Methods

##### `new`

```rust
fn new (max_concurrent_tasks : Option < usize > , scheduler_poll_interval_ms : Option < u64 > , task_timeout_seconds : Option < u64 > , pipeline_timeout_seconds : Option < u64 > , db_pool_size : Option < u32 > , enable_recovery : Option < bool > , enable_cron_scheduling : Option < bool > , cron_poll_interval_seconds : Option < u64 > , cron_max_catchup_executions : Option < usize > , cron_enable_recovery : Option < bool > , cron_recovery_interval_seconds : Option < u64 > , cron_lost_threshold_minutes : Option < i32 > , cron_max_recovery_age_seconds : Option < u64 > , cron_max_recovery_attempts : Option < usize > ,) -> Self
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.new](../../../../cloaca/python/bindings/context.md#new)

Creates a new DefaultRunnerConfig with customizable parameters

<details>
<summary>Source</summary>

```rust
    pub fn new(
        max_concurrent_tasks: Option<usize>,
        scheduler_poll_interval_ms: Option<u64>,
        task_timeout_seconds: Option<u64>,
        pipeline_timeout_seconds: Option<u64>,
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
    ) -> Self {
        use std::time::Duration;

        let mut builder = crate::runner::DefaultRunnerConfig::builder();

        if let Some(val) = max_concurrent_tasks {
            builder = builder.max_concurrent_tasks(val);
        }
        if let Some(val) = scheduler_poll_interval_ms {
            builder = builder.scheduler_poll_interval(Duration::from_millis(val));
        }
        if let Some(val) = task_timeout_seconds {
            builder = builder.task_timeout(Duration::from_secs(val));
        }
        if let Some(val) = pipeline_timeout_seconds {
            builder = builder.pipeline_timeout(Some(Duration::from_secs(val)));
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

        PyDefaultRunnerConfig {
            inner: builder.build(),
        }
    }
```

</details>



##### `default`

```rust
fn default () -> Self
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.default](../../../../cloaca/python/bindings/context.md#default)

Creates a DefaultRunnerConfig with all default values

<details>
<summary>Source</summary>

```rust
    pub fn default() -> Self {
        PyDefaultRunnerConfig {
            inner: crate::runner::DefaultRunnerConfig::default(),
        }
    }
```

</details>



##### `max_concurrent_tasks`

```rust
fn max_concurrent_tasks (& self) -> usize
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.max_concurrent_tasks](../../../../cloaca/python/bindings/context.md#max_concurrent_tasks)

<details>
<summary>Source</summary>

```rust
    pub fn max_concurrent_tasks(&self) -> usize {
        self.inner.max_concurrent_tasks()
    }
```

</details>



##### `scheduler_poll_interval_ms`

```rust
fn scheduler_poll_interval_ms (& self) -> u64
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.scheduler_poll_interval_ms](../../../../cloaca/python/bindings/context.md#scheduler_poll_interval_ms)

<details>
<summary>Source</summary>

```rust
    pub fn scheduler_poll_interval_ms(&self) -> u64 {
        self.inner.scheduler_poll_interval().as_millis() as u64
    }
```

</details>



##### `task_timeout_seconds`

```rust
fn task_timeout_seconds (& self) -> u64
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.task_timeout_seconds](../../../../cloaca/python/bindings/context.md#task_timeout_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn task_timeout_seconds(&self) -> u64 {
        self.inner.task_timeout().as_secs()
    }
```

</details>



##### `pipeline_timeout_seconds`

```rust
fn pipeline_timeout_seconds (& self) -> Option < u64 >
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.pipeline_timeout_seconds](../../../../cloaca/python/bindings/context.md#pipeline_timeout_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn pipeline_timeout_seconds(&self) -> Option<u64> {
        self.inner.pipeline_timeout().map(|d| d.as_secs())
    }
```

</details>



##### `db_pool_size`

```rust
fn db_pool_size (& self) -> u32
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.db_pool_size](../../../../cloaca/python/bindings/context.md#db_pool_size)

<details>
<summary>Source</summary>

```rust
    pub fn db_pool_size(&self) -> u32 {
        self.inner.db_pool_size()
    }
```

</details>



##### `enable_recovery`

```rust
fn enable_recovery (& self) -> bool
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.enable_recovery](../../../../cloaca/python/bindings/context.md#enable_recovery)

<details>
<summary>Source</summary>

```rust
    pub fn enable_recovery(&self) -> bool {
        self.inner.enable_recovery()
    }
```

</details>



##### `enable_cron_scheduling`

```rust
fn enable_cron_scheduling (& self) -> bool
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.enable_cron_scheduling](../../../../cloaca/python/bindings/context.md#enable_cron_scheduling)

<details>
<summary>Source</summary>

```rust
    pub fn enable_cron_scheduling(&self) -> bool {
        self.inner.enable_cron_scheduling()
    }
```

</details>



##### `cron_poll_interval_seconds`

```rust
fn cron_poll_interval_seconds (& self) -> u64
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.cron_poll_interval_seconds](../../../../cloaca/python/bindings/context.md#cron_poll_interval_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn cron_poll_interval_seconds(&self) -> u64 {
        self.inner.cron_poll_interval().as_secs()
    }
```

</details>



##### `cron_max_catchup_executions`

```rust
fn cron_max_catchup_executions (& self) -> usize
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.cron_max_catchup_executions](../../../../cloaca/python/bindings/context.md#cron_max_catchup_executions)

<details>
<summary>Source</summary>

```rust
    pub fn cron_max_catchup_executions(&self) -> usize {
        self.inner.cron_max_catchup_executions()
    }
```

</details>



##### `cron_enable_recovery`

```rust
fn cron_enable_recovery (& self) -> bool
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.cron_enable_recovery](../../../../cloaca/python/bindings/context.md#cron_enable_recovery)

<details>
<summary>Source</summary>

```rust
    pub fn cron_enable_recovery(&self) -> bool {
        self.inner.cron_enable_recovery()
    }
```

</details>



##### `cron_recovery_interval_seconds`

```rust
fn cron_recovery_interval_seconds (& self) -> u64
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.cron_recovery_interval_seconds](../../../../cloaca/python/bindings/context.md#cron_recovery_interval_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn cron_recovery_interval_seconds(&self) -> u64 {
        self.inner.cron_recovery_interval().as_secs()
    }
```

</details>



##### `cron_lost_threshold_minutes`

```rust
fn cron_lost_threshold_minutes (& self) -> i32
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.cron_lost_threshold_minutes](../../../../cloaca/python/bindings/context.md#cron_lost_threshold_minutes)

<details>
<summary>Source</summary>

```rust
    pub fn cron_lost_threshold_minutes(&self) -> i32 {
        self.inner.cron_lost_threshold_minutes()
    }
```

</details>



##### `cron_max_recovery_age_seconds`

```rust
fn cron_max_recovery_age_seconds (& self) -> u64
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.cron_max_recovery_age_seconds](../../../../cloaca/python/bindings/context.md#cron_max_recovery_age_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn cron_max_recovery_age_seconds(&self) -> u64 {
        self.inner.cron_max_recovery_age().as_secs()
    }
```

</details>



##### `cron_max_recovery_attempts`

```rust
fn cron_max_recovery_attempts (& self) -> usize
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.cron_max_recovery_attempts](../../../../cloaca/python/bindings/context.md#cron_max_recovery_attempts)

<details>
<summary>Source</summary>

```rust
    pub fn cron_max_recovery_attempts(&self) -> usize {
        self.inner.cron_max_recovery_attempts()
    }
```

</details>



##### `set_max_concurrent_tasks`

```rust
fn set_max_concurrent_tasks (& mut self , value : usize)
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.set_max_concurrent_tasks](../../../../cloaca/python/bindings/context.md#set_max_concurrent_tasks)

<details>
<summary>Source</summary>

```rust
    pub fn set_max_concurrent_tasks(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.max_concurrent_tasks(value));
    }
```

</details>



##### `set_scheduler_poll_interval_ms`

```rust
fn set_scheduler_poll_interval_ms (& mut self , value : u64)
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.set_scheduler_poll_interval_ms](../../../../cloaca/python/bindings/context.md#set_scheduler_poll_interval_ms)

<details>
<summary>Source</summary>

```rust
    pub fn set_scheduler_poll_interval_ms(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.scheduler_poll_interval(std::time::Duration::from_millis(value)));
    }
```

</details>



##### `set_task_timeout_seconds`

```rust
fn set_task_timeout_seconds (& mut self , value : u64)
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.set_task_timeout_seconds](../../../../cloaca/python/bindings/context.md#set_task_timeout_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn set_task_timeout_seconds(&mut self, value: u64) {
        self.inner = self.rebuild(|b| b.task_timeout(std::time::Duration::from_secs(value)));
    }
```

</details>



##### `set_pipeline_timeout_seconds`

```rust
fn set_pipeline_timeout_seconds (& mut self , value : Option < u64 >)
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.set_pipeline_timeout_seconds](../../../../cloaca/python/bindings/context.md#set_pipeline_timeout_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn set_pipeline_timeout_seconds(&mut self, value: Option<u64>) {
        self.inner =
            self.rebuild(|b| b.pipeline_timeout(value.map(std::time::Duration::from_secs)));
    }
```

</details>



##### `set_db_pool_size`

```rust
fn set_db_pool_size (& mut self , value : u32)
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.set_db_pool_size](../../../../cloaca/python/bindings/context.md#set_db_pool_size)

<details>
<summary>Source</summary>

```rust
    pub fn set_db_pool_size(&mut self, value: u32) {
        self.inner = self.rebuild(|b| b.db_pool_size(value));
    }
```

</details>



##### `set_enable_recovery`

```rust
fn set_enable_recovery (& mut self , value : bool)
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.set_enable_recovery](../../../../cloaca/python/bindings/context.md#set_enable_recovery)

<details>
<summary>Source</summary>

```rust
    pub fn set_enable_recovery(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.enable_recovery(value));
    }
```

</details>



##### `set_enable_cron_scheduling`

```rust
fn set_enable_cron_scheduling (& mut self , value : bool)
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.set_enable_cron_scheduling](../../../../cloaca/python/bindings/context.md#set_enable_cron_scheduling)

<details>
<summary>Source</summary>

```rust
    pub fn set_enable_cron_scheduling(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.enable_cron_scheduling(value));
    }
```

</details>



##### `set_cron_poll_interval_seconds`

```rust
fn set_cron_poll_interval_seconds (& mut self , value : u64)
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.set_cron_poll_interval_seconds](../../../../cloaca/python/bindings/context.md#set_cron_poll_interval_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_poll_interval_seconds(&mut self, value: u64) {
        self.inner = self.rebuild(|b| b.cron_poll_interval(std::time::Duration::from_secs(value)));
    }
```

</details>



##### `set_cron_max_catchup_executions`

```rust
fn set_cron_max_catchup_executions (& mut self , value : usize)
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.set_cron_max_catchup_executions](../../../../cloaca/python/bindings/context.md#set_cron_max_catchup_executions)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_max_catchup_executions(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.cron_max_catchup_executions(value));
    }
```

</details>



##### `set_cron_enable_recovery`

```rust
fn set_cron_enable_recovery (& mut self , value : bool)
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.set_cron_enable_recovery](../../../../cloaca/python/bindings/context.md#set_cron_enable_recovery)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_enable_recovery(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.cron_enable_recovery(value));
    }
```

</details>



##### `set_cron_recovery_interval_seconds`

```rust
fn set_cron_recovery_interval_seconds (& mut self , value : u64)
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.set_cron_recovery_interval_seconds](../../../../cloaca/python/bindings/context.md#set_cron_recovery_interval_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_recovery_interval_seconds(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.cron_recovery_interval(std::time::Duration::from_secs(value)));
    }
```

</details>



##### `set_cron_lost_threshold_minutes`

```rust
fn set_cron_lost_threshold_minutes (& mut self , value : i32)
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.set_cron_lost_threshold_minutes](../../../../cloaca/python/bindings/context.md#set_cron_lost_threshold_minutes)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_lost_threshold_minutes(&mut self, value: i32) {
        self.inner = self.rebuild(|b| b.cron_lost_threshold_minutes(value));
    }
```

</details>



##### `set_cron_max_recovery_age_seconds`

```rust
fn set_cron_max_recovery_age_seconds (& mut self , value : u64)
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.set_cron_max_recovery_age_seconds](../../../../cloaca/python/bindings/context.md#set_cron_max_recovery_age_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_max_recovery_age_seconds(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.cron_max_recovery_age(std::time::Duration::from_secs(value)));
    }
```

</details>



##### `set_cron_max_recovery_attempts`

```rust
fn set_cron_max_recovery_attempts (& mut self , value : usize)
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.set_cron_max_recovery_attempts](../../../../cloaca/python/bindings/context.md#set_cron_max_recovery_attempts)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_max_recovery_attempts(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.cron_max_recovery_attempts(value));
    }
```

</details>



##### `to_dict`

```rust
fn to_dict (& self , py : Python < '_ >) -> PyResult < PyObject >
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.to_dict](../../../../cloaca/python/bindings/context.md#to_dict)

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
            "pipeline_timeout_seconds",
            self.inner.pipeline_timeout().map(|d| d.as_secs()),
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



##### `__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.bindings.context.DefaultRunnerConfig.__repr__](../../../../cloaca/python/bindings/context.md#__repr__)

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



##### `to_rust_config` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">pub(crate)</span>


```rust
fn to_rust_config (& self) -> crate :: runner :: DefaultRunnerConfig
```

Get the inner Rust config (for internal use)

<details>
<summary>Source</summary>

```rust
    pub(crate) fn to_rust_config(&self) -> crate::runner::DefaultRunnerConfig {
        self.inner.clone()
    }
```

</details>



##### `rebuild` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn rebuild (& self , apply : impl FnOnce (crate :: runner :: DefaultRunnerConfigBuilder ,) -> crate :: runner :: DefaultRunnerConfigBuilder ,) -> crate :: runner :: DefaultRunnerConfig
```

<details>
<summary>Source</summary>

```rust
    fn rebuild(
        &self,
        apply: impl FnOnce(
            crate::runner::DefaultRunnerConfigBuilder,
        ) -> crate::runner::DefaultRunnerConfigBuilder,
    ) -> crate::runner::DefaultRunnerConfig {
        let c = &self.inner;
        let builder = crate::runner::DefaultRunnerConfig::builder()
            .max_concurrent_tasks(c.max_concurrent_tasks())
            .scheduler_poll_interval(c.scheduler_poll_interval())
            .task_timeout(c.task_timeout())
            .pipeline_timeout(c.pipeline_timeout())
            .db_pool_size(c.db_pool_size())
            .enable_recovery(c.enable_recovery())
            .enable_cron_scheduling(c.enable_cron_scheduling())
            .cron_poll_interval(c.cron_poll_interval())
            .cron_max_catchup_executions(c.cron_max_catchup_executions())
            .cron_enable_recovery(c.cron_enable_recovery())
            .cron_recovery_interval(c.cron_recovery_interval())
            .cron_lost_threshold_minutes(c.cron_lost_threshold_minutes())
            .cron_max_recovery_age(c.cron_max_recovery_age())
            .cron_max_recovery_attempts(c.cron_max_recovery_attempts());
        apply(builder).build()
    }
```

</details>
