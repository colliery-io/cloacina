---
title: "cloacina-workflow::retry"
description: "API reference of the Rust module cloacina-workflow::retry."
editUrl: false
sidebar:
  order: 35
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Retry Policy System**

This module provides a comprehensive retry policy system for Cloacina tasks,
including configurable backoff strategies, jitter, and conditional retry logic.

**Overview**

The retry system allows tasks to define sophisticated retry behavior:
- **Configurable retry limits** with per-task policies
- **Multiple backoff strategies** including exponential, linear, and custom
- **Jitter support** to prevent thundering herd problems
- **Conditional retries** based on error types and conditions

**Usage**

```rust
use cloacina_workflow::retry::{RetryPolicy, BackoffStrategy, RetryCondition};
use std::time::Duration;

let policy = RetryPolicy::builder()
    .max_attempts(5)
    .backoff_strategy(BackoffStrategy::Exponential {
        base: 2.0,
        multiplier: 1.0
    })
    .initial_delay(Duration::from_millis(100))
    .max_delay(Duration::from_secs(30))
    .with_jitter(true)
    .retry_condition(RetryCondition::AllErrors)
    .build();
```

## Structs

### <span id="struct.RetryPolicy"></span><code style="font-size: 0.85em">struct RetryPolicy</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`, `PartialEq`

Comprehensive retry policy configuration for tasks.

This struct defines how a task should behave when it fails, including
the number of retry attempts, backoff strategy, delays, and conditions
under which retries should be attempted.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `max_attempts` | `i32` | Maximum number of retry attempts (not including the initial attempt) |
| `backoff_strategy` | `BackoffStrategy` | The backoff strategy to use for calculating delays between retries |
| `initial_delay` | `Duration` | Initial delay before the first retry attempt |
| `max_delay` | `Duration` | Maximum delay between retry attempts (caps exponential growth) |
| `jitter` | `bool` | Whether to add random jitter to delays to prevent thundering herd |
| `retry_conditions` | `Vec<RetryCondition>` | Conditions that determine whether a retry should be attempted |

#### Methods

##### <span id="RetryPolicy.builder"></span><code>builder</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn builder() -> RetryPolicyBuilder
```

Creates a new RetryPolicyBuilder for fluent configuration.

<details>
<summary>Source</summary>

```rust
    pub fn builder() -> RetryPolicyBuilder {
        RetryPolicyBuilder::new()
    }
```

</details>

##### <span id="RetryPolicy.calculate_delay"></span><code>calculate_​delay</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn calculate_delay(&self, attempt: i32) -> Duration
```

Calculates the delay before the next retry attempt.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `attempt` | `i32` | The current attempt number (1-based) |

**Returns:**

The duration to wait before the next retry attempt.

<details>
<summary>Source</summary>

```rust
    pub fn calculate_delay(&self, attempt: i32) -> Duration {
        let base_delay = match &self.backoff_strategy {
            BackoffStrategy::Fixed => self.initial_delay,

            BackoffStrategy::Linear { multiplier } => {
                let millis = self.initial_delay.as_millis() as f64 * attempt as f64 * multiplier;
                Duration::from_millis(millis as u64)
            }

            BackoffStrategy::Exponential { base, multiplier } => {
                let millis =
                    self.initial_delay.as_millis() as f64 * multiplier * base.powi(attempt - 1);
                Duration::from_millis(millis as u64)
            }

            BackoffStrategy::Custom { .. } => {
                // For now, fall back to exponential backoff for custom functions
                let millis = self.initial_delay.as_millis() as f64 * 2.0_f64.powi(attempt - 1);
                Duration::from_millis(millis as u64)
            }
        };

        // Cap the delay at max_delay
        let capped_delay = std::cmp::min(base_delay, self.max_delay);

        // Add jitter if enabled
        if self.jitter {
            self.add_jitter(capped_delay)
        } else {
            capped_delay
        }
    }
```

</details>

##### <span id="RetryPolicy.should_retry"></span><code>should_​retry</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn should_retry(&self, error: &TaskError, attempt: i32) -> bool
```

Determines whether a retry should be attempted based on the error and retry conditions.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `error` | `&TaskError` | The error that caused the task to fail |
| `attempt` | `i32` | The current attempt number |

**Returns:**

`true` if the task should be retried, `false` otherwise.

<details>
<summary>Source</summary>

```rust
    pub fn should_retry(&self, error: &TaskError, attempt: i32) -> bool {
        // Check if we've exceeded the maximum number of attempts
        if attempt >= self.max_attempts {
            return false;
        }

        // Check retry conditions
        self.retry_conditions
            .iter()
            .any(|condition| match condition {
                RetryCondition::AllErrors => true,
                RetryCondition::Never => false,
                RetryCondition::TransientOnly => self.is_transient_error(error),
                RetryCondition::ErrorPattern { patterns } => {
                    let error_msg = error.to_string().to_lowercase();
                    patterns
                        .iter()
                        .any(|pattern| error_msg.contains(&pattern.to_lowercase()))
                }
            })
    }
```

</details>

##### <span id="RetryPolicy.calculate_retry_at"></span><code>calculate_​retry_​at</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn calculate_retry_at(
    &self,
    attempt: i32,
    now: NaiveDateTime,
) -> NaiveDateTime
```

Calculates the absolute timestamp when the next retry should occur.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `attempt` | `i32` | The current attempt number |
| `now` | `NaiveDateTime` | The current timestamp |

**Returns:**

A NaiveDateTime representing when the retry should be attempted.

<details>
<summary>Source</summary>

```rust
    pub fn calculate_retry_at(&self, attempt: i32, now: NaiveDateTime) -> NaiveDateTime {
        let delay = self.calculate_delay(attempt);
        now + chrono::Duration::from_std(delay).unwrap_or_default()
    }
```

</details>

### <span id="struct.RetryPolicyBuilder"></span><code style="font-size: 0.85em">struct RetryPolicyBuilder</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`

Builder for creating RetryPolicy instances with a fluent API.

#### Methods

##### <span id="RetryPolicyBuilder.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new() -> Self
```

Creates a new RetryPolicyBuilder with default values.

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        Self {
            policy: RetryPolicy::default(),
        }
    }
```

</details>

##### <span id="RetryPolicyBuilder.max_attempts"></span><code>max_​attempts</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn max_attempts(mut self, max_attempts: i32) -> Self
```

Sets the maximum number of retry attempts.

<details>
<summary>Source</summary>

```rust
    pub fn max_attempts(mut self, max_attempts: i32) -> Self {
        self.policy.max_attempts = max_attempts;
        self
    }
```

</details>

##### <span id="RetryPolicyBuilder.backoff_strategy"></span><code>backoff_​strategy</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn backoff_strategy(mut self, strategy: BackoffStrategy) -> Self
```

Sets the backoff strategy.

<details>
<summary>Source</summary>

```rust
    pub fn backoff_strategy(mut self, strategy: BackoffStrategy) -> Self {
        self.policy.backoff_strategy = strategy;
        self
    }
```

</details>

##### <span id="RetryPolicyBuilder.initial_delay"></span><code>initial_​delay</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn initial_delay(mut self, delay: Duration) -> Self
```

Sets the initial delay before the first retry.

<details>
<summary>Source</summary>

```rust
    pub fn initial_delay(mut self, delay: Duration) -> Self {
        self.policy.initial_delay = delay;
        self
    }
```

</details>

##### <span id="RetryPolicyBuilder.max_delay"></span><code>max_​delay</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn max_delay(mut self, delay: Duration) -> Self
```

Sets the maximum delay between retries.

<details>
<summary>Source</summary>

```rust
    pub fn max_delay(mut self, delay: Duration) -> Self {
        self.policy.max_delay = delay;
        self
    }
```

</details>

##### <span id="RetryPolicyBuilder.with_jitter"></span><code>with_​jitter</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_jitter(mut self, jitter: bool) -> Self
```

Enables or disables jitter.

<details>
<summary>Source</summary>

```rust
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.policy.jitter = jitter;
        self
    }
```

</details>

##### <span id="RetryPolicyBuilder.retry_condition"></span><code>retry_​condition</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn retry_condition(mut self, condition: RetryCondition) -> Self
```

Adds a retry condition.

<details>
<summary>Source</summary>

```rust
    pub fn retry_condition(mut self, condition: RetryCondition) -> Self {
        self.policy.retry_conditions = vec![condition];
        self
    }
```

</details>

##### <span id="RetryPolicyBuilder.retry_conditions"></span><code>retry_​conditions</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn retry_conditions(mut self, conditions: Vec<RetryCondition>) -> Self
```

Adds multiple retry conditions.

<details>
<summary>Source</summary>

```rust
    pub fn retry_conditions(mut self, conditions: Vec<RetryCondition>) -> Self {
        self.policy.retry_conditions = conditions;
        self
    }
```

</details>

##### <span id="RetryPolicyBuilder.build"></span><code>build</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn build(self) -> RetryPolicy
```

Builds the RetryPolicy.

<details>
<summary>Source</summary>

```rust
    pub fn build(self) -> RetryPolicy {
        self.policy
    }
```

</details>

## Enums

### <span id="enum.BackoffStrategy"></span><code style="font-size: 0.85em">enum BackoffStrategy</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Different backoff strategies for calculating retry delays.

Each strategy defines how the delay between retry attempts should increase.
The actual delay is calculated based on the attempt number and the strategy's parameters.

#### Variants

- **`Fixed`** - Fixed delay - same delay for every retry attempt
- **`Linear`** - Linear backoff - delay increases linearly with each attempt
  delay = initial_delay * attempt * multiplier
- **`Exponential`** - Exponential backoff - delay increases exponentially with each attempt
  delay = initial_delay * multiplier * (base ^ attempt)
- **`Custom`** - Custom backoff function (reserved for future extensibility)

### <span id="enum.RetryCondition"></span><code style="font-size: 0.85em">enum RetryCondition</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Conditions that determine whether a failed task should be retried.

These conditions are used to evaluate whether a task should be retried
based on the type of error or specific error patterns.

#### Variants

- **`AllErrors`** - Retry on all errors (default behavior)
- **`Never`** - Never retry (equivalent to max_attempts = 0)
- **`TransientOnly`** - Retry only for transient errors (network, timeout, etc.)
- **`ErrorPattern`** - Retry only if error message contains any of the specified patterns
