---
title: "cloacina::models::task_execution_metadata"
description: "Task Execution Metadata Module"
editUrl: false
sidebar:
  order: 143
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Task Execution Metadata Module

This module defines domain structures for task execution metadata.
These are API-level types; backend-specific models handle database storage.

## Structs

### <span id="struct.TaskExecutionMetadata"></span><code style="font-size: 0.85em">struct TaskExecutionMetadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Represents a task execution metadata record (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `task_execution_id` | `UniversalUuid` |  |
| `workflow_execution_id` | `UniversalUuid` |  |
| `task_name` | `String` |  |
| `context_id` | `Option<UniversalUuid>` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |

### <span id="struct.NewTaskExecutionMetadata"></span><code style="font-size: 0.85em">struct NewTaskExecutionMetadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Structure for creating new task execution metadata (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_execution_id` | `UniversalUuid` |  |
| `workflow_execution_id` | `UniversalUuid` |  |
| `task_name` | `String` |  |
| `context_id` | `Option<UniversalUuid>` |  |
