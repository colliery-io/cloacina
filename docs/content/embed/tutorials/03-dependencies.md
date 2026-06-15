---
title: "03 — Dependencies & parallelism"
description: "Express a DAG with dependencies; independent tasks run in parallel."
weight: 13
---

# 03 — Dependencies & parallelism

The `dependencies` list on each [Task]({{< ref "/engine/workflows/task" >}}) defines
the DAG edges. Tasks with no dependency between them run **in parallel**.

## A diamond

`fetch` → (`transform_a`, `transform_b` in parallel) → `combine`.

{{< tabs "t03" >}}
{{< tab "Rust" >}}
```rust
#[workflow(name = "diamond", description = "Fan out then fan in")]
pub mod diamond {
    use super::*;

    #[task(id = "fetch", dependencies = [])]
    pub async fn fetch(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> { Ok(()) }

    #[task(id = "transform_a", dependencies = ["fetch"])]
    pub async fn transform_a(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> { Ok(()) }

    #[task(id = "transform_b", dependencies = ["fetch"])]
    pub async fn transform_b(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> { Ok(()) }

    #[task(id = "combine", dependencies = ["transform_a", "transform_b"])]
    pub async fn combine(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> { Ok(()) }
}
```
{{< /tab >}}
{{< tab "Python" >}}
```python
with cloaca.WorkflowBuilder("diamond") as builder:
    builder.description("Fan out then fan in")

    @cloaca.task(id="fetch")
    def fetch(context): return context

    @cloaca.task(id="transform_a", dependencies=["fetch"])
    def transform_a(context): return context

    @cloaca.task(id="transform_b", dependencies=["fetch"])
    def transform_b(context): return context

    @cloaca.task(id="combine", dependencies=["transform_a", "transform_b"])
    def combine(context): return context
```
{{< /tab >}}
{{< /tabs >}}

`transform_a` and `transform_b` both depend only on `fetch`, so the engine runs
them concurrently; `combine` waits for both. The DAG is validated when the
workflow is built — cycles and missing tasks are rejected.

## Next

- **[04 — Error handling & retries]({{< ref "/embed/tutorials" >}})**
- Explanation: [Trigger Rules]({{< ref "/workflows/explanation/trigger-rules" >}})
