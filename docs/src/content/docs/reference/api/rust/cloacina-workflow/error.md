---
title: "cloacina-workflow::error"
description: "API reference of the Rust module cloacina-workflow::error."
sidebar:
  order: 48
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>




## Enums

### <span id="enum.ContextError"></span>`enum ContextError`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Errors that can occur during context operations.

This minimal version only includes errors that can occur without database
or runtime dependencies.

#### Variants

- **`Serialization`** - JSON serialization/deserialization error
- **`KeyNotFound`** - Key not found in context
- **`TypeMismatch`** - Type mismatch when retrieving a value
- **`KeyExists`** - Key already exists when inserting
- **`Database`** - Database operation failed (infrastructure error, not a key issue)
- **`ConnectionPool`** - Connection pool exhausted or unavailable



### <span id="enum.TaskError"></span>`enum TaskError`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Errors that can occur during task execution.

Task errors encompass execution failures, context issues, and
any other problems that prevent a task from completing successfully.

#### Variants

- **`ExecutionFailed`** - Task execution failed with a message
- **`DependencyNotSatisfied`** - Task dependency not satisfied
- **`Timeout`** - Task exceeded timeout
- **`ContextError`** - Context operation error within a task
- **`ValidationFailed`** - Task validation failed
- **`Unknown`** - Unknown error
- **`ReadinessCheckFailed`** - Task readiness check failed
- **`TriggerRuleFailed`** - Trigger rule evaluation failed



### <span id="enum.CheckpointError"></span>`enum CheckpointError`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Errors that can occur during task checkpointing.

Checkpoint errors occur when tasks attempt to save intermediate state
for recovery purposes.

#### Variants

- **`SaveFailed`** - Failed to save checkpoint
- **`LoadFailed`** - Failed to load checkpoint
- **`Serialization`** - Checkpoint serialization error
- **`StorageError`** - Checkpoint storage error
- **`ValidationFailed`** - Checkpoint validation failed



