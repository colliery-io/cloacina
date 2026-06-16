---
title: "Conditional Retries"
description: "Use retry_condition to retry transient failures but skip non-recoverable ones"
weight: 25
aliases:
  - "/workflows/how-to-guides/conditional-retries/"

---

# Conditional Retries

By default, `#[task]` retries every failure up to `retry_attempts`.
That's the right call for transient errors (network, lock contention)
and the wrong call for permanent ones (validation, permissions). The
`retry_condition` attribute on the task macro lets you decide which.

The retry-condition machinery has shipped since the original retry-policy
work; this guide pins the vocabulary, shows the supported patterns, and
points at the matching examples.

## Vocabulary

`retry_condition` accepts a string literal. Four forms:

| value | meaning |
|-------|---------|
| `"all"` (or omitted) | Retry on every error. Default behavior. |
| `"never"` | Never retry, regardless of `retry_attempts`. |
| `"transient"` | Retry only on transient-flavored errors (timeout, "connection", "network", "temporary", "unavailable", "busy", "overloaded", "rate limit"). |
| `"foo,bar"` (comma list) | Retry only when the error message contains any of the listed substrings (case-insensitive). |

The transient matcher is intentionally string-based on the error's
`Display` impl. That keeps the policy serializable for packaged
workflows — no closure or trait-object plumbing.

## Rust

```rust
#[task(
    id = "flaky_api_call",
    dependencies = [],
    retry_attempts = 3,
    retry_delay_ms = 100,
    retry_condition = "transient",
)]
async fn flaky_api_call(context: &mut Context<Value>) -> Result<(), TaskError> {
    // ... returning an error whose message contains "connection refused"
    // will be retried; a "validation failed" error will not.
}

#[task(
    id = "validation_check",
    dependencies = [],
    retry_attempts = 3,
    retry_condition = "never",
)]
async fn validation_check(context: &mut Context<Value>) -> Result<(), TaskError> {
    // No matter what the error is, this task will fail after a single
    // attempt. `retry_attempts = 3` is a no-op here.
}
```

A runnable end-to-end example lives at
[`examples/features/workflows/conditional-retries`](https://github.com/colliery-io/cloacina/tree/main/examples/features/workflows/conditional-retries).

## Python

```python
import cloaca

with cloaca.WorkflowBuilder("flaky_pipeline") as builder:
    @cloaca.task(
        id="flaky_api_call",
        retry_attempts=3,
        retry_delay_ms=100,
        retry_condition="transient",
    )
    def flaky_api_call(context):
        raise RuntimeError("connection refused (simulated)")  # retried

    @cloaca.task(
        id="validation_check",
        retry_attempts=3,
        retry_condition="never",
    )
    def validation_check(context):
        raise RuntimeError("invalid input")  # not retried
```

End-to-end coverage:
[`tests/python/test_scenario_33_retry_condition.py`](https://github.com/colliery-io/cloacina/blob/main/tests/python/test_scenario_33_retry_condition.py).

## Custom substring patterns

When the predefined sets don't fit, use a comma-separated pattern list:

```rust
#[task(
    id = "scrape_endpoint",
    retry_attempts = 5,
    retry_condition = "rate limit,5xx,deadline exceeded",
)]
```

Each pattern is matched as a case-insensitive substring against the
error's `Display` impl. If any pattern matches, the task retries.

## How it interacts with retry_attempts

`retry_attempts` is still the upper bound — `retry_condition` only
decides whether to schedule the next attempt within that budget.

```text
attempt failed
  │
  ├── attempt < retry_attempts? ── no ──► Failed
  │
  └── yes
        │
        └── retry_condition.should_retry(error)? ── no ──► Failed
                │
                └── yes ──► schedule retry with backoff
```

So `retry_attempts = 3, retry_condition = "never"` collapses to "fail
on first error." `retry_attempts = 1, retry_condition = "all"` means
"retry once on any error" — same as before.

## See also

- [`retry_attempts`, `retry_delay_ms`, `retry_backoff` reference]({{< ref "/reference/macros" >}}) — the rest of the retry-policy knobs
- [`cloacina_workflow::RetryPolicy`](https://docs.rs/cloacina-workflow) — the underlying policy struct
- Source: [`crates/cloacina-workflow/src/retry.rs`](https://github.com/colliery-io/cloacina/blob/main/crates/cloacina-workflow/src/retry.rs)
