# cloacina::models::task_execution <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Task Execution Model

This module defines domain structures for tracking task executions.
These are API-level types; backend-specific models handle database storage.

## Structs

### `cloacina::models::task_execution::TaskExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Represents a task execution record (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `pipeline_execution_id` | `UniversalUuid` |  |
| `task_name` | `String` |  |
| `status` | `String` |  |
| `started_at` | `Option < UniversalTimestamp >` |  |
| `completed_at` | `Option < UniversalTimestamp >` |  |
| `attempt` | `i32` |  |
| `max_attempts` | `i32` |  |
| `error_details` | `Option < String >` |  |
| `trigger_rules` | `String` |  |
| `task_configuration` | `String` |  |
| `retry_at` | `Option < UniversalTimestamp >` |  |
| `last_error` | `Option < String >` |  |
| `recovery_attempts` | `i32` |  |
| `last_recovery_at` | `Option < UniversalTimestamp >` |  |
| `sub_status` | `Option < String >` |  |
| `claimed_by` | `Option < UniversalUuid >` |  |
| `heartbeat_at` | `Option < UniversalTimestamp >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### `cloacina::models::task_execution::NewTaskExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Structure for creating new task executions (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `pipeline_execution_id` | `UniversalUuid` |  |
| `task_name` | `String` |  |
| `status` | `String` |  |
| `attempt` | `i32` |  |
| `max_attempts` | `i32` |  |
| `trigger_rules` | `String` |  |
| `task_configuration` | `String` |  |
