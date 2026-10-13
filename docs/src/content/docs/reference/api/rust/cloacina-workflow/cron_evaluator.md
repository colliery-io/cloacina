---
title: "cloacina-workflow::cron_evaluator"
description: "Timezone-aware cron expression evaluator"
editUrl: false
sidebar:
  order: 31
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Timezone-aware cron expression evaluator

This module provides functionality for parsing and evaluating cron expressions
with proper timezone support. It uses the `croner` crate for cron parsing and
`chrono-tz` for timezone handling.

**Examples:**

```rust
use cloacina::cron_evaluator::CronEvaluator;
use chrono::{DateTime, Utc};

// Create evaluator for daily 9 AM EST/EDT
let evaluator = CronEvaluator::new("0 9 * * *", "America/New_York")?;

// Find next execution after current time
let now = Utc::now();
let next = evaluator.next_execution(now)?;

println!("Next execution: {}", next);
```

## Structs

### <span id="struct.CronEvaluator"></span><code style="font-size: 0.85em">struct CronEvaluator</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Timezone-aware cron expression evaluator.

This struct provides methods for evaluating cron expressions in specific timezones,
handling daylight saving time transitions automatically.

**Examples:**

```rust
use cloacina::cron_evaluator::CronEvaluator;
use chrono::Utc;

// Daily at 2 AM Eastern Time (handles EST/EDT automatically)
let evaluator = CronEvaluator::new("0 2 * * *", "America/New_York")?;
let next = evaluator.next_execution(Utc::now())?;

// Hourly during business hours in London
let evaluator = CronEvaluator::new("0 9-17 * * 1-5", "Europe/London")?;
let next = evaluator.next_execution(Utc::now())?;
```

#### Methods

##### <span id="CronEvaluator.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(cron_expr: &str, timezone_str: &str) -> Result<Self, CronError>
```

Creates a new cron evaluator with the specified expression and timezone.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `cron_expr` | `&str` | Standard cron expression (5 fields: minute hour day month weekday) |
| `timezone_str` | `&str` | IANA timezone name (e.g., "America/New_York", "Europe/London", "UTC") |

**Returns:**

* `Result<Self, CronError>` - New evaluator instance or error

**Examples:**

```rust
use cloacina::cron_evaluator::CronEvaluator;

// Daily at 9 AM Eastern Time
let evaluator = CronEvaluator::new("0 9 * * *", "America/New_York")?;

// Every 15 minutes during UTC business hours
let evaluator = CronEvaluator::new("*/15 9-17 * * 1-5", "UTC")?;

// Monthly on the 1st at midnight Pacific Time
let evaluator = CronEvaluator::new("0 0 1 * *", "America/Los_Angeles")?;
```

<details>
<summary>Source</summary>

```rust
    pub fn new(cron_expr: &str, timezone_str: &str) -> Result<Self, CronError> {
        let cron = parse_cron(cron_expr).map_err(|e| CronError::CronParsingError(e.to_string()))?;

        // Parse the timezone
        let timezone: Tz = timezone_str
            .parse()
            .map_err(|_| CronError::InvalidTimezone(timezone_str.to_string()))?;

        Ok(Self {
            cron,
            timezone,
            expression: cron_expr.to_string(),
            timezone_str: timezone_str.to_string(),
        })
    }
```

</details>

##### <span id="CronEvaluator.next_execution"></span><code>next_​execution</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn next_execution(
    &self,
    after: DateTime<Utc>,
) -> Result<DateTime<Utc>, CronError>
```

Finds the next execution time after the given timestamp.

This method converts the UTC timestamp to the evaluator's timezone,
finds the next cron match in that timezone, then converts back to UTC.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `after` | `DateTime<Utc>` | UTC timestamp to find the next execution after |

**Returns:**

* `Result<DateTime<Utc>, CronError>` - Next execution time in UTC

**Examples:**

```rust
use cloacina::cron_evaluator::CronEvaluator;
use chrono::Utc;

let evaluator = CronEvaluator::new("0 14 * * *", "America/New_York")?;
let now = Utc::now();
let next = evaluator.next_execution(now)?;

// Next execution will be at 2 PM Eastern Time, converted to UTC
// During EST: 7 PM UTC, During EDT: 6 PM UTC
```

<details>
<summary>Source</summary>

```rust
    pub fn next_execution(&self, after: DateTime<Utc>) -> Result<DateTime<Utc>, CronError> {
        // Convert UTC time to the target timezone
        let local_time = self.timezone.from_utc_datetime(&after.naive_utc());

        // Find the next execution in the local timezone, skipping the extra
        // spring-forward runs croner 3+ adds (see `is_real_fire_time`).
        let mut from = local_time;
        for _ in 0..4 {
            let next_local = self
                .cron
                .find_next_occurrence(&from, false)
                .map_err(|e| CronError::CronParsingError(e.to_string()))?;
            if self.is_real_fire_time(&next_local)? {
                // Convert back to UTC for storage and comparison
                return Ok(next_local.with_timezone(&Utc));
            }
            from = next_local;
        }
        Err(CronError::NoNextExecution)
    }
```

</details>

##### <span id="CronEvaluator.next_executions"></span><code>next_​executions</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn next_executions(
    &self,
    after: DateTime<Utc>,
    limit: usize,
) -> Result<Vec<DateTime<Utc>>, CronError>
```

Finds multiple next execution times after the given timestamp.

This is useful for catchup policies that need to run multiple missed executions.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `after` | `DateTime<Utc>` | UTC timestamp to find executions after |
| `limit` | `usize` | Maximum number of executions to return |

**Returns:**

* `Result<Vec<DateTime<Utc>>, CronError>` - List of next execution times in UTC

**Examples:**

```rust
use cloacina::cron_evaluator::CronEvaluator;
use chrono::Utc;

let evaluator = CronEvaluator::new("0 */6 * * *", "UTC").unwrap();
let now = Utc::now();
let next_executions = evaluator.next_executions(now, 5)?;

// Returns next 5 executions every 6 hours
```

<details>
<summary>Source</summary>

```rust
    pub fn next_executions(
        &self,
        after: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<DateTime<Utc>>, CronError> {
        let mut executions = Vec::with_capacity(limit);
        let mut current_time = after;

        for _ in 0..limit {
            match self.next_execution(current_time) {
                Ok(next_time) => {
                    executions.push(next_time);
                    current_time = next_time;
                }
                Err(CronError::NoNextExecution) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(executions)
    }
```

</details>

##### <span id="CronEvaluator.executions_between"></span><code>executions_​between</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn executions_between(
    &self,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    max_executions: usize,
) -> Result<Vec<DateTime<Utc>>, CronError>
```

Finds all execution times between two timestamps.

This is useful for implementing catchup policies that need to execute
all missed schedules within a time range.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `start` | `DateTime<Utc>` | Start of the time range (inclusive) |
| `end` | `DateTime<Utc>` | End of the time range (exclusive) |
| `max_executions` | `usize` | Maximum number of executions to prevent runaway |

**Returns:**

* `Result<Vec<DateTime<Utc>>, CronError>` - List of execution times in the range

**Examples:**

```rust
use cloacina::cron_evaluator::CronEvaluator;
use chrono::{Duration, Utc};

let evaluator = CronEvaluator::new("0 * * * *", "UTC")?; // Hourly
let start = Utc::now() - Duration::hours(6);
let end = Utc::now();
let missed = evaluator.executions_between(start, end, 10)?;

// Returns up to 6 hourly executions from the past 6 hours
```

<details>
<summary>Source</summary>

```rust
    pub fn executions_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        max_executions: usize,
    ) -> Result<Vec<DateTime<Utc>>, CronError> {
        let mut executions = Vec::new();
        let mut current_time = start;

        for _ in 0..max_executions {
            match self.next_execution(current_time) {
                Ok(next_time) => {
                    if next_time >= end {
                        break;
                    }
                    executions.push(next_time);
                    current_time = next_time;
                }
                Err(CronError::NoNextExecution) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(executions)
    }
```

</details>

##### <span id="CronEvaluator.expression"></span><code>expression</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn expression(&self) -> &str
```

Returns the original cron expression string.

<details>
<summary>Source</summary>

```rust
    pub fn expression(&self) -> &str {
        &self.expression
    }
```

</details>

##### <span id="CronEvaluator.timezone_str"></span><code>timezone_​str</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn timezone_str(&self) -> &str
```

Returns the timezone string.

<details>
<summary>Source</summary>

```rust
    pub fn timezone_str(&self) -> &str {
        &self.timezone_str
    }
```

</details>

##### <span id="CronEvaluator.timezone"></span><code>timezone</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn timezone(&self) -> Tz
```

Returns the timezone object.

<details>
<summary>Source</summary>

```rust
    pub fn timezone(&self) -> Tz {
        self.timezone
    }
```

</details>

##### <span id="CronEvaluator.validate_expression"></span><code>validate_​expression</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn validate_expression(cron_expr: &str) -> Result<(), CronError>
```

Validates a cron expression without creating an evaluator.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `cron_expr` | `&str` | Cron expression to validate |

**Returns:**

* `Result<(), CronError>` - Success or validation error

<details>
<summary>Source</summary>

```rust
    pub fn validate_expression(cron_expr: &str) -> Result<(), CronError> {
        parse_cron(cron_expr).map_err(|e| CronError::CronParsingError(e.to_string()))?;
        Ok(())
    }
```

</details>

##### <span id="CronEvaluator.validate_timezone"></span><code>validate_​timezone</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn validate_timezone(timezone_str: &str) -> Result<(), CronError>
```

Validates a timezone string.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `timezone_str` | `&str` | Timezone string to validate |

**Returns:**

* `Result<(), CronError>` - Success or validation error

<details>
<summary>Source</summary>

```rust
    pub fn validate_timezone(timezone_str: &str) -> Result<(), CronError> {
        timezone_str
            .parse::<Tz>()
            .map_err(|_| CronError::InvalidTimezone(timezone_str.to_string()))?;
        Ok(())
    }
```

</details>

##### <span id="CronEvaluator.validate"></span><code>validate</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn validate(
    cron_expr: &str) -> Result<(,
    timezone_str: &str) -> Result<(,
) -> Result<(), CronError>
```

Validates both cron expression and timezone.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `cron_expr` | `&str` | Cron expression to validate |
| `timezone_str` | `&str` | Timezone string to validate |

**Returns:**

* `Result<(), CronError>` - Success or validation error

<details>
<summary>Source</summary>

```rust
    pub fn validate(cron_expr: &str, timezone_str: &str) -> Result<(), CronError> {
        Self::validate_expression(cron_expr)?;
        Self::validate_timezone(timezone_str)?;
        Ok(())
    }
```

</details>

## Enums

### <span id="enum.CronError"></span><code style="font-size: 0.85em">enum CronError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors that can occur during cron evaluation.

#### Variants

- **`InvalidExpression`** - Invalid cron expression format.
- **`InvalidTimezone`** - Invalid timezone string.
- **`NoNextExecution`** - No next execution time found (e.g., end of time range).
- **`CronParsingError`** - Error from the croner crate.
