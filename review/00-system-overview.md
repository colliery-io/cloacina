# System Overview

## Summary

Cloacina is a Rust-based workflow orchestration platform by Colliery Software that provides two deployment models: (1) an embedded library for building resilient task pipelines directly within Rust and Python applications, and (2) a deployable service (`cloacinactl`) for centralized workflow management with an HTTP API, multi-tenancy, and a local daemon mode. The core engine offers automatic retry with configurable backoff, database-backed state persistence (PostgreSQL and SQLite via Diesel MultiConnection for runtime backend selection), DAG-based dependency resolution, content-based workflow versioning, cron and event-trigger scheduling, package signing, and a computation graph subsystem for reactive stream processing. Python bindings are provided via PyO3 under the name "Cloaca."

## Repository Structure

```
cloacina/                          # Root -- Rust workspace (Cargo.toml)
+-- crates/                        # All Rust crates
|   +-- cloacina/                  # Core library (lib + cdylib for Python wheel)
|   |   +-- src/
|   |   |   +-- lib.rs             # Library root, module declarations, PyO3 #[pymodule]
|   |   |   +-- computation_graph/ # Reactive stream processing (accumulators, reactors)
|   |   |   +-- context.rs         # Shared data Context between tasks
|   |   |   +-- cron_evaluator.rs  # Cron expression parsing and evaluation
|   |   |   +-- cron_recovery.rs   # Recovery of missed cron executions
|   |   |   +-- crypto/            # AES-GCM encryption, Ed25519 signing
|   |   |   +-- dal/               # Data Access Layer (unified runtime backend + filesystem)
|   |   |   +-- database/          # Connection pooling, schema, migrations, universal types
|   |   |   +-- dispatcher/        # Task dispatch routing from scheduler to executors
|   |   |   +-- error.rs           # Comprehensive error hierarchy
|   |   |   +-- executor/          # Pipeline and task execution engine
|   |   |   +-- graph.rs           # Workflow DAG representation (petgraph)
|   |   |   +-- logging.rs         # Structured logging setup
|   |   |   +-- models/            # Diesel ORM models for all database tables
|   |   |   +-- packaging/         # Workflow packaging into .cloacina archives (fidius)
|   |   |   +-- python/            # PyO3 bindings -- @task, @trigger, WorkflowBuilder, etc.
|   |   |   +-- registry/          # Dynamic workflow loading, reconciler, package validation
|   |   |   +-- retry.rs           # RetryPolicy, BackoffStrategy, RetryCondition
|   |   |   +-- runner/            # DefaultRunner -- unified execution coordinator
|   |   |   +-- scheduler.rs       # Unified scheduler (cron + triggers in one loop)
|   |   |   +-- security/          # Key management, package signing, API keys, audit
|   |   |   +-- task.rs            # Task trait, TaskRegistry, global registry
|   |   |   +-- task_scheduler/    # Lower-level scheduling loop, stale claim sweeper
|   |   |   +-- trigger/           # Event-based trigger system
|   |   |   +-- workflow/          # Workflow struct, builder, metadata, versioning
|   |   +-- tests/                 # Integration tests (DAL, executor, scheduler, signing, etc.)
|   +-- cloacina-build/            # Build-time helper -- sets Python rpath for runtime linking
|   +-- cloacina-computation-graph/ # Core types for computation graph plugins (minimal deps)
|   +-- cloacina-macros/           # Procedural macros: #[task], workflow!, #[trigger], etc.
|   +-- cloacina-testing/          # Lightweight test runner -- no database required
|   +-- cloacina-workflow/         # Minimal types for authoring workflows (no runtime deps)
|   +-- cloacina-workflow-plugin/  # Fidius plugin interface (FFI ABI contract)
|   +-- cloacinactl/               # CLI binary: daemon, serve, config, admin
|       +-- src/
|           +-- main.rs            # CLI entry point (clap)
|           +-- commands/          # daemon, serve, config, cleanup_events, watcher
|           +-- server/            # Axum HTTP handlers: auth, keys, tenants, workflows, ws
+-- docs/                          # Hugo documentation site (content + themes)
+-- docker/                        # Dockerfile.test for CI
+-- examples/                      # Tutorials, feature showcases, performance benchmarks
|   +-- tutorials/                 # Step-by-step learning (Rust workflows, Python, CG)
|   +-- features/                  # Feature showcases (workflows, computation graphs)
|   +-- performance/               # Benchmarks (simple, parallel, pipeline, CG)
+-- tests/python/                  # Python integration tests (31 scenario files + conftest)
+-- soak-packages/                 # Pre-built .cloacina packages for soak testing
+-- .angreal/                      # Angreal task runner (project automation)
+-- .github/workflows/             # CI, nightly, performance, docs, release workflows
+-- .metis/                        # Metis project management (vision, initiatives, ADRs, specs)
|   +-- adrs/                      # CLOACI-A-0001 (MultiConnection), CLOACI-A-0002 (outbox)
|   +-- specifications/            # CLOACI-S-0001 through CLOACI-S-0007
|   +-- vision.md                  # Project vision document
+-- .pre-commit-config.yaml        # Pre-commit hooks (license, fmt, clippy, ruff, yamllint)
+-- install.sh                     # curl-pipe installer for cloacinactl binary
+-- Cargo.toml                     # Workspace manifest (v0.4.0)
```

## Key Entrypoints

1. **Library entrypoint**: `crates/cloacina/src/lib.rs`
   - Declares all public modules and re-exports key types
   - Contains the `#[pymodule] fn cloaca()` PyO3 entry point for the Python wheel
   - Defines the `prelude` module for convenient `use cloacina::prelude::*` imports

2. **CLI entrypoint**: `crates/cloacinactl/src/main.rs`
   - `cloacinactl daemon` -- local daemon watching for .cloacina packages, runs cron/triggers (SQLite)
   - `cloacinactl serve` -- HTTP API server backed by PostgreSQL with auth and multi-tenancy
   - `cloacinactl config {get|set|list}` -- manage `~/.cloacina/config.toml`
   - `cloacinactl admin cleanup-events` -- purge old execution events from the database

3. **Macro entrypoints**: `crates/cloacina-macros/src/lib.rs`
   - `#[task]` -- transforms async functions into registered Task implementations
   - `workflow!` -- constructs workflows with dependency validation
   - `#[trigger]` -- defines event-based triggers
   - `#[computation_graph]`, `#[stream_accumulator]`, `#[polling_accumulator]`, `#[batch_accumulator]`, `#[passthrough_accumulator]` -- computation graph macros

4. **Plugin interface**: `crates/cloacina-workflow-plugin/src/lib.rs`
   - `CloacinaPlugin` trait (via fidius) -- FFI contract for packaged workflow .so/.dylib files
   - Methods: `get_task_metadata`, `execute_task`, `get_graph_metadata`, `execute_graph`

## Architecture

### Component Diagram

```
+----------------------------------------------------------------------+
|                      User Interface Layer                             |
|                                                                      |
|  +--------------+  +--------------+  +----------------------------+  |
|  | Library API  |  | cloacinactl  |  | Python Bindings (Cloaca)   |  |
|  | (prelude::*) |  | CLI (clap)   |  | (PyO3 #[pymodule])        |  |
|  +------+-------+  +------+-------+  +------------+---------------+  |
|         |                 |                       |                   |
|         |  +--------------+--------------+        |                  |
|         |  |  HTTP Server (axum)         |        |                  |
|         |  |  - Auth middleware          |        |                  |
|         |  |  - Tenant management        |        |                  |
|         |  |  - WebSocket endpoints      |        |                  |
|         |  +-----------------------------+        |                  |
+---------|--------------------------------------------+---------------+
          |                                        |
+---------+----------------------------------------+-------------------+
|                      Core Engine Layer                                |
|                                                                      |
|  +-------------+  +----------+  +----------+  +------------------+   |
|  |DefaultRunner|  |Scheduler |  |Dispatcher|  |  Registry /      |   |
|  |(coordinator)|  |(cron +   |  |(routing) |  |  Reconciler      |   |
|  |             |  | triggers)|  |          |  |  (package mgmt)  |   |
|  +------+------+  +----+-----+  +----+-----+  +-------+----------+  |
|         |              |             |                 |              |
|  +------+------+  +----+-----+  +----+------+  +------+-----------+  |
|  | Executor    |  |TaskSched |  |ThreadTask |  | Packaging /      |  |
|  | (Pipeline   |  |(schedule |  |Executor   |  | Signing /        |  |
|  |  Executor)  |  | loop)    |  |           |  | Validation       |  |
|  +-------------+  +----------+  +-----------+  +------------------+  |
|                                                                      |
|  +------------------------------------------------------------------+|
|  | Computation Graph Subsystem                                       ||
|  |  - Accumulators (stream, polling, batch, state, passthrough)      ||
|  |  - Reactor (criteria evaluation, graph execution)                 ||
|  |  - ReactiveScheduler (spawn, supervise, restart)                  ||
|  |  - StreamBackend (pluggable: Kafka, etc.)                         ||
|  |  - WebSocket endpoints for real-time data ingestion               ||
|  +------------------------------------------------------------------+|
+----------------------------------+------------------------------------+
                                   |
+----------------------------------+------------------------------------+
|                      Persistence Layer                                |
|                                                                      |
|  +----------------+  +------------------+  +----------------------+  |
|  | DAL (unified)  |  | Database Module   |  | Filesystem DAL     |  |
|  | - CRUD ops     |  | - AnyConnection   |  | - Package storage  |  |
|  | - Outbox       |  |   (PG + SQLite)   |  | - Registry         |  |
|  | - Events       |  | - Migrations      |  |                    |  |
|  | - Checkpoints  |  | - Schema          |  |                    |  |
|  +--------+-------+  +--------+---------+   +----------+---------+  |
|           |                    |                        |            |
|     +-----+-----+       +-----+----+             +-----+------+    |
|     | PostgreSQL |       | SQLite   |             | Filesystem |    |
|     +-----------+        +----------+             +------------+    |
+----------------------------------------------------------------------+
```

### Key Architectural Patterns

1. **Runtime Database Backend Selection (ADR-1)**: Uses Diesel `MultiConnection` enum (`AnyConnection`) to select PostgreSQL or SQLite at runtime based on the connection URL. Universal types (`UniversalUuid`, `UniversalTimestamp`, `UniversalBool`) bridge type differences.

2. **Outbox Pattern (ADR-2)**: Task distribution uses a `task_outbox` table as a transient work queue. Workers claim from the outbox rather than polling `task_executions`. PostgreSQL gets optional LISTEN/NOTIFY for near-instant wakeup; SQLite uses polling.

3. **Execution Event Sourcing (ADR-2)**: An append-only `execution_events` table captures the full state transition history for debugging and audit trails.

4. **Plugin Architecture**: Packaged workflows are compiled as cdylib shared libraries using the fidius plugin system. The `CloacinaPlugin` trait defines the FFI boundary. Packages are `.cloacina` archives (bzip2 tar) containing source + manifest.

5. **Global Registry Pattern**: Tasks, workflows, triggers, and computation graphs are registered at program startup via `#[ctor]`-generated constructors into global registries (`once_cell::Lazy<Arc<RwLock<HashMap>>>`).

6. **Dispatcher/Executor Decoupling**: The scheduler pushes `TaskReadyEvent`s to a `Dispatcher` which routes to `TaskExecutor` backends via configurable routing rules, enabling pluggable execution strategies.

7. **Reactive Computation Graphs**: A separate subsystem for continuous stream processing with accumulators (event consumers) and reactors (graph executors). The `ReactiveScheduler` spawns and supervises these as long-lived tokio tasks with auto-restart on panic.

## Primary Workflows

### 1. Library Workflow Execution (Embedded Mode)

1. User defines tasks with `#[task]` macro, specifying `id`, `dependencies`, and optional `retry_policy` / `trigger_rules`.
2. Tasks are auto-registered in the global task registry via `#[ctor]`.
3. User builds a workflow with `workflow!` macro or `WorkflowBuilder`.
4. User creates a `DefaultRunner` with a database URL.
5. The runner connects to the database, runs migrations, and starts background scheduler/executor tasks.
6. User calls `runner.execute("workflow_name", Context::new()).await?`.
7. The `PipelineExecutor` creates a `PipelineExecution` record, resolves the task DAG, and executes tasks in topological order with parallel execution of independent tasks.
8. Context data flows between tasks via the shared `Context` with database persistence.
9. Failed tasks are retried according to their `RetryPolicy`.
10. Results are returned as `PipelineResult` with per-task status.

### 2. Daemon Mode (cloacinactl daemon)

1. Initializes `~/.cloacina/` directory structure (packages/, logs/, config.toml).
2. Creates a SQLite database at `~/.cloacina/cloacina.db`.
3. Creates `DefaultRunner` and `FilesystemWorkflowRegistry` watching specified directories.
4. Runs initial reconciliation -- scans watch dirs for `.cloacina` packages, loads them via fidius.
5. Registers cron schedules and triggers from loaded package manifests.
6. Starts filesystem watcher (notify crate) for live package discovery.
7. Event loop: responds to filesystem changes (reconcile), periodic ticks, SIGHUP (config reload), SIGINT/SIGTERM (graceful shutdown with timeout).

### 3. API Server Mode (cloacinactl serve)

1. Connects to PostgreSQL, applies migrations, bootstraps admin API key.
2. Starts axum HTTP server with routes for health, keys, tenants, workflows, executions, triggers, WebSocket.
3. Auth middleware validates API keys from `X-API-Key` header using an LRU cache.
4. Tenant-scoped operations use PostgreSQL schema-based isolation.
5. Starts ReactiveScheduler supervision loop for computation graphs with auto-restart.

### 4. Computation Graph (Reactive Stream Processing)

1. User defines computation graph with `#[computation_graph]` macro.
2. Accumulators receive events via WebSocket or Kafka, process them, and emit serialized boundaries to the reactor.
3. The reactor maintains an `InputCache`, evaluates `ReactionCriteria` (when_all, when_any), and fires the compiled graph function when criteria are met.
4. Results are output as `GraphResult`.
5. State is persisted to database tables for crash recovery.
6. The `ReactiveScheduler` supervises all running computation graphs and auto-restarts crashed tasks.

## Public Interface Surface

### Library API (Rust)
- `cloacina::prelude::*` -- primary import set
- Key types: `Task`, `Context`, `Workflow`, `WorkflowBuilder`, `DefaultRunner`, `PipelineExecutor`
- Macros: `#[task]`, `workflow!`, `#[trigger]`, computation graph macros
- Configuration: `DefaultRunnerConfig`, `SchedulerConfig`, `ExecutorConfig`, `RoutingConfig`

### Python API (Cloaca)
- Decorators: `@task(...)`, `@trigger(...)`
- Classes: `Context`, `WorkflowBuilder`, `DefaultRunner`, `PipelineResult`, `RetryPolicy`
- Admin: `DatabaseAdmin`, `TenantConfig`, `TenantCredentials` (Postgres only)
- Computation graph: `ComputationGraphBuilder`, `@node`, accumulator decorators

### CLI (cloacinactl)
- `cloacinactl daemon [--watch-dir DIR] [--poll-interval MS]`
- `cloacinactl serve [--bind ADDR] [--database-url URL] [--bootstrap-key KEY]`
- `cloacinactl config {get|set|list}`
- `cloacinactl admin cleanup-events [--older-than DURATION] [--dry-run]`

### REST API (cloacinactl serve)
- Health: `GET /health`, `GET /ready`, `GET /metrics`
- Keys: `POST /auth/keys`, `GET /auth/keys`, `DELETE /auth/keys/{id}`
- Tenants: `POST /tenants`, `GET /tenants`, `DELETE /tenants/{schema_name}`
- Workflows: `POST /tenants/{id}/workflows` (multipart upload), `GET`, `DELETE`
- Executions: `POST /tenants/{id}/workflows/{name}/execute`, `GET /tenants/{id}/executions[/{id}[/events]]`
- Triggers: `GET /tenants/{id}/triggers[/{name}]`
- Reactive health: `GET /v1/health/accumulators`, `GET /v1/health/reactors[/{name}]`
- WebSocket: `GET /v1/ws/accumulator/{name}`, `GET /v1/ws/reactor/{name}`

### Configuration Surface
- `~/.cloacina/config.toml` -- TOML config
- Environment variables: `DATABASE_URL`, `CLOACINA_BOOTSTRAP_KEY`, `CLOACA_BACKEND`, `KAFKA_BROKER_URL`
- Feature flags (compile-time): `postgres`, `sqlite`, `macros`, `kafka`, `auth`, `extension-module`, `packaged`

## Dependency Graph

### Internal Crate Dependencies
```
cloacina (core library)
+-- cloacina-workflow (minimal authoring types)
|   +-- cloacina-macros (optional, proc macros)
+-- cloacina-macros (optional, proc macros)
+-- cloacina-computation-graph (core CG types)
+-- cloacina-workflow-plugin (fidius FFI interface)
+-- cloacina-build (build.rs helper)

cloacinactl (CLI binary)
+-- cloacina (core)
+-- cloacina-workflow-plugin
+-- axum + tower (HTTP server)
+-- notify (filesystem watching)
+-- clap (CLI parsing)

cloacina-testing (test utilities, no DB)
+-- cloacina-workflow
```

### Major External Dependencies
- **Database**: diesel 2.1, diesel_migrations, deadpool-diesel
- **Async runtime**: tokio (full features)
- **Serialization**: serde, serde_json, bincode, toml
- **Crypto**: ed25519-dalek, aes-gcm, sha2
- **Python**: pyo3 0.25, pythonize
- **Plugin system**: fidius 0.0.5, libloading
- **Graph**: petgraph
- **Cron**: croner 2.1
- **Streaming**: rdkafka (optional, Kafka support)
- **HTTP**: axum 0.8, tower
- **Filesystem watching**: notify 7

## Build and Deployment

### Building
- Full build: `cargo build`
- Python wheel: `maturin develop --features extension-module`
- CLI binary: `cargo build -p cloacinactl`

### Testing
- Angreal tasks are the canonical way to run tests
- `angreal cloacina unit` -- unit tests
- `angreal cloacina integration` -- integration tests with backing services
- `angreal cloacina macros` -- macro validation tests
- `angreal cloacina server-soak` -- end-to-end server soak test
- `angreal cloacina auth-integration` -- auth integration tests
- `angreal cloacina ws-integration` -- WebSocket integration tests
- `angreal cloaca test` -- Python binding tests

### CI Pipelines
- `ci.yml` -- main CI: change detection, quick checks (fmt, clippy), test matrix
- `cloacina.yml` -- feature builds, unit tests, integration tests, macro tests
- `nightly.yml` -- nightly: full suite + coverage + macOS
- `performance.yml` -- benchmark runs
- `examples-docs.yml` -- tutorial and example validation
- `unified_release.yml` -- release: crates.io + PyPI + wheels

### Deployment
- `install.sh` -- curl-pipe installer for `cloacinactl` binary
- `cloacinactl serve` targets PostgreSQL environments
- `cloacinactl daemon` targets local/lightweight SQLite environments

## Conventions and Implicit Knowledge

1. **Universal types**: `UniversalUuid`, `UniversalTimestamp`, `UniversalBool`, `DbBinary` bridge PostgreSQL/SQLite type differences.

2. **Content-based versioning**: Workflows versioned by hash of task code and dependency structure.

3. **Namespace system**: Tasks have a 4-part namespace: `schema/source/workflow/task_id`.

4. **Build script requirement**: All binaries depending on `cloacina` must use `cloacina-build` in `build.rs` for Python rpath.

5. **Global registries with #[ctor]**: Macros generate `#[ctor::ctor]` functions that auto-register at program startup.

6. **Fidius plugin system**: `cloacina-workflow-plugin` is the single source of truth for the FFI ABI.

7. **Serial test execution**: Integration tests use `#[serial]` for shared database state.

8. **Angreal automation**: `.angreal/` contains the canonical project automation -- always use angreal tasks over raw commands.

9. **License**: Apache 2.0 with automated header insertion via pre-commit.

10. **Load-once pattern**: Plugin dylibs are cached in `PluginHandleCache` and never dlclosed to avoid inventory linked-list corruption.

## Open Questions

1. **Specification documents**: CLOACI-S-0001 through S-0007 in `.metis/specifications/` contain design contracts not fully traced here.

2. **Multi-tenant migration strategy**: How tenant schema migrations are managed in PostgreSQL multi-tenant mode.

3. **Python wheel publishing**: Exact maturin/PyPI pipeline and `extension-module` feature interaction.

4. **Kafka production readiness**: The `StreamBackend` trait and `KafkaStreamBackend` exist but production hardening level is unclear.
