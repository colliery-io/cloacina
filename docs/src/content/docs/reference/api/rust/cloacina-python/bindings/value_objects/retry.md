---
title: "cloacina-python::bindings::value_objects::retry"
description: "API reference of the Rust module cloacina-python::bindings::value_objects::retry."
editUrl: false
sidebar:
  order: 11
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

## Structs

### <span id="struct.PyRetryPolicy"></span><code style="font-size: 0.85em">class RetryPolicy</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.RetryPolicy`](../../../../../cloaca/#class.RetryPolicy)

Python wrapper for RetryPolicy

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `cloacina::retry::RetryPolicy` |  |

#### Methods

##### <span id="PyRetryPolicy.builder"></span><code>builder</code>

```rust
fn builder() -> PyRetryPolicyBuilder
```

> **Python API**: [`cloaca.RetryPolicy.builder`](../../../../../cloaca/#RetryPolicy.builder)

Create a builder for constructing RetryPolicy

<details>
<summary>Source</summary>

```rust
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

##### <span id="PyRetryPolicy.default"></span><code>default</code>

```rust
fn default() -> Self
```

> **Python API**: [`cloaca.RetryPolicy.default`](../../../../../cloaca/#RetryPolicy.default)

Create a default RetryPolicy

<details>
<summary>Source</summary>

```rust
    pub fn default() -> Self {
        Self {
            inner: cloacina::retry::RetryPolicy::default(),
        }
    }
```

</details>

##### <span id="PyRetryPolicy.should_retry"></span><code>should_​retry</code>

```rust
fn should_retry(&self, attempt: i32, _error_type: &str) -> bool
```

> **Python API**: [`cloaca.RetryPolicy.should_retry`](../../../../../cloaca/#RetryPolicy.should_retry)

Check if a retry should be attempted

<details>
<summary>Source</summary>

```rust
    pub fn should_retry(&self, attempt: i32, _error_type: &str) -> bool {
        // For now, use a simple retry condition check
        // In the future, this could be enhanced to parse error_type
        attempt < self.inner.max_attempts
    }
```

</details>

##### <span id="PyRetryPolicy.calculate_delay"></span><code>calculate_​delay</code>

```rust
fn calculate_delay(&self, attempt: i32) -> f64
```

> **Python API**: [`cloaca.RetryPolicy.calculate_delay`](../../../../../cloaca/#RetryPolicy.calculate_delay)

Calculate delay for a given attempt

<details>
<summary>Source</summary>

```rust
    pub fn calculate_delay(&self, attempt: i32) -> f64 {
        let duration = self.inner.calculate_delay(attempt);
        duration.as_secs_f64()
    }
```

</details>

##### <span id="PyRetryPolicy.max_attempts"></span><code>max_​attempts</code>

```rust
fn max_attempts(&self) -> i32
```

> **Python API**: [`cloaca.RetryPolicy.max_attempts`](../../../../../cloaca/#RetryPolicy.max_attempts)

Get maximum number of attempts

<details>
<summary>Source</summary>

```rust
    pub fn max_attempts(&self) -> i32 {
        self.inner.max_attempts
    }
```

</details>

##### <span id="PyRetryPolicy.initial_delay"></span><code>initial_​delay</code>

```rust
fn initial_delay(&self) -> f64
```

> **Python API**: [`cloaca.RetryPolicy.initial_delay`](../../../../../cloaca/#RetryPolicy.initial_delay)

Get initial delay in seconds

<details>
<summary>Source</summary>

```rust
    pub fn initial_delay(&self) -> f64 {
        self.inner.initial_delay.as_secs_f64()
    }
```

</details>

##### <span id="PyRetryPolicy.max_delay"></span><code>max_​delay</code>

```rust
fn max_delay(&self) -> f64
```

> **Python API**: [`cloaca.RetryPolicy.max_delay`](../../../../../cloaca/#RetryPolicy.max_delay)

Get maximum delay in seconds

<details>
<summary>Source</summary>

```rust
    pub fn max_delay(&self) -> f64 {
        self.inner.max_delay.as_secs_f64()
    }
```

</details>

##### <span id="PyRetryPolicy.with_jitter"></span><code>with_​jitter</code>

```rust
fn with_jitter(&self) -> bool
```

> **Python API**: [`cloaca.RetryPolicy.with_jitter`](../../../../../cloaca/#RetryPolicy.with_jitter)

Check if jitter is enabled

<details>
<summary>Source</summary>

```rust
    pub fn with_jitter(&self) -> bool {
        self.inner.jitter
    }
```

</details>

##### <span id="PyRetryPolicy.__repr__"></span><code>_​_​repr_​_​</code>

```rust
fn __repr__(&self) -> String
```

> **Python API**: [`cloaca.RetryPolicy.__repr__`](../../../../../cloaca/#RetryPolicy.__repr__)

String representation

<details>
<summary>Source</summary>

```rust
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

##### <span id="PyRetryPolicy.from_rust"></span><code>from_​rust</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_rust(policy: cloacina::retry::RetryPolicy) -> Self
```

Convert from Rust RetryPolicy (for internal use)

<details>
<summary>Source</summary>

```rust
    pub fn from_rust(policy: cloacina::retry::RetryPolicy) -> Self {
        Self { inner: policy }
    }
```

</details>

##### <span id="PyRetryPolicy.to_rust"></span><code>to_​rust</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn to_rust(&self) -> cloacina::retry::RetryPolicy
```

Convert to Rust RetryPolicy (for internal use)

<details>
<summary>Source</summary>

```rust
    pub fn to_rust(&self) -> cloacina::retry::RetryPolicy {
        self.inner.clone()
    }
```

</details>

### <span id="struct.PyBackoffStrategy"></span><code style="font-size: 0.85em">class BackoffStrategy</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.BackoffStrategy`](../../../../../cloaca/#class.BackoffStrategy)

Python wrapper for BackoffStrategy

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `cloacina::retry::BackoffStrategy` |  |

#### Methods

##### <span id="PyBackoffStrategy.fixed"></span><code>fixed</code>

```rust
fn fixed() -> Self
```

> **Python API**: [`cloaca.BackoffStrategy.fixed`](../../../../../cloaca/#BackoffStrategy.fixed)

Fixed delay strategy

<details>
<summary>Source</summary>

```rust
    pub fn fixed() -> Self {
        Self {
            inner: cloacina::retry::BackoffStrategy::Fixed,
        }
    }
```

</details>

##### <span id="PyBackoffStrategy.linear"></span><code>linear</code>

```rust
fn linear(multiplier: f64) -> Self
```

> **Python API**: [`cloaca.BackoffStrategy.linear`](../../../../../cloaca/#BackoffStrategy.linear)

Linear backoff strategy

<details>
<summary>Source</summary>

```rust
    pub fn linear(multiplier: f64) -> Self {
        Self {
            inner: cloacina::retry::BackoffStrategy::Linear { multiplier },
        }
    }
```

</details>

##### <span id="PyBackoffStrategy.exponential"></span><code>exponential</code>

```rust
fn exponential(base: f64, multiplier: Option<f64>) -> Self
```

> **Python API**: [`cloaca.BackoffStrategy.exponential`](../../../../../cloaca/#BackoffStrategy.exponential)

Exponential backoff strategy

CLOACI-T-0882: `multiplier` is `Option` and defaults to 1.0 via
`unwrap_or` below, but pyo3 does NOT infer optionality from `Option<T>`
— without an explicit `signature`, the argument stayed REQUIRED and
`BackoffStrategy.exponential(2.0)` raised
`TypeError: missing 1 required positional argument: 'multiplier'`.
Nothing had ever constructed one of these from Python, so the gap
between the declared default and the callable signature went unseen.

<details>
<summary>Source</summary>

```rust
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

##### <span id="PyBackoffStrategy.__repr__"></span><code>_​_​repr_​_​</code>

```rust
fn __repr__(&self) -> String
```

> **Python API**: [`cloaca.BackoffStrategy.__repr__`](../../../../../cloaca/#BackoffStrategy.__repr__)

String representation

<details>
<summary>Source</summary>

```rust
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

### <span id="struct.PyRetryCondition"></span><code style="font-size: 0.85em">class RetryCondition</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.RetryCondition`](../../../../../cloaca/#class.RetryCondition)

Python wrapper for RetryCondition

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `cloacina::retry::RetryCondition` |  |

#### Methods

##### <span id="PyRetryCondition.never"></span><code>never</code>

```rust
fn never() -> Self
```

> **Python API**: [`cloaca.RetryCondition.never`](../../../../../cloaca/#RetryCondition.never)

Never retry

<details>
<summary>Source</summary>

```rust
    pub fn never() -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::Never,
        }
    }
```

</details>

##### <span id="PyRetryCondition.transient_only"></span><code>transient_​only</code>

```rust
fn transient_only() -> Self
```

> **Python API**: [`cloaca.RetryCondition.transient_only`](../../../../../cloaca/#RetryCondition.transient_only)

Retry only on transient errors

<details>
<summary>Source</summary>

```rust
    pub fn transient_only() -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::TransientOnly,
        }
    }
```

</details>

##### <span id="PyRetryCondition.all_errors"></span><code>all_​errors</code>

```rust
fn all_errors() -> Self
```

> **Python API**: [`cloaca.RetryCondition.all_errors`](../../../../../cloaca/#RetryCondition.all_errors)

Retry on all errors

<details>
<summary>Source</summary>

```rust
    pub fn all_errors() -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::AllErrors,
        }
    }
```

</details>

##### <span id="PyRetryCondition.error_pattern"></span><code>error_​pattern</code>

```rust
fn error_pattern(patterns: Vec<String>) -> Self
```

> **Python API**: [`cloaca.RetryCondition.error_pattern`](../../../../../cloaca/#RetryCondition.error_pattern)

Retry on specific error patterns

<details>
<summary>Source</summary>

```rust
    pub fn error_pattern(patterns: Vec<String>) -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::ErrorPattern { patterns },
        }
    }
```

</details>

##### <span id="PyRetryCondition.__repr__"></span><code>_​_​repr_​_​</code>

```rust
fn __repr__(&self) -> String
```

> **Python API**: [`cloaca.RetryCondition.__repr__`](../../../../../cloaca/#RetryCondition.__repr__)

String representation

<details>
<summary>Source</summary>

```rust
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

### <span id="struct.PyRetryPolicyBuilder"></span><code style="font-size: 0.85em">class RetryPolicyBuilder</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.RetryPolicyBuilder`](../../../../../cloaca/#class.RetryPolicyBuilder)

Python wrapper for RetryPolicy::Builder

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `max_attempts` | `Option<i32>` |  |
| `backoff_strategy` | `Option<cloacina::retry::BackoffStrategy>` |  |
| `initial_delay` | `Option<Duration>` |  |
| `max_delay` | `Option<Duration>` |  |
| `retry_condition` | `Option<cloacina::retry::RetryCondition>` |  |
| `with_jitter` | `Option<bool>` |  |

#### Methods

##### <span id="PyRetryPolicyBuilder.max_attempts"></span><code>max_​attempts</code>

```rust
fn max_attempts(&self, attempts: i32) -> Self
```

> **Python API**: [`cloaca.RetryPolicyBuilder.max_attempts`](../../../../../cloaca/#RetryPolicyBuilder.max_attempts)

Set maximum number of retry attempts

<details>
<summary>Source</summary>

```rust
    pub fn max_attempts(&self, attempts: i32) -> Self {
        let mut new_builder = self.clone();
        new_builder.max_attempts = Some(attempts);
        new_builder
    }
```

</details>

##### <span id="PyRetryPolicyBuilder.initial_delay"></span><code>initial_​delay</code>

```rust
fn initial_delay(&self, delay_seconds: f64) -> Self
```

> **Python API**: [`cloaca.RetryPolicyBuilder.initial_delay`](../../../../../cloaca/#RetryPolicyBuilder.initial_delay)

Set initial delay

<details>
<summary>Source</summary>

```rust
    pub fn initial_delay(&self, delay_seconds: f64) -> Self {
        let mut new_builder = self.clone();
        new_builder.initial_delay = Some(Duration::from_secs_f64(delay_seconds));
        new_builder
    }
```

</details>

##### <span id="PyRetryPolicyBuilder.max_delay"></span><code>max_​delay</code>

```rust
fn max_delay(&self, delay_seconds: f64) -> Self
```

> **Python API**: [`cloaca.RetryPolicyBuilder.max_delay`](../../../../../cloaca/#RetryPolicyBuilder.max_delay)

Set maximum delay

<details>
<summary>Source</summary>

```rust
    pub fn max_delay(&self, delay_seconds: f64) -> Self {
        let mut new_builder = self.clone();
        new_builder.max_delay = Some(Duration::from_secs_f64(delay_seconds));
        new_builder
    }
```

</details>

##### <span id="PyRetryPolicyBuilder.backoff_strategy"></span><code>backoff_​strategy</code>

```rust
fn backoff_strategy(&self, strategy: PyBackoffStrategy) -> Self
```

> **Python API**: [`cloaca.RetryPolicyBuilder.backoff_strategy`](../../../../../cloaca/#RetryPolicyBuilder.backoff_strategy)

Set backoff strategy

<details>
<summary>Source</summary>

```rust
    pub fn backoff_strategy(&self, strategy: PyBackoffStrategy) -> Self {
        let mut new_builder = self.clone();
        new_builder.backoff_strategy = Some(strategy.inner);
        new_builder
    }
```

</details>

##### <span id="PyRetryPolicyBuilder.retry_condition"></span><code>retry_​condition</code>

```rust
fn retry_condition(&self, condition: PyRetryCondition) -> Self
```

> **Python API**: [`cloaca.RetryPolicyBuilder.retry_condition`](../../../../../cloaca/#RetryPolicyBuilder.retry_condition)

Set retry condition

<details>
<summary>Source</summary>

```rust
    pub fn retry_condition(&self, condition: PyRetryCondition) -> Self {
        let mut new_builder = self.clone();
        new_builder.retry_condition = Some(condition.inner);
        new_builder
    }
```

</details>

##### <span id="PyRetryPolicyBuilder.with_jitter"></span><code>with_​jitter</code>

```rust
fn with_jitter(&self, jitter: bool) -> Self
```

> **Python API**: [`cloaca.RetryPolicyBuilder.with_jitter`](../../../../../cloaca/#RetryPolicyBuilder.with_jitter)

Enable/disable jitter

<details>
<summary>Source</summary>

```rust
    pub fn with_jitter(&self, jitter: bool) -> Self {
        let mut new_builder = self.clone();
        new_builder.with_jitter = Some(jitter);
        new_builder
    }
```

</details>

##### <span id="PyRetryPolicyBuilder.build"></span><code>build</code>

```rust
fn build(&self) -> PyRetryPolicy
```

> **Python API**: [`cloaca.RetryPolicyBuilder.build`](../../../../../cloaca/#RetryPolicyBuilder.build)

Build the RetryPolicy

<details>
<summary>Source</summary>

```rust
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
