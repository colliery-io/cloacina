---
title: "cloacina::dal::unified::task_execution"
description: "Task Execution Data Access Layer for Unified Backend Support"
editUrl: false
sidebar:
  order: 86
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Task Execution Data Access Layer for Unified Backend Support

This module provides the data access layer for managing task executions in the workflow system
with runtime backend selection between PostgreSQL and SQLite.

Key features:
- Task state management (Ready, Running, Completed, Failed, Skipped)
- Retry mechanism with configurable backoff
- Recovery system for handling orphaned tasks
- Atomic task claiming for distributed execution
- Workflow completion and failure detection

## Structs

### <span id="struct.RetryStats"></span><code style="font-size: 0.85em">struct RetryStats</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Default`

Statistics about retry behavior for a workflow execution.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tasks_with_retries` | `i32` | Number of tasks that required at least one retry. |
| `total_retries` | `i32` | Total number of retry attempts across all tasks. |
| `max_attempts_used` | `i32` | Maximum number of attempts used by any single task. |
| `tasks_exhausted_retries` | `i32` | Number of tasks that exhausted all retry attempts and failed. |

### <span id="struct.ClaimResult"></span><code style="font-size: 0.85em">struct ClaimResult</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`

Result structure for atomic task claiming operations.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` | Unique identifier of the claimed task |
| `workflow_execution_id` | `UniversalUuid` | ID of the workflow execution this task belongs to |
| `task_name` | `String` | Name of the task that was claimed |
| `attempt` | `i32` | Current attempt number for this task |

### <span id="struct.StaleClaim"></span><code style="font-size: 0.85em">struct StaleClaim</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

A task with a stale claim (heartbeat expired).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_id` | `UniversalUuid` | Task execution ID. |
| `claimed_by` | `UniversalUuid` | The runner that claimed it (now presumed dead). |
| `heartbeat_at` | `chrono::DateTime<chrono::Utc>` | Last heartbeat timestamp. |
| `recovery_attempts` | `i32` | How many times this task has already been recovered from a stale claim. Lets the sweeper cap recycling (CLOACI-T-0914) instead of re-Readying a runner-killing task forever. |

### <span id="struct.TaskExecutionDAL"></span><code style="font-size: 0.85em">struct TaskExecutionDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Data access layer for task execution operations with runtime backend selection.

#### Methods

##### <span id="TaskExecutionDAL.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL) -> Self
```

Creates a new TaskExecutionDAL instance.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
```

</details>

## Enums

### <span id="enum.RunnerClaimResult"></span><code style="font-size: 0.85em">enum RunnerClaimResult</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Result of attempting to claim a task for a specific runner.

#### Variants

- **`Claimed`** - Successfully claimed the task.
- **`AlreadyClaimed`** - Another runner already claimed this task.

### <span id="enum.HeartbeatResult"></span><code style="font-size: 0.85em">enum HeartbeatResult</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Result of a heartbeat attempt.

#### Variants

- **`Ok`** - Heartbeat updated successfully — this runner still owns the claim.
- **`ClaimLost`** - Claim was lost — another runner has claimed this task (or claim was released).
