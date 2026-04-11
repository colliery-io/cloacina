# cloacina::python::bindings::value_objects::retry <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


## Structs

### `cloacina::python::bindings::value_objects::retry::RetryPolicy`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicy](../../../../../cloaca/python/bindings/value_objects/retry.md#class-retrypolicy)

Python wrapper for RetryPolicy

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `crate :: retry :: RetryPolicy` |  |

#### Methods

##### `builder`

```rust
fn builder () -> PyRetryPolicyBuilder
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicy.builder](../../../../../cloaca/python/bindings/value_objects/retry.md#builder)

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



##### `default`

```rust
fn default () -> Self
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicy.default](../../../../../cloaca/python/bindings/value_objects/retry.md#default)

Create a default RetryPolicy

<details>
<summary>Source</summary>

```rust
    pub fn default() -> Self {
        Self {
            inner: crate::retry::RetryPolicy::default(),
        }
    }
```

</details>



##### `should_retry`

```rust
fn should_retry (& self , attempt : i32 , _error_type : & str) -> bool
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicy.should_retry](../../../../../cloaca/python/bindings/value_objects/retry.md#should_retry)

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



##### `calculate_delay`

```rust
fn calculate_delay (& self , attempt : i32) -> f64
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicy.calculate_delay](../../../../../cloaca/python/bindings/value_objects/retry.md#calculate_delay)

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



##### `max_attempts`

```rust
fn max_attempts (& self) -> i32
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicy.max_attempts](../../../../../cloaca/python/bindings/value_objects/retry.md#max_attempts)

Get maximum number of attempts

<details>
<summary>Source</summary>

```rust
    pub fn max_attempts(&self) -> i32 {
        self.inner.max_attempts
    }
```

</details>



##### `initial_delay`

```rust
fn initial_delay (& self) -> f64
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicy.initial_delay](../../../../../cloaca/python/bindings/value_objects/retry.md#initial_delay)

Get initial delay in seconds

<details>
<summary>Source</summary>

```rust
    pub fn initial_delay(&self) -> f64 {
        self.inner.initial_delay.as_secs_f64()
    }
```

</details>



##### `max_delay`

```rust
fn max_delay (& self) -> f64
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicy.max_delay](../../../../../cloaca/python/bindings/value_objects/retry.md#max_delay)

Get maximum delay in seconds

<details>
<summary>Source</summary>

```rust
    pub fn max_delay(&self) -> f64 {
        self.inner.max_delay.as_secs_f64()
    }
```

</details>



##### `with_jitter`

```rust
fn with_jitter (& self) -> bool
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicy.with_jitter](../../../../../cloaca/python/bindings/value_objects/retry.md#with_jitter)

Check if jitter is enabled

<details>
<summary>Source</summary>

```rust
    pub fn with_jitter(&self) -> bool {
        self.inner.jitter
    }
```

</details>



##### `__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicy.__repr__](../../../../../cloaca/python/bindings/value_objects/retry.md#__repr__)

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



##### `from_rust` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_rust (policy : crate :: retry :: RetryPolicy) -> Self
```

Convert from Rust RetryPolicy (for internal use)

<details>
<summary>Source</summary>

```rust
    pub fn from_rust(policy: crate::retry::RetryPolicy) -> Self {
        Self { inner: policy }
    }
```

</details>



##### `to_rust` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn to_rust (& self) -> crate :: retry :: RetryPolicy
```

Convert to Rust RetryPolicy (for internal use)

<details>
<summary>Source</summary>

```rust
    pub fn to_rust(&self) -> crate::retry::RetryPolicy {
        self.inner.clone()
    }
```

</details>





### `cloacina::python::bindings::value_objects::retry::BackoffStrategy`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.bindings.value_objects.retry.BackoffStrategy](../../../../../cloaca/python/bindings/value_objects/retry.md#class-backoffstrategy)

Python wrapper for BackoffStrategy

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `crate :: retry :: BackoffStrategy` |  |

#### Methods

##### `fixed`

```rust
fn fixed () -> Self
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.BackoffStrategy.fixed](../../../../../cloaca/python/bindings/value_objects/retry.md#fixed)

Fixed delay strategy

<details>
<summary>Source</summary>

```rust
    pub fn fixed() -> Self {
        Self {
            inner: crate::retry::BackoffStrategy::Fixed,
        }
    }
```

</details>



##### `linear`

```rust
fn linear (multiplier : f64) -> Self
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.BackoffStrategy.linear](../../../../../cloaca/python/bindings/value_objects/retry.md#linear)

Linear backoff strategy

<details>
<summary>Source</summary>

```rust
    pub fn linear(multiplier: f64) -> Self {
        Self {
            inner: crate::retry::BackoffStrategy::Linear { multiplier },
        }
    }
```

</details>



##### `exponential`

```rust
fn exponential (base : f64 , multiplier : Option < f64 >) -> Self
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.BackoffStrategy.exponential](../../../../../cloaca/python/bindings/value_objects/retry.md#exponential)

Exponential backoff strategy

<details>
<summary>Source</summary>

```rust
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

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.BackoffStrategy.__repr__](../../../../../cloaca/python/bindings/value_objects/retry.md#__repr__)

String representation

<details>
<summary>Source</summary>

```rust
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





### `cloacina::python::bindings::value_objects::retry::RetryCondition`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryCondition](../../../../../cloaca/python/bindings/value_objects/retry.md#class-retrycondition)

Python wrapper for RetryCondition

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `crate :: retry :: RetryCondition` |  |

#### Methods

##### `never`

```rust
fn never () -> Self
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryCondition.never](../../../../../cloaca/python/bindings/value_objects/retry.md#never)

Never retry

<details>
<summary>Source</summary>

```rust
    pub fn never() -> Self {
        Self {
            inner: crate::retry::RetryCondition::Never,
        }
    }
```

</details>



##### `transient_only`

```rust
fn transient_only () -> Self
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryCondition.transient_only](../../../../../cloaca/python/bindings/value_objects/retry.md#transient_only)

Retry only on transient errors

<details>
<summary>Source</summary>

```rust
    pub fn transient_only() -> Self {
        Self {
            inner: crate::retry::RetryCondition::TransientOnly,
        }
    }
```

</details>



##### `all_errors`

```rust
fn all_errors () -> Self
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryCondition.all_errors](../../../../../cloaca/python/bindings/value_objects/retry.md#all_errors)

Retry on all errors

<details>
<summary>Source</summary>

```rust
    pub fn all_errors() -> Self {
        Self {
            inner: crate::retry::RetryCondition::AllErrors,
        }
    }
```

</details>



##### `error_pattern`

```rust
fn error_pattern (patterns : Vec < String >) -> Self
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryCondition.error_pattern](../../../../../cloaca/python/bindings/value_objects/retry.md#error_pattern)

Retry on specific error patterns

<details>
<summary>Source</summary>

```rust
    pub fn error_pattern(patterns: Vec<String>) -> Self {
        Self {
            inner: crate::retry::RetryCondition::ErrorPattern { patterns },
        }
    }
```

</details>



##### `__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryCondition.__repr__](../../../../../cloaca/python/bindings/value_objects/retry.md#__repr__)

String representation

<details>
<summary>Source</summary>

```rust
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





### `cloacina::python::bindings::value_objects::retry::RetryPolicyBuilder`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicyBuilder](../../../../../cloaca/python/bindings/value_objects/retry.md#class-retrypolicybuilder)

Python wrapper for RetryPolicy::Builder

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `max_attempts` | `Option < i32 >` |  |
| `backoff_strategy` | `Option < crate :: retry :: BackoffStrategy >` |  |
| `initial_delay` | `Option < Duration >` |  |
| `max_delay` | `Option < Duration >` |  |
| `retry_condition` | `Option < crate :: retry :: RetryCondition >` |  |
| `with_jitter` | `Option < bool >` |  |

#### Methods

##### `max_attempts`

```rust
fn max_attempts (& self , attempts : i32) -> Self
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicyBuilder.max_attempts](../../../../../cloaca/python/bindings/value_objects/retry.md#max_attempts)

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



##### `initial_delay`

```rust
fn initial_delay (& self , delay_seconds : f64) -> Self
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicyBuilder.initial_delay](../../../../../cloaca/python/bindings/value_objects/retry.md#initial_delay)

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



##### `max_delay`

```rust
fn max_delay (& self , delay_seconds : f64) -> Self
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicyBuilder.max_delay](../../../../../cloaca/python/bindings/value_objects/retry.md#max_delay)

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



##### `backoff_strategy`

```rust
fn backoff_strategy (& self , strategy : PyBackoffStrategy) -> Self
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicyBuilder.backoff_strategy](../../../../../cloaca/python/bindings/value_objects/retry.md#backoff_strategy)

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



##### `retry_condition`

```rust
fn retry_condition (& self , condition : PyRetryCondition) -> Self
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicyBuilder.retry_condition](../../../../../cloaca/python/bindings/value_objects/retry.md#retry_condition)

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



##### `with_jitter`

```rust
fn with_jitter (& self , jitter : bool) -> Self
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicyBuilder.with_jitter](../../../../../cloaca/python/bindings/value_objects/retry.md#with_jitter)

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



##### `build`

```rust
fn build (& self) -> PyRetryPolicy
```

> **Python API**: [cloaca.python.bindings.value_objects.retry.RetryPolicyBuilder.build](../../../../../cloaca/python/bindings/value_objects/retry.md#build)

Build the RetryPolicy

<details>
<summary>Source</summary>

```rust
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
