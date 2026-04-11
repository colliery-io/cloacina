# cloaca.python.bindings.value_objects.retry <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


## Classes

### `cloaca.python.bindings.value_objects.retry.RetryPolicy`

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicy](../../../../rust/cloacina/python/bindings/value_objects/retry.md#class-retrypolicy)

Python wrapper for RetryPolicy

#### Methods

##### `builder`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">builder</span>() -> <span style="color: var(--md-default-fg-color--light);">PyRetryPolicyBuilder</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicy::builder](../../../../rust/cloacina/python/bindings/value_objects/retry.md#builder)

Create a builder for constructing RetryPolicy

<details>
<summary>Source</summary>

```python
    pub fn builder() -> PyRetryPolicyBuilder {
        PyRetryPolicyBuilder {
            max_attempts: None,
            backoff_strategy: None,
            initial_delay: None,
            max_delay: None,
            retry_condition: None,
            with_jitter: None,
        }
    }
```

</details>



##### `default`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">default</span>() -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicy::default](../../../../rust/cloacina/python/bindings/value_objects/retry.md#default)

Create a default RetryPolicy

<details>
<summary>Source</summary>

```python
    pub fn default() -> Self {
        Self {
            inner: crate::retry::RetryPolicy::default(),
        }
    }
```

</details>



##### `should_retry`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">should_retry</span>(attempt: int, _error_type: str) -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicy::should_retry](../../../../rust/cloacina/python/bindings/value_objects/retry.md#should_retry)

Check if a retry should be attempted

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `attempt` | `int` |  |
| `_error_type` | `str` |  |


<details>
<summary>Source</summary>

```python
    pub fn should_retry(&self, attempt: i32, _error_type: &str) -> bool {
        // For now, use a simple retry condition check
        // In the future, this could be enhanced to parse error_type
        attempt < self.inner.max_attempts
    }
```

</details>



##### `calculate_delay`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">calculate_delay</span>(attempt: int) -> <span style="color: var(--md-default-fg-color--light);">float</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicy::calculate_delay](../../../../rust/cloacina/python/bindings/value_objects/retry.md#calculate_delay)

Calculate delay for a given attempt

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `attempt` | `int` |  |


<details>
<summary>Source</summary>

```python
    pub fn calculate_delay(&self, attempt: i32) -> f64 {
        let duration = self.inner.calculate_delay(attempt);
        duration.as_secs_f64()
    }
```

</details>



##### `max_attempts`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">max_attempts</span>() -> <span style="color: var(--md-default-fg-color--light);">int</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicy::max_attempts](../../../../rust/cloacina/python/bindings/value_objects/retry.md#max_attempts)

Get maximum number of attempts

<details>
<summary>Source</summary>

```python
    pub fn max_attempts(&self) -> i32 {
        self.inner.max_attempts
    }
```

</details>



##### `initial_delay`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">initial_delay</span>() -> <span style="color: var(--md-default-fg-color--light);">float</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicy::initial_delay](../../../../rust/cloacina/python/bindings/value_objects/retry.md#initial_delay)

Get initial delay in seconds

<details>
<summary>Source</summary>

```python
    pub fn initial_delay(&self) -> f64 {
        self.inner.initial_delay.as_secs_f64()
    }
```

</details>



##### `max_delay`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">max_delay</span>() -> <span style="color: var(--md-default-fg-color--light);">float</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicy::max_delay](../../../../rust/cloacina/python/bindings/value_objects/retry.md#max_delay)

Get maximum delay in seconds

<details>
<summary>Source</summary>

```python
    pub fn max_delay(&self) -> f64 {
        self.inner.max_delay.as_secs_f64()
    }
```

</details>



##### `with_jitter`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">with_jitter</span>() -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicy::with_jitter](../../../../rust/cloacina/python/bindings/value_objects/retry.md#with_jitter)

Check if jitter is enabled

<details>
<summary>Source</summary>

```python
    pub fn with_jitter(&self) -> bool {
        self.inner.jitter
    }
```

</details>



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicy::__repr__](../../../../rust/cloacina/python/bindings/value_objects/retry.md#__repr__)

String representation

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        format!(
            "RetryPolicy(max_attempts={}, initial_delay={}s, max_delay={}s, jitter={})",
            self.max_attempts(),
            self.initial_delay(),
            self.max_delay(),
            self.with_jitter()
        )
    }
```

</details>





### `cloaca.python.bindings.value_objects.retry.BackoffStrategy`

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyBackoffStrategy](../../../../rust/cloacina/python/bindings/value_objects/retry.md#class-backoffstrategy)

Python wrapper for BackoffStrategy

#### Methods

##### `fixed`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">fixed</span>() -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyBackoffStrategy::fixed](../../../../rust/cloacina/python/bindings/value_objects/retry.md#fixed)

Fixed delay strategy

<details>
<summary>Source</summary>

```python
    pub fn fixed() -> Self {
        Self {
            inner: crate::retry::BackoffStrategy::Fixed,
        }
    }
```

</details>



##### `linear`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">linear</span>(multiplier: float) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyBackoffStrategy::linear](../../../../rust/cloacina/python/bindings/value_objects/retry.md#linear)

Linear backoff strategy

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `multiplier` | `float` |  |


<details>
<summary>Source</summary>

```python
    pub fn linear(multiplier: f64) -> Self {
        Self {
            inner: crate::retry::BackoffStrategy::Linear { multiplier },
        }
    }
```

</details>



##### `exponential`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">exponential</span>(base: float, multiplier: Optional[float]) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyBackoffStrategy::exponential](../../../../rust/cloacina/python/bindings/value_objects/retry.md#exponential)

Exponential backoff strategy

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `base` | `float` |  |
| `multiplier` | `Optional[float]` |  |


<details>
<summary>Source</summary>

```python
    pub fn exponential(base: f64, multiplier: Option<f64>) -> Self {
        Self {
            inner: crate::retry::BackoffStrategy::Exponential {
                base,
                multiplier: multiplier.unwrap_or(1.0),
            },
        }
    }
```

</details>



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyBackoffStrategy::__repr__](../../../../rust/cloacina/python/bindings/value_objects/retry.md#__repr__)

String representation

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        match &self.inner {
            crate::retry::BackoffStrategy::Fixed => "BackoffStrategy.Fixed".to_string(),
            crate::retry::BackoffStrategy::Linear { multiplier } => {
                format!("BackoffStrategy.Linear(multiplier={})", multiplier)
            }
            crate::retry::BackoffStrategy::Exponential { base, multiplier } => {
                format!(
                    "BackoffStrategy.Exponential(base={}, multiplier={})",
                    base, multiplier
                )
            }
            crate::retry::BackoffStrategy::Custom { function_name } => {
                format!("BackoffStrategy.Custom(function_name='{}')", function_name)
            }
        }
    }
```

</details>





### `cloaca.python.bindings.value_objects.retry.RetryCondition`

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryCondition](../../../../rust/cloacina/python/bindings/value_objects/retry.md#class-retrycondition)

Python wrapper for RetryCondition

#### Methods

##### `never`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">never</span>() -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryCondition::never](../../../../rust/cloacina/python/bindings/value_objects/retry.md#never)

Never retry

<details>
<summary>Source</summary>

```python
    pub fn never() -> Self {
        Self {
            inner: crate::retry::RetryCondition::Never,
        }
    }
```

</details>



##### `transient_only`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">transient_only</span>() -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryCondition::transient_only](../../../../rust/cloacina/python/bindings/value_objects/retry.md#transient_only)

Retry only on transient errors

<details>
<summary>Source</summary>

```python
    pub fn transient_only() -> Self {
        Self {
            inner: crate::retry::RetryCondition::TransientOnly,
        }
    }
```

</details>



##### `all_errors`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">all_errors</span>() -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryCondition::all_errors](../../../../rust/cloacina/python/bindings/value_objects/retry.md#all_errors)

Retry on all errors

<details>
<summary>Source</summary>

```python
    pub fn all_errors() -> Self {
        Self {
            inner: crate::retry::RetryCondition::AllErrors,
        }
    }
```

</details>



##### `error_pattern`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">error_pattern</span>(patterns: List[str]) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryCondition::error_pattern](../../../../rust/cloacina/python/bindings/value_objects/retry.md#error_pattern)

Retry on specific error patterns

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `patterns` | `List[str]` |  |


<details>
<summary>Source</summary>

```python
    pub fn error_pattern(patterns: Vec<String>) -> Self {
        Self {
            inner: crate::retry::RetryCondition::ErrorPattern { patterns },
        }
    }
```

</details>



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryCondition::__repr__](../../../../rust/cloacina/python/bindings/value_objects/retry.md#__repr__)

String representation

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        match &self.inner {
            crate::retry::RetryCondition::Never => "RetryCondition.Never".to_string(),
            crate::retry::RetryCondition::TransientOnly => {
                "RetryCondition.TransientOnly".to_string()
            }
            crate::retry::RetryCondition::AllErrors => "RetryCondition.AllErrors".to_string(),
            crate::retry::RetryCondition::ErrorPattern { patterns } => {
                format!("RetryCondition.ErrorPattern(patterns={:?})", patterns)
            }
        }
    }
```

</details>





### `cloaca.python.bindings.value_objects.retry.RetryPolicyBuilder`

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicyBuilder](../../../../rust/cloacina/python/bindings/value_objects/retry.md#class-retrypolicybuilder)

Python wrapper for RetryPolicy::Builder

#### Methods

##### `max_attempts`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">max_attempts</span>(attempts: int) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicyBuilder::max_attempts](../../../../rust/cloacina/python/bindings/value_objects/retry.md#max_attempts)

Set maximum number of retry attempts

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `attempts` | `int` |  |


<details>
<summary>Source</summary>

```python
    pub fn max_attempts(&self, attempts: i32) -> Self {
        let mut new_builder = self.clone();
        new_builder.max_attempts = Some(attempts);
        new_builder
    }
```

</details>



##### `initial_delay`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">initial_delay</span>(delay_seconds: float) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicyBuilder::initial_delay](../../../../rust/cloacina/python/bindings/value_objects/retry.md#initial_delay)

Set initial delay

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `delay_seconds` | `float` |  |


<details>
<summary>Source</summary>

```python
    pub fn initial_delay(&self, delay_seconds: f64) -> Self {
        let mut new_builder = self.clone();
        new_builder.initial_delay = Some(Duration::from_secs_f64(delay_seconds));
        new_builder
    }
```

</details>



##### `max_delay`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">max_delay</span>(delay_seconds: float) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicyBuilder::max_delay](../../../../rust/cloacina/python/bindings/value_objects/retry.md#max_delay)

Set maximum delay

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `delay_seconds` | `float` |  |


<details>
<summary>Source</summary>

```python
    pub fn max_delay(&self, delay_seconds: f64) -> Self {
        let mut new_builder = self.clone();
        new_builder.max_delay = Some(Duration::from_secs_f64(delay_seconds));
        new_builder
    }
```

</details>



##### `backoff_strategy`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">backoff_strategy</span>(strategy: PyBackoffStrategy) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicyBuilder::backoff_strategy](../../../../rust/cloacina/python/bindings/value_objects/retry.md#backoff_strategy)

Set backoff strategy

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `strategy` | `PyBackoffStrategy` |  |


<details>
<summary>Source</summary>

```python
    pub fn backoff_strategy(&self, strategy: PyBackoffStrategy) -> Self {
        let mut new_builder = self.clone();
        new_builder.backoff_strategy = Some(strategy.inner);
        new_builder
    }
```

</details>



##### `retry_condition`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">retry_condition</span>(condition: PyRetryCondition) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicyBuilder::retry_condition](../../../../rust/cloacina/python/bindings/value_objects/retry.md#retry_condition)

Set retry condition

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `condition` | `PyRetryCondition` |  |


<details>
<summary>Source</summary>

```python
    pub fn retry_condition(&self, condition: PyRetryCondition) -> Self {
        let mut new_builder = self.clone();
        new_builder.retry_condition = Some(condition.inner);
        new_builder
    }
```

</details>



##### `with_jitter`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">with_jitter</span>(jitter: bool) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicyBuilder::with_jitter](../../../../rust/cloacina/python/bindings/value_objects/retry.md#with_jitter)

Enable/disable jitter

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `jitter` | `bool` |  |


<details>
<summary>Source</summary>

```python
    pub fn with_jitter(&self, jitter: bool) -> Self {
        let mut new_builder = self.clone();
        new_builder.with_jitter = Some(jitter);
        new_builder
    }
```

</details>



##### `build`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">build</span>() -> <span style="color: var(--md-default-fg-color--light);">PyRetryPolicy</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::value_objects::retry::PyRetryPolicyBuilder::build](../../../../rust/cloacina/python/bindings/value_objects/retry.md#build)

Build the RetryPolicy

<details>
<summary>Source</summary>

```python
    pub fn build(&self) -> PyRetryPolicy {
        let mut builder = crate::retry::RetryPolicy::builder();

        if let Some(attempts) = self.max_attempts {
            builder = builder.max_attempts(attempts);
        }
        if let Some(strategy) = &self.backoff_strategy {
            builder = builder.backoff_strategy(strategy.clone());
        }
        if let Some(delay) = self.initial_delay {
            builder = builder.initial_delay(delay);
        }
        if let Some(delay) = self.max_delay {
            builder = builder.max_delay(delay);
        }
        if let Some(condition) = &self.retry_condition {
            builder = builder.retry_condition(condition.clone());
        }
        if let Some(jitter) = self.with_jitter {
            builder = builder.with_jitter(jitter);
        }

        PyRetryPolicy {
            inner: builder.build(),
        }
    }
```

</details>
