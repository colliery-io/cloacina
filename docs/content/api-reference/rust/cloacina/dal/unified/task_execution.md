# cloacina::dal::unified::task_execution <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Task Execution Data Access Layer for Unified Backend Support

This module provides the data access layer for managing task executions in the pipeline system
with runtime backend selection between PostgreSQL and SQLite.
Key features:
- Task state management (Ready, Running, Completed, Failed, Skipped)
- Retry mechanism with configurable backoff
- Recovery system for handling orphaned tasks
- Atomic task claiming for distributed execution
- Pipeline completion and failure detection

## Structs

### `cloacina::dal::unified::task_execution::RetryStats`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Default`

Statistics about retry behavior for a pipeline execution.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tasks_with_retries` | `i32` | Number of tasks that required at least one retry. |
| `total_retries` | `i32` | Total number of retry attempts across all tasks. |
| `max_attempts_used` | `i32` | Maximum number of attempts used by any single task. |
| `tasks_exhausted_retries` | `i32` | Number of tasks that exhausted all retry attempts and failed. |



### `cloacina::dal::unified::task_execution::ClaimResult`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`

Result structure for atomic task claiming operations.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` | Unique identifier of the claimed task |
| `pipeline_execution_id` | `UniversalUuid` | ID of the pipeline execution this task belongs to |
| `task_name` | `String` | Name of the task that was claimed |
| `attempt` | `i32` | Current attempt number for this task |



### `cloacina::dal::unified::task_execution::StaleClaim`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

A task with a stale claim (heartbeat expired).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_id` | `UniversalUuid` | Task execution ID. |
| `claimed_by` | `UniversalUuid` | The runner that claimed it (now presumed dead). |
| `heartbeat_at` | `chrono :: DateTime < chrono :: Utc >` | Last heartbeat timestamp. |



### `cloacina::dal::unified::task_execution::TaskExecutionDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Data access layer for task execution operations with runtime backend selection.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



## Enums

### `cloacina::dal::unified::task_execution::RunnerClaimResult` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Result of attempting to claim a task for a specific runner.

#### Variants

- **`Claimed`** - Successfully claimed the task.
- **`AlreadyClaimed`** - Another runner already claimed this task.



### `cloacina::dal::unified::task_execution::HeartbeatResult` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Result of a heartbeat attempt.

#### Variants

- **`Ok`** - Heartbeat updated successfully — this runner still owns the claim.
- **`ClaimLost`** - Claim was lost — another runner has claimed this task (or claim was released).
