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

#[task(dependencies = ["fetch"])]
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

@cloaca.task(dependencies=["fetch"])
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
    retry_attempts=5,
    retry_backoff="exponential",
    retry_delay_ms=1000,
)
def flaky(context):
    return context
```
{{< /tab >}}
{{< /tabs >}}

See [Conditional Retries]({{< ref "/embed/how-to/conditional-retries" >}})
for retry-condition patterns.

## Documenting a task: `what:` / `why:`

A `#[task]` doc-comment (Rust `///`) or `@task` docstring (Python) can carry
structured documentation that the compiler lifts into the package manifest and
surfaces on each `WorkflowTaskNode` (as `doc_what` / `doc_why`). The convention
is line-leading, case-insensitive `what:` and `why:` markers: text following a
`what:` line routes into the `what` field, text following a `why:` line routes
into `why`. With **no markers**, the whole comment becomes `what` and `why` is
left empty.

{{< tabs "task-doc" >}}
{{< tab "Rust" >}}
```rust
#[task(id = "validate")]
/// what: validates the incoming order
/// why: downstream pricing assumes a clean order
async fn validate(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    Ok(())
}
```
{{< /tab >}}
{{< tab "Python" >}}
```python
@cloaca.task(id="extract")
def extract(context):
    """
    what: pulls rows from the source
    why: the rest of the graph needs them staged
    """
    return context
```
{{< /tab >}}
{{< /tabs >}}

The parsing is best-effort and **degrades gracefully** — it is never required and
never fails the build. A task with no doc-comment contributes no docs; a source
file that fails to parse simply contributes nothing rather than erroring. The
extractor lives in `crates/cloacina-compiler/src/doc_parse.rs` and runs against
the unpacked package source at build time (CLOACI-I-0126 / T-0752).

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
