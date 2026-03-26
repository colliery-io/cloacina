---
title: "Testing Workflows"
description: "How to unit test Cloacina workflow logic without a database"
weight: 10
---

# Testing Workflows

This guide shows how to use `cloacina-testing` to unit test your workflow task logic without a database, scheduler, or background threads.

## Prerequisites

- Cloacina added to your project
- Basic understanding of the `Task` trait and `Context`

## Setup

Add `cloacina-testing` as a dev dependency:

```toml
[dev-dependencies]
cloacina-testing = { path = "../crates/cloacina-testing" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Testing a Simple Pipeline

Register your tasks with `TestRunner` and call `run()`. Tasks execute in dependency order with context flowing through.

```rust
use cloacina_testing::TestRunner;
use cloacina_workflow::Context;
use std::sync::Arc;

#[tokio::test]
async fn test_my_pipeline() {
    let result = TestRunner::new()
        .register(Arc::new(FetchDataTask))
        .register(Arc::new(TransformTask))
        .register(Arc::new(LoadTask))
        .run(Context::new())
        .await
        .unwrap();

    result.assert_all_completed();
}
```

## Verifying Context Output

The `TestResult` contains the final context after all tasks have run. Use it to verify your tasks produced the expected data.

```rust
use serde_json::json;

#[tokio::test]
async fn test_task_produces_expected_output() {
    let result = TestRunner::new()
        .register(Arc::new(ComputeMetricsTask))
        .run(Context::new())
        .await
        .unwrap();

    result.assert_all_completed();
    assert_eq!(result.context.get("total_count"), Some(&json!(42)));
}
```

## Testing Failure Cascading

When a task fails, all of its transitive dependents are automatically skipped. Independent branches continue to execute.

```rust
#[tokio::test]
async fn test_failure_skips_dependents_but_not_siblings() {
    // Pipeline:  fetch (fails) -> transform (skipped)
    //            validate (independent, succeeds)
    let result = TestRunner::new()
        .register(Arc::new(FailingFetchTask))
        .register(Arc::new(TransformTask))       // depends on fetch
        .register(Arc::new(ValidateConfigTask))   // independent
        .run(Context::new())
        .await
        .unwrap();

    result.assert_task_failed("fetch");
    result.assert_task_skipped("transform");
    result.assert_task_completed("validate_config");
}
```

## Testing Dependency Ordering

The runner resolves dependencies via topological sort. You can test diamond-shaped and complex dependency graphs.

```rust
#[tokio::test]
async fn test_diamond_dependency() {
    // A -> B, A -> C, B+C -> D
    let result = TestRunner::new()
        .register(Arc::new(TaskA))
        .register(Arc::new(TaskB))  // depends on A
        .register(Arc::new(TaskC))  // depends on A
        .register(Arc::new(TaskD))  // depends on B and C
        .run(Context::new())
        .await
        .unwrap();

    result.assert_all_completed();
}
```

## Cycle Detection

If your dependency graph has a cycle, `run()` returns an error instead of deadlocking.

```rust
#[tokio::test]
async fn test_cycle_is_rejected() {
    let result = TestRunner::new()
        .register(Arc::new(TaskX))  // depends on Y
        .register(Arc::new(TaskY))  // depends on X
        .run(Context::new())
        .await;

    assert!(result.is_err());
}
```

## Testing Subsets of a Workflow

You don't have to register every task in a workflow. Unregistered dependencies are silently ignored, so you can test individual tasks or subgraphs in isolation.

```rust
#[tokio::test]
async fn test_single_task_in_isolation() {
    // TransformTask depends on FetchTask, but we can test it alone
    // by providing the expected context directly
    let mut ctx = Context::new();
    let _ = ctx.insert("raw_data", json!({"rows": [1, 2, 3]}));

    let result = TestRunner::new()
        .register(Arc::new(TransformTask))
        .run(ctx)
        .await
        .unwrap();

    result.assert_all_completed();
    assert!(result.context.get("transformed_data").is_some());
}
```

## Inspecting Individual Task Outcomes

Use index access or the `task_outcomes` map to inspect specific tasks.

```rust
#[tokio::test]
async fn test_inspect_outcomes() {
    let result = TestRunner::new()
        .register(Arc::new(MyTask))
        .run(Context::new())
        .await
        .unwrap();

    // Index access
    assert!(result["my_task"].is_completed());

    // Or iterate
    for (task_id, outcome) in &result.task_outcomes {
        println!("{}: {}", task_id, outcome);
    }
}
```

## Testing with BoundaryEmitter (Continuous Feature)

If you're testing continuous/reactive tasks, enable the `continuous` feature and use `BoundaryEmitter` to simulate detector output.

```toml
[dev-dependencies]
cloacina-testing = { path = "../crates/cloacina-testing", features = ["continuous"] }
```

```rust
use cloacina_testing::BoundaryEmitter;
use chrono::Utc;

#[tokio::test]
async fn test_continuous_task_with_boundaries() {
    let ctx = BoundaryEmitter::new()
        .emit_time_range(
            Utc::now() - chrono::Duration::hours(1),
            Utc::now(),
        )
        .into_context();

    let result = TestRunner::new()
        .register(Arc::new(ProcessBoundaryTask))
        .run(ctx)
        .await
        .unwrap();

    result.assert_all_completed();
}
```

## Summary

| Want to... | Use |
|---|---|
| Run tasks in dependency order | `TestRunner::new().register(...).run(ctx)` |
| Assert all tasks passed | `result.assert_all_completed()` |
| Check a specific task | `result.assert_task_completed("id")` / `result["id"].is_completed()` |
| Verify output data | `result.context.get("key")` |
| Test failure cascading | `assert_task_failed` + `assert_task_skipped` |
| Test in isolation | Provide pre-built context, register only the task under test |
| Simulate continuous boundaries | `BoundaryEmitter::new().emit_time_range(...).into_context()` |
