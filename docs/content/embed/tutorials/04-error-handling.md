---
title: "04 — Error handling & retries"
description: "Fail a task, and configure automatic retries with backoff."
weight: 14
aliases:
  - "/python/workflows/tutorials/04-error-handling/"
  - "/workflows/tutorials/library/04-error-handling/"

---

# 04 — Error handling & retries

A task that returns an error (Rust) or raises (Python) is marked failed. Configure
**retries** on the task to recover from transient failures.

## A task that retries

{{< tabs "t04" >}}
{{< tab "Rust" >}}
```rust
#[task(
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

## What you learned

You can now define tasks, pass data, express a DAG, and handle failure — embedded
in your own process. That's the foundation; the rest of the track builds on it.

## Next

- **[05 — Cron scheduling]({{< ref "/embed/tutorials/05-cron-scheduling" >}})** — run a workflow on a schedule.
- Retry patterns: [Conditional Retries]({{< ref "/embed/how-to/conditional-retries" >}})

The track continues through scheduling, multi-tenancy, triggers, deferral, and the
registry (05–09), then computation graphs (10–13). The
[tutorials index]({{< ref "/embed/tutorials" >}}) lists the full sequence.
