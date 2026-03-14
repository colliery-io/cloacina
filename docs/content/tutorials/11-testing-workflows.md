---
title: "11 - Testing Your Workflows"
description: "Unit test your task logic without a database using cloacina-testing"
weight: 21
reviewer: "dstorey"
review_date: "2025-03-14"
---

## Overview

When developing Cloacina workflows, you need a fast way to verify your task logic without standing up a full database or runner infrastructure. The `cloacina-testing` crate provides an in-process test runner that executes tasks in dependency order, propagates context between them, and gives you clear assertion helpers for verifying outcomes.

No database. No scheduler. No background threads. Just your tasks and a context.

## Prerequisites

- A Rust project with `cloacina-workflow` for task definitions
- `tokio` with the `macros` and `rt-multi-thread` features for async tests

## Setup

Add `cloacina-testing` as a dev dependency in your `Cargo.toml`:

```toml
[dev-dependencies]
cloacina-testing = { version = "0.3" }
cloacina-workflow = { version = "0.3" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Writing Your First Test

Suppose you have two tasks: one that normalizes data and one that validates it.

```rust
use cloacina_workflow::*;

#[task(id = "normalize")]
async fn normalize(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let raw = context.get("raw_score").unwrap().as_f64().unwrap();
    context.insert("score", serde_json::json!(raw / 100.0))?;
    Ok(())
}

#[task(id = "validate", dependencies = ["normalize"])]
async fn validate(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let score = context.get("score").unwrap().as_f64().unwrap();
    context.insert("score_valid", serde_json::json!(score >= 0.0 && score <= 1.0))?;
    Ok(())
}
```

Test them with `TestRunner`:

```rust
use cloacina_testing::TestRunner;
use cloacina_workflow::Context;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_score_pipeline() {
    let mut ctx = Context::new();
    ctx.insert("raw_score", json!(85.0)).unwrap();

    let result = TestRunner::new()
        .register(Arc::new(normalize_task()))
        .register(Arc::new(validate_task()))
        .run(ctx)
        .await
        .unwrap();

    result.assert_all_completed();
    assert_eq!(result.context.get("score"), Some(&json!(0.85)));
    assert_eq!(result.context.get("score_valid"), Some(&json!(true)));
}
```

The `TestRunner`:
1. Builds a dependency graph from your registered tasks
2. Topologically sorts them (normalize runs before validate)
3. Executes each task, passing the context forward
4. Records the outcome of each task

## Testing Failure Scenarios

When a task fails, the runner records the failure and **skips all dependent tasks**:

```rust
#[tokio::test]
async fn test_failure_skips_dependents() {
    // Don't provide 'raw_score' — normalize will fail
    let result = TestRunner::new()
        .register(Arc::new(normalize_task()))
        .register(Arc::new(validate_task()))
        .run(Context::new())
        .await
        .unwrap();

    result.assert_task_failed("normalize");
    result.assert_task_skipped("validate");
}
```

Independent tasks on separate branches continue executing even when one branch fails:

```rust
#[tokio::test]
async fn test_independent_branch_continues() {
    let result = TestRunner::new()
        .register(Arc::new(failing_task()))   // fails
        .register(Arc::new(dependent_task())) // skipped (depends on failing_task)
        .register(Arc::new(standalone_task())) // succeeds (no dependency on failing_task)
        .run(Context::new())
        .await
        .unwrap();

    result.assert_task_failed("failing");
    result.assert_task_skipped("dependent");
    result.assert_task_completed("standalone");
}
```

## Using Assertion Helpers

`TestResult` provides several assertion methods:

| Method | Purpose |
|--------|---------|
| `assert_all_completed()` | Panics if any task is not `Completed` |
| `assert_task_completed(id)` | Panics if the specific task is not `Completed` |
| `assert_task_failed(id)` | Panics if the specific task is not `Failed` |
| `assert_task_skipped(id)` | Panics if the specific task is not `Skipped` |

You can also index directly into the result:

```rust
assert!(result["normalize"].is_completed());
assert!(result["validate"].is_failed());

// Get the error from a failed task
let error = result["validate"].unwrap_error();
```

Panic messages are developer-friendly:

```text
assertion failed: expected task 'validate' to be Completed, but was Failed(Task execution failed: missing field)
```

## Cycle Detection

If your tasks have circular dependencies, the runner returns an error before executing anything:

```rust
#[tokio::test]
async fn test_cycle_detected() {
    let result = TestRunner::new()
        .register(Arc::new(task_a())) // depends on task_b
        .register(Arc::new(task_b())) // depends on task_a
        .run(Context::new())
        .await;

    assert!(result.is_err());
}
```

## Key Behaviors

- **Context propagation**: Each task receives the context produced by the previous task in execution order
- **No retries**: Tasks are executed exactly once, with no retry logic
- **No timeouts**: Tests run to completion (or failure) without time limits
- **Unregistered dependencies**: If a task declares a dependency that isn't registered, it is silently ignored. This allows testing subsets of a workflow.
- **Deterministic**: No concurrency, no background threads. Same inputs always produce the same outputs.

## Next Steps

- For integration testing with a real database, use the `DefaultRunner` with a SQLite connection
- For testing continuous/reactive tasks, enable the `continuous` feature flag for `BoundaryEmitter` and `MockDataConnection`
