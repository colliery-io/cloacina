# API Design Review

## Summary

Cloacina exposes a well-structured set of interfaces across Rust, Python, HTTP, and CLI surfaces, with generally consistent patterns within each layer. The Rust API is ergonomic, the builder pattern is used consistently, and the prelude provides a clean entry point. However, significant cross-layer inconsistencies erode the API's coherence: the "pipeline" vs "workflow" naming collision leaks into user-facing JSON and Python config properties; the Python API has stub methods that raise at runtime; the HTTP API path structure documented in the system overview differs from the actual tenant-scoped routes; and the `DefaultRunnerConfig` has a dangerously large surface area (28 fields) with some builder validation that panics rather than returning errors.

## Interface Inventory

| Interface | Type | Primary Consumers |
|-----------|------|-------------------|
| `cloacina::prelude::*` | Rust library API | Application developers |
| `#[task]`, `#[workflow]`, `#[trigger]`, `#[computation_graph]` | Procedural macros | Application developers |
| `DefaultRunner` / `DefaultRunnerBuilder` | Rust runner API | Application developers |
| `DefaultRunnerConfig` / `DefaultRunnerConfigBuilder` | Rust config API | Application developers |
| `Dispatcher` / `TaskExecutor` traits | Rust extension traits | Plugin authors |
| `cloaca` Python wheel (`Context`, `DefaultRunner`, etc.) | Python bindings | Python developers |
| `cloacinactl daemon|serve|config|admin` | CLI | Operators |
| `/v1/*` HTTP REST API | HTTP service | External clients |
| `/v1/ws/*` WebSocket endpoints | WebSocket API | Real-time clients |
| `~/.cloacina/config.toml` | Configuration file | Operators |
| Error types (`TaskError`, `WorkflowExecutionError`, etc.) | Error hierarchy | All consumers |

## Consistency Assessment

**Within-layer consistency (Good):**
- Rust API consistently uses builder patterns, async/await, and Result types
- Python API consistently uses kwargs with defaults and property getters/setters
- HTTP API uses consistent JSON error format with `error`/`code` fields
- CLI uses Clap-derived commands with consistent flag naming

**Cross-layer consistency (Poor):**
- "Pipeline" terminology leaks into Python config (`pipeline_timeout_seconds`), HTTP response JSON (`pipeline_name` in internal model), and Rust error messages
- Rust uses `Duration`, Python uses seconds/milliseconds integers, HTTP has no explicit timeout config -- three different time representations with no bridging documentation
- Python `DefaultRunner.start()` and `.stop()` raise `NotImplementedError`-equivalent at runtime, while Rust `DefaultRunner` auto-starts on construction
- HTTP API returns tenant-scoped `execution_id` as string, Rust API returns `Uuid` -- format mismatch for cross-layer usage

## Findings

## API-001: Python runner exposes non-functional `start()` and `stop()` methods
**Severity**: Major
**Location**: `crates/cloacina/src/python/bindings/runner.rs` lines 1411-1428
**Confidence**: High

### Description
`PyDefaultRunner` exposes public `start()` and `stop()` methods that unconditionally raise `ValueError` with the message "Runner startup requires async runtime support. This will be implemented in a future update." These methods appear in the Python API as callable methods, inviting users to call them. Discovering they are stubs requires actually invoking them at runtime.

This violates the principle of least surprise: a method that exists should work, or should not exist. The Rust `DefaultRunner` handles start/stop internally (auto-starts in constructor, explicit `shutdown()`), making these Python stubs additionally confusing since the actual lifecycle is different.

### Evidence
```rust
pub fn start(&self) -> PyResult<()> {
    Err(PyValueError::new_err(
        "Runner startup requires async runtime support. \
         This will be implemented in a future update.",
    ))
}

pub fn stop(&self) -> PyResult<()> {
    Err(PyValueError::new_err(
        "Runner shutdown requires async runtime support. \
         This will be implemented in a future update.",
    ))
}
```

Meanwhile, `shutdown()` at line 1438 works correctly. Users must discover that `shutdown()` is the right method through trial and error.

### Suggested Resolution
Remove `start()` and `stop()` entirely. The Python `DefaultRunner` already auto-starts on construction (matching the Rust behavior) and has a working `shutdown()` method. If these are planned for a future release, remove them until they work -- placeholder methods that raise are worse than absent methods.

---

## API-002: "pipeline" terminology exposed in Python config API
**Severity**: Major
**Location**: `crates/cloacina/src/python/bindings/context.rs` lines 38, 55, 80-81, 144-145, 215-217, 283-284
**Confidence**: High

### Description
The Python `DefaultRunnerConfig` class exposes `pipeline_timeout_seconds` as a constructor parameter, getter, and setter. The public API uses "workflow" everywhere else (`workflow_name`, `execute("workflow_name")`), but this config property uses "pipeline". A Python user who reads "pipeline_timeout_seconds" will wonder if this is a different concept from a workflow.

This is a cross-layer manifestation of LEG-001 (pipeline vs workflow naming collision), but uniquely harmful here because it surfaces directly in the Python user-facing API.

### Evidence
```python
# Constructor parameter:
config = DefaultRunnerConfig(pipeline_timeout_seconds=3600)

# Getter:
config.pipeline_timeout_seconds  # 3600

# Setter:
config.pipeline_timeout_seconds = 7200
```

The Rust `DefaultRunnerConfig` also uses `pipeline_timeout` internally (line 64 of config.rs), but the Rust API is less exposed since config fields are private behind getter methods.

### Suggested Resolution
Rename to `workflow_timeout_seconds` in the Python bindings. The Rust side can alias `pipeline_timeout()` to `workflow_timeout()` (deprecating the former) to maintain backward compatibility. The Python bindings should present the consistent "workflow" terminology to users who will never see the internal database layer.

---

## API-003: `DefaultRunnerConfigBuilder.build()` panics instead of returning `Result`
**Severity**: Major
**Location**: `crates/cloacina/src/runner/default_runner/config.rs` lines 461-474
**Confidence**: High

### Description
The `DefaultRunnerConfigBuilder::build()` method uses `assert!` macros for validation, causing panics on invalid configuration. Three conditions trigger panics:
1. `max_concurrent_tasks == 0`
2. `db_pool_size == 0`
3. `stale_claim_threshold <= heartbeat_interval`

In a library API, panics are inappropriate for input validation. A user who constructs a config with `max_concurrent_tasks(0)` will get a panic with no recovery path, potentially crashing their application. This is a standard Rust API antipattern -- builders should return `Result<Config, ConfigError>`.

### Evidence
```rust
pub fn build(self) -> DefaultRunnerConfig {
    assert!(
        self.config.max_concurrent_tasks > 0,
        "max_concurrent_tasks must be > 0"
    );
    assert!(self.config.db_pool_size > 0, "db_pool_size must be > 0");
    assert!(
        self.config.stale_claim_threshold > self.config.heartbeat_interval,
        "stale_claim_threshold ({:?}) must be greater than heartbeat_interval ({:?})",
        self.config.stale_claim_threshold,
        self.config.heartbeat_interval
    );
    self.config
}
```

### Suggested Resolution
Change `build()` to return `Result<DefaultRunnerConfig, WorkflowExecutionError>` (or a new `ConfigError` type). Replace `assert!` with validation that returns descriptive errors. Alternatively, at minimum, use `checked_build()` for the fallible version and make the current `build()` document its panics with `# Panics`.

---

## API-004: HTTP API route structure differs from documented paths
**Severity**: Major
**Location**: `crates/cloacinactl/src/commands/serve.rs` lines 298-396; `review/00-system-overview.md` lines 464-483
**Confidence**: High

### Description
The system overview documents HTTP routes as:
- `POST /workflows/{workflow_id}/execute`
- `GET /executions/{execution_id}`
- `GET /executions`
- `GET /workflows`
- `POST /accumulators/{graph_id}/{accumulator_id}`

The actual routes in `serve.rs` are tenant-scoped:
- `POST /v1/tenants/{tenant_id}/workflows/{name}/execute`
- `GET /v1/tenants/{tenant_id}/executions/{exec_id}`
- `GET /v1/tenants/{tenant_id}/executions`
- `GET /v1/tenants/{tenant_id}/workflows`
- No `/accumulators` POST endpoint (only WebSocket `/v1/ws/accumulator/{name}`)

Every authenticated route is nested under `/v1/tenants/{tenant_id}/`, which is a fundamentally different API shape from what's documented. The `POST /accumulators/{graph_id}/{accumulator_id}` endpoint does not exist at all -- accumulator interaction is WebSocket-only.

### Evidence
```rust
// Actual routes from serve.rs:
.route("/tenants/{tenant_id}/workflows/{name}/execute",
    post(crate::server::executions::execute_workflow))
.route("/tenants/{tenant_id}/executions",
    get(crate::server::executions::list_executions))
```

vs documented (system overview):
```
POST /workflows/{workflow_id}/execute
GET /executions
```

### Suggested Resolution
Update the system overview to reflect the actual tenant-scoped route structure. Additionally, consider whether non-tenant routes should exist for single-tenant deployments. If the intent is that all routes are tenant-scoped, document this clearly in the API reference.

---

## API-005: `list_executions` returns only active executions, not all
**Severity**: Major
**Location**: `crates/cloacinactl/src/server/executions.rs` lines 96-135
**Confidence**: High

### Description
The `GET /tenants/{tenant_id}/executions` endpoint's handler calls `dal.workflow_execution().get_active_executions()`, which returns only currently active (non-terminal) executions. A consumer expecting a list of all executions (including completed and failed) will see only running ones. The endpoint name "list_executions" and the route path suggest a complete listing, not a filtered one.

There is no pagination, no filtering by status, and no way to retrieve historical executions through this endpoint.

### Evidence
```rust
pub async fn list_executions(...) -> impl IntoResponse {
    // ...
    match dal.workflow_execution().get_active_executions().await {
        Ok(executions) => {
            // returns only active ones
```

### Suggested Resolution
Either rename the endpoint to `/active-executions` to make the filtering explicit, or change the implementation to accept query parameters for status filtering and pagination (`?status=Running&limit=50&offset=0`). The latter is the standard REST pattern and enables both "show me everything" and "show me what's running" use cases.

---

## API-006: `DefaultRunnerConfig` has 28 fields with no logical grouping
**Severity**: Minor
**Location**: `crates/cloacina/src/runner/default_runner/config.rs` lines 58-90
**Confidence**: High

### Description
`DefaultRunnerConfig` has 28 configuration fields in a flat structure. Fields span multiple domains: concurrency (`max_concurrent_tasks`), database (`db_pool_size`), cron scheduling (7 `cron_*` fields), trigger scheduling (3 `trigger_*` fields), registry reconciliation (4 `registry_*` fields), task claiming (4 fields), and routing. The builder has 28 corresponding setter methods.

A user who only wants to adjust `max_concurrent_tasks` must scan past 27 other options. The flat structure provides no guidance about which settings belong together or which are safe to change independently.

### Evidence
The builder has 28 setter methods, all at the same level:
```rust
.max_concurrent_tasks(8)          // Concurrency
.cron_poll_interval(...)          // Cron
.trigger_base_poll_interval(...)  // Triggers
.registry_reconcile_interval(...) // Registry
.stale_claim_threshold(...)       // Claiming
.routing_config(...)              // Dispatch
```

### Suggested Resolution
Group related fields into sub-configs: `CronConfig`, `TriggerConfig`, `RegistryConfig`, `ClaimingConfig`. The builder can then offer `with_cron(CronConfig)` for bulk configuration and individual field setters for simple cases. The `#[non_exhaustive]` on `DefaultRunnerConfig` already supports adding nested structs without breaking changes.

---

## API-007: `Context` uses different method semantics in Rust vs Python
**Severity**: Minor
**Location**: Rust `Context` at `crates/cloacina-workflow/src/context.rs`; Python `PyContext` at `crates/cloacina/src/python/context.rs` lines 66-96
**Confidence**: High

### Description
The Rust `Context` has `insert()` (fails if key exists) and `update()` (fails if key absent) as two distinct operations. The Python `PyContext` adds a third operation `set()` (line 66) that inserts-or-updates, which does not exist in the Rust API. Additionally, Rust `insert()` returns `Result<(), ContextError>`, while Python `insert()` raises `ValueError`, and `update()` raises `KeyError` -- different exception types for analogous failures.

A Python developer who learns `set()` as the primary method will look for a corresponding Rust method and not find one. A developer porting code from Python to Rust must convert `ctx.set("key", value)` to a `match ctx.get("key") { Some(_) => ctx.update(...), None => ctx.insert(...) }` pattern.

### Evidence
Python `PyContext`:
```rust
pub fn set(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
    if self.inner.get(key).is_some() {
        self.inner.update(key, json_value)
    } else {
        self.inner.insert(key, json_value)
    }
}
```

No corresponding `set()` or `upsert()` method exists on the Rust `Context`.

### Suggested Resolution
Add a `set()` or `upsert()` method to the Rust `Context` that matches the Python `set()` behavior. This is a natural convenience method that many users will want. Alternatively, if the insert/update distinction is intentional for safety, document why and note the Python `set()` as a convenience wrapper.

---

## API-008: Error types use wrong terminology in user-facing messages
**Severity**: Minor
**Location**: `crates/cloacina/src/executor/pipeline_executor.rs` lines 104, 107; `crates/cloacina/src/error.rs` lines 210, 233, 288
**Confidence**: High

### Description
Multiple error variants display "Pipeline" in their user-facing error messages, despite the types being named with "Workflow":

- `WorkflowExecutionError::ExecutionFailed` displays "Pipeline execution failed: {message}"
- `WorkflowExecutionError::Timeout` displays "Pipeline timeout after {timeout_seconds}s"
- `ValidationError::ExecutionFailed` displays "Pipeline execution failed: {message}"
- `ValidationError::PipelineRecoveryFailed` displays "Pipeline recovery failed: {pipeline_id}"
- `ExecutorError::PipelineNotFound` displays "Pipeline execution not found: {0}"

When a Python or HTTP consumer receives "Pipeline timeout after 300s", they may not understand this refers to a workflow timeout.

### Evidence
```rust
#[error("Pipeline execution failed: {message}")]
ExecutionFailed { message: String },

#[error("Pipeline timeout after {timeout_seconds}s")]
Timeout { timeout_seconds: u64 },
```

### Suggested Resolution
Update error message strings to use "Workflow" consistently: "Workflow execution failed", "Workflow timeout after {timeout_seconds}s", etc. Also rename `PipelineRecoveryFailed` to `WorkflowRecoveryFailed` and `PipelineNotFound` to `WorkflowNotFound` (the latter already exists in `WorkflowExecutionError` but not in `ExecutorError`).

---

## API-009: Python `DefaultRunner` creation panics on failure instead of raising
**Severity**: Minor
**Location**: `crates/cloacina/src/python/bindings/runner.rs` lines 318-322, 660-663
**Confidence**: High

### Description
Both `PyDefaultRunner::new()` and `PyDefaultRunner::with_config()` create the Rust `DefaultRunner` inside a background thread using `.expect("Failed to create DefaultRunner")`. If the database is unreachable or migrations fail, this `expect` causes a thread panic. The panic is caught by the join handle, but manifests to the Python caller as a confusing runtime error from the channel disconnecting, not as a clear database connection error.

### Evidence
```rust
let runner = rt.block_on(async {
    crate::DefaultRunner::new(&database_url)
        .await
        .expect("Failed to create DefaultRunner")  // Panics on failure
});
```

In `with_config`:
```rust
let runner = rt.block_on(async {
    crate::DefaultRunner::with_config(&database_url, rust_config)
        .await
        .expect("Failed to create DefaultRunner")  // Panics on failure
});
```

### Suggested Resolution
Propagate the error through a channel back to the Python thread, then raise it as a descriptive `PyRuntimeError` or `PyConnectionError`. The current `oneshot` channel pattern used for execute messages could be reused for initialization.

---

## API-010: `get_workflow` performs full list-and-filter instead of direct lookup
**Severity**: Minor
**Location**: `crates/cloacinactl/src/server/workflows.rs` lines 157-195
**Confidence**: High

### Description
The `GET /tenants/{tenant_id}/workflows/{name}` endpoint calls `registry.list_workflows().await` and then does `.find(|w| w.package_name == name)` in memory. This means every single-workflow lookup loads all workflows, which is both inefficient and semantically misleading -- the HTTP endpoint path suggests a direct lookup by name, but the implementation is a linear scan.

This becomes a consumer-facing issue when the workflow list is large: the endpoint is slower than expected, and if the registry has many entries, the latency is proportional to the total number of workflows, not O(1).

### Evidence
```rust
pub async fn get_workflow(...) -> impl IntoResponse {
    // ...
    match registry.list_workflows().await {
        Ok(workflows) => {
            let found = workflows.into_iter().find(|w| w.package_name == name);
            // ...
```

### Suggested Resolution
Add a `get_workflow_by_name(name: &str)` method to the `WorkflowRegistry` trait and use it here. This provides the expected O(1) lookup behavior and avoids loading unnecessary data.

---

## API-011: Python and Rust config surface parity gap
**Severity**: Minor
**Location**: `crates/cloacina/src/python/bindings/context.rs` lines 34-50 vs `crates/cloacina/src/runner/default_runner/config.rs` lines 58-90
**Confidence**: High

### Description
The Python `PyDefaultRunnerConfig` exposes 14 of the 28 Rust config fields. The following fields are not accessible from Python:

- `enable_trigger_scheduling` / `trigger_base_poll_interval` / `trigger_poll_timeout`
- `enable_registry_reconciler` / `registry_reconcile_interval` / `registry_enable_startup_reconciliation` / `registry_storage_path` / `registry_storage_backend`
- `enable_claiming` / `heartbeat_interval` / `stale_claim_sweep_interval` / `stale_claim_threshold`
- `runner_id` / `runner_name`
- `routing_config`

A Python user who needs to disable task claiming or configure the registry storage backend has no way to do so. There is no documentation indicating which fields are intentionally excluded vs. which are gaps.

### Evidence
Python constructor signature has 14 parameters:
```python
DefaultRunnerConfig(
    max_concurrent_tasks, scheduler_poll_interval_ms, task_timeout_seconds,
    pipeline_timeout_seconds, db_pool_size, enable_recovery,
    enable_cron_scheduling, cron_poll_interval_seconds,
    cron_max_catchup_executions, cron_enable_recovery,
    cron_recovery_interval_seconds, cron_lost_threshold_minutes,
    cron_max_recovery_age_seconds, cron_max_recovery_attempts
)
```

Rust `DefaultRunnerConfig` has 28 fields.

### Suggested Resolution
Either expose the remaining fields in the Python config class, or document which fields are intentionally omitted (and provide a way to pass raw config through if needed). Priority fields to add: `enable_claiming`, `enable_trigger_scheduling`, and `enable_registry_reconciler`, as these control major features.

---

## API-012: `DefaultRunnerBuilder` and `DefaultRunnerConfigBuilder` are two separate builders for the same system
**Severity**: Minor
**Location**: `crates/cloacina/src/runner/default_runner/config.rs` lines 254-474 (`DefaultRunnerConfigBuilder`) and lines 509-722 (`DefaultRunnerBuilder`)
**Confidence**: High

### Description
Creating a `DefaultRunner` requires choosing between two overlapping builder patterns:

1. `DefaultRunnerBuilder` -- sets `database_url`, `schema`, `config`, `runtime`, and `routing_config`
2. `DefaultRunnerConfigBuilder` -- sets all 28 configuration fields

The overlap is that `DefaultRunnerBuilder` has a `with_config(DefaultRunnerConfig)` method and also a `routing_config(RoutingConfig)` method. The `routing_config` on the builder duplicates the `routing_config` inside `DefaultRunnerConfig`. A user who sets `routing_config` on both will find that the builder's version silently overrides the config's version.

### Evidence
```rust
// These two approaches can conflict:
let runner = DefaultRunner::builder()
    .database_url("postgres://...")
    .with_config(
        DefaultRunnerConfig::builder()
            .routing_config(Some(config_a))
            .build()
    )
    .routing_config(config_b)  // Overrides config_a silently
    .build()
    .await?;
```

### Suggested Resolution
Remove `routing_config()` from `DefaultRunnerBuilder`, leaving it solely on `DefaultRunnerConfigBuilder`. The `DefaultRunnerBuilder` should only set properties that are orthogonal to `DefaultRunnerConfig` (database_url, schema, runtime). Alternatively, merge the two builders into a single builder.

---

## API-013: Well-designed DAL accessor pattern (positive)
**Severity**: Observation
**Location**: `crates/cloacina/src/dal/unified/mod.rs`
**Confidence**: High

### Description
The DAL provides an excellent API for consumers. The accessor pattern (`dal.task_execution().mark_completed(id)`, `dal.workflow_execution().get_active_executions()`) is highly discoverable, groups related operations naturally, and reads almost as natural language. This pattern extends consistently across all entity types (context, schedule, execution events, etc.).

### Suggested Resolution
No change needed. This is exemplary API design.

---

## API-014: Well-designed `Dispatcher` and `TaskExecutor` extension traits (positive)
**Severity**: Observation
**Location**: `crates/cloacina/src/dispatcher/traits.rs`
**Confidence**: High

### Description
The `Dispatcher` and `TaskExecutor` traits provide a clean extension point for custom executor backends. The traits have a minimal, well-documented surface area (4 methods each), include example implementations in doc comments (Kubernetes, serverless), and provide both the execution path (`execute`) and capacity management (`has_capacity`, `metrics`). The routing configuration with glob patterns (`RoutingRule::new("ml::*", "gpu")`) is elegant and discoverable.

### Suggested Resolution
No change needed.

---

## API-015: HTTP API error format is consistent and machine-readable (positive)
**Severity**: Observation
**Location**: `crates/cloacinactl/src/server/error.rs`
**Confidence**: High

### Description
The `ApiError` type provides a consistent error format across all HTTP endpoints: `{"error": "...", "code": "..."}` with appropriate HTTP status codes. The factory methods (`bad_request`, `not_found`, `internal`, `unauthorized`, `forbidden`, `too_many_requests`) cover the common cases and enforce the `code` parameter for machine-readability. This is a well-designed API error surface.

### Suggested Resolution
Consider adding an optional `request_id` field to the error response body (the `x-request-id` header is already generated by middleware). The doc comment at line 37 mentions `request_id` in the schema but the implementation does not include it.

---

## API-016: Macro API is ergonomic and well-documented (positive)
**Severity**: Observation
**Location**: `crates/cloacina-macros/src/lib.rs`
**Confidence**: High

### Description
The procedural macro API (`#[task]`, `#[workflow]`, `#[trigger]`, `#[computation_graph]`) provides an ergonomic entry point. The macro attributes use clear, keyword-based syntax (`id = "name"`, `dependencies = ["dep"]`, `cron = "0 2 * * *"`), doc comments include working examples for every variant, and compile-time validation catches errors (cyclic dependencies, missing tasks) before runtime. The dual-mode code generation (embedded vs packaged) is transparent to the user.

### Suggested Resolution
No change needed.

---

## API-017: `WorkflowExecutor` trait has both sync and async execution but inconsistent naming
**Severity**: Observation
**Location**: `crates/cloacina/src/executor/pipeline_executor.rs` (trait definition); `crates/cloacina/src/runner/default_runner/pipeline_executor_impl.rs`
**Confidence**: Medium

### Description
The `WorkflowExecutor` trait provides four execution methods:
- `execute()` -- synchronous (blocks until completion)
- `execute_async()` -- returns a handle immediately
- `execute_with_callback()` -- blocks with status callbacks
- `get_execution_status()`, `get_execution_result()`, `cancel_execution()`, `pause_execution()`, `resume_execution()` -- management methods

In Rust async, `execute()` is actually async (returns a Future) but blocks on the Future -- the naming suggests it is synchronous relative to the workflow, not the call itself. The `execute_async()` name implies the regular `execute()` is sync, which is misleading since both are `async fn`. A clearer naming would be `execute_and_wait()` vs `execute()` or `start()`.

### Suggested Resolution
Consider renaming `execute()` to `execute_and_wait()` or `run()`, and `execute_async()` to `execute()` or `start()`. The current naming follows a common Rust convention but may confuse users who expect `execute_async` to be the async version of a sync `execute`.
