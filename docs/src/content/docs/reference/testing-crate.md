---
title: "cloacina-testing API Reference"
description: "API documentation for the cloacina-testing crate — no-database test utilities for Cloacina workflows"
sidebar:
  order: 30
---

The `cloacina-testing` crate provides a lightweight, in-process test runner that executes tasks in dependency order without any database, scheduler, or background threads. It is designed for unit testing task logic.

## Overview

```rust
use cloacina_testing::{TestRunner, TestResult, TaskOutcome};
use cloacina_workflow::Context;
```

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
cloacina-testing = "0.12"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

(Inside the cloacina repo itself, workspace members use a `path`
dependency instead.)

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

## Boundary and Mock Utilities

The following types ship unconditionally (the crate defines no cargo
features). `BoundaryEmitter` and `MockDataConnection` are re-exported
at the crate root; `ComputationBoundary` and `ConnectionDescriptor`
live at `cloacina_testing::boundary::ComputationBoundary` and
`cloacina_testing::mock::ConnectionDescriptor`.

### BoundaryEmitter

Simulates detector output for testing continuous/event-driven tasks. Produces a `Context` matching the format an accumulator's `drain()` would produce.

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

## Sync Connector Testing

`cloacina_testing::connectors` builds Sync connectors and tests them
(CLOACINA-T-0979, CLOACINA-T-0983, CLOACINA-T-0986).

### Building and staging

```rust
/// Build a guest crate to a release wasm32-wasip2 component; once per process.
pub fn build_component(crate_dir: &Path) -> Vec<u8>;

/// Build the crate and stage it under `root` as a provider package named
/// after the crate, with a `source` and a `destination` member.
pub fn stage_connector(root: &Path, crate_dir: &Path) -> PathBuf;
```

### The conformance kit

`connectors::conformance` runs a connector's conformance file. The file
format and how to run it are in
[Test a Connector with the Conformance Kit](../../how-to/sync/test-a-connector/).

```rust
/// Run the file at `path`; `${NAME}`s come from the environment.
pub fn run(path: &Path, host: &dyn Host, service: Option<&dyn Service>) -> Report;

/// Run the file with `vars` for its `${NAME}`s first.
pub fn run_with(
    path: &Path,
    vars: &BTreeMap<String, String>,
    host: &dyn Host,
    service: Option<&dyn Service>,
) -> Report;

/// Stages and loads connectors. The Sync crate's tests wrap the real
/// connector host.
pub trait Host {
    fn prepare(&self, crate_dir: &Path) -> Result<String, String>;
    fn load(
        &self,
        package: &str,
        member: &str,
        config: &Config,
        grants: Option<&Value>,
        credential: Option<&Credential>,
    ) -> Result<Box<dyn Connector>, String>;
}

/// Runs a file's setup and verify steps against the test service.
pub trait Service {
    fn run(&self, step: &Map<String, Value>) -> Result<Vec<Value>, String>;
}
```

`Report::assert_passed` panics with every failure; `Report::failures` lists
them, each with its `method`, `case` and `difference`. The kit does not
depend on the engine, so it publishes before `cloacina`.

### The mock JSON API

`connectors::rest_mock::RestMock` is a mock JSON API on a free local port, for
the REST runtime's tests (CLOACINA-T-0990). An endpoint serves records paged by
page number, offset, next-page token or `Link` header; it can wrap them under a
key, filter them by an incremental parameter, require a header, answer `429`
first, or answer a fixed body. A sink takes writes under a path prefix and can
refuse one by a body field. Every request is kept, with its parameters
(query and POST body), headers and body.

It is also a `Service`: `endpoint` steps configure an endpoint, `sink` steps
add a sink, and `requests_to` and `requests_under` steps return what was sent.

```rust
let mock = RestMock::start();
mock.serve("/user/repos", Endpoint {
    records,
    paging: Paging::Page { page_param: "page".into(), size_param: "per_page".into() },
    require_header: Some(("Authorization".into(), "Bearer t".into())),
    ..Default::default()
});
let sent = mock.requests_to("/user/repos");
```

A conformance read case can also `resume_after` a checkpoint, and a write case
can send CDC `change_batches`.

### The mock Snowflake SQL API

`connectors::snowflake_mock::SnowflakeMock` is an in-memory SQL API on a free
local port. It runs the statements the Snowflake connector builds, can answer
`202` before settling and split results into partitions, and checks the
key-pair JWT, with the key's fingerprint when given. It is also a `Service`:
a `table` step with `columns` and `data` loads a table, and `rows_of` returns
one.

```rust
let mock = SnowflakeMock::start(MockOptions {
    fingerprint: Some("SHA256:...".into()),
    asynchronous: true,
    partition_rows: 5,
    ..MockOptions::new("testorg-sync", "sync_test")
});
mock.load_table("ORDERS", &["ID", "NAME"], rows);
let written = mock.rows("SF_DST");
let refused = mock.rejected();
```
