---
title: "02 — Passing data with Context"
description: "Move data between dependent tasks through the Context."
weight: 12
aliases:
  - "/python/workflows/tutorials/02-context-handling/"
  - "/workflows/tutorials/library/02-context-handling/"

---

# 02 — Passing data with Context

A [Context]({{< ref "/engine/workflows/context" >}}) is the typed, persisted
container that flows through a workflow. One task writes; a downstream task reads.

## Two dependent tasks

`produce` writes a value; `consume` depends on it and reads it.

{{< tabs "t02" >}}
{{< tab "Rust" >}}
```rust
#[workflow(name = "pipeline", description = "Pass data downstream")]
pub mod pipeline {
    use super::*;

    #[task]
    pub async fn produce(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        ctx.insert("numbers", serde_json::json!([1, 2, 3]))?;
        Ok(())
    }

    #[task(dependencies = ["produce"])]
    pub async fn consume(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let nums = ctx.get("numbers").cloned().unwrap_or_default();
        ctx.insert("sum", serde_json::json!(/* sum of nums */ 6))?;
        Ok(())
    }
}
```
{{< /tab >}}
{{< tab "Python" >}}
```python
with cloaca.WorkflowBuilder("pipeline") as builder:
    builder.description("Pass data downstream")

    @cloaca.task()
    def produce(context):
        context.set("numbers", [1, 2, 3])
        return context

    @cloaca.task(dependencies=["produce"])
    def consume(context):
        nums = context.get("numbers")
        context.set("sum", sum(nums))
        return context
```
{{< /tab >}}
{{< /tabs >}}

The `dependencies` list makes `consume` run after `produce`, so the value is
present when it reads. The context is persisted, so it survives a restart and is
returned as `result.final_context`.

## Next

- **[03 — Dependencies & parallelism]({{< ref "/embed/tutorials/03-dependencies" >}})**
- Reference: [Context]({{< ref "/engine/workflows/context" >}})
