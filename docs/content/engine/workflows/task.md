---
title: "Task"
description: "A unit of work in a workflow — an async function with an id, dependencies, and optional retries."
weight: 12
---

# Task

A **Task** is the unit of work in a [Workflow]({{< ref "/engine/workflows/workflow" >}})
— and the **unit of scheduling**. It is an async function with a unique `id`, a
list of `dependencies` (the DAG edges), and optional retry behavior. It receives a
[Context]({{< ref "/engine/workflows/context" >}}), does its work, and returns the
(possibly modified) context.

## Mental model

- A task is **contained by** a workflow; its `dependencies` declare which tasks
  must complete first.
- It **reads and writes** the shared context.
- It is **claimed atomically** and **retried** on failure per its retry policy.
  Because execution is at-least-once, a task must be **idempotent**.

## Interfaces

A task with one dependency, declared in each interface:

{{< tabs "task-define" >}}
{{< tab "Rust" >}}
```rust
use cloacina::{task, Context, TaskError};

#[task(id = "transform", dependencies = ["fetch"])]
async fn transform(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let raw = context.get("raw").cloned().unwrap_or_default();
    context.insert("transformed", raw)?;
    Ok(())
}
```
{{< /tab >}}
{{< tab "Python" >}}
```python
import cloaca

@cloaca.task(id="transform", dependencies=["fetch"])
def transform(context):
    context.set("transformed", context.get("raw"))
    return context
```
{{< /tab >}}
{{< /tabs >}}

## Retries

Retry behavior is configured on the task itself. The discrete knobs:
`retry_attempts`, `retry_backoff` (`fixed` / `linear` / `exponential`),
`retry_delay_ms`, `retry_max_delay_ms`, `retry_condition`
(`never` / `transient` / `all`), and `retry_jitter`.

{{< tabs "task-retry" >}}
{{< tab "Rust" >}}
```rust
#[task(
    id = "flaky",
    dependencies = [],
    retry_attempts = 5,
    retry_backoff = "exponential",
    retry_delay_ms = 1000,
)]
async fn flaky(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // ...
    Ok(())
}
```
{{< /tab >}}
{{< tab "Python" >}}
```python
@cloaca.task(
    id="flaky",
    retry_attempts=5,
    retry_backoff="exponential",
    retry_delay_ms=1000,
)
def flaky(context):
    return context
```
{{< /tab >}}
{{< /tabs >}}

See [Conditional Retries]({{< ref "/workflows/how-to-guides/conditional-retries" >}})
for retry-condition patterns.

## Key facts

- **`id`** is the task's identity within its workflow. In Python it defaults to
  the function name if omitted.
- **`dependencies`** are task ids (or, in Python, task functions) that must
  complete first.
- Tasks are **async**; offload blocking/CPU-bound work (`tokio::task::spawn_blocking`
  in Rust, or a blocking call in Python).

## See also

- [Workflow]({{< ref "/engine/workflows/workflow" >}}) · [Context]({{< ref "/engine/workflows/context" >}}) · [Runner]({{< ref "/engine/workflows/runner" >}})
- Full task API: [Reference]({{< ref "/reference" >}})
