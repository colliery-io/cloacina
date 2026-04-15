# Cloacina System Overview

## Summary

Cloacina is a **Rust library for embedded workflow orchestration** designed to be integrated directly into applications. Unlike standalone orchestration services (Airflow, Prefect), Cloacina provides in-process task execution with persistent state management, automatic retry, dependency resolution, and optional multi-tenancy support. It uses a database (PostgreSQL or SQLite) as the backing store for execution state and supports both Rust and Python task definitions.

**Key characteristics:**
- Embedded library, not a standalone service
- Content-versioned workflows based on task code fingerprints
- DAG-based task dependency management with compile-time validation
- Database-backed persistent state with optional PostgreSQL schema-based multi-tenancy
- Async-first architecture on tokio
- Procedural macros for convenient task and workflow definition
- Python bindings via PyO3 (installable as `cloaca` wheel)
- Support for packaged workflows in `.cloacina` format (bzip2 tar archives)

**Version:** 0.5.0 (released April 10, 2026)
**License:** Apache License 2.0
**Repository:** https://github.com/colliery-io/cloacina

---

## Repository Structure

```
cloacina/
├── crates/                           # Core Rust libraries
│   ├── cloacina/                     # Main library: task execution, workflow orchestration, persistence
│   ├── cloacina-macros/              # Procedural macros: @task, @workflow, @trigger, @computation_graph
│   ├── cloacina-computation-graph/   # Computation graph runtime types (accumulators, reactors)
│   ├── cloacina-workflow/            # Public re-export crate for external use
│   ├── cloacina-workflow-plugin/     # Type definitions for packaged workflow plugins
│   ├── cloacina-build/               # Build-time utilities
│   ├── cloacina-testing/             # Test utilities and mocks
│   └── cloacinactl/                  # CLI and HTTP server (daemon, serve, config, admin commands)
│
├── examples/                          # Comprehensive example gallery
│   ├── features/                     # Feature showcase: workflows, computation graphs
│   │   ├── workflows/                # 10+ workflow examples (cron, triggers, DAG, multi-tenant, etc.)
│   │   └── computation-graphs/       # Reactive pipeline examples
│   ├── performance/                  # Benchmarks and stress tests
│   ├── tutorials/                    # Step-by-step learning path (15+ modules)
│   └── soak-packages/                # Long-running scenarios (market maker with routing)
│
├── docs/                              # Documentation site (Hugo static site)
│   ├── content/                      # Source markdown pages
│   │   ├── workflows/                # Workflow tutorials, guides, API reference
│   │   ├── computation-graphs/       # Computation graph docs
│   │   ├── python/                   # Python bindings reference
│   │   ├── platform/                 # HTTP API, multi-tenancy, security
│   │   └── _index.md                 # Homepage
│   └── themes/geekdoc/               # Hugo theme with JavaScript bundles
│
├── docker/                            # Docker build configurations
├── .angreal/                          # Angreal (workflow) CLI tasks
├── .github/                           # GitHub workflows (CI/CD)
├── .metis/                            # Metis project documentation index
├── Cargo.toml                         # Workspace configuration (8 crates)
├── Cargo.lock                         # Locked dependencies
├── README.md                          # Project overview
├── CHANGELOG.md                       # Release notes (v0.4.0 initial, v0.5.0 computation graphs)
├── plissken.toml                      # Plissken configuration
└── LICENSE                            # Apache 2.0

```

### Directory Organization Principle

The codebase is organized by **functional layer**:
- **Core library** (`crates/cloacina/`) implements the workflow engine, task execution, and persistence
- **Supporting libraries** (macros, workflow, testing) provide specialized functionality
- **CLI tool** (`cloacinactl/`) exposes server and admin functions
- **Examples** demonstrate real-world usage patterns
- **Documentation** provides user-facing guides and API reference

---

## Key Entrypoints

### 1. **Library Entrypoint: `crates/cloacina/src/lib.rs`**

The main Cloacina library provides:
- **Core modules** exposed in the prelude:
  - `Task` trait and task registry
  - `Workflow` and `WorkflowBuilder` for DAG construction
  - `Context<T>` for type-safe inter-task data flow
  - `DefaultRunner` for coordinating execution
  - `TaskScheduler` for task readiness planning
  - `Executor` components for task execution

- **PyO3 module entry point** (`#[pymodule] fn cloaca()`) that exports:
  - `PyContext`, `PyTaskHandle`, `PyWorkflowBuilder`
  - `@task` and `@trigger` decorators
  - `PyDefaultRunner` for execution
  - Computation graph builders and accumulators

**Key re-exports:**
```rust
pub use cloacina_workflow;  // Makes cloaca-workflow available to external crates
pub use cloacina_computation_graph;  // Computation graph types
pub use prelude::{...};  // Common types
```

### 2. **CLI Entrypoint: `crates/cloacinactl/src/main.rs`**

The command-line interface provides three command modes:

**`daemon`**: Local scheduler for packaged workflows
- Watches directories for `.cloacina` packages
- Executes cron schedules and triggers locally
- No remote coordination

**`serve`**: HTTP API server with multi-tenancy
- Binds to configurable address (default 0.0.0.0:8080)
- PostgreSQL-backed with schema isolation per tenant
- REST endpoints for workflows, tasks, executions, health
- WebSocket endpoints for computation graph interactions
- Single-use ticket authentication for WebSocket

**`config`/`admin`**: Configuration and maintenance
- `config get/set/list` for configuration files
- `admin cleanup-events` to remove old execution records

### 3. **Procedural Macros: `crates/cloacina-macros/src/lib.rs`**

Core attribute macros:

**`#[task]`** - Defines a task with:
- Unique `id` and `dependencies` list
- Optional `retry_policy` and `trigger_rules`
- Automatic code fingerprinting for versioning
- Generates `#[ctor]` auto-registration

**`#[workflow]`** - Wraps a module containing `#[task]` functions:
- Auto-discovers and validates task dependencies
- Generates registration code
- Supports both embedded (auto-registration) and packaged (FFI export) modes

**`#[computation_graph]`** - Reactive data flow:
- Sources (accumulators), reactors with firing rules
- Compile-time topology validation and cycle detection
- Generates code for runtime reactor execution

**`#[trigger]`** - Schedule-based or condition-based workflow triggers:
- Cron expressions or poll intervals
- Custom Python triggers via `@trigger` decorator

---

## Architecture Overview

### High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│  User Application                                           │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Tasks: @[task] async fn ...                          │  │
│  │ Workflows: @[workflow] pub mod ...                   │  │
│  │ Triggers: @[trigger] async fn ...                    │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  Cloacina Core Engine                                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐   │
│  │Task Registry│  │Workflow      │  │Context         │   │
│  │             │  │Builder/DAG   │  │Management      │   │
│  └─────────────┘  └──────────────┘  └────────────────┘   │
│         ↑                ↑                   ↑              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │         TaskScheduler (Execution Planner)           │  │
│  │  - Converts Workflow to database execution plan     │  │
│  │  - Tracks task state: Pending→Ready→Running→Done   │  │
│  │  - Evaluates trigger rules and dependencies         │  │
│  │  - Pushes TaskReadyEvent to dispatcher              │  │
│  └──────────────────────────────────────────────────────┘  │
│         ↓                                                   │
│  ┌──────────────────────────────────────────────────────┐  │
│  │         Dispatcher (Routing Layer)                   │  │
│  │  - Routes TaskReadyEvent to executor based on rules  │  │
│  │  - Supports glob-based routing patterns              │  │
│  │  - Enables pluggable executor backends               │  │
│  └──────────────────────────────────────────────────────┘  │
│         ↓                                                   │
│  ┌──────────────────────────────────────────────────────┐  │
│  │    ThreadTaskExecutor (Thread-Pool Based)           │  │
│  │  - Claims tasks atomically (database locking)        │  │
│  │  - Lazy-loads dependency context                     │  │
│  │  - Executes with configurable timeout               │  │
│  │  - Updates context and task state                    │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐   │
│  │Cron         │  │Trigger       │  │Computation     │   │
│  │Scheduler    │  │Scheduler     │  │Graph Scheduler │   │
│  └─────────────┘  └──────────────┘  └────────────────┘   │
│         ↑                ↑                   ↑              │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  Persistence Layer (Data Access Layer)                      │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐   │
│  │Database     │  │Unified DAL   │  │Workflow        │   │
│  │Connection   │  │(CRUD ops)    │  │Registry        │   │
│  └─────────────┘  └──────────────┘  └────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  PostgreSQL / SQLite Database                               │
│  ┌──────────────────────────────────────────────────────┐  │
│  │Tables: pipeline_executions, task_executions,        │  │
│  │        contexts, recovery_events, workflows,        │  │
│  │        api_keys, signing_keys, trusted_keys, etc.   │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Execution Flow

```
1. User calls: runner.execute("workflow_name", initial_context)
                    ↓
2. Executor loads workflow from global registry
                    ↓
3. CreatePipelineExecution (insert into pipeline_executions table)
                    ↓
4. TaskScheduler::schedule_workflow()
   - Creates database task_execution records for all tasks
   - Sets initial tasks to Pending state
                    ↓
5. Scheduler Loop (runs every 1 second by default)
   - Poll for tasks in Pending state whose dependencies are satisfied
   - Evaluate trigger rules from context
   - Transition ready tasks to Ready state
   - Emit TaskReadyEvent(task_execution_id)
                    ↓
6. Dispatcher receives TaskReadyEvent
   - Routes to appropriate executor based on task name/tags
                    ↓
7. ThreadTaskExecutor::execute_task()
   - Atomically claim task (database update with claimed_by lock)
   - Load task function from global registry
   - Load dependency contexts from database
   - Merge contexts if multiple dependencies
   - Execute task function async with timeout
   - Catch errors and apply retry policy
   - Update context and task_execution status in database
   - Mark task as Completed or Failed
                    ↓
8. Scheduler detects task completion and marks downstream tasks as Ready
                    ↓
9. Repeat until all tasks complete or permanent failure
                    ↓
10. Return WorkflowExecutionResult with final status and context
```

### Computation Graph System (v0.5.0 Addition)

Alongside workflows, Cloacina v0.5.0 introduced **Computation Graphs** - reactive, event-driven data processing:

**Components:**
- **Accumulators** - Sources that collect events (passthrough, polling, batch, Kafka stream)
- **Reactor** - Stateful processor with firing rules (WhenAny, WhenAll) and input strategies (Latest, Sequential)
- **Reactive Scheduler** - Manages accumulator/reactor lifecycle with supervisor and crash recovery

**Integration:**
- Reconciler routes packages based on `has_computation_graph()` detection
- WebSocket endpoints for external event injection and manual commands
- Health state machines for monitoring accumulator/reactor health

---

## Primary Workflows

### Workflow 1: Simple Task Pipeline

**User code:**
```rust
#[task(id = "fetch", dependencies = [])]
async fn fetch_data(ctx: &mut Context<Value>) -> Result<(), TaskError> {
    ctx.insert("data", json!({"users": [1,2,3]}))?;
    Ok(())
}

#[task(id = "process", dependencies = ["fetch"])]
async fn process_data(ctx: &mut Context<Value>) -> Result<(), TaskError> {
    if let Some(data) = ctx.get("data") {
        ctx.insert("result", json!({"processed": data}))?;
    }
    Ok(())
}

let workflow = workflow! {
    name: "etl",
    description: "Extract and transform",
    tasks: [fetch_data, process_data]
};

let runner = DefaultRunner::new("postgres://...").await?;
let result = runner.execute("etl", Context::new()).await?;
```

**Happy path:**
1. `runner.execute()` creates pipeline_execution record
2. Scheduler creates task_execution records for fetch, process (both Pending)
3. Fetch has no dependencies → immediately becomes Ready
4. Scheduler dispatches fetch to executor
5. Fetch runs, inserts "data" into context
6. Process dependencies satisfied → becomes Ready
7. Process runs, reads "data", inserts "result"
8. Pipeline completes with status = Success

**Error path:**
1. Process task encounters error (e.g., null pointer in data)
2. Error captured as TaskError
3. Retry policy evaluated (default: exponential backoff, max 3 attempts)
4. If retries exhausted: mark task as Failed
5. Dependent tasks stuck in Pending
6. Pipeline marked as Failed after cleanup
7. Recovery event logged for audit

### Workflow 2: Multi-Tenant Isolation

**Setup:**
```rust
let tenant_a = DefaultRunner::with_schema(
    "postgres://localhost/db",
    "tenant_a"
).await?;

let tenant_b = DefaultRunner::with_schema(
    "postgres://localhost/db",
    "tenant_b"
).await?;
```

**Execution:**
- Both runners share same PostgreSQL database
- Each uses isolated schema (tenant_a.pipeline_executions, tenant_a.task_executions)
- DAL queries automatically prepend schema name
- Zero collision risk: impossible to read another tenant's data

### Workflow 3: Packaged Workflow (Distribution)

**Package structure:**
```
my-workflow/
├── Cargo.toml (cdylib crate)
├── package.toml (metadata: name, version, interface, tasks/triggers)
├── src/lib.rs (@[workflow] and @[trigger])
└── build.rs (sets up cloacina-build)
```

**Packaging:**
```bash
cd my-workflow
cloacinactl pack -o my-workflow-1.0.tar.bz2
```

Creates `.cloacina` package via fidius protocol:
- Bzip2 tar archive of source code
- Manifest with task metadata
- Digital signature (Ed25519)

**Loading at runtime:**
```rust
let runner = DefaultRunner::builder()
    .with_workflow_registry(file_based_registry)
    .build()
    .await?;

runner.execute("packaged_workflow_name", ctx).await?;
```

Reconciler detects `.cloacina` file, verifies signature, loads via FFI.

### Workflow 4: Cron-Scheduled Trigger

**User code:**
```rust
#[trigger(on = "nightly_job", cron = "0 2 * * *")]
pub async fn nightly_job_trigger() -> Result<TriggerResult, TriggerError> {
    Ok(TriggerResult::Fire(Context::new()))
}
```

**Execution:**
1. Trigger registered globally
2. Cron scheduler evaluates `"0 2 * * *"` (2am daily)
3. At scheduled time, trigger fires
4. Workflow automatically executed with provided context
5. Execution tracked like manual invocation

---

## Public Interface Surface

### Rust API

**Primary entry points:**

```rust
// Library usage
use cloacina::prelude::*;

// Define tasks
#[task(id = "my_task", dependencies = [])]
async fn my_task(ctx: &mut Context<Value>) -> Result<(), TaskError> { ... }

// Create workflow
let workflow = workflow! { name: "...", tasks: [...] };

// Execute
let runner = DefaultRunner::new("postgres://...").await?;
let result = runner.execute("workflow_name", ctx).await?;

// Query execution history
runner.get_execution(execution_id).await?;
runner.list_executions(filter).await?;
```

**Type-safe interfaces:**
- `Task` trait for custom task implementations
- `Workflow` struct with validation
- `Context<T>` with `T: Serialize + Deserialize`
- `TaskScheduler` for scheduling logic
- `DefaultDispatcher` for custom routing
- `ThreadTaskExecutor` for execution

### Python API (via `cloaca` wheel)

```python
from cloaca import task, workflow, Context, DefaultRunner

@task(id="fetch", dependencies=[])
async def fetch_data(ctx: Context):
    ctx.insert("data", {"users": [1, 2, 3]})

@task(id="process", dependencies=["fetch"])
async def process_data(ctx: Context):
    data = ctx.get("data")
    ctx.insert("result", {"processed": data})

# Build workflow programmatically
builder = WorkflowBuilder("etl", "Extract and load")
builder.add_task(fetch_data)
builder.add_task(process_data)
workflow = builder.build()

# Execute
runner = DefaultRunner("postgres://localhost/mydb")
result = runner.execute("etl", Context())
print(result.status)  # Success or Failed
```

### HTTP REST API (via `cloacinactl serve`)

**Base path:** `/v1`

**Workflow execution:**
- `POST /workflows/{workflow_id}/execute` - Start execution
- `GET /executions/{execution_id}` - Get execution status
- `GET /executions` - List executions with filter
- `GET /workflows` - List registered workflows

**Computation graphs:**
- `POST /accumulators/{graph_id}/{accumulator_id}` - Push event
- `POST /reactors/{graph_id}/{reactor_id}/commands` - Manual command (ForceFire, Pause, etc.)
- `GET /health/accumulators` - Health of all accumulators
- `GET /health/reactors/{graph_id}` - Reactor health

**Administration:**
- `POST /tenants` - Create tenant (with schema isolation)
- `GET /api-keys` - List API keys
- `POST /api-keys` - Create API key

**WebSocket:**
- `WS /ws/accumulators/{graph_id}/{accumulator_id}?ticket={single_use_ticket}` - Event stream
- `WS /ws/reactors/{graph_id}/{reactor_id}/commands?ticket={...}` - Manual control

**Authentication:**
- API key header: `X-API-Key: <key>`
- Single-use WebSocket ticket: `GET /auth/ws-ticket?resource=...`

### Configuration Surface

**Environment variables:**
```bash
DATABASE_URL                          # PostgreSQL or SQLite connection
RUST_LOG                             # Tracing filter (debug, info, warn, error)
CLOACINA_VAR_*                       # Custom variables (accessible via var() macro)
CLOACINA_BOOTSTRAP_KEY               # Initial admin API key
```

**Configuration file (~/.cloacina/config.toml):**
```toml
[database]
url = "postgres://localhost/cloacina"

[daemon]
poll_interval_ms = 500
watch_dirs = ["/workflows"]

[server]
bind = "0.0.0.0:8080"

[security]
require_signatures = true
trust_anchor_path = "/certs/root.pem"
```

### Event/Message Surface

**Database-persisted execution events:**
- `ExecutionEvent` - Workflow start/complete/fail
- `TaskExecution` - Task state transitions with timestamps
- `RecoveryEvent` - Stale claim sweeps, retries
- `ScheduleExecution` - Cron trigger fires

**Computation graph events:**
- Accumulator boundary push (WebSocket)
- Reactor manual commands (WebSocket)
- Reactor state changes (health state machine)

---

## Dependency Graph

### Crate Dependencies

```
External:
├── tokio (async runtime)
├── diesel (ORM for queries)
├── serde_json (context serialization)
├── pyo3 (Python bindings)
├── petgraph (dependency DAG algorithms)
├── ed25519-dalek (package signing)
├── aes-gcm (key encryption)
├── croner (cron expression parsing)
├── rdkafka (Kafka stream backend, optional)
├── tokio-postgres (PostgreSQL async client)
└── libsqlite3-sys (SQLite bindings)

Internal Crate Dependencies:
cloacina (main)
├── cloacina-macros (procedural macros)
├── cloacina-workflow (re-export crate)
├── cloacina-computation-graph (reactor/accumulator runtime)
├── cloacina-workflow-plugin (FFI type definitions)
└── cloacina-build (build utilities)

cloacinactl (CLI server)
├── cloacina (core library)
└── clap (CLI argument parsing)

cloacina-testing (test utilities)
└── cloacina (core)

cloacina-macros
├── cloacina-workflow (for visibility to macro users)
└── proc_macro_error (diagnostics)
```

### Module Dependencies (within `cloacina/`)

```
lib.rs (exports)
├── prelude (convenient exports)
├── task (Task trait, registry, namespace)
├── workflow (Workflow, WorkflowBuilder, DependencyGraph)
├── context (Context<T> for inter-task data)
├── execution_planner (TaskScheduler → task readiness)
├── executor (ThreadTaskExecutor, WorkflowExecutor)
├── dispatcher (Dispatcher, TaskReadyEvent routing)
├── registry (WorkflowRegistry, PackageLoader, Reconciler)
├── packaging (workflow package creation via fidius)
├── computation_graph (Accumulators, Reactors, Scheduler)
├── database (connection pool, schema, migrations)
├── dal (data access layer: CRUD for all entities)
├── models (database record types)
├── security (signing, key management, audit)
├── python (PyO3 bindings and executor interface)
├── trigger (Trigger trait, cron/event-based execution)
├── cron_trigger_scheduler (Unified cron + trigger scheduler)
├── runner (DefaultRunner, DefaultRunnerConfig, DefaultRunnerBuilder)
└── logging, error, retry, var, crypto

Key flows:
Task definition (macro) → Task registry → Workflow → Scheduler → Dispatcher → Executor → Database

Dependency graph:
task → workflow → execution_planner → dispatcher → executor → database
                    ↑
                  trigger (triggers → scheduler)

computation_graph → independent reactive system (packaging_bridge integrates with reconciler)
```

---

## Build and Deployment

### Building from Source

**Requirements:**
- Rust 1.70+ (MSRV)
- PostgreSQL client libraries (libpq) OR SQLite dev headers
- Python 3.9+ (for Python bindings)

**Build steps:**
```bash
# Build all crates
cargo build --release

# Build with specific backends
cargo build --release --no-default-features --features "postgres,macros"
cargo build --release --no-default-features --features "sqlite,macros"

# Build Python wheel (requires maturin)
pip install maturin
maturin build --release
```

**Build artifacts:**
- Library: `target/release/libcloacina.rlib` (static library)
- Shared: `target/release/libcloacina.so` (for Python bindings)
- Binary: `target/release/cloacinactl` (CLI tool)
- Python wheel: `target/wheels/cloaca-*.whl`

### Database Migrations

**Automatic:** DefaultRunner applies migrations on startup
```rust
let runner = DefaultRunner::new("postgres://...").await?;
// Migrations applied automatically via diesel_migrations
```

**Schema creation:** Two sets of SQL migrations:
1. **PostgreSQL-specific** (`migrations/postgres/`)
2. **SQLite-compatible** (`migrations/sqlite/`)

Migrations handle:
- Table creation (pipeline_executions, task_executions, contexts, etc.)
- Index creation for performance
- Partitioning schemes (if PostgreSQL)

### Deployment Models

**Model 1: Embedded Library**
- Cloacina integrated into application code
- Tasks/workflows defined via macros in application
- Database configured via connection string
- Execution runs within application process

**Model 2: Daemon Mode**
- Separate `cloacinactl daemon` process
- Watches directories for `.cloacina` packages
- Executes cron schedules and triggers
- No HTTP API (local-only)

**Model 3: HTTP Service**
- `cloacinactl serve` exposes REST API
- PostgreSQL backend with multi-tenant schema isolation
- API key authentication
- WebSocket support for computation graphs

**Docker:**
```dockerfile
FROM rust:latest as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM ubuntu:22.04
RUN apt-get install -y libpq5
COPY --from=builder /app/target/release/cloacinactl /usr/local/bin/
CMD ["cloacinactl", "serve", "--bind", "0.0.0.0:8080"]
```

### Testing

**Test types:**
1. **Unit tests** - Function-level logic (scattered in modules)
2. **Integration tests** - Full workflows with database (in `tests/integration/`)
3. **Performance tests** - Benchmarks (in `examples/performance/`)
4. **Soak tests** - Long-running scenarios (in `examples/soak-packages/`)

**Running tests:**
```bash
# All tests
cargo test --all

# Integration tests only
cargo test --test integration --features "postgres,sqlite"

# Specific test module
cargo test scheduler::basic_scheduling

# With logging
RUST_LOG=debug cargo test -- --nocapture
```

**Test database setup:**
- Uses `#[serial]` for deterministic test isolation
- SQLite for most tests (in-memory or temp file)
- PostgreSQL tests in separate integration test suite
- Fixtures provide DatabaseFixture with automatic cleanup

---

## Conventions and Implicit Knowledge

### Naming Conventions

**Tasks:**
- Unique string ID, lowercase with underscores: `"fetch_data"`, `"process_items"`
- Namespace format: `"module.submodule.task_name"`

**Workflows:**
- Lowercase with underscores: `"etl_pipeline"`, `"nightly_job"`
- Version suffix optional: `"workflow_v1"`

**Triggers:**
- Descriptive action names: `"nightly_job_trigger"`, `"inventory_check"`

**Context keys:**
- Lowercase with underscores: `"user_id"`, `"processed_data"`
- Domain-specific prefixes for clarity: `"etl_raw_data"`, `"email_result"`

### Code Organization Patterns

**Task definition pattern:**
```rust
#[task(id = "task_name", dependencies = ["dep1", "dep2"])]
async fn task_name(ctx: &mut Context<Value>) -> Result<(), TaskError> {
    // Implementation
    Ok(())
}
```

**Workflow composition pattern:**
```rust
#[workflow(name = "workflow_name", description = "...")]
pub mod workflow_name {
    use super::*;

    #[task(id = "step1", dependencies = [])]
    pub async fn step1(...) { ... }

    #[task(id = "step2", dependencies = ["step1"])]
    pub async fn step2(...) { ... }
}
```

**Packaged workflow pattern:**
```rust
// In package: my-workflow/src/lib.rs
#[packaged_workflow]
#[workflow(name = "packaged_workflow")]
pub mod my_workflow { ... }
```

### Error Handling Conventions

**Task errors are terminal:**
- Task returns `TaskError` → automatically retried per policy
- After max retries: task marked Failed
- Dependent tasks remain Pending (workflow fails)

**Validation happens at macro time:**
- Cyclic dependencies detected in `#[workflow]` macro
- Missing task definitions cause compile error
- Wrong dependency names detected at registration time

### Context Type Conventions

**Standard: `Context<serde_json::Value>`**
- Most flexible, works with any JSON-serializable data
- Used in examples and tutorials

**Custom: `Context<T>` where T implements Serialize**
- Type-safe context for specific domain models
- Rarely used; adds compile-time overhead

### Implicit Behaviors

1. **Task claim locking:** Database-level optimistic locking prevents duplicate execution
2. **Context merging:** When task has multiple dependencies, contexts merged with later overrides
3. **Trigger rule evaluation:** Happens lazily after all dependencies complete
4. **Retry backoff:** Exponential by default (1s, 2s, 4s, etc.) unless configured
5. **Auto-fingerprinting:** Task code hashed for versioning; hash changes force new execution
6. **Schema isolation:** PostgreSQL users isolated by schema; SQLite by file path
7. **Graceful degradation:** HTTP server continues if one executor fails
8. **Computation graph packaging:** Detected via `has_computation_graph()` in metadata

---

## Open Questions and Uncertainties

### 1. **Python Binding Completeness**
- How complete are Python bindings vs. Rust API?
- Are all error types accessible from Python?
- Are computation graphs fully exposed via Python API?
**Status:** Python bindings documented in `/docs/content/python/` but some advanced features may be missing or incomplete.

### 2. **Scaling Characteristics**
- What's the practical max tasks per workflow?
- How does performance degrade with 10k+ concurrent task_executions?
- Are there known bottlenecks in the scheduler loop?
**Status:** Performance tests exist but real-world scaling guidance is limited.

### 3. **Migration Path from Standalone Orchestration**
- How difficult is migrating from Airflow/Prefect workflows?
- What features from standalone systems are not supported?
**Status:** No explicit migration guide found; library positioning is "embedded" not "replace existing service".

### 4. **Computation Graph Maturity**
- How battle-tested is the computation graph system (v0.5.0 is recent)?
- What are the known limitations vs. reactive frameworks?
**Status:** Recent addition (April 2026); may have edge cases.

### 5. **Security Model Clarity**
- How is secret management intended to work (no vault integration)?
- What's the threat model for package signatures?
- How secure is the single-use WebSocket ticket mechanism?
**Status:** Security module exists but threat model documentation is implicit.

### 6. **Kafka Stream Backend**
- Is the Kafka stream accumulator production-ready?
- What's the failure behavior when Kafka is unavailable?
**Status:** Exists but not heavily documented; KRaft mode only, no ZooKeeper.

### 7. **Filesystem DAL Capabilities**
- How does the filesystem-based DAL compare to database-backed?
- What are the consistency guarantees?
**Status:** Exists but appears to be a secondary option to database storage.

### 8. **Cron Recovery Service**
- Under what conditions does cron recovery activate?
- What data loss is possible?
**Status:** Component exists in code but behavior not well documented.

---

## Summary of Key Architectural Decisions

| Decision | Rationale | Tradeoff |
|----------|-----------|----------|
| **Embedded library, not standalone service** | Deep integration with host app, no separate ops burden | Must embed in each application instance |
| **Database-backed execution state** | Reliable recovery, audit trail, multi-tenant isolation | Adds persistence latency |
| **Content-based versioning (code fingerprints)** | Automatic version management, reproducibility | Requires task code to be stable |
| **PostgreSQL + SQLite support** | Dev convenience (SQLite) + production robustness (PostgreSQL) | Maintained two code paths; migration complexity |
| **DAG-only workflows (no loops)** | Compile-time guarantees; simpler reasoning | Can't model iterative processes natively |
| **Macro-based task definition** | Type-safe, auto-registration, compile-time validation | Magic; harder to debug; limited introspection |
| **Push-based execution (TaskReadyEvent)** | Clean separation of concerns; pluggable executors | Database as message broker (eventual consistency) |
| **Schema-based multi-tenancy** | Zero collision risk; no filter overhead | PostgreSQL-specific; SQLite uses files |
| **Optional computation graphs (v0.5.0)** | Complement workflows for reactive use cases | Adds complexity; two execution models to maintain |
| **Python bindings via PyO3** | Native Python experience; single implementation | Binary wheels; complex CI/CD |

---

## Conclusion

Cloacina is a well-engineered **embedded workflow orchestration library** with a clear architectural vision: deep integration into applications, persistent state management, type-safe task definition via macros, and optional multi-tenancy. The codebase is organized by functional layer, thoroughly tested, and documented with examples. Recent additions (v0.5.0) introduce reactive computation graphs, expanding use cases beyond traditional DAG workflows.

The system is production-ready for workloads that fit its model: multi-step business processes within a single application, with retry/recovery, dependency management, and persistent audit trails. It trades off distributed coordination and horizontal scaling for simplicity and embeddedness.

Key strength areas:
- Clean separation of concerns (scheduler, dispatcher, executor, storage)
- Comprehensive error handling and recovery
- Type-safe Rust API with macro sugar
- Python bindings for mixed-language projects
- Multi-tenancy with schema isolation

Areas for deeper review:
- Scaling behavior under high task concurrency
- Python API completeness
- Security model formality
- Computation graph maturity and production readiness
