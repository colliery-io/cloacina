# cloacina::models::schedule <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified schedule management module for both cron and trigger-based workflow execution.

This module provides domain structures for the unified `schedules` and
`schedule_executions` tables, replacing the separate cron and trigger models.

## Structs

### `cloacina::models::schedule::Schedule`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Represents a unified schedule record (domain type).

Contains fields for both cron and trigger schedules. Fields irrelevant
to the `schedule_type` will be `None`.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `schedule_type` | `String` |  |
| `workflow_name` | `String` |  |
| `enabled` | `UniversalBool` |  |
| `cron_expression` | `Option < String >` |  |
| `timezone` | `Option < String >` |  |
| `catchup_policy` | `Option < String >` |  |
| `start_date` | `Option < UniversalTimestamp >` |  |
| `end_date` | `Option < UniversalTimestamp >` |  |
| `trigger_name` | `Option < String >` |  |
| `poll_interval_ms` | `Option < i32 >` |  |
| `allow_concurrent` | `Option < UniversalBool >` |  |
| `next_run_at` | `Option < UniversalTimestamp >` |  |
| `last_run_at` | `Option < UniversalTimestamp >` |  |
| `last_poll_at` | `Option < UniversalTimestamp >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |

#### Methods

##### `get_type` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_type (& self) -> ScheduleType
```

Returns the schedule type as an enum.

<details>
<summary>Source</summary>

```rust
    pub fn get_type(&self) -> ScheduleType {
        ScheduleType::from(self.schedule_type.as_str())
    }
```

</details>



##### `is_cron` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_cron (& self) -> bool
```

Returns true if this is a cron schedule.

<details>
<summary>Source</summary>

```rust
    pub fn is_cron(&self) -> bool {
        self.get_type() == ScheduleType::Cron
    }
```

</details>



##### `is_trigger` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_trigger (& self) -> bool
```

Returns true if this is a trigger schedule.

<details>
<summary>Source</summary>

```rust
    pub fn is_trigger(&self) -> bool {
        self.get_type() == ScheduleType::Trigger
    }
```

</details>



##### `is_enabled` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_enabled (& self) -> bool
```

Returns true if the schedule is enabled.

<details>
<summary>Source</summary>

```rust
    pub fn is_enabled(&self) -> bool {
        self.enabled.is_true()
    }
```

</details>



##### `poll_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn poll_interval (& self) -> Option < Duration >
```

Returns the poll interval as a Duration (trigger schedules only).

<details>
<summary>Source</summary>

```rust
    pub fn poll_interval(&self) -> Option<Duration> {
        self.poll_interval_ms
            .map(|ms| Duration::from_millis(ms as u64))
    }
```

</details>



##### `allows_concurrent` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn allows_concurrent (& self) -> bool
```

Returns true if concurrent executions are allowed (trigger schedules only).

<details>
<summary>Source</summary>

```rust
    pub fn allows_concurrent(&self) -> bool {
        self.allow_concurrent
            .as_ref()
            .map(|b| b.is_true())
            .unwrap_or(false)
    }
```

</details>





### `cloacina::models::schedule::NewSchedule`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Structure for creating new schedule records.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `schedule_type` | `String` |  |
| `workflow_name` | `String` |  |
| `enabled` | `Option < UniversalBool >` |  |
| `cron_expression` | `Option < String >` |  |
| `timezone` | `Option < String >` |  |
| `catchup_policy` | `Option < String >` |  |
| `start_date` | `Option < UniversalTimestamp >` |  |
| `end_date` | `Option < UniversalTimestamp >` |  |
| `trigger_name` | `Option < String >` |  |
| `poll_interval_ms` | `Option < i32 >` |  |
| `allow_concurrent` | `Option < UniversalBool >` |  |
| `next_run_at` | `Option < UniversalTimestamp >` |  |

#### Methods

##### `cron` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn cron (workflow_name : & str , cron_expression : & str , next_run_at : UniversalTimestamp ,) -> Self
```

Create a new cron schedule.

<details>
<summary>Source</summary>

```rust
    pub fn cron(
        workflow_name: &str,
        cron_expression: &str,
        next_run_at: UniversalTimestamp,
    ) -> Self {
        Self {
            schedule_type: "cron".to_string(),
            workflow_name: workflow_name.to_string(),
            enabled: Some(UniversalBool::new(true)),
            cron_expression: Some(cron_expression.to_string()),
            timezone: Some("UTC".to_string()),
            catchup_policy: Some("skip".to_string()),
            start_date: None,
            end_date: None,
            trigger_name: None,
            poll_interval_ms: None,
            allow_concurrent: None,
            next_run_at: Some(next_run_at),
        }
    }
```

</details>



##### `trigger` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn trigger (trigger_name : & str , workflow_name : & str , poll_interval : Duration) -> Self
```

Create a new trigger schedule.

<details>
<summary>Source</summary>

```rust
    pub fn trigger(trigger_name: &str, workflow_name: &str, poll_interval: Duration) -> Self {
        Self {
            schedule_type: "trigger".to_string(),
            workflow_name: workflow_name.to_string(),
            enabled: Some(UniversalBool::new(true)),
            cron_expression: None,
            timezone: None,
            catchup_policy: None,
            start_date: None,
            end_date: None,
            trigger_name: Some(trigger_name.to_string()),
            poll_interval_ms: Some(poll_interval.as_millis() as i32),
            allow_concurrent: Some(UniversalBool::new(false)),
            next_run_at: None,
        }
    }
```

</details>





### `cloacina::models::schedule::ScheduleExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Represents a schedule execution record (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `schedule_id` | `UniversalUuid` |  |
| `pipeline_execution_id` | `Option < UniversalUuid >` |  |
| `scheduled_time` | `Option < UniversalTimestamp >` |  |
| `claimed_at` | `Option < UniversalTimestamp >` |  |
| `context_hash` | `Option < String >` |  |
| `started_at` | `UniversalTimestamp` |  |
| `completed_at` | `Option < UniversalTimestamp >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### `cloacina::models::schedule::NewScheduleExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Structure for creating new schedule execution records.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `UniversalUuid` |  |
| `pipeline_execution_id` | `Option < UniversalUuid >` |  |
| `scheduled_time` | `Option < UniversalTimestamp >` |  |
| `claimed_at` | `Option < UniversalTimestamp >` |  |
| `context_hash` | `Option < String >` |  |



## Enums

### `cloacina::models::schedule::CatchupPolicy` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Enum representing the different catchup policies for missed cron executions.

#### Variants

- **`Skip`**
- **`RunAll`**



### `cloacina::models::schedule::ScheduleType` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


The type of schedule — determines which fields are relevant.

#### Variants

- **`Cron`**
- **`Trigger`**
