---
title: "04 — Error handling & retries"
description: "Fail a task, and configure automatic retries with backoff."
weight: 14
---

# 04 — Error handling & retries

A task that returns an error (Rust) or raises (Python) is marked failed. Configure
**retries** on the task to recover from transient failures.

## A task that retries

{{< tabs "t04" >}}
{{< tab "Rust" >}}
```rust
#[task(
    id = "flaky",
    dependencies = [],
    retry_attempts = 3,
    retry_backoff = "exponential",
    retry_delay_ms = 500,
)]
pub async fn flaky(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // return Err(TaskError::...) to fail; the runner retries per the policy
    Ok(())
}
```
{{< /tab >}}
{{< tab "Python" >}}
```python
@cloaca.task(
    id="flaky",
    retry_attempts=3,
    retry_backoff="exponential",
    retry_delay_ms=500,
)
def flaky(context):
    # raise to fail; the runner retries per the policy
    return context
```
{{< /tab >}}
{{< /tabs >}}

Retries back off between attempts (`fixed` / `linear` / `exponential`). Use
`retry_condition` to retry only `transient` errors. Because execution is
**at-least-once**, a task may run more than once — keep it idempotent.

## What you learned (the track)

You can now define tasks, pass data, express a DAG, and handle failure — embedded
in your own process. To take it further:

- **Going to production embedded?** → [Running embedded in production]({{< ref "/embed/how-to" >}})
- **Event-driven, in-process?** → [Computation Graphs]({{< ref "/engine/computation-graphs/computation-graph" >}})
- Retry patterns: [Conditional Retries]({{< ref "/embed/how-to/conditional-retries" >}})
