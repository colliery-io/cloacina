# cloacina::cron_evaluator <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Timezone-aware cron expression evaluator

This module provides functionality for parsing and evaluating cron expressions
with proper timezone support. It uses the `croner` crate for cron parsing and
`chrono-tz` for timezone handling.

**Examples:**

```rust
use cloacina::cron_evaluator::CronEvaluator;
use chrono::{DateTime, Utc};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
// Create evaluator for daily 9 AM EST/EDT
let evaluator = CronEvaluator::new("0 9 * * *", "America/New_York")?;

// Find next execution after current time
let now = Utc::now();
let next = evaluator.next_execution(now)?;

println!("Next execution: {}", next);
# Ok(())
# }
```

## Structs

### `cloacina::cron_evaluator::CronEvaluator`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Timezone-aware cron expression evaluator.

This struct provides methods for evaluating cron expressions in specific timezones,
handling daylight saving time transitions automatically.

**Examples:**

```rust
use cloacina::cron_evaluator::CronEvaluator;
use chrono::Utc;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
// Daily at 2 AM Eastern Time (handles EST/EDT automatically)
let evaluator = CronEvaluator::new("0 2 * * *", "America/New_York")?;
let next = evaluator.next_execution(Utc::now())?;

// Hourly during business hours in London
let evaluator = CronEvaluator::new("0 9-17 * * 1-5", "Europe/London")?;
let next = evaluator.next_execution(Utc::now())?;
# Ok(())
# }
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `cron` | `Cron` | Parsed cron expression |
| `timezone` | `Tz` | Timezone for interpreting the cron expression |
| `expression` | `String` | Original cron expression string for debugging |
| `timezone_str` | `String` | Original timezone string for debugging |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (cron_expr : & str , timezone_str : & str) -> Result < Self , CronError >
```

Creates a new cron evaluator with the specified expression and timezone.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `cron_expr` | `-` | Standard cron expression (5 fields: minute hour day month weekday) |
| `timezone_str` | `-` | IANA timezone name (e.g., "America/New_York", "Europe/London", "UTC") |


**Returns:**

* `Result<Self, CronError>` - New evaluator instance or error

**Examples:**

```rust
use cloacina::cron_evaluator::CronEvaluator;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
// Daily at 9 AM Eastern Time
let evaluator = CronEvaluator::new("0 9 * * *", "America/New_York")?;

// Every 15 minutes during UTC business hours
let evaluator = CronEvaluator::new("*/15 9-17 * * 1-5", "UTC")?;

// Monthly on the 1st at midnight Pacific Time
let evaluator = CronEvaluator::new("0 0 1 * *", "America/Los_Angeles")?;
# Ok(())
# }
```

<details>
<summary>Source</summary>

```rust
    pub fn new(cron_expr: &str, timezone_str: &str) -> Result<Self, CronError> {
        let cron = Cron::new(cron_expr)
            .with_seconds_optional() // Enable optional seconds support
            .parse()
            .map_err(|e| CronError::CronParsingError(e.to_string()))?;

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



##### `next_execution` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn next_execution (& self , after : DateTime < Utc >) -> Result < DateTime < Utc > , CronError >
```

Finds the next execution time after the given timestamp.

This method converts the UTC timestamp to the evaluator's timezone,
finds the next cron match in that timezone, then converts back to UTC.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `after` | `-` | UTC timestamp to find the next execution after |


**Returns:**

* `Result<DateTime<Utc>, CronError>` - Next execution time in UTC

**Examples:**

```rust
use cloacina::cron_evaluator::CronEvaluator;
use chrono::Utc;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let evaluator = CronEvaluator::new("0 14 * * *", "America/New_York")?;
let now = Utc::now();
let next = evaluator.next_execution(now)?;

// Next execution will be at 2 PM Eastern Time, converted to UTC
// During EST: 7 PM UTC, During EDT: 6 PM UTC
# Ok(())
# }
```

<details>
<summary>Source</summary>

```rust
    pub fn next_execution(&self, after: DateTime<Utc>) -> Result<DateTime<Utc>, CronError> {
        // Convert UTC time to the target timezone
        let local_time = self.timezone.from_utc_datetime(&after.naive_utc());

        // Find the next execution in the local timezone
        let next_local = self
            .cron
            .find_next_occurrence(&local_time, false)
            .map_err(|e| CronError::CronParsingError(e.to_string()))?;

        // Convert back to UTC for storage and comparison
        Ok(next_local.with_timezone(&Utc))
    }
```

</details>



##### `next_executions` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn next_executions (& self , after : DateTime < Utc > , limit : usize ,) -> Result < Vec < DateTime < Utc > > , CronError >
```

Finds multiple next execution times after the given timestamp.

This is useful for catchup policies that need to run multiple missed executions.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `after` | `-` | UTC timestamp to find executions after |
| `limit` | `-` | Maximum number of executions to return |


**Returns:**

* `Result<Vec<DateTime<Utc>>, CronError>` - List of next execution times in UTC

**Examples:**

```rust
use cloacina::cron_evaluator::CronEvaluator;
use chrono::Utc;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let evaluator = CronEvaluator::new("0 */6 * * *", "UTC").unwrap();
let now = Utc::now();
let next_executions = evaluator.next_executions(now, 5)?;

// Returns next 5 executions every 6 hours
# Ok(())
# }
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



##### `executions_between` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn executions_between (& self , start : DateTime < Utc > , end : DateTime < Utc > , max_executions : usize ,) -> Result < Vec < DateTime < Utc > > , CronError >
```

Finds all execution times between two timestamps.

This is useful for implementing catchup policies that need to execute
all missed schedules within a time range.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `start` | `-` | Start of the time range (inclusive) |
| `end` | `-` | End of the time range (exclusive) |
| `max_executions` | `-` | Maximum number of executions to prevent runaway |


**Returns:**

* `Result<Vec<DateTime<Utc>>, CronError>` - List of execution times in the range

**Examples:**

```rust
use cloacina::cron_evaluator::CronEvaluator;
use chrono::{Duration, Utc};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let evaluator = CronEvaluator::new("0 * * * *", "UTC")?; // Hourly
let start = Utc::now() - Duration::hours(6);
let end = Utc::now();
let missed = evaluator.executions_between(start, end, 10)?;

// Returns up to 6 hourly executions from the past 6 hours
# Ok(())
# }
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



##### `expression` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn expression (& self) -> & str
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



##### `timezone_str` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn timezone_str (& self) -> & str
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



##### `timezone` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn timezone (& self) -> Tz
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



##### `validate_expression` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn validate_expression (cron_expr : & str) -> Result < () , CronError >
```

Validates a cron expression without creating an evaluator.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `cron_expr` | `-` | Cron expression to validate |


**Returns:**

* `Result<(), CronError>` - Success or validation error

<details>
<summary>Source</summary>

```rust
    pub fn validate_expression(cron_expr: &str) -> Result<(), CronError> {
        Cron::new(cron_expr)
            .with_seconds_optional() // Enable optional seconds support
            .parse()
            .map_err(|e| CronError::CronParsingError(e.to_string()))?;
        Ok(())
    }
```

</details>



##### `validate_timezone` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn validate_timezone (timezone_str : & str) -> Result < () , CronError >
```

Validates a timezone string.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `timezone_str` | `-` | Timezone string to validate |


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



##### `validate` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn validate (cron_expr : & str , timezone_str : & str) -> Result < () , CronError >
```

Validates both cron expression and timezone.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `cron_expr` | `-` | Cron expression to validate |
| `timezone_str` | `-` | Timezone string to validate |


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

### `cloacina::cron_evaluator::CronError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during cron evaluation.

#### Variants

- **`InvalidExpression`** - Invalid cron expression format.
- **`InvalidTimezone`** - Invalid timezone string.
- **`NoNextExecution`** - No next execution time found (e.g., end of time range).
- **`CronParsingError`** - Error from the croner crate.
