# cloaca.python.bindings.context <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


## Classes

### `cloaca.python.bindings.context.DefaultRunnerConfig`

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig](../../../rust/cloacina/python/bindings/context.md#class-defaultrunnerconfig)

PyDefaultRunnerConfig - Python wrapper for Rust DefaultRunnerConfig

#### Methods

##### `new`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">new</span>(max_concurrent_tasks: Optional[int], scheduler_poll_interval_ms: Optional[int], task_timeout_seconds: Optional[int], pipeline_timeout_seconds: Optional[int], db_pool_size: Optional[int], enable_recovery: Optional[bool], enable_cron_scheduling: Optional[bool], cron_poll_interval_seconds: Optional[int], cron_max_catchup_executions: Optional[int], cron_enable_recovery: Optional[bool], cron_recovery_interval_seconds: Optional[int], cron_lost_threshold_minutes: Optional[int], cron_max_recovery_age_seconds: Optional[int], cron_max_recovery_attempts: Optional[int]) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::new](../../../rust/cloacina/python/bindings/context.md#new)

Creates a new DefaultRunnerConfig with customizable parameters

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `max_concurrent_tasks` | `Optional[int]` |  |
| `scheduler_poll_interval_ms` | `Optional[int]` |  |
| `task_timeout_seconds` | `Optional[int]` |  |
| `pipeline_timeout_seconds` | `Optional[int]` |  |
| `db_pool_size` | `Optional[int]` |  |
| `enable_recovery` | `Optional[bool]` |  |
| `enable_cron_scheduling` | `Optional[bool]` |  |
| `cron_poll_interval_seconds` | `Optional[int]` |  |
| `cron_max_catchup_executions` | `Optional[int]` |  |
| `cron_enable_recovery` | `Optional[bool]` |  |
| `cron_recovery_interval_seconds` | `Optional[int]` |  |
| `cron_lost_threshold_minutes` | `Optional[int]` |  |
| `cron_max_recovery_age_seconds` | `Optional[int]` |  |
| `cron_max_recovery_attempts` | `Optional[int]` |  |


<details>
<summary>Source</summary>

```python
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

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">default</span>() -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::default](../../../rust/cloacina/python/bindings/context.md#default)

Creates a DefaultRunnerConfig with all default values

<details>
<summary>Source</summary>

```python
    pub fn default() -> Self {
        PyDefaultRunnerConfig {
            inner: crate::runner::DefaultRunnerConfig::default(),
        }
    }
```

</details>



##### `max_concurrent_tasks`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">max_concurrent_tasks</span>() -> <span style="color: var(--md-default-fg-color--light);">int</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::max_concurrent_tasks](../../../rust/cloacina/python/bindings/context.md#max_concurrent_tasks)

<details>
<summary>Source</summary>

```python
    pub fn max_concurrent_tasks(&self) -> usize {
        self.inner.max_concurrent_tasks()
    }
```

</details>



##### `scheduler_poll_interval_ms`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">scheduler_poll_interval_ms</span>() -> <span style="color: var(--md-default-fg-color--light);">int</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::scheduler_poll_interval_ms](../../../rust/cloacina/python/bindings/context.md#scheduler_poll_interval_ms)

<details>
<summary>Source</summary>

```python
    pub fn scheduler_poll_interval_ms(&self) -> u64 {
        self.inner.scheduler_poll_interval().as_millis() as u64
    }
```

</details>



##### `task_timeout_seconds`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">task_timeout_seconds</span>() -> <span style="color: var(--md-default-fg-color--light);">int</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::task_timeout_seconds](../../../rust/cloacina/python/bindings/context.md#task_timeout_seconds)

<details>
<summary>Source</summary>

```python
    pub fn task_timeout_seconds(&self) -> u64 {
        self.inner.task_timeout().as_secs()
    }
```

</details>



##### `pipeline_timeout_seconds`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">pipeline_timeout_seconds</span>() -> <span style="color: var(--md-default-fg-color--light);">Optional[int]</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::pipeline_timeout_seconds](../../../rust/cloacina/python/bindings/context.md#pipeline_timeout_seconds)

<details>
<summary>Source</summary>

```python
    pub fn pipeline_timeout_seconds(&self) -> Option<u64> {
        self.inner.pipeline_timeout().map(|d| d.as_secs())
    }
```

</details>



##### `db_pool_size`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">db_pool_size</span>() -> <span style="color: var(--md-default-fg-color--light);">int</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::db_pool_size](../../../rust/cloacina/python/bindings/context.md#db_pool_size)

<details>
<summary>Source</summary>

```python
    pub fn db_pool_size(&self) -> u32 {
        self.inner.db_pool_size()
    }
```

</details>



##### `enable_recovery`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">enable_recovery</span>() -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::enable_recovery](../../../rust/cloacina/python/bindings/context.md#enable_recovery)

<details>
<summary>Source</summary>

```python
    pub fn enable_recovery(&self) -> bool {
        self.inner.enable_recovery()
    }
```

</details>



##### `enable_cron_scheduling`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">enable_cron_scheduling</span>() -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::enable_cron_scheduling](../../../rust/cloacina/python/bindings/context.md#enable_cron_scheduling)

<details>
<summary>Source</summary>

```python
    pub fn enable_cron_scheduling(&self) -> bool {
        self.inner.enable_cron_scheduling()
    }
```

</details>



##### `cron_poll_interval_seconds`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">cron_poll_interval_seconds</span>() -> <span style="color: var(--md-default-fg-color--light);">int</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::cron_poll_interval_seconds](../../../rust/cloacina/python/bindings/context.md#cron_poll_interval_seconds)

<details>
<summary>Source</summary>

```python
    pub fn cron_poll_interval_seconds(&self) -> u64 {
        self.inner.cron_poll_interval().as_secs()
    }
```

</details>



##### `cron_max_catchup_executions`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">cron_max_catchup_executions</span>() -> <span style="color: var(--md-default-fg-color--light);">int</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::cron_max_catchup_executions](../../../rust/cloacina/python/bindings/context.md#cron_max_catchup_executions)

<details>
<summary>Source</summary>

```python
    pub fn cron_max_catchup_executions(&self) -> usize {
        self.inner.cron_max_catchup_executions()
    }
```

</details>



##### `cron_enable_recovery`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">cron_enable_recovery</span>() -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::cron_enable_recovery](../../../rust/cloacina/python/bindings/context.md#cron_enable_recovery)

<details>
<summary>Source</summary>

```python
    pub fn cron_enable_recovery(&self) -> bool {
        self.inner.cron_enable_recovery()
    }
```

</details>



##### `cron_recovery_interval_seconds`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">cron_recovery_interval_seconds</span>() -> <span style="color: var(--md-default-fg-color--light);">int</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::cron_recovery_interval_seconds](../../../rust/cloacina/python/bindings/context.md#cron_recovery_interval_seconds)

<details>
<summary>Source</summary>

```python
    pub fn cron_recovery_interval_seconds(&self) -> u64 {
        self.inner.cron_recovery_interval().as_secs()
    }
```

</details>



##### `cron_lost_threshold_minutes`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">cron_lost_threshold_minutes</span>() -> <span style="color: var(--md-default-fg-color--light);">int</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::cron_lost_threshold_minutes](../../../rust/cloacina/python/bindings/context.md#cron_lost_threshold_minutes)

<details>
<summary>Source</summary>

```python
    pub fn cron_lost_threshold_minutes(&self) -> i32 {
        self.inner.cron_lost_threshold_minutes()
    }
```

</details>



##### `cron_max_recovery_age_seconds`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">cron_max_recovery_age_seconds</span>() -> <span style="color: var(--md-default-fg-color--light);">int</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::cron_max_recovery_age_seconds](../../../rust/cloacina/python/bindings/context.md#cron_max_recovery_age_seconds)

<details>
<summary>Source</summary>

```python
    pub fn cron_max_recovery_age_seconds(&self) -> u64 {
        self.inner.cron_max_recovery_age().as_secs()
    }
```

</details>



##### `cron_max_recovery_attempts`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">cron_max_recovery_attempts</span>() -> <span style="color: var(--md-default-fg-color--light);">int</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::cron_max_recovery_attempts](../../../rust/cloacina/python/bindings/context.md#cron_max_recovery_attempts)

<details>
<summary>Source</summary>

```python
    pub fn cron_max_recovery_attempts(&self) -> usize {
        self.inner.cron_max_recovery_attempts()
    }
```

</details>



##### `set_max_concurrent_tasks`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_max_concurrent_tasks</span>(value: int)</code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::set_max_concurrent_tasks](../../../rust/cloacina/python/bindings/context.md#set_max_concurrent_tasks)

<details>
<summary>Source</summary>

```python
    pub fn set_max_concurrent_tasks(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.max_concurrent_tasks(value));
    }
```

</details>



##### `set_scheduler_poll_interval_ms`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_scheduler_poll_interval_ms</span>(value: int)</code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::set_scheduler_poll_interval_ms](../../../rust/cloacina/python/bindings/context.md#set_scheduler_poll_interval_ms)

<details>
<summary>Source</summary>

```python
    pub fn set_scheduler_poll_interval_ms(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.scheduler_poll_interval(std::time::Duration::from_millis(value)));
    }
```

</details>



##### `set_task_timeout_seconds`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_task_timeout_seconds</span>(value: int)</code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::set_task_timeout_seconds](../../../rust/cloacina/python/bindings/context.md#set_task_timeout_seconds)

<details>
<summary>Source</summary>

```python
    pub fn set_task_timeout_seconds(&mut self, value: u64) {
        self.inner = self.rebuild(|b| b.task_timeout(std::time::Duration::from_secs(value)));
    }
```

</details>



##### `set_pipeline_timeout_seconds`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_pipeline_timeout_seconds</span>(value: Optional[int])</code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::set_pipeline_timeout_seconds](../../../rust/cloacina/python/bindings/context.md#set_pipeline_timeout_seconds)

<details>
<summary>Source</summary>

```python
    pub fn set_pipeline_timeout_seconds(&mut self, value: Option<u64>) {
        self.inner =
            self.rebuild(|b| b.pipeline_timeout(value.map(std::time::Duration::from_secs)));
    }
```

</details>



##### `set_db_pool_size`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_db_pool_size</span>(value: int)</code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::set_db_pool_size](../../../rust/cloacina/python/bindings/context.md#set_db_pool_size)

<details>
<summary>Source</summary>

```python
    pub fn set_db_pool_size(&mut self, value: u32) {
        self.inner = self.rebuild(|b| b.db_pool_size(value));
    }
```

</details>



##### `set_enable_recovery`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_enable_recovery</span>(value: bool)</code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::set_enable_recovery](../../../rust/cloacina/python/bindings/context.md#set_enable_recovery)

<details>
<summary>Source</summary>

```python
    pub fn set_enable_recovery(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.enable_recovery(value));
    }
```

</details>



##### `set_enable_cron_scheduling`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_enable_cron_scheduling</span>(value: bool)</code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::set_enable_cron_scheduling](../../../rust/cloacina/python/bindings/context.md#set_enable_cron_scheduling)

<details>
<summary>Source</summary>

```python
    pub fn set_enable_cron_scheduling(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.enable_cron_scheduling(value));
    }
```

</details>



##### `set_cron_poll_interval_seconds`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_cron_poll_interval_seconds</span>(value: int)</code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::set_cron_poll_interval_seconds](../../../rust/cloacina/python/bindings/context.md#set_cron_poll_interval_seconds)

<details>
<summary>Source</summary>

```python
    pub fn set_cron_poll_interval_seconds(&mut self, value: u64) {
        self.inner = self.rebuild(|b| b.cron_poll_interval(std::time::Duration::from_secs(value)));
    }
```

</details>



##### `set_cron_max_catchup_executions`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_cron_max_catchup_executions</span>(value: int)</code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::set_cron_max_catchup_executions](../../../rust/cloacina/python/bindings/context.md#set_cron_max_catchup_executions)

<details>
<summary>Source</summary>

```python
    pub fn set_cron_max_catchup_executions(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.cron_max_catchup_executions(value));
    }
```

</details>



##### `set_cron_enable_recovery`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_cron_enable_recovery</span>(value: bool)</code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::set_cron_enable_recovery](../../../rust/cloacina/python/bindings/context.md#set_cron_enable_recovery)

<details>
<summary>Source</summary>

```python
    pub fn set_cron_enable_recovery(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.cron_enable_recovery(value));
    }
```

</details>



##### `set_cron_recovery_interval_seconds`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_cron_recovery_interval_seconds</span>(value: int)</code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::set_cron_recovery_interval_seconds](../../../rust/cloacina/python/bindings/context.md#set_cron_recovery_interval_seconds)

<details>
<summary>Source</summary>

```python
    pub fn set_cron_recovery_interval_seconds(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.cron_recovery_interval(std::time::Duration::from_secs(value)));
    }
```

</details>



##### `set_cron_lost_threshold_minutes`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_cron_lost_threshold_minutes</span>(value: int)</code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::set_cron_lost_threshold_minutes](../../../rust/cloacina/python/bindings/context.md#set_cron_lost_threshold_minutes)

<details>
<summary>Source</summary>

```python
    pub fn set_cron_lost_threshold_minutes(&mut self, value: i32) {
        self.inner = self.rebuild(|b| b.cron_lost_threshold_minutes(value));
    }
```

</details>



##### `set_cron_max_recovery_age_seconds`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_cron_max_recovery_age_seconds</span>(value: int)</code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::set_cron_max_recovery_age_seconds](../../../rust/cloacina/python/bindings/context.md#set_cron_max_recovery_age_seconds)

<details>
<summary>Source</summary>

```python
    pub fn set_cron_max_recovery_age_seconds(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.cron_max_recovery_age(std::time::Duration::from_secs(value)));
    }
```

</details>



##### `set_cron_max_recovery_attempts`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set_cron_max_recovery_attempts</span>(value: int)</code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::set_cron_max_recovery_attempts](../../../rust/cloacina/python/bindings/context.md#set_cron_max_recovery_attempts)

<details>
<summary>Source</summary>

```python
    pub fn set_cron_max_recovery_attempts(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.cron_max_recovery_attempts(value));
    }
```

</details>



##### `to_dict`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">to_dict</span>() -> <span style="color: var(--md-default-fg-color--light);">Any</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::to_dict](../../../rust/cloacina/python/bindings/context.md#to_dict)

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

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::context::PyDefaultRunnerConfig::__repr__](../../../rust/cloacina/python/bindings/context.md#__repr__)

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
