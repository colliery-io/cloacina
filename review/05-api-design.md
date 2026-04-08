# API Design Review

## Summary

Cloacina's API surfaces are generally well-designed for the common case: the macro-based task/workflow definition achieves a genuinely low-ceremony experience for Rust users, the Python bindings faithfully mirror the Rust concepts with idiomatic Python patterns (context managers, decorators, keyword-only arguments), and the CLI is immediately understandable. However, the API suffers from a terminology split between "Pipeline" (the execution engine) and "Workflow" (the user-facing concept) that leaks through every surface, inconsistent REST API error response shapes, missing API versioning on the REST surface, and several configuration footguns where invalid string values are silently accepted.

## Interface Inventory

| Interface | Location | Consumers |
|-----------|----------|-----------|
| Rust library API (prelude, macros) | `crates/cloacina/src/lib.rs`, `crates/cloacina-macros/` | Rust application developers |
| Python bindings (Cloaca) | `crates/cloacina/src/python/` | Python application developers |
| REST API | `crates/cloacinactl/src/server/` | HTTP clients, frontends, integrations |
| CLI (cloacinactl) | `crates/cloacinactl/src/main.rs` | Operators, DevOps |
| Macro system | `crates/cloacina-macros/src/lib.rs` | Rust/packaged workflow authors |
| Configuration | `DefaultRunnerConfig`, `~/.cloacina/config.toml`, env vars | All consumers |
| Plugin/FFI interface | `crates/cloacina-workflow-plugin/` | Packaged workflow cdylibs |

## Consistency Assessment

### Cross-Surface Consistency

**Naming**: The system uses "Workflow" in user-facing contexts (macros, CLI, REST, Python) but "Pipeline" in the execution engine (`PipelineExecutor`, `PipelineResult`, `PipelineStatus`, `PipelineExecution`, `PipelineError`). This split propagates across all surfaces:
- REST: `POST /tenants/{id}/workflows/{name}/execute` returns `execution_id` but `list_executions` returns `pipeline_name` in the JSON.
- Python: `result.status` returns `"Completed"` but the Rust type is `PipelineStatus::Completed`.
- CLI: `cloacinactl serve` logs "workflow" but the database table is `pipeline_executions`.

**Error shapes**: The REST API uses ad-hoc JSON error responses. Some endpoints return `{"error": "message"}`, while the execute endpoint returns a structured `{"execution_id": ..., "status": "scheduled"}` on success but `{"error": "..."}` on failure. There is no consistent error envelope (no `code` field, no `details` array, no `request_id`).

**Auth approach**: The REST API uses `Authorization: Bearer <key>` headers. The system overview document incorrectly states `X-API-Key` header, creating a documentation/implementation mismatch.

## Findings

### API-01: Pipeline/Workflow terminology split leaks through all consumer surfaces (Major)

**Severity**: Major
**Location**: `crates/cloacina/src/executor/pipeline_executor.rs`, `crates/cloacinactl/src/server/executions.rs`, Python bindings
**Confidence**: High

**Description**: Consumers interact with "Workflows" but receive "Pipeline" types back. A Rust consumer writes:
```rust
let result = runner.execute("my_workflow", context).await?;
// result is PipelineResult, not WorkflowResult
// result.status is PipelineStatus, not WorkflowStatus
// errors are PipelineError, not WorkflowError (which is a different type for construction errors)
```
The REST API compounds this: `POST /tenants/{id}/workflows/{name}/execute` returns `{"execution_id": ...}` but `GET /tenants/{id}/executions` returns objects with `pipeline_name` (not `workflow_name`). The Python bindings expose `PyPipelineResult` with a `status` field.

Meanwhile, `WorkflowError` already exists as a separate type for construction errors (`DuplicateTask`, `InvalidDependency`, `CyclicDependency`), creating ambiguity: `PipelineError` is the execution error type, `WorkflowError` is the construction error type, but their names do not convey this distinction.

**Impact**: Consumer confusion. A user searching docs for "workflow error" finds `WorkflowError` (construction) when they wanted `PipelineError` (execution). API clients see `pipeline_name` in responses for something they uploaded as a "workflow."

**Suggested Resolution**: Rename execution-layer types to use "Workflow" consistently: `WorkflowExecution`, `WorkflowResult`, `WorkflowStatus`, `WorkflowExecutionError`. Rename the existing `WorkflowError` to `WorkflowBuildError` or `WorkflowConstructionError`. The database table can keep its name for migration compatibility, but the public Rust and REST APIs should present a unified vocabulary.

---

### API-02: REST API has no consistent error response format (Major)

**Severity**: Major
**Location**: All handlers in `crates/cloacinactl/src/server/`
**Confidence**: High

**Description**: Error responses are constructed inline in each handler as ad-hoc JSON:
```rust
// In workflows.rs
Json(serde_json::json!({"error": msg}))

// In executions.rs
Json(serde_json::json!({"error": format!("{}", e)}))

// In tenants.rs
Json(serde_json::json!({"error": format!("{}", e)}))
```

All errors use the same `{"error": "string"}` shape, but:
1. There is no machine-readable error code -- clients cannot distinguish "workflow not found" from "insufficient permissions" without parsing the message string.
2. There is no `request_id` or correlation identifier for log tracing.
3. HTTP status codes are inconsistent for the same logical error: `register_workflow_package` returns 400 for registration failures (which could be internal errors), while `list_workflows` returns 500 for the same registry failure.
4. Internal errors (`format!("{}", e)`) leak implementation details to clients (e.g., Diesel error messages).

**Impact**: API clients cannot programmatically handle errors without string matching. Operational debugging is harder without request correlation. Error messages may expose internal details.

**Suggested Resolution**: Define a standard error response struct:
```rust
struct ApiError {
    error: String,         // human-readable message
    code: String,          // machine-readable code like "workflow_not_found"
    status: u16,           // HTTP status code
    request_id: Option<String>,
}
```
Implement `IntoResponse` for `ApiError` and use it consistently across all handlers. Separate user-facing messages from internal error details.

---

### API-03: REST API routes lack version prefix for tenant/workflow/execution endpoints (Major)

**Severity**: Major
**Location**: `crates/cloacinactl/src/commands/serve.rs` lines 177-242
**Confidence**: High

**Description**: The REST API mixes versioned and unversioned routes:
- **Versioned**: `/v1/health/accumulators`, `/v1/health/reactors`, `/v1/ws/accumulator/{name}`, `/v1/ws/reactor/{name}` -- these all use `/v1/` prefix.
- **Unversioned**: `/tenants`, `/auth/keys`, `/tenants/{id}/workflows`, `/tenants/{id}/executions` -- these have no version prefix.

The computation graph subsystem was added later and correctly uses `/v1/` prefixes, but the core CRUD routes do not. This means a breaking change to the tenant/workflow/execution API requires either breaking all clients or maintaining backward compatibility indefinitely.

**Impact**: Cannot evolve the core API shape (add pagination, change response structure, rename fields) without breaking existing clients. The inconsistency between `/v1/` and unversioned routes is confusing -- consumers cannot tell which routes are stable.

**Suggested Resolution**: Add `/v1/` prefix to all authenticated routes. The existing unversioned routes can remain as aliases during a deprecation period. All new routes should use versioned prefixes.

---

### API-04: Configuration accepts freeform strings where enums would prevent misuse (Minor)

**Severity**: Minor
**Location**: `crates/cloacina/src/runner/default_runner/config.rs` line 424, `crates/cloacina/src/python/task.rs` line 306, `crates/cloacinactl/src/server/keys.rs` line 39
**Confidence**: High

**Description**: Several configuration values use `String` where an enum or validated type would prevent silent misconfiguration:

1. `registry_storage_backend(impl Into<String>)` accepts any string. Valid values are "filesystem", "database", "sqlite", "postgres" but this is only enforced by a `match` in `services.rs:278` that silently falls through to the default (filesystem) on any unrecognized value. A typo like `"filsystm"` or `"postres"` would silently use the wrong backend.

2. Python `retry_backoff` parameter accepts strings ("fixed", "linear", "exponential") but defaults to `Fixed` for any unrecognized value -- including typos like "exponentail".

3. `CreateKeyRequest.role` accepts any string ("admin", "write", "read") but the `can_write()` and `can_admin()` methods only check for these exact values. A typo like "writ" would create a key with no permissions, silently denying all operations.

**Impact**: Silent misconfiguration. Users make a typo and get default behavior without any warning, potentially running with incorrect security permissions or storage backends.

**Suggested Resolution**: Use enums (`StorageBackend::Filesystem | StorageBackend::Database`, `BackoffStrategy::Fixed | ...`, `KeyRole::Admin | KeyRole::Write | KeyRole::Read`) and validate at the API boundary. Return explicit errors for unrecognized values rather than silently defaulting.

---

### API-05: get_workflow handler lists all workflows and filters in memory (Minor)

**Severity**: Minor
**Location**: `crates/cloacinactl/src/server/workflows.rs` lines 166-214
**Confidence**: High

**Description**: The `GET /tenants/{id}/workflows/{name}` handler calls `registry.list_workflows().await` to fetch all workflows, then uses `.find(|w| w.package_name == name)` to locate the requested one. This is an O(n) scan that loads all workflow metadata into memory when only one record is needed.

Similarly, the `GET /tenants/{id}/triggers/{name}` handler in `triggers.rs` lists all schedules (up to 100) and then filters by name in memory.

**Impact**: Inefficient for registries with many workflows. The API response time grows linearly with the number of registered workflows. This also means the API cannot distinguish between "workflow not found" and "registry query failed" since both paths go through the list operation.

**Suggested Resolution**: Add a `get_workflow_by_name(name: &str)` method to the `WorkflowRegistry` trait and use it directly in the handler. This would also produce a clearer 404 response.

---

### API-06: Prelude exports Pipeline terminology alongside Workflow types (Minor)

**Severity**: Minor
**Location**: `crates/cloacina/src/lib.rs` lines 453-483
**Confidence**: High

**Description**: The prelude, which is the recommended import for users, exposes both naming conventions:
```rust
pub mod prelude {
    pub use crate::workflow::{Workflow, WorkflowBuilder, WorkflowMetadata};  // "Workflow"
    pub use crate::executor::{
        PipelineExecution, PipelineExecutor, PipelineResult, PipelineStatus,  // "Pipeline"
    };
}
```
A new user doing `use cloacina::prelude::*` gets both `Workflow` and `PipelineResult` in scope, reinforcing the impression that these are different concepts when they are different phases of the same concept.

**Impact**: Newcomer confusion. The prelude should present a unified vocabulary.

**Suggested Resolution**: When API-01 is addressed by renaming types, update the prelude accordingly. In the interim, add a doc comment to the prelude explaining the relationship: "Pipeline types represent the execution of a Workflow."

---

### API-07: Python config uses `_seconds` and `_ms` suffixes inconsistently (Minor)

**Severity**: Minor
**Location**: `tests/python/test_scenario_01_basic_api.py` lines 582-608
**Confidence**: High

**Description**: The Python `DefaultRunnerConfig` exposes time-related properties with inconsistent unit suffixes:
- `scheduler_poll_interval_ms` -- milliseconds
- `task_timeout_seconds` -- seconds
- `pipeline_timeout_seconds` -- seconds
- `cron_poll_interval_seconds` -- seconds
- `cron_recovery_interval_seconds` -- seconds
- `cron_max_recovery_age_seconds` -- seconds

The Rust `DefaultRunnerConfig` builder uses `Duration` for all time values, which is unit-agnostic. The Python bindings made a choice to expose some as `_ms` and most as `_seconds`, creating an inconsistency where the consumer must mentally track which unit each field uses.

**Impact**: Easy to misconfigure. A user setting `scheduler_poll_interval_ms = 100` might assume other `_seconds` fields also take milliseconds, or vice versa.

**Suggested Resolution**: Choose one convention for the Python API. Since most fields use `_seconds`, convert `scheduler_poll_interval_ms` to `scheduler_poll_interval_seconds` (accepting a float for sub-second precision). Alternatively, use `timedelta` objects or accept both units with clear naming.

---

### API-08: WorkflowBuilder context manager has two different workflow registration paths (Minor)

**Severity**: Minor
**Location**: `crates/cloacina/src/python/workflow.rs` lines 149-215
**Confidence**: High

**Description**: The Python `WorkflowBuilder` supports two patterns that produce slightly different behavior:

1. **Context manager pattern** (recommended):
```python
with WorkflowBuilder("my_wf") as builder:
    @task(id="t1")
    def t1(ctx): return ctx
# __exit__ auto-discovers tasks from registry and registers workflow
```

2. **Manual pattern**:
```python
builder = WorkflowBuilder("my_wf")
builder.add_task("t1")
workflow = builder.build()
```

In the context manager pattern, `__exit__` discovers tasks from the global registry by matching the (tenant, package, workflow) tuple and creates a new `Workflow` instance from scratch (line 176-195). This means the `builder.description()` and `builder.tag()` calls made during the `with` block are lost -- `__exit__` creates a fresh `Workflow::new(workflow_id)` that does not carry the description or tags set on the builder.

**Impact**: Users who set description/tags via the context manager may find them missing from the registered workflow. The manual `builder.build()` path does preserve them.

**Suggested Resolution**: In `__exit__`, carry the description and tags from `self.inner` to the new `Workflow` object, or reuse `self.inner.build()` instead of creating a fresh `Workflow::new()`.

---

### API-09: Macro system provides excellent consumer ergonomics (Observation, Positive)

**Severity**: Observation (Positive)
**Location**: `crates/cloacina-macros/src/lib.rs`, tutorials in `examples/tutorials/`
**Confidence**: High

**Description**: The macro system achieves a remarkably low-ceremony API for defining tasks and workflows:
```rust
#[workflow(name = "etl", description = "ETL pipeline")]
pub mod etl {
    #[task(id = "extract", dependencies = [])]
    pub async fn extract(ctx: &mut Context<Value>) -> Result<(), TaskError> { ... }

    #[task(id = "transform", dependencies = ["extract"])]
    pub async fn transform(ctx: &mut Context<Value>) -> Result<(), TaskError> { ... }
}
```

Key strengths:
- Tasks are normal async functions with a thin attribute layer -- no trait implementation boilerplate.
- Dependencies are declared inline as string IDs, matching how users think about task ordering.
- The `#[workflow]` module macro auto-discovers contained `#[task]` functions and validates the dependency graph at compile time.
- Retry policy parameters (`retry_attempts`, `retry_delay_ms`, `retry_backoff`) are inline attributes with sensible names.
- The `workflow!` declarative macro offers an alternative for users who prefer struct-like syntax.

The computation graph macros (`#[computation_graph]`, `#[stream_accumulator]`, etc.) extend this pattern consistently, using the same attribute-on-function convention.

---

### API-10: Python bindings faithfully mirror Rust API with idiomatic Python patterns (Observation, Positive)

**Severity**: Observation (Positive)
**Location**: `crates/cloacina/src/python/`, `tests/python/test_scenario_01_basic_api.py`
**Confidence**: High

**Description**: The Python bindings (Cloaca) make consistently good API choices:
- `@task(id=..., dependencies=[...], retry_attempts=...)` uses keyword-only arguments (enforced by `*` in the signature), preventing positional argument mistakes.
- `WorkflowBuilder` works as a context manager (`with WorkflowBuilder("name") as builder:`), which is the Pythonic pattern for scoped resource management.
- `Context` supports Python's `__getitem__`, `__setitem__`, `__delitem__`, `__contains__`, and `__len__` protocols, making it feel like a native dict.
- Dependencies accept both strings and function references (`dependencies=["task_a"]` or `dependencies=[task_a]`), matching the flexibility users expect.
- `DefaultRunnerConfig` has a `to_dict()` method and a `default()` class method, both idiomatic Python conventions.
- Error messages include actionable guidance (e.g., "Task 'X' not found in registry. Make sure it was decorated with @task.").

---

### API-11: CLI (cloacinactl) is well-structured and self-documenting (Observation, Positive)

**Severity**: Observation (Positive)
**Location**: `crates/cloacinactl/src/main.rs`
**Confidence**: High

**Description**: The CLI achieves several API design best practices:
- Top-level commands map directly to deployment modes (`daemon`, `serve`) and administration tasks (`config`, `admin`), making the command hierarchy immediately understandable.
- Environment variable fallbacks are declared inline (`#[arg(long, env = "DATABASE_URL")]`), so `--help` shows them automatically.
- The `config get/set/list` subcommand follows the conventional key-value config pattern.
- The `admin cleanup-events` command includes `--dry-run` and a human-friendly duration format (`--older-than 90d`).
- The `--home` flag defaults to `~/.cloacina/` but allows override, following XDG-like conventions.
- All doc comments on enum variants double as `--help` output.

---

### API-12: DefaultRunnerConfig builder provides granular control with safe defaults (Observation, Positive)

**Severity**: Observation (Positive)
**Location**: `crates/cloacina/src/runner/default_runner/config.rs`
**Confidence**: High

**Description**: The `DefaultRunnerConfig::builder()` pattern is well-executed:
- 30+ configuration knobs, each with a sensible default value.
- `#[non_exhaustive]` on the config struct prevents breaking changes when new fields are added.
- `#[must_use]` on `DefaultRunner` reminds callers to call `shutdown()`.
- The builder consumes `self` (moving) on each method, preventing partial configuration from being accidentally reused.
- Defaults are centralized in `DefaultRunnerConfigBuilder::default()`, making them easy to audit.
- The `DefaultRunner::builder()` provides a separate builder for the runner itself (database URL + schema + config), keeping concerns separate.

One area for improvement: the `DefaultRunnerConfig` builder does not validate at `.build()` time -- it is possible to set contradictory values (e.g., `enable_cron_scheduling(true)` with `cron_poll_interval(Duration::ZERO)`).

---

### API-13: REST API success responses lack a consistent envelope (Minor)

**Severity**: Minor
**Location**: All handlers in `crates/cloacinactl/src/server/`
**Confidence**: High

**Description**: Success responses use different shapes depending on the resource type:

- `POST /auth/keys` returns `{"id", "name", "key", "permissions", ...}` -- flat object.
- `GET /auth/keys` returns `{"keys": [...]}` -- wrapped in a collection key.
- `POST /tenants/{id}/workflows` returns `{"package_id", "tenant_id"}` -- flat.
- `GET /tenants/{id}/workflows` returns `{"tenant_id", "workflows": [...]}` -- wrapped.
- `POST /tenants/{id}/workflows/{name}/execute` returns `{"execution_id", "workflow_name", "tenant_id", "status"}` -- flat.
- `GET /tenants/{id}/executions/{id}` returns `{"tenant_id", "execution_id", "status"}` -- status is `format!("{:?}", status)` which produces Rust debug formatting (e.g., `"Running"` vs `"running"`).

The `status` field in `get_execution` uses `format!("{:?}", status)` which produces Title-Cased Rust enum debug output, while the `execute_workflow` hardcodes `"status": "scheduled"` as a lowercase string. This means clients see both `"Completed"` and `"scheduled"` as status values with different casing conventions.

**Impact**: Clients cannot write generic JSON parsing for all endpoints. Status value casing is inconsistent, requiring case-insensitive comparison.

**Suggested Resolution**: Standardize status strings (lowercase: `"completed"`, `"running"`, `"scheduled"`). Consider a response envelope like `{"data": {...}, "meta": {"request_id": ..., "tenant_id": ...}}` for uniformity.

---

### API-14: WebSocket auth supports dual token sources (query param + header) -- good design (Observation, Positive)

**Severity**: Observation (Positive)
**Location**: `crates/cloacinactl/src/server/ws.rs` lines 42-64
**Confidence**: High

**Description**: The WebSocket handlers accept auth tokens from either the `Authorization: Bearer` header or a `?token=<pak>` query parameter, with the header taking priority. This is a practical accommodation for browser-based WebSocket clients that cannot set custom headers on the upgrade request. The implementation correctly validates auth before upgrading to WebSocket, preventing unauthenticated connections from consuming resources.

---

### API-15: DAL accessor pattern provides excellent API discoverability (Observation, Positive)

**Severity**: Observation (Positive)
**Location**: `crates/cloacina/src/dal/unified/mod.rs`
**Confidence**: High

**Description**: The `dal.context()`, `dal.task_execution()`, `dal.pipeline_execution()`, etc. accessor pattern provides a discoverable, IDE-friendly API. Each accessor returns a typed sub-DAL with scoped methods, so `dal.` followed by autocomplete immediately shows all available resource types. This is the internal API equivalent of a well-designed REST resource hierarchy. (Previously noted as LEG-08; confirmed as a strong API design pattern.)

---

### API-16: list_executions returns active executions despite being named "list" (Minor)

**Severity**: Minor
**Location**: `crates/cloacinactl/src/server/executions.rs` lines 98-142
**Confidence**: High

**Description**: `GET /tenants/{id}/executions` calls `dal.pipeline_execution().get_active_executions()`, which only returns currently active (non-terminal) executions. The endpoint name and HTTP path suggest it lists all executions, but it actually filters to active ones. There are no query parameters for filtering by status, pagination, or time range.

For an API consumer trying to retrieve historical executions or check the outcome of a completed workflow, this endpoint will return an empty list.

**Impact**: Consumer surprise -- listing executions returns an empty or incomplete set. There is no way to retrieve completed execution history through the REST API without knowing the specific execution ID.

**Suggested Resolution**: Either rename the endpoint to `/tenants/{id}/executions/active` or (preferably) add query parameters: `?status=running&limit=50&offset=0` to support both active and historical queries.
