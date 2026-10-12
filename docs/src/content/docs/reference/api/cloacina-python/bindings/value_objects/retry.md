---
title: "cloacina-python.bindings.value_objects.retry"
description: "API reference of the Python module cloacina-python.bindings.value_objects.retry."
sidebar:
  order: 5
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


## Classes

### <span id="class.RetryPolicy"></span>`class RetryPolicy`

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicy](../../../../rust/cloacina-python/bindings/value_objects/retry/#struct.PyRetryPolicy)

Python wrapper for RetryPolicy

#### Methods

##### <span id="RetryPolicy.builder"></span>`builder`

```python
def builder() -> PyRetryPolicyBuilder
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicy::builder](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.builder)

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



##### <span id="RetryPolicy.default"></span>`default`

```python
def default() -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicy::default](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.default)

Create a default RetryPolicy

<details>
<summary>Source</summary>

```python
    pub fn default() -> Self {
        Self {
            inner: cloacina::retry::RetryPolicy::default(),
        }
    }
```

</details>



##### <span id="RetryPolicy.should_retry"></span>`should_retry`

```python
def should_retry(attempt: int, _error_type:  str) -> bool
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicy::should_retry](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.should_retry)

Check if a retry should be attempted

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `attempt` | `int` |  |
| `_error_type` | ` str` |  |


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



##### <span id="RetryPolicy.calculate_delay"></span>`calculate_delay`

```python
def calculate_delay(attempt: int) -> float
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicy::calculate_delay](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.calculate_delay)

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



##### <span id="RetryPolicy.max_attempts"></span>`max_attempts`

```python
def max_attempts() -> int
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicy::max_attempts](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.max_attempts)

Get maximum number of attempts

<details>
<summary>Source</summary>

```python
    pub fn max_attempts(&self) -> i32 {
        self.inner.max_attempts
    }
```

</details>



##### <span id="RetryPolicy.initial_delay"></span>`initial_delay`

```python
def initial_delay() -> float
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicy::initial_delay](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.initial_delay)

Get initial delay in seconds

<details>
<summary>Source</summary>

```python
    pub fn initial_delay(&self) -> f64 {
        self.inner.initial_delay.as_secs_f64()
    }
```

</details>



##### <span id="RetryPolicy.max_delay"></span>`max_delay`

```python
def max_delay() -> float
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicy::max_delay](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.max_delay)

Get maximum delay in seconds

<details>
<summary>Source</summary>

```python
    pub fn max_delay(&self) -> f64 {
        self.inner.max_delay.as_secs_f64()
    }
```

</details>



##### <span id="RetryPolicy.with_jitter"></span>`with_jitter`

```python
def with_jitter() -> bool
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicy::with_jitter](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.with_jitter)

Check if jitter is enabled

<details>
<summary>Source</summary>

```python
    pub fn with_jitter(&self) -> bool {
        self.inner.jitter
    }
```

</details>



##### <span id="RetryPolicy.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicy::__repr__](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.__repr__)

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





### <span id="class.BackoffStrategy"></span>`class BackoffStrategy`

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyBackoffStrategy](../../../../rust/cloacina-python/bindings/value_objects/retry/#struct.PyBackoffStrategy)

Python wrapper for BackoffStrategy

#### Methods

##### <span id="BackoffStrategy.fixed"></span>`fixed`

```python
def fixed() -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyBackoffStrategy::fixed](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyBackoffStrategy.fixed)

Fixed delay strategy

<details>
<summary>Source</summary>

```python
    pub fn fixed() -> Self {
        Self {
            inner: cloacina::retry::BackoffStrategy::Fixed,
        }
    }
```

</details>



##### <span id="BackoffStrategy.linear"></span>`linear`

```python
def linear(multiplier: float) -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyBackoffStrategy::linear](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyBackoffStrategy.linear)

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
            inner: cloacina::retry::BackoffStrategy::Linear { multiplier },
        }
    }
```

</details>



##### <span id="BackoffStrategy.exponential"></span>`exponential`

```python
def exponential(base: float, multiplier: Option < f64 >) -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyBackoffStrategy::exponential](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyBackoffStrategy.exponential)

Exponential backoff strategy

CLOACI-T-0882: `multiplier` is `Option` and defaults to 1.0 via
`unwrap_or` below, but pyo3 does NOT infer optionality from `Option<T>`
— without an explicit `signature`, the argument stayed REQUIRED and
`BackoffStrategy.exponential(2.0)` raised
`TypeError: missing 1 required positional argument: 'multiplier'`.
Nothing had ever constructed one of these from Python, so the gap
between the declared default and the callable signature went unseen.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `base` | `float` |  |
| `multiplier` | `Option < f64 >` |  |


<details>
<summary>Source</summary>

```python
    pub fn exponential(base: f64, multiplier: Option<f64>) -> Self {
        Self {
            inner: cloacina::retry::BackoffStrategy::Exponential {
                base,
                multiplier: multiplier.unwrap_or(1.0),
            },
        }
    }
```

</details>



##### <span id="BackoffStrategy.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyBackoffStrategy::__repr__](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyBackoffStrategy.__repr__)

String representation

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        match &self.inner {
            cloacina::retry::BackoffStrategy::Fixed => "BackoffStrategy.Fixed".to_string(),
            cloacina::retry::BackoffStrategy::Linear { multiplier } => {
                format!("BackoffStrategy.Linear(multiplier={})", multiplier)
            }
            cloacina::retry::BackoffStrategy::Exponential { base, multiplier } => {
                format!(
                    "BackoffStrategy.Exponential(base={}, multiplier={})",
                    base, multiplier
                )
            }
            cloacina::retry::BackoffStrategy::Custom { function_name } => {
                format!("BackoffStrategy.Custom(function_name='{}')", function_name)
            }
        }
    }
```

</details>





### <span id="class.RetryCondition"></span>`class RetryCondition`

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryCondition](../../../../rust/cloacina-python/bindings/value_objects/retry/#struct.PyRetryCondition)

Python wrapper for RetryCondition

#### Methods

##### <span id="RetryCondition.never"></span>`never`

```python
def never() -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryCondition::never](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryCondition.never)

Never retry

<details>
<summary>Source</summary>

```python
    pub fn never() -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::Never,
        }
    }
```

</details>



##### <span id="RetryCondition.transient_only"></span>`transient_only`

```python
def transient_only() -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryCondition::transient_only](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryCondition.transient_only)

Retry only on transient errors

<details>
<summary>Source</summary>

```python
    pub fn transient_only() -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::TransientOnly,
        }
    }
```

</details>



##### <span id="RetryCondition.all_errors"></span>`all_errors`

```python
def all_errors() -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryCondition::all_errors](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryCondition.all_errors)

Retry on all errors

<details>
<summary>Source</summary>

```python
    pub fn all_errors() -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::AllErrors,
        }
    }
```

</details>



##### <span id="RetryCondition.error_pattern"></span>`error_pattern`

```python
def error_pattern(patterns: Vec < String >) -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryCondition::error_pattern](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryCondition.error_pattern)

Retry on specific error patterns

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `patterns` | `Vec < String >` |  |


<details>
<summary>Source</summary>

```python
    pub fn error_pattern(patterns: Vec<String>) -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::ErrorPattern { patterns },
        }
    }
```

</details>



##### <span id="RetryCondition.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryCondition::__repr__](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryCondition.__repr__)

String representation

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        match &self.inner {
            cloacina::retry::RetryCondition::Never => "RetryCondition.Never".to_string(),
            cloacina::retry::RetryCondition::TransientOnly => {
                "RetryCondition.TransientOnly".to_string()
            }
            cloacina::retry::RetryCondition::AllErrors => "RetryCondition.AllErrors".to_string(),
            cloacina::retry::RetryCondition::ErrorPattern { patterns } => {
                format!("RetryCondition.ErrorPattern(patterns={:?})", patterns)
            }
        }
    }
```

</details>





### <span id="class.RetryPolicyBuilder"></span>`class RetryPolicyBuilder`

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder](../../../../rust/cloacina-python/bindings/value_objects/retry/#struct.PyRetryPolicyBuilder)

Python wrapper for RetryPolicy::Builder

#### Methods

##### <span id="RetryPolicyBuilder.max_attempts"></span>`max_attempts`

```python
def max_attempts(attempts: int) -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder::max_attempts](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicyBuilder.max_attempts)

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



##### <span id="RetryPolicyBuilder.initial_delay"></span>`initial_delay`

```python
def initial_delay(delay_seconds: float) -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder::initial_delay](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicyBuilder.initial_delay)

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



##### <span id="RetryPolicyBuilder.max_delay"></span>`max_delay`

```python
def max_delay(delay_seconds: float) -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder::max_delay](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicyBuilder.max_delay)

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



##### <span id="RetryPolicyBuilder.backoff_strategy"></span>`backoff_strategy`

```python
def backoff_strategy(strategy: PyBackoffStrategy) -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder::backoff_strategy](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicyBuilder.backoff_strategy)

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



##### <span id="RetryPolicyBuilder.retry_condition"></span>`retry_condition`

```python
def retry_condition(condition: PyRetryCondition) -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder::retry_condition](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicyBuilder.retry_condition)

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



##### <span id="RetryPolicyBuilder.with_jitter"></span>`with_jitter`

```python
def with_jitter(jitter: bool) -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder::with_jitter](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicyBuilder.with_jitter)

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



##### <span id="RetryPolicyBuilder.build"></span>`build`

```python
def build() -> PyRetryPolicy
```

> **Rust Implementation**: [cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder::build](../../../../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicyBuilder.build)

Build the RetryPolicy

<details>
<summary>Source</summary>

```python
    pub fn build(&self) -> PyRetryPolicy {
        let mut builder = cloacina::retry::RetryPolicy::builder();

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





