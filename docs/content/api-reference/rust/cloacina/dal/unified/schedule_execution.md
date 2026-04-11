# cloacina::dal::unified::schedule_execution <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified Schedule Execution DAL with runtime backend selection

This module provides operations for the unified `schedule_executions` table
that replaces the separate `cron_executions` and `trigger_executions` tables.
Works with both PostgreSQL and SQLite backends, selecting the appropriate
implementation at runtime based on the database connection type.

## Structs

### `cloacina::dal::unified::schedule_execution::ScheduleExecutionStats`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`

Statistics about schedule execution performance

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `total_executions` | `i64` | Total number of executions attempted |
| `successful_executions` | `i64` | Number of executions that successfully handed off to pipeline executor |
| `lost_executions` | `i64` | Number of executions that were lost (started but never completed within expected time) |
| `success_rate` | `f64` | Success rate as a percentage |



### `cloacina::dal::unified::schedule_execution::ScheduleExecutionDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Data access layer for unified schedule execution operations with runtime backend selection.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
