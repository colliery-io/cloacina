---
title: "Context"
description: "The typed, serializable container that carries data between tasks — persisted and recovered."
weight: 13
---

# Context

A **Context** is the typed, serializable container that carries data between
[Tasks]({{< ref "/engine/workflows/task" >}}) in a [Workflow]({{< ref "/engine/workflows/workflow" >}}).
One task writes a value; a downstream task reads it. The context is **persisted**
with the execution, so it survives restarts and is available in the final result.

## Mental model

- The context **flows through the DAG**: each task receives it, may mutate it, and
  returns it.
- It is **persisted per execution** — recovery restores it.
- The final context is returned on the execution result (`final_context`).

## Interfaces

{{< tabs "context-use" >}}
{{< tab "Rust" >}}
`Context<T>` is generic over the (serde-serializable) value type; most workflows
use `Context<serde_json::Value>`:

```rust
use cloacina::Context;

let mut ctx = Context::new();
ctx.insert("raw", serde_json::json!([1, 2, 3]))?;
let raw = ctx.get("raw");          // Option<&Value>
```
{{< /tab >}}
{{< tab "Python" >}}
```python
import cloaca

ctx = cloaca.Context({"job_id": "job_001"})
ctx.set("raw", [1, 2, 3])
raw = ctx.get("raw")
```
{{< /tab >}}
{{< /tabs >}}

## Key facts

- **Serializable:** values must be JSON-serializable; the context is stored in the
  database between task executions.
- **Typed (Rust):** `Context<T>` is generic; the common case is
  `Context<serde_json::Value>`.
- **Initial + final:** you pass an initial context to `execute`; the result exposes
  the `final_context`.

## See also

- [Task]({{< ref "/engine/workflows/task" >}}) · [Workflow]({{< ref "/engine/workflows/workflow" >}}) · [Runner]({{< ref "/engine/workflows/runner" >}})
- [Context management]({{< ref "/engine/explanation/context-management" >}}) (design)
