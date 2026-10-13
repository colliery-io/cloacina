---
title: "cloacina-workflow"
description: "API reference of the Rust module cloacina-workflow."
editUrl: false
sidebar:
  order: 29
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Cloacina Workflow - Minimal Types for Workflow Authoring**

This crate provides the minimal set of types needed to compile Cloacina workflows
without pulling in heavy runtime dependencies like database drivers.

**Purpose**

Workflow authors who only need to compile workflows can depend on this lightweight
crate instead of the full `cloacina` crate. This provides:

- Faster compile times
- Smaller binary sizes
- Easier cross-compilation (no native database drivers)
- Clear separation between "authoring workflows" and "running workflows"

**Types Provided**

- [`Context`](../../rust/cloacina-workflow/context/#struct.Context) - Data container for sharing values between tasks
- [`Task`](../../rust/cloacina-workflow/task/#trait.Task) - Trait defining executable tasks
- [`TaskState`](../../rust/cloacina-workflow/task/#enum.TaskState) - Task execution state enum
- [`TaskNamespace`](../../rust/cloacina-workflow/namespace/#struct.TaskNamespace) - Hierarchical task identification
- [`TaskError`](../../rust/cloacina-workflow/error/#enum.TaskError), [`ContextError`](../../rust/cloacina-workflow/error/#enum.ContextError), [`CheckpointError`](../../rust/cloacina-workflow/error/#enum.CheckpointError) - Error types
- [`RetryPolicy`](../../rust/cloacina-workflow/retry/#struct.RetryPolicy), [`BackoffStrategy`](../../rust/cloacina-workflow/retry/#enum.BackoffStrategy), [`RetryCondition`](../../rust/cloacina-workflow/retry/#enum.RetryCondition) - Retry configuration

**Usage**

```rust
use cloacina_workflow::{Context, TaskError};

// Create a context
let mut ctx = Context::<serde_json::Value>::new();
ctx.insert("key", serde_json::json!("value")).unwrap();

// Access data
let value = ctx.get("key").unwrap();
```

**With the Task Macro**

The macros are included by default, so you only need one import:

```rust
use cloacina_workflow::{task, packaged_workflow, Context, TaskError};

#[task(id = "my_task", dependencies = [])]
async fn my_task(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    ctx.insert("result", serde_json::json!("done"))?;
    Ok(())
}
```

## Re-exports

- [`Context`](../../rust/cloacina-workflow/context/#struct.Context)
- [`CheckpointError`](../../rust/cloacina-workflow/error/#enum.CheckpointError)
- [`ContextError`](../../rust/cloacina-workflow/error/#enum.ContextError)
- [`TaskError`](../../rust/cloacina-workflow/error/#enum.TaskError)
- `InputSlot` (`input_interface::InputSlot`)
- [`schema_for`](../../rust/cloacina-workflow/input_interface/#fn.schema_for)
- [`TaskNamespace`](../../rust/cloacina-workflow/namespace/#struct.TaskNamespace)
- [`parse_namespace`](../../rust/cloacina-workflow/namespace/#fn.parse_namespace)
- [`BackoffStrategy`](../../rust/cloacina-workflow/retry/#enum.BackoffStrategy)
- [`RetryCondition`](../../rust/cloacina-workflow/retry/#enum.RetryCondition)
- [`RetryPolicy`](../../rust/cloacina-workflow/retry/#struct.RetryPolicy)
- [`RetryPolicyBuilder`](../../rust/cloacina-workflow/retry/#struct.RetryPolicyBuilder)
- [`SecretAccessError`](../../rust/cloacina-workflow/secret/#enum.SecretAccessError)
- [`SecretResolver`](../../rust/cloacina-workflow/secret/#trait.SecretResolver)
- [`SecretResolverError`](../../rust/cloacina-workflow/secret/#enum.SecretResolverError)
- [`Task`](../../rust/cloacina-workflow/task/#trait.Task)
- [`TaskState`](../../rust/cloacina-workflow/task/#enum.TaskState)
- [`Trigger`](../../rust/cloacina-workflow/trigger/#trait.Trigger)
- [`TriggerError`](../../rust/cloacina-workflow/trigger/#enum.TriggerError)
- [`TriggerResult`](../../rust/cloacina-workflow/trigger/#enum.TriggerResult)
- [`task`](../../rust/cloacina-python/task/#fn.task)
- `trigger` (`cloacina_macros::trigger`)
- `workflow` (`cloacina_macros::workflow`)
