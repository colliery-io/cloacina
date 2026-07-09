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

To *see* retries fire, the task has to actually fail. Here `flaky` fails its first
two attempts and succeeds on the third — so with `retry_attempts = 3` the workflow
still completes. A module-level counter tracks the attempt across retries (the task
function takes no `self`, so per-task state lives in a static / module global).

{{< tabs "t04" >}}
{{< tab "Rust" >}}
```rust
use std::sync::atomic::{AtomicU32, Ordering};

// Attempt counter shared across retries of `flaky`.
static ATTEMPTS: AtomicU32 = AtomicU32::new(0);

#[task(
    retry_attempts = 3,
    retry_backoff = "exponential",
    retry_delay_ms = 500,
)]
pub async fn flaky(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let attempt = ATTEMPTS.fetch_add(1, Ordering::SeqCst) + 1;
    if attempt < 3 {
        // Fail the first two attempts; the runner retries per the policy.
        return Err(TaskError::ExecutionFailed {
            message: format!("transient failure (attempt {attempt})"),
            task_id: "flaky".to_string(),
            timestamp: chrono::Utc::now(),
        });
    }
    info!("flaky succeeded on attempt {attempt}");
    Ok(())
}
```
{{< /tab >}}
{{< tab "Python" >}}
```python
# Attempt counter shared across retries of `flaky`.
_attempts = {"flaky": 0}

@cloaca.task(
    retry_attempts=3,
    retry_backoff="exponential",
    retry_delay_ms=500,
)
def flaky(context):
    _attempts["flaky"] += 1
    attempt = _attempts["flaky"]
    if attempt < 3:
        # Fail the first two attempts; the runner retries per the policy.
        raise RuntimeError(f"transient failure (attempt {attempt})")
    print(f"flaky succeeded on attempt {attempt}")
    return context
```
{{< /tab >}}
{{< /tabs >}}

Run it and the runner logs a scheduled retry after each failure, then the success
on the third attempt:

```
INFO  Scheduled retry for task flaky in 500ms (attempt 2)
INFO  Scheduled retry for task flaky in 1s (attempt 3)
INFO  flaky succeeded on attempt 3
```

The delay grows between attempts because the backoff is `exponential` (the exact
values include a little jitter). Without the counter — a task body that always
returns `Ok(())` — nothing fails, so you'd never see these lines.

Retries back off between attempts (`fixed` / `linear` / `exponential`). By default
a task retries on **any** error; set `retry_condition = "transient"` to retry only
transient failures (timeouts, connection/network errors). Because execution is
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
