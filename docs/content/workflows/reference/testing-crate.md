---
title: "cloacina-testing API Reference"
description: "API documentation for the cloacina-testing crate — no-database test utilities for Cloacina workflows"
weight: 30
---

# cloacina-testing API Reference

The `cloacina-testing` crate provides a lightweight, in-process test runner that executes tasks in dependency order without any database, scheduler, or background threads. It is designed for unit testing task logic.

## Overview

```rust
use cloacina_testing::{TestRunner, TestResult, TaskOutcome};
use cloacina_workflow::Context;
```

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
cloacina-testing = { path = "../crates/cloacina-testing" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## TestRunner

A no-DB, in-process task executor for unit tests. Runs tasks sequentially in topological (dependency) order.

```rust
pub struct TestRunner { /* private */ }

impl TestRunner {
    /// Create a new empty test runner.
    pub fn new() -> Self;

    /// Register a task. Returns `self` for chaining.
    pub fn register(self, task: Arc<dyn Task>) -> Self;

    /// Execute all registered tasks in dependency order.
    /// Context propagates from each task to the next.
    /// If a task fails, its transitive dependents are skipped.
    pub async fn run(
        &self,
        initial_context: Context<serde_json::Value>,
    ) -> Result<TestResult, TestRunnerError>;
}
```

### Behavior

- Tasks are executed **sequentially** in topological order
- **Context propagation**: each task receives the context produced by the previous task
- **Failure cascading**: if a task fails, all transitive dependents are marked `Skipped`
- **Unregistered dependencies** are silently ignored, allowing you to test subsets of a workflow
- **Cycle detection**: returns `TestRunnerError::CyclicDependency` if the dependency graph has cycles

## TestResult

The result of running tasks through a `TestRunner`.

```rust
pub struct TestResult {
    /// The final context after all tasks have executed.
    pub context: Context<serde_json::Value>,
    /// Per-task outcomes in execution order.
    pub task_outcomes: IndexMap<String, TaskOutcome>,
}
```

### Index Access

You can index a `TestResult` by task ID:

```rust
assert!(result["my_task"].is_completed());
```

Panics if the task ID is not found.

## TaskOutcome

The outcome of a single task execution.

```rust
pub enum TaskOutcome {
    Completed,
    Failed(TaskError),
    Skipped,
}

impl TaskOutcome {
    pub fn is_completed(&self) -> bool;
    pub fn is_failed(&self) -> bool;
    pub fn is_skipped(&self) -> bool;

    /// Returns the error if Failed, panics otherwise.
    pub fn unwrap_error(&self) -> &TaskError;
}
```

## Assertion Helpers

Convenience methods on `TestResult` for cleaner test assertions. All panic with descriptive messages on failure.

```rust
impl TestResult {
    /// Assert all tasks completed successfully.
    pub fn assert_all_completed(&self);

    /// Assert a specific task completed.
    pub fn assert_task_completed(&self, task_id: &str);

    /// Assert a specific task failed.
    pub fn assert_task_failed(&self, task_id: &str);

    /// Assert a specific task was skipped.
    pub fn assert_task_skipped(&self, task_id: &str);
}
```

## TestRunnerError

```rust
pub enum TestRunnerError {
    /// The dependency graph contains a cycle.
    CyclicDependency { cycle: Vec<String> },
}
```

## Continuous Feature (Feature-Gated)

The following types are available behind the `continuous` feature flag.

```toml
[dev-dependencies]
cloacina-testing = { path = "../crates/cloacina-testing", features = ["continuous"] }
```

### BoundaryEmitter

Simulates detector output for testing continuous/reactive tasks. Produces a `Context` matching the format an accumulator's `drain()` would produce.

```rust
pub struct BoundaryEmitter { /* private */ }

impl BoundaryEmitter {
    pub fn new() -> Self;

    /// Emit a time-range boundary.
    pub fn emit_time_range(self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self;

    /// Emit an offset-range boundary.
    pub fn emit_offset_range(self, start: i64, end: i64) -> Self;

    /// Emit a raw ComputationBoundary.
    pub fn emit(self, boundary: ComputationBoundary) -> Self;

    /// Convert emitted boundaries into a Context.
    pub fn into_context(self) -> Context<serde_json::Value>;
}
```

### ComputationBoundary

Placeholder type representing a slice of data to process.

```rust
pub enum ComputationBoundary {
    TimeRange { start: DateTime<Utc>, end: DateTime<Utc> },
    OffsetRange { start: i64, end: i64 },
}
```

### MockDataConnection

A mock data connection for testing tasks that interact with external data sources.

```rust
pub struct MockDataConnection<T: Any + Send + Sync + Clone> { /* private */ }

impl<T: Any + Send + Sync + Clone> MockDataConnection<T> {
    pub fn new(handle: T, descriptor: ConnectionDescriptor) -> Self;

    /// Get a clone of the underlying handle.
    pub fn connect(&self) -> T;

    /// Get the connection descriptor.
    pub fn descriptor(&self) -> &ConnectionDescriptor;

    /// Returns empty JSON object for mocks.
    pub fn system_metadata(&self) -> Value;
}
```

### ConnectionDescriptor

```rust
pub struct ConnectionDescriptor {
    pub system_type: String,
    pub location: String,
}
```
