---
id: cloacina-testing-crate-public-test
level: initiative
title: "cloacina-testing Crate — Public Test Infrastructure for Consumers"
short_code: "CLOACI-I-0027"
created_at: 2026-03-13T02:48:48.225839+00:00
updated_at: 2026-03-13T02:48:48.225839+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: cloacina-testing-crate-public-test
---

# cloacina-testing Crate — Public Test Infrastructure for Consumers

## Context

Consumers who want to test their task logic today are forced to either bring up a full `DefaultRunner` (which requires a live database) or reverse-engineer the internal `TestFixture` pattern from `tests/fixtures.rs`, which is private to the library. There is no public `MockTask`, no in-process test runner, nothing.

This is a significant gap for adoption. Users writing Cloacina workflows need to be able to unit test their task logic without standing up infrastructure. This also becomes critical for the continuous scheduling initiatives (CLOACI-I-0023 through I-0026), where testing boundary emissions and accumulator behavior should be possible without a live Postgres instance.

**No dependencies** — this initiative can proceed independently of the continuous scheduling work and delivers immediate value to existing users.

## Goals & Non-Goals

**Goals:**
- Ship a `cloacina-testing` crate (separate workspace member) for consumer-facing test utilities
- Implement `TestRunner` — a no-DB, in-process task executor that runs tasks sequentially in dependency order
- Implement `TestResult` with per-task outcomes and final context
- Support existing `#[task]` macro tasks without modification
- Support future `#[continuous_task]` tasks with simulated `DataSourceMap` and boundary emissions
- Provide `MockDataConnection` for testing continuous tasks without real external systems
- Provide `BoundaryEmitter` for simulating detector output in tests
- Clean, minimal API — test infrastructure should be as simple as the example in the problem statement

**Non-Goals:**
- Replacing the existing integration test infrastructure (full runner + DB tests remain valuable)
- Mocking the database layer (this is about task logic testing, not DAL testing)
- Performance testing utilities (separate concern)
- Python/Cloaca test utilities (follow-on, after Python continuous task support lands)

## Detailed Design

### TestRunner — Core

A no-DB, in-process task executor for unit tests. Runs tasks sequentially in dependency order without any scheduler, dispatcher, or database.

```rust
// cloacina-testing/src/lib.rs

/// A no-DB, in-process task executor for unit tests.
/// Runs tasks sequentially in dependency order without any scheduler or DB.
pub struct TestRunner {
    tasks: IndexMap<String, Arc<dyn Task>>,
}

impl TestRunner {
    pub fn new() -> Self { ... }

    pub fn register(mut self, task: Arc<dyn Task>) -> Self { ... }

    /// Execute tasks in topological order, returning the final context.
    /// No DB, no retry machinery, no background threads.
    pub async fn run(
        &self,
        initial_context: Context<serde_json::Value>,
    ) -> TestResult { ... }
}

pub struct TestResult {
    pub context: Context<serde_json::Value>,
    pub task_outcomes: IndexMap<String, TaskOutcome>,
}

pub enum TaskOutcome {
    Completed,
    Failed(TaskError),
    Skipped,
}
```

Consumer test:

```rust
#[tokio::test]
async fn test_my_pipeline() {
    let result = TestRunner::new()
        .register(Arc::new(NormalizeTask))
        .register(Arc::new(ValidateTask))
        .run(Context::new())
        .await;

    assert!(result.task_outcomes["validate"].is_completed());
    assert_eq!(result.context.get("score_valid"), Some(&json!(true)));
}
```

### TestRunner Internals

1. Collect registered tasks
2. Build a `DependencyGraph` from task dependency declarations (reuse existing graph infrastructure)
3. Topological sort
4. Execute sequentially — pass context from upstream tasks to downstream
5. For each task: call the task function directly, capture result, record outcome
6. On failure: record `TaskOutcome::Failed`, skip dependents (mark as `Skipped`)
7. Return `TestResult` with final context and all outcomes

No retries, no timeouts, no concurrency — this is deterministic task logic testing.

### Continuous Task Testing — BoundaryEmitter & MockDataConnection

For testing `#[continuous_task]` functions (once CLOACI-I-0023 lands):

```rust
/// Simulates detector output for testing continuous tasks.
pub struct BoundaryEmitter {
    boundaries: Vec<ComputationBoundary>,
}

impl BoundaryEmitter {
    pub fn new() -> Self { ... }
    pub fn emit(mut self, boundary: ComputationBoundary) -> Self { ... }
    pub fn emit_time_range(mut self, start: DateTime, end: DateTime) -> Self { ... }
    pub fn emit_offset_range(mut self, start: i64, end: i64) -> Self { ... }

    /// Build the context that an accumulator's drain() would produce.
    pub fn into_context(self) -> Context<serde_json::Value> { ... }
}

/// A mock DataConnection for tests — returns a user-provided value from connect().
pub struct MockDataConnection<T: Any + Send + Sync> {
    handle: T,
    descriptor: ConnectionDescriptor,
}

impl<T: Any + Send + Sync> DataConnection for MockDataConnection<T> {
    fn connect(&self) -> Result<Box<dyn Any>> { Ok(Box::new(self.handle.clone())) }
    fn descriptor(&self) -> ConnectionDescriptor { self.descriptor.clone() }
    fn system_metadata(&self) -> Value { json!({}) }
}
```

Consumer test for a continuous task:

```rust
#[tokio::test]
async fn test_aggregate_hourly() {
    let mock_pool = setup_test_db(); // user's test DB or mock

    let mut inputs = DataSourceMap::new();
    inputs.insert("raw_events", MockDataConnection::new(
        mock_pool.clone(),
        ConnectionDescriptor { system_type: "postgres".into(), location: "test".into() },
    ));

    let ctx = BoundaryEmitter::new()
        .emit_time_range(hour_ago, now)
        .into_context();

    let result = aggregate_hourly(&mut ctx, &inputs).await;
    assert!(result.is_ok());
    assert_eq!(ctx.get("rows_processed"), Some(&json!(42)));
}
```

### Assertion Helpers

Convenience methods on `TestResult` and `TaskOutcome`:

```rust
impl TestResult {
    pub fn assert_all_completed(&self);
    pub fn assert_task_completed(&self, task_id: &str);
    pub fn assert_task_failed(&self, task_id: &str);
    pub fn assert_task_skipped(&self, task_id: &str);
}

impl TaskOutcome {
    pub fn is_completed(&self) -> bool;
    pub fn is_failed(&self) -> bool;
    pub fn is_skipped(&self) -> bool;
    pub fn unwrap_error(&self) -> &TaskError;
}
```

### Crate Structure

```
crates/
  cloacina-testing/
    Cargo.toml          # depends on cloacina (for Task trait, Context, etc.)
    src/
      lib.rs            # TestRunner, TestResult, TaskOutcome
      boundary.rs       # BoundaryEmitter (feature-gated on "continuous")
      mock.rs           # MockDataConnection (feature-gated on "continuous")
      assertions.rs     # Assertion helpers
```

Feature flags:
- Default: `TestRunner`, `TestResult`, `TaskOutcome`, assertion helpers
- `continuous`: `BoundaryEmitter`, `MockDataConnection` (available once I-0023 lands)

## Alternatives Considered

- **Feature flag on `cloacina` crate instead of separate crate**: Rejected — test utilities shouldn't bloat the main crate. Separate crate keeps the dependency clean and signals "this is for your `[dev-dependencies]`."
- **Full mock runner with mock DB**: Rejected — over-engineered. Users testing task logic don't need a mock database. They need to call their function with a context and check the result. Integration tests with a real DB are a separate concern.
- **Expose internal `TestFixture`**: Rejected — `TestFixture` is tightly coupled to internal test patterns. A public API should be purpose-built for consumers with a stable contract.

## Implementation Plan

### Phase 1: TestRunner Core
- [ ] Create `cloacina-testing` crate in workspace
- [ ] `TestRunner` struct with `register()` and `run()`
- [ ] `TestResult` and `TaskOutcome` types
- [ ] Dependency graph construction from registered tasks
- [ ] Topological execution with context propagation
- [ ] Failure handling: record outcome, skip dependents
- [ ] Unit tests for TestRunner itself

### Phase 2: Assertion Helpers
- [ ] `TestResult` assertion methods
- [ ] `TaskOutcome` convenience methods
- [ ] Clear panic messages for assertion failures

### Phase 3: Continuous Task Testing (feature-gated)
- [ ] `BoundaryEmitter` with builder API
- [ ] `MockDataConnection` generic impl
- [ ] `DataSourceMap` test construction helpers
- [ ] Feature-gate behind `continuous` flag
- [ ] Tests for boundary emission and mock connections

### Phase 4: Documentation & Examples
- [ ] Crate-level documentation with usage examples
- [ ] Example: testing a simple workflow pipeline
- [ ] Example: testing a continuous task with mocked data source
- [ ] Add to tutorials or "testing your workflows" guide
