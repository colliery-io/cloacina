# cloacina::execution_planner::recovery <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Task recovery and orphan detection.

This module handles detection and recovery of tasks that were orphaned
due to system interruptions or crashes.

## Structs

### `cloacina::execution_planner::recovery::RecoveryManager`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Recovery operations for the scheduler.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
| `runtime` | `Arc < Runtime >` |  |



## Enums

### `cloacina::execution_planner::recovery::RecoveryResult` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Result of attempting to recover a task.

#### Variants

- **`Recovered`** - Task was successfully recovered and reset for retry.
- **`Abandoned`** - Task was abandoned due to exceeding recovery limits.
