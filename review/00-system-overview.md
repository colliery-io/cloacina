# Cloacina System Overview

> Discovery Agent baseline. Branch: `i-0102-fidius-and-plugin-shell`. Workspace version `0.5.1` (`Cargo.toml:7`). 482 source files indexed (`/.metis/code-index.md`).

---

## 1. Summary

Cloacina is a Rust workflow orchestration system with first-class Python bindings, packaged as a Cargo workspace under `crates/` plus example crates under `examples/`. The library positions itself as an **embedded** orchestration framework — applications link the `cloacina` crate, define DAGs of `#[task]`-decorated async functions or Python `@task` functions, and execute them via a `DefaultRunner` that owns scheduling, dispatch, persistence, and recovery. State persists to PostgreSQL or SQLite through the diesel-based unified DAL (`crates/cloacina/src/dal/unified/`); backend selection is runtime, driven by URL scheme, with both backends compiled in by default. Multi-tenancy uses Postgres schema isolation (`Database::try_new_with_schema`) or per-file SQLite databases.

On top of the embedded core, the workspace ships three out-of-process binaries: a long-running HTTP+WebSocket API service (`cloacina-server`, `crates/cloacina-server/src/main.rs`), a DB-queue-driven build service (`cloacina-compiler`, `crates/cloacina-compiler/src/main.rs`), and a CLI/local scheduler (`cloacinactl`, `crates/cloacinactl/src/main.rs` — provides `daemon`, `server`, `compiler`, `package`, `workflow`, `graph`, `execution`, `tenant`, `key`, `trigger` nouns). Workflows can be authored in-process (linked into your binary) or shipped as `.cloacina` packages — bzip2 source archives that the compiler service builds into cdylibs. The cdylibs export a fidius-based plugin ABI (`crates/cloacina-workflow-plugin/src/lib.rs`) and are dynamically loaded by the runner's reconciler.

Beyond the workflow core, Cloacina is mid-rollout on a parallel execution model called the **computation graph** (`crates/cloacina-computation-graph/src/lib.rs`, runtime in `crates/cloacina/src/computation_graph/`). Where a workflow's quantum is the task, a computation graph's quantum is the graph traversal — once triggered, all nodes run in-process with in-memory channels. Computation graphs are fired by **reactors** — specialized triggers that consume **accumulator** boundary events and decide when to fire (modes: `WhenAny`, `WhenAll`). Accumulators come in passthrough, polling, batch, and stream-backed flavors (Kafka via `rdkafka`). The active branch (`i-0102-fidius-and-plugin-shell`) is implementing CLOACI-I-0102: a unified Rust plugin shell macro `cloacina::package!()` that emits one fidius plugin per cdylib for any combination of declared primitives (tasks, workflows, reactors, triggers, computation graphs) — replacing the per-macro plugin emission that existed before. There are 32 active initiatives and 11 specifications under `.metis/`.

## 2. Repository Structure

Top-level layout (relevant to code reviewers):

```
cloacina/
├── Cargo.toml                       # workspace root (12 members, examples excluded)
├── README.md
├── crates/                          # all library + binary crates
├── examples/                        # backend examples + tutorials + fixtures (excluded from workspace)
│   ├── features/                    # feature-specific demos (computation-graphs/, workflows/)
│   ├── fixtures/                    # test fixtures for I-0102 (reactor-only, trigger-only, mixed)
│   ├── performance/                 # micro-benchmarks
│   └── tutorials/                   # numbered tutorials, Rust + Python
├── docs/                            # Hugo + geekdoc theme; generated site sources
├── tests/python/                    # pytest scenarios that exercise the cloaca wheel
├── scripts/                         # check_credential_logging.py (lint helper)
├── .angreal/                        # angreal task definitions — canonical dev/CI surface
├── .metis/                          # planning artifacts (vision, initiatives, specs, ADRs, code-index)
└── review/                          # this review's outputs
```

**Organizing principle:** strict separation of (a) authoring crates (`cloacina-workflow`, `cloacina-computation-graph`, `cloacina-workflow-plugin`) — minimal-dep crates a packaged cdylib can compile against; (b) the engine crate (`cloacina`) — full runtime with DAL, executor, scheduler, registries; (c) the binaries (`cloacina-server`, `cloacina-compiler`, `cloacinactl`); and (d) ancillaries (`cloacina-macros`, `cloacina-build`, `cloacina-python`, `cloacina-testing`). The split exists so packaged cdylibs don't transitively pull in pyo3, diesel, or kafka — see `cloacina-workflow/Cargo.toml` (just `serde_json`, `chrono`, optional macros).

### Workspace members (12 crates, `Cargo.toml:2`)

| Crate | One-line role | Cargo path |
|---|---|---|
| `cloacina` | Engine: runtime, DAL, executor, scheduler, registries, computation graph, packaging, security. `lib` + `cdylib` | `crates/cloacina` |
| `cloacina-workflow` | Minimal-dependency authoring surface (`Context`, `Task`, `TaskNamespace`, `Trigger`, retry types). Pulled in by every cdylib package | `crates/cloacina-workflow` |
| `cloacina-computation-graph` | Minimal CG types (`InputCache`, `GraphResult`, `SourceName`, `ReactionMode`, `Reactor` trait, `Graph` trait). bincode wire format helpers | `crates/cloacina-computation-graph` |
| `cloacina-workflow-plugin` | The fidius plugin interface (`CloacinaPlugin` trait v2) + wire-format types + the unified `cloacina::package!()` shell macro | `crates/cloacina-workflow-plugin` |
| `cloacina-macros` | Proc-macros: `#[task]`, `#[workflow]`, `#[trigger]`, `#[reactor]`, `#[computation_graph]`, `#[passthrough_accumulator]` and friends | `crates/cloacina-macros` |
| `cloacina-build` | Build-script helpers used by `cloacina-server`, `cloacina-python`, examples | `crates/cloacina-build` |
| `cloacina-python` | pyo3 bindings + `cloaca` Python module entrypoint. Hosts the `PythonRuntime` impl that core dispatches into | `crates/cloacina-python` |
| `cloacina-server` | Axum-based HTTP API + WebSocket endpoints. `bin: cloacina-server`, `lib: cloacina_server` | `crates/cloacina-server` |
| `cloacina-compiler` | DB-queue-driven cargo build service. Polls `workflow_packages` for `pending` rows, builds, writes cdylib bytes back. `bin: cloacina-compiler` | `crates/cloacina-compiler` |
| `cloacinactl` | CLI: `daemon`, `server`, `compiler`, `package`, `workflow`, `graph`, `execution`, `tenant`, `key`, `trigger`, `status`, `config`, `admin`, `completions` | `crates/cloacinactl` |
| `cloacina-testing` | Test harness helpers (assertions, mocks, boundary helpers) | `crates/cloacina-testing` |

Examples are excluded from the workspace (`Cargo.toml:3`) so they compile as independent crates against published deps — important for the packaged-cdylib path where examples build standalone .so/.dylibs.

### Core crate directory conventions (`crates/cloacina/src/`)

Each subsystem is a module with a `mod.rs` and (typically) a sibling `tests/` integration directory under `crates/cloacina/tests/integration/`:

- `computation_graph/` — accumulators, reactors, scheduler, registry, types, packaging bridge
- `dal/unified/` — backend-agnostic DAL with per-table modules (`task_execution/`, `schedule/`, `schedule_execution/`, `workflow_packages/`, `api_keys/`, `checkpoint.rs`, `task_outbox.rs`, etc.)
- `database/` — connection pool wrappers (deadpool-diesel), migrations, schema (diesel macros), `UniversalUuid`/`UniversalTimestamp`/`UniversalBool` type aliases
- `dispatcher/` — push-based task dispatch (replaces older polling-based dispatch); `DefaultDispatcher`, `Router`, `TaskExecutor` trait
- `execution_planner/` — task scheduler loop, context manager, state manager, trigger rules, stale-claim sweeper
- `executor/` — `ThreadTaskExecutor`, `WorkflowExecutor` trait, slot-based concurrency, task handles
- `models/` — diesel model definitions per table
- `packaging/` — `package_workflow()` driver + manifest schemas + `package.toml` parsing + platform helpers
- `python_runtime.rs` — dispatch trait the server uses to invoke `cloacina-python`
- `registry/` — workflow package registry (database + filesystem backends), reconciler that loads/unloads packages, package loader, task registrar
- `runner/default_runner/` — `DefaultRunner` (the embedded entry point) + service manager, configuration, cron API
- `runtime.rs` — `Runtime` struct (scoped registries: tasks, workflows, triggers, CGs, trigger-less CGs, reactors, stream backends)
- `security/` — API keys, audit, package signing (Ed25519), DB-backed key manager, package verification
- `task/` — task namespace
- `trigger/` — `Trigger` trait + `TriggerResult` (`Skip` | `Fire(Option<Context>)`)
- `workflow/` — workflow definition (`Workflow`, `WorkflowBuilder`, `DependencyGraph`)
- `cron_evaluator.rs`, `cron_recovery.rs`, `cron_trigger_scheduler.rs` — unified scheduler that drives cron + custom-poll triggers
- `crypto/` — key encryption, signing
- `inventory_entries.rs` — re-exports of macro-emitted `inventory::submit!` types

## 3. Key Entrypoints

### Binaries

| Binary | Source | Notes |
|---|---|---|
| `cloacina-server` | `crates/cloacina-server/src/main.rs:67` (`#[tokio::main] async fn main`) → calls `cloacina_server::run()` at `crates/cloacina-server/src/lib.rs:112` | Axum HTTP API. Args: `--bind`, `--database-url` (env `DATABASE_URL`), `--bootstrap-key` (env `CLOACINA_BOOTSTRAP_KEY`), `--require-signatures`, `--reconcile-interval-s`, `--home`. Always uses Postgres in default features (`crates/cloacina-server/Cargo.toml:23`). Default bind `127.0.0.1:8080`. |
| `cloacina-compiler` | `crates/cloacina-compiler/src/main.rs:89` → `cloacina_compiler::run()` at `crates/cloacina-compiler/src/lib.rs:39` | Build service. Args: `--bind` (default `127.0.0.1:9000`), `--database-url`, `--poll-interval-ms` (default 2000), `--heartbeat-interval-s`, `--stale-threshold-s`, `--sweep-interval-s`, `--cargo-flag`, `--cargo-target-dir`. Polls `workflow_packages` for `build_status='pending'`, runs cargo, writes cdylib bytes back. |
| `cloacinactl` | `crates/cloacinactl/src/main.rs:227` → `run()` at line 238 | CLI noun-verb dispatcher. Top-level subcommands: `daemon`, `server`, `compiler`, `package`, `workflow`, `graph`, `execution`, `tenant`, `key`, `trigger`, `status`, `config`, `admin`, `completions`. Strict noun-verb except for documented `status` exception (`crates/cloacinactl/src/nouns/mod.rs:37`). |

The `daemon` and `server` and `compiler` nouns each have `start`/`stop`/`status`/`health` verbs (`crates/cloacinactl/src/nouns/daemon/mod.rs`, `server/mod.rs`, `compiler/mod.rs`). The `daemon start` path lands in `crates/cloacinactl/src/commands/daemon.rs:121` (the `pub async fn run()` that wires DefaultRunner+SQLite+filesystem-watch+reconciler).

### Library entrypoints

- **Embedded `cloacina`**: `cloacina::runner::DefaultRunner::new(database_url).await` (`crates/cloacina/src/runner/default_runner/mod.rs:80`) or `DefaultRunner::with_config(...)` (line 102) or `DefaultRunner::builder().database_url(...).schema(...).build().await`. The runner constructs a scoped `Runtime`, runs migrations, instantiates `TaskScheduler`, `ThreadTaskExecutor`, `DefaultDispatcher`, and the `ServiceManager`, and starts background services.
- **Server library**: `cloacina_server::run(home, bind, database_url, verbose, bootstrap_key, require_signatures, reconcile_interval)` (`crates/cloacina-server/src/lib.rs:112`).
- **Compiler library**: `cloacina_compiler::run(config: CompilerConfig)` (`crates/cloacina-compiler/src/lib.rs:39`).
- **Public prelude**: `cloacina::prelude::*` (`crates/cloacina/src/lib.rs:453`) re-exports the most-used types — `Context`, `Task`, `TaskRegistry`, `TaskState`, `Workflow`, `WorkflowBuilder`, `Trigger`, `RetryPolicy`, `TaskScheduler`, `WorkflowExecutor`, `DefaultRunner`, `UniversalBool/Timestamp/Uuid`, plus the macros when `feature = "macros"`.
- **Re-exports of computation graph types**: `cloacina::computation_graph::*` (`crates/cloacina/src/lib.rs:490`) and a workspace-style re-export `cloacina_computation_graph` from the engine crate root for packaged CGs that use full paths (`lib.rs:435`).

### Python entry

- **PyO3 module entrypoint**: `cloaca` is registered via `#[pymodule] fn cloaca` at `crates/cloacina-python/src/lib.rs:87`. Maturin points at this crate (per the docstring) — moved here from the engine crate by CLOACI-T-0529 to stop pyo3 leaking into non-Python binaries.
- **Public surface** (registered in the pymodule): `task` (decorator), `trigger` (decorator), `reactor` (class decorator), `passthrough_accumulator`/`stream_accumulator`/`polling_accumulator`/`batch_accumulator` (decorators), `node`, `WorkflowBuilder`, `PyWorkflow`, `PyDefaultRunner`, `PyContext`, `Context`, `TriggerResult`, `PyRetryPolicy`/`PyRetryPolicyBuilder`/`PyBackoffStrategy`/`PyRetryCondition`, `PyTaskNamespace`, `PyWorkflowContext`, `PyComputationGraphBuilder`, `PyDatabaseAdmin` (postgres feature), `PyTenantConfig`, `PyTenantCredentials`, `var`/`var_or` (env-var helpers), `register_workflow`.
- **Server-side install**: `cloacina-server` calls `cloacina_python::install()` at startup (`crates/cloacina-server/src/lib.rs:125`) which registers `CloacinaPythonRuntime` (`crates/cloacina-python/src/runtime_impl.rs`) into core's `python_runtime` dispatch slot. The compiler service deliberately does not (CLOACI-T-0529).
- **Loader path for uploaded Python packages**: `import_and_register_python_workflow` and `import_python_computation_graph` in `crates/cloacina-python/src/loader.rs`. The loader unpacks the `.cloacina` archive, validates a stdlib deny-list (`STDLIB_DENY_LIST` at `loader.rs:43`), pushes a `PyWorkflowContext`, imports the module (which fires `@task`/`@trigger` decorator side-effects against the current `ScopedRuntime`), then pops. The `cloaca` module is synthesized into `sys.modules` if not already importable via `ensure_cloaca_module()` (line 98).

### Startup / shutdown lifecycle

**`cloacina-server`** (`crates/cloacina-server/src/lib.rs:112-345`):
1. `cloacina_python::install()` — register Python runtime impl into core
2. Set up rolling-file + stderr tracing layers; conditional OTLP layer if `OTEL_EXPORTER_OTLP_ENDPOINT` is set (`telemetry` feature)
3. Install Prometheus recorder + describe metrics (`cloacina_workflows_total`, `cloacina_tasks_total`, `cloacina_api_requests_total`, etc.)
4. Build `DefaultRunner` with `registry_storage_backend("database")`
5. Construct `EndpointRegistry` and `ComputationGraphScheduler::with_dal(...)` and wire scheduler into runner via `runner.set_graph_scheduler(scheduler.clone())`
6. Build `AppState` (database, runner, key cache, endpoint registry, graph scheduler, security config, ws tickets, metrics handle, tenant DB cache)
7. `bootstrap_admin_key` — create initial admin key + persist plaintext to `~/.cloacina/bootstrap-key` mode 0600
8. `build_router(state)` (line 371) wires routes
9. Listen on TCP and `axum::serve(listener, app).with_graceful_shutdown(...)`
10. Shutdown order: SIGINT/SIGTERM → cancel `shutdown_tx` → graph scheduler `shutdown_all().await` → runner `shutdown()` (30s timeout)

**`cloacinactl daemon start`** (`crates/cloacinactl/src/commands/daemon.rs:121`):
1. Create home dir + logs dir, install dual logging
2. Open SQLite DB at `~/.cloacina/cloacina.db?mode=rwc&_journal_mode=WAL`
3. Build `DefaultRunner` with SQLite
4. Spawn health socket (`~/.cloacina/daemon.sock`) + health pulse (60s)
5. Build `FilesystemWorkflowRegistry` over watch directories (default + CLI + config-file)
6. `RegistryReconciler::new(registry, config, shutdown_rx)` and run **initial reconciliation**
7. Spawn `PackageWatcher` (`notify` crate, debounced) — emits reconcile signals on filesystem events
8. Event loop: filesystem-change → reconcile; `interval(poll_ms)` → periodic reconcile; SIGINT/SIGTERM → break; SIGHUP → reload config + diff watch dirs
9. Shutdown: send shutdown signal, abort health tasks, race `runner.shutdown()` vs `shutdown_timeout` vs second SIGINT (force-exit)

**`cloacina-compiler`** (`crates/cloacina-compiler/src/lib.rs:39`):
1. Install logging (file + stderr)
2. Open Postgres pool, run migrations
3. Build `WorkflowRegistryImpl` over `UnifiedRegistryStorage`
4. Spawn HTTP /health + /v1/status endpoint (`crates/cloacina-compiler/src/health.rs`)
5. Run `loopp::run(registry, config, shutdown)` — polling build loop with sweeper for stale `building` rows
6. SIGINT → cancel CancellationToken → join HTTP handle, exit

### Modes of operation

- **Fully embedded**: `DefaultRunner::new(...)` linked into your binary; tasks compiled in via `inventory::submit!` from macros; SQLite or Postgres.
- **Daemon mode**: `cloacinactl daemon start` runs SQLite-backed long-lived process that watches a directory for `.cloacina` packages and reconciles them. No HTTP API.
- **Server mode**: `cloacina-server` runs Postgres-backed HTTP service with API key auth, multipart workflow upload, multi-tenant support, WebSocket endpoints for accumulators/reactors. Pairs with one or more `cloacina-compiler` instances.
- **Compiler mode**: `cloacina-compiler` is a stateless build worker — multiple instances coordinate via DB-row claiming.
- **Python authoring**: import `cloaca`, decorate functions, instantiate `cloaca.DefaultRunner(database_url)`, call `runner.execute(name, context)`. Tutorials live under `examples/tutorials/python/`.

## 4. Architecture

### Major components

```
┌─────────────────────────────────────────────────────────────────────┐
│                        CLI / Server / Daemon                        │
│  cloacinactl    cloacina-server    cloacinactl daemon               │
└──────┬──────────────────┬──────────────────────┬────────────────────┘
       │                  │                      │
       ▼                  ▼                      ▼
┌──────────────────────────────────────────────────────────────────────┐
│                          DefaultRunner                               │
│                  (crates/cloacina/src/runner)                        │
│                                                                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ Runtime  │  │ Service  │  │ TaskScheduler│  │ Default         │  │
│  │ (scoped  │  │ Manager  │  │ (planner)    │  │ Dispatcher      │  │
│  │ regs)    │  └──────────┘  └──────────────┘  └─────────────────┘  │
│  └──────────┘                                                        │
│                                                                      │
│  Background services managed by ServiceManager:                      │
│    - SchedulerLoop       - StaleClaimSweeper      - CronRecovery     │
│    - Unified Scheduler   - RegistryReconciler     - GraphScheduler   │
└──────────────────────────────────────────────────────────────────────┘
            │                                            │
            ▼                                            ▼
┌────────────────────────────────┐          ┌───────────────────────────┐
│   Unified DAL (Diesel)          │          │  Computation Graph        │
│   - workflow_executions          │          │  Scheduler                │
│   - task_executions              │          │  (reactors + accumulators)│
│   - task_outbox                  │          │  - EndpointRegistry       │
│   - schedules / schedule_execs   │          │  - WS handlers            │
│   - workflow_packages            │          │  - StreamBackend          │
│   - workflow_registry            │          └───────────────────────────┘
│   - api_keys / signing_keys      │
│   - accumulator_checkpoints      │
│   - reactor_state                │
│   - state_accumulator_buffer     │
│   - execution_events             │
│   - recovery_events              │
└────────────────────────────────┘
            │
            ▼
┌────────────────────────────────┐
│    Database (PG or SQLite)      │
└────────────────────────────────┘
```

### Executor

Executor lives at `crates/cloacina/src/executor/` (`mod.rs:47-60`).
- `TaskExecutor` trait (`crates/cloacina/src/dispatcher/traits.rs`) — receives `TaskReadyEvent`s pushed by the dispatcher.
- `ThreadTaskExecutor` (`crates/cloacina/src/executor/thread_task_executor.rs`) — the default implementation. Runs tasks on a tokio task pool, atomic claim via DB lock (`task_outbox` row), heartbeat updates, semaphore-based concurrency limit (`ExecutorConfig.max_concurrent_tasks`), per-task timeout cancellation.
- `WorkflowExecutor` trait + impl (`crates/cloacina/src/executor/workflow_executor.rs`) — coordinates a workflow's full lifecycle (return type `WorkflowExecutionResult`, status callbacks).
- `TaskHandle` (`crates/cloacina/src/executor/task_handle.rs`) — exposes the running task's runtime context; supports the `with_task_handle` / `take_task_handle` / `return_task_handle` thread-local pattern used by `#[task(invokes = computation_graph(...))]`.
- `SlotToken` (`slot_token.rs`) — RAII concurrency slot.

### Scheduler

Two scheduling layers exist and they are orthogonal:

1. **Task scheduler** (`crates/cloacina/src/execution_planner/`) — converts `Workflow` definitions into rows in `workflow_executions` + `task_executions`, evaluates trigger rules and dependency state, and marks ready tasks for dispatch via the `Dispatcher`. Pieces:
   - `TaskScheduler` (`crates/cloacina/src/execution_planner/mod.rs:145+`) — public type
   - `SchedulerLoop` (`scheduler_loop.rs`) — background loop that polls `task_executions` for state transitions
   - `ContextManager` (`context_manager.rs`) — merges contexts across multi-dependency edges
   - `StateManager` (`state_manager.rs`) — task state machine (NotStarted → Pending → Ready → Running → Completed/Failed/Skipped/Abandoned)
   - `TriggerRule` (`trigger_rules.rs`) — conditional task execution by context predicate
   - `StaleClaimSweeper` (`stale_claim_sweeper.rs`) — background loop releasing stale heartbeat-expired claims
2. **Cron + custom-poll trigger scheduler** (`crates/cloacina/src/cron_trigger_scheduler.rs`) — unified `Scheduler` that drives both cron schedules and custom-poll `Trigger` impls. One run loop ticks at `trigger_base_poll_interval` (default 1s) and every `cron_poll_interval` (default 30s) queries due cron schedules. Atomic claim prevents duplicate cron firings across instances. Records every handoff in `schedule_executions`.

### Reactor

A **reactor** in Cloacina is a specialized trigger that consumes accumulator boundary events and fires a downstream computation graph traversal when its firing criteria are met. Per CLOACI-S-0011 (`/.metis/specifications/CLOACI-S-0011/specification.md`), reactor is a **noun**, not a subsystem — and "reactive scheduler", "reactive computation graph", and "reactive subsystem" are banned terms.

Implementation:
- Type: `Reactor` (`crates/cloacina/src/computation_graph/reactor.rs:233-265`) — owns a `CompiledGraphFn`, a `ReactionCriteria` (`WhenAny` | `WhenAll`), an `InputStrategy` (`Latest` | `Sequential`), a `DirtyFlags` per source, and channels for boundary input, manual commands, and shutdown.
- States (`ReactorHealth`, `reactor.rs:47-59`): `Starting` | `Warming` | `Live` | `Degraded`.
- Operator commands (`ReactorCommand`, line 175): `ForceFire`, `FireWith`, `GetState`, `Pause`, `Resume`. Sent over the `/v1/ws/reactor/{name}` WebSocket; auth-gated by `ReactorAuthPolicy` with per-operation permissions (`crates/cloacina/src/computation_graph/registry.rs:172`).
- Wire-format trait: `cloacina_computation_graph::Reactor` (`crates/cloacina-computation-graph/src/lib.rs:325`) — provides `NAME`, `ACCUMULATORS`, `REACTION_MODE` const associated items used by macros.
- Runtime-side description: `ReactorRegistration` (line 339) with `name`, `accumulator_names`, `reaction_mode`.

### Computation graph (CG)

The CG is the execution quantum for the reactor pathway. Once triggered, the entire DAG of nodes runs as a single unit using in-memory channels.
- Macro: `#[computation_graph]` (`crates/cloacina-macros/src/lib.rs:137`); declares topology via `react = when_any(alpha, beta)` and `graph = { node(deps) => { ... } }` arms; nodes are async functions in the module body.
- Compiled form: `CompiledGraphFn = Arc<dyn Fn(InputCache) -> Pin<Box<dyn Future<Output = GraphResult>>>>` (`crates/cloacina-computation-graph/src/lib.rs:247`).
- Outputs: `GraphResult::Completed { outputs: Vec<Box<dyn Any + Send>> }` or `Error(GraphError)`.
- Trigger-less variant: `TriggerlessGraph` / `TriggerlessGraphFn` (`crates/cloacina/src/computation_graph/triggerless.rs`) — operates on `Context<Value>`, invocable directly by a workflow task (the `#[task(invokes = computation_graph("name"))]` form).
- Scheduler: `ComputationGraphScheduler` (`crates/cloacina/src/computation_graph/scheduler.rs:267-279`). Owns running graphs, restart-on-panic supervision (`check_and_restart_failed`, `start_supervision`), exponential backoff with cap (line 254-264). Wired into `cloacina-server` `AppState` and into `DefaultRunner` via `runner.set_graph_scheduler(...)`.

### Accumulator

An **accumulator** is the reactor's stream-input adapter. It consumes an external stream and emits boundary events to the reactor.
- Trait: `Accumulator` (`crates/cloacina/src/computation_graph/accumulator.rs:100`) — `process(event) -> Option<Output>` plus optional `init`.
- Variants: passthrough (`PassthroughAccumulatorFactory`), polling (`PollingAccumulator`, periodic async poll function), batch (`BatchAccumulator` with `flush_interval` and/or `max_buffer_size`), state (`StateAccumulator` — bounded VecDeque buffer), stream-backed (`StreamBackendAccumulatorFactory`, `accumulator.rs` + `stream_backend.rs`).
- Health: `AccumulatorHealth` (`Starting | Connecting | Live | Disconnected | SocketOnly`, line 42-53). Reported via `tokio::sync::watch` and aggregated by the endpoint registry.
- Stream backends: `StreamBackend` trait + registry (`crates/cloacina/src/computation_graph/stream_backend.rs`); concrete impls include `KafkaStreamBackend` (gated `feature = "kafka"`, uses `rdkafka` 0.39) and `MockBackend`.
- Persistence: `CheckpointHandle` writes accumulator state to `accumulator_checkpoints` table; boundary buffers flushed to `state_accumulator_buffer` for state accumulators; reactor state to `reactor_state` (snapshot of the cache + dirty flags).

### Packaging system

- Packages are bzip2-compressed source tarballs with extension `.cloacina`, packed via `fidius_core::package::pack_package` (`crates/cloacina/src/packaging/mod.rs:65`).
- Manifest: `package.toml` containing `[package]` (Cargo-like name/version + interface metadata) + `[metadata]` (`CloacinaMetadata` struct in `crates/cloacina-workflow-plugin/src/types.rs:288`). `#[serde(deny_unknown_fields)]` rejects legacy `package_type` and `[[triggers]]` keys with a friendly migration hint (`crates/cloacina/src/registry/reconciler/loading.rs:172-200`).
- Plugin ABI: `CloacinaPlugin` trait v2 (`crates/cloacina-workflow-plugin/src/lib.rs:712`) — 9 methods total, the last 7 marked `#[optional(since = 2)]` for graceful degradation. Method indices are pinned constants (`METHOD_GET_TASK_METADATA = 0` … `METHOD_INVOKE_TRIGGERLESS_GRAPH = 8`).
- Unified shell macro: `cloacina::package!()` (`crates/cloacina-workflow-plugin/src/lib.rs:110`) — single-line invocation at crate root that emits a `pub mod _ffi { ... }` containing the entire plugin impl. The body walks `inventory::iter::<TaskEntry>` / `WorkflowDescriptorEntry` / `ComputationGraphEntry` / `ReactorEntry` / `TriggerEntry` / `TriggerlessGraphEntry` and projects each into wire-format metadata. Single-emission is enforced by a `__cloacina_package_marker` module guard.
- Registry storage: `WorkflowRegistryImpl` (`crates/cloacina/src/registry/workflow_registry/mod.rs:43`) over a `RegistryStorage` backend. Backends: `UnifiedRegistryStorage` (DB-backed, `crates/cloacina/src/dal/unified/workflow_registry_storage.rs`) and `FilesystemRegistryStorage` (`crates/cloacina/src/registry/storage/`).
- Filesystem registry for the daemon: `FilesystemWorkflowRegistry` (`crates/cloacina/src/registry/workflow_registry/filesystem.rs`) — wraps watch directories.
- Reconciler: `RegistryReconciler` (`crates/cloacina/src/registry/reconciler/mod.rs`); driver in `loading.rs`. Loads packages by:
  1. `WorkflowRegistry::get_workflow` (success rows only) — returns source archive + cdylib bytes
  2. Unpack source via `fidius_core::package::unpack_package`
  3. Load + validate manifest
  4. Dispatch by language: Rust → write cdylib bytes to temp file, dlopen via fidius-host, drive metadata FFI calls; Python → cloacina-python's loader path imports the entry module
  5. Register tasks/workflows/triggers/reactors/CGs into the runner's `Runtime`; cron-shaped triggers also register schedules through the unified scheduler
  6. The post-I-0102 precedence-ordered loader is **in flight on this branch** (the spec is locked but the rebuild around the fixed pipeline is being landed by T-A through T-E — see CLOACI-I-0102 in `.metis/initiatives/`)
- Compiler service: `cloacina-compiler` polls `workflow_packages.build_status = 'pending'` rows, claims by setting status to `building` with heartbeat, runs cargo, writes resulting cdylib bytes back into `compiled_data`. Sweeper resets stuck `building` rows past `stale_threshold_s` (`crates/cloacina-compiler/src/loopp.rs`). Content-hash artifact reuse: a fresh row whose source hash matches an earlier `success` row reuses its compiled bytes (`workflow_registry/mod.rs:325` — `find_success_by_hash`).

### Storage / DAL

- **Backend selection is runtime, not compile-time** — `Database::new` (`crates/cloacina/src/database/connection/mod.rs`) parses the URL scheme and constructs either a Postgres or SQLite deadpool. Both backends are compiled in by default via the `postgres` and `sqlite` features (`crates/cloacina/Cargo.toml:20`).
- **Universal types** for cross-backend compat: `UniversalUuid`, `UniversalTimestamp`, `UniversalBool`, `UniversalBinary` (`crates/cloacina/src/database/universal_types.rs`). Each table has Unified*Model + NewUnified*Model rows.
- **DAL** (`crates/cloacina/src/dal/unified/mod.rs:93`): single `DAL` struct with per-area accessors (`api_keys()`, `checkpoint()`, `context()`, `workflow_execution()`, `task_execution()`, `task_execution_metadata()`, `task_outbox()`, `recovery_event()`, `execution_event()`, `schedule()`, `schedule_execution()`, `workflow_packages()`, `workflow_registry()`). Each module has `_postgres` and `_sqlite` paired functions and the public method dispatches based on `dal.backend()`.
- Tables (visible from the unified types): contexts, workflow_executions, task_executions, task_execution_metadata, task_outbox, recovery_events, execution_events, schedules, schedule_executions, workflow_registry, workflow_packages, signing_keys, trusted_keys, key_trust_acls, package_signatures, api_keys, accumulator_checkpoints, accumulator_boundaries, reactor_state, state_accumulator_buffers.
- Migrations are applied at runtime by `database.run_migrations().await` during `DefaultRunner::with_config` (`crates/cloacina/src/runner/default_runner/mod.rs:111`).

### Multi-tenant model

- **Postgres**: schema-based isolation. `Database::try_new_with_schema(url, "cloacina", pool, Some(tenant_id))` creates a pool whose `search_path` includes the tenant schema. Tenant creation/removal via `DatabaseAdmin` (`crates/cloacina/src/database/admin.rs`, gated `feature = "auth"`/`postgres`). Server admin keys can manage tenants via `POST /v1/tenants` / `DELETE /v1/tenants/{name}`. Per-tenant DBs are cached in `TenantDatabaseCache` (`crates/cloacina-server/src/lib.rs:43-92`), lazily populated.
- **SQLite**: file-per-tenant (`sqlite://./tenant_a.db`).
- `auth` feature (postgres-only) adds the API-key DAL, key trust ACLs, and admin endpoints. The default `cloacina-server` feature set includes only `postgres`; the `cloacinactl` default includes `postgres,sqlite,kafka`.

### Async runtime

- All async paths use **tokio 1.x with `features = ["full"]`**. The server, daemon, compiler, and Python bindings each construct a tokio runtime in `#[tokio::main]` or — for the Python single-thread-bound case — a dedicated thread (`crates/cloacina-python/src/bindings/runner.rs:23-47`, with shutdown signals via `mpsc + oneshot`).
- Packaged cdylibs **own their own tokio runtime** to bridge the FFI boundary. `cloacina::package!()` initializes `OnceLock<Runtime>` lazily for each shape — see `cdylib_runtime` patterns in `cloacina-workflow-plugin/src/lib.rs:217-221` (task), `333-340` (CG), `480-487` (trigger), `577-583` (trigger-less CG).
- Channels: `tokio::sync::mpsc` (work queues), `watch` (shutdown, health), `oneshot` (RPC responses), `parking_lot::RwLock` (registries).

### Plugin / FFI bridge ("fidius")

`fidius` is a third-party plugin ABI library used as the FFI transport. Cloacina depends on `fidius-host = "0.2.1"`, `fidius-core = "0.2.1"`, and `fidius` (re-exported through `cloacina-workflow-plugin`).
- Wire format per memory note: debug builds use JSON, release builds use bincode. Single-arg methods need a `(T,)` tuple in `call_method`.
- The `#[fidius::plugin_interface(version = 2, buffer = PluginAllocated)]` macro on `CloacinaPlugin` (`crates/cloacina-workflow-plugin/src/lib.rs:712`) generates the host call site + plugin impl scaffold. `#[optional(since = 2)]` marks methods that older v1 plugins may not implement — the host receives `CallError::NotImplemented { bit }` and treats it as "no entries of that kind".
- Host loading: `fidius_host::loader::load_library(path)` returns a `Loaded`; `fidius_host::PluginHandle::from_loaded(plugin)` produces the call handle. Used in `crates/cloacina/src/registry/loader/package_loader.rs` (extract metadata) and `crates/cloacina/src/registry/reconciler/loading.rs:70` (load fresh handle from cdylib bytes for trigger registration). Method indices: `METHOD_GET_TASK_METADATA = 0` … `METHOD_INVOKE_TRIGGERLESS_GRAPH = 8`.
- The packaged-CG path uses an additional bridge module: `crates/cloacina/src/computation_graph/packaging_bridge.rs`. `LoadedGraphPlugin` keeps a `PluginHandle` + temp dir alive; `execute_graph_via_ffi` dispatches a graph invocation across the boundary; `dispatch_runtime_reactors_into_scheduler` and `dispatch_package_reactors_into_scheduler` (lines 548, 617) wire reactor declarations from in-process inventory or packaged metadata into the `ComputationGraphScheduler`.

## 5. Primary Workflows

### W1 — Submit & execute a workflow via CLI (sqlite, single-process)

1. Author writes `#[task]`/`#[workflow]` Rust code linked into a binary.
2. Macros (`crates/cloacina-macros/src/`) emit `inventory::submit!` entries for `TaskEntry`, `WorkflowEntry`, etc., and the workflow gets a content fingerprint.
3. At binary startup: `let runner = DefaultRunner::new("sqlite://./cloacina.db").await?;`
4. `DefaultRunner::with_config` (`crates/cloacina/src/runner/default_runner/mod.rs:102`) constructs `Runtime::new()` which calls `seed_from_inventory()` (`crates/cloacina/src/runtime.rs:127`), draining `inventory::iter::<TaskEntry>` and registering each constructor in the runtime's task registry. Migrations run.
5. Caller invokes `runner.execute_async("my_workflow", context).await` (or `runner.execute(...)`).
6. `WorkflowExecutor` (impl in `crates/cloacina/src/runner/default_runner/workflow_executor_impl.rs`) creates a `workflow_executions` row, materializes `task_executions` for every task in the DAG, evaluates initial readiness, inserts ready ones into `task_outbox`.
7. The `SchedulerLoop` (background, default 1s tick) detects ready tasks and emits `TaskReadyEvent`s to the `DefaultDispatcher` (`crates/cloacina/src/dispatcher/default.rs`).
8. The dispatcher routes by task name pattern (`RoutingConfig`) — by default, every task to `"default"` → the registered `ThreadTaskExecutor`.
9. `ThreadTaskExecutor` claims the task atomically (DB row update with heartbeat), loads dependency context via `DependencyLoader`, runs the task with a per-task timeout, writes context updates back (atomic), updates state. On failure: increments retry counter via `RetryPolicy` and either retries or marks failed.
10. `WorkflowExecutor` polls until all tasks reach a terminal state, returns `WorkflowExecutionResult` (status, exec_id, final context).
- **Notable error path**: dependency failure short-circuits — downstream tasks marked `Skipped`. Failed tasks past `RetryPolicy.max_attempts` mark the workflow `Failed` with `reason="dependency_failed"` (the metric `cloacina_workflows_total{reason}` enumerates this).

### W2 — Server: triggered event → schedules → packaged workflow execution

1. Operator: `cloacinactl tenant create my_tenant ...` against `cloacina-server`.
2. Operator: `cloacinactl package publish ./my-pkg --release` — runs `cargo build`, packs via `fidius_core::package::pack_package`, optionally signs (Ed25519, `crates/cloacina/src/security/`), uploads via `POST /v1/tenants/my_tenant/workflows` (multipart) — a `.cloacina` archive.
3. Server's `upload_workflow` handler (`crates/cloacina-server/src/routes/workflows.rs:36`):
   - Auth check (`AuthenticatedKey.can_access_tenant` + `can_write`)
   - If `require_signatures`, calls `cloacina::security::verify_package_bytes` against the tenant's `verification_org_id` trusted-key list
   - Calls `WorkflowRegistry::register_workflow(bytes)` which: validates bzip2 prefix, unpacks to temp dir, parses `package.toml` via `fidius_core::package::load_manifest::<CloacinaMetadata>`, computes content hash, supersedes any prior active row, stores binary, inserts a `pending` row in `workflow_packages`
4. A `cloacina-compiler` worker polls, claims the row (status → `building`, heartbeat), reads the source archive via `get_source_for_build`, runs `cargo build --release --lib` in a per-package work dir (with `CARGO_TARGET_DIR` if configured for sharing), reads the resulting `.so`/`.dylib`, writes bytes into `compiled_data`, marks `success`.
5. The server's `RegistryReconciler` (running as a service inside `DefaultRunner`) detects the new success row at the next tick, calls `load_package(metadata)` (`crates/cloacina/src/registry/reconciler/loading.rs:111`):
   - Fetches archive + compiled bytes
   - For Rust packages: writes cdylib to a temp file, `fidius_host::loader::load_library`, calls plugin methods 0–8 to extract metadata (tasks, workflows, graphs, reactors, triggers, trigger-less graphs)
   - Routes each kind through the precedence-ordered loader (cron triggers → custom triggers → reactors → trigger-less CGs → reactor-bound CGs → workflows)
   - For workflows: registers the workflow constructor (a closure that returns a `Workflow`) into the scoped Runtime; for triggers in metadata, binds them
6. Trigger fires (cron tick or external event). The unified scheduler claims the schedule row, builds an initial context, and either:
   - Enqueues a `workflow_executions` row + ready tasks (cron/triggered workflow path), or
   - Pushes a boundary into the reactor (reactor + CG path).
7. Tasks in the DAG eventually call into the cdylib via `execute_task` FFI calls, dispatched by the `FfiTaskAdapter` registered at load time.
- **Notable error paths**: package upload returns 4xx for empty file, missing `file` field, bad bzip2 magic, manifest parse failure (with the migration hint for `package_type` / `[[triggers]]`), or signature verification failure when `require_signatures` is on but no `verification_org_id` is set (returns 403 `signature_verification_unconfigured`).

### W3 — Daemon: cron schedule fires → enqueues run

1. Operator drops a `.cloacina` package into `~/.cloacina/packages/` (or another `--watch-dir`).
2. The `notify`-based `PackageWatcher` (`crates/cloacinactl/src/commands/watcher.rs`) emits a debounced reconcile signal.
3. The daemon's `RegistryReconciler` sees the new file (`FilesystemWorkflowRegistry::list_workflows` enumerates packages in the watch directories), unpacks, registers tasks/workflows.
4. If the package's `get_trigger_metadata` returns cron-shaped entries, the reconciler's `step_load_cron_triggers` calls `CronWorkflowRegistrar` (the `DalCronRegistrar` injected at startup) which inserts/upserts a row in `schedules` with `schedule_type = 'cron'` + `cron_expression` + `timezone`.
5. Custom-poll triggers go through `register_triggers_from_reconcile` (`crates/cloacinactl/src/commands/daemon.rs:405`) — for each non-cron trigger, the daemon calls `runtime.get_trigger(name)` (which resolves through the scoped runtime's trigger registry, including `FfiTriggerImpl` adapters for cdylib-declared triggers) and `scheduler.register_trigger`.
6. The unified `Scheduler` run loop (`crates/cloacina/src/cron_trigger_scheduler.rs`) ticks every 1s. Every 30s (configurable) it queries `dal.schedule().get_due_cron_schedules(now)` for schedules whose `next_run_at <= now`. It claims atomically via `claim_and_update_cron`, builds a context, and asks the `WorkflowExecutor` to schedule a new execution.
7. The schedule's row is updated with `last_run_at`, `next_run_at` (computed via `croner` + `chrono-tz`), and a row in `schedule_executions` records the handoff.
8. The new `workflow_executions` row flows through the same execute pipeline as W1.
- **Notable error path**: catchup mode handles missed cron firings up to `max_catchup_executions` (`SchedulerConfig`, default 100). `CronRecoveryService` (`crates/cloacina/src/cron_recovery.rs`) repairs schedules whose handoff was lost (lost_executions sweeper).

### W4 — Python user: `@task` DAG → runs in-process

1. User writes Python:
   ```python
   import cloaca
   @cloaca.task(id="extract")
   async def extract(ctx): ctx["raw"] = fetch()
   @cloaca.task(id="load", dependencies=["extract"])
   async def load(ctx): persist(ctx["raw"])
   with cloaca.WorkflowBuilder("etl") as wb:
       wb.add_task(extract); wb.add_task(load)
   runner = cloaca.DefaultRunner("sqlite://./pipeline.db")
   runner.execute("etl", cloaca.Context())
   ```
2. `import cloaca` triggers `#[pymodule] fn cloaca` (`crates/cloacina-python/src/lib.rs:87`) which installs a process-default `ScopedRuntime` keyed thread-locally (`runtime_scope::current_runtime`) — the empty `cloacina::Runtime`.
3. `@cloaca.task` is `task::task` (the PyO3 function). Its returned `TaskDecorator.__call__` builds a `PythonTaskWrapper` (which implements `cloacina::Task`) and registers it into the current ScopedRuntime keyed by `TaskNamespace`.
4. `WorkflowBuilder.__enter__` pushes a `PyWorkflowContext` (`tenant`, `package`, `workflow`); `__exit__` collects all tasks declared while the context was active, builds a `Workflow`, and registers it.
5. `cloaca.DefaultRunner(url)` constructs `PyDefaultRunner` (`crates/cloacina-python/src/bindings/runner.rs`) — a thin wrapper that spawns a dedicated tokio runtime thread and tunnels `RuntimeMessage`s via `mpsc` + `oneshot`.
6. `runner.execute("etl", ctx)` sends `RuntimeMessage::Execute` to the runtime thread, which calls `cloacina::executor::WorkflowExecutor::execute(...)` and returns the result over `oneshot`.
7. Inside the runner thread, the same scheduler/dispatcher/executor pipeline as W1 runs — `Task::execute` for Python tasks dispatches through `PythonTaskWrapper::execute` which acquires the GIL via pyo3 and calls the user's async function.

### W5 — Packaging: Rust source → compiled `.cloacina` → loaded by server/daemon

1. **Author** writes a Rust crate with `crate-type = ["cdylib"]` (and optionally `"rlib"` for testing), a `package.toml`, and uses `#[task]`/`#[workflow]` (or `#[reactor]`/`#[trigger]`/`#[computation_graph]`) macros plus the **single-line shell** `cloacina::package!();` at crate root (post-I-0102).
2. **Pack**: `cloacinactl package pack ./crate` — calls `cloacina::packaging::package_workflow` (`crates/cloacina/src/packaging/mod.rs:46`), validates structure (`validate_rust_crate_structure`, `validate_cargo_toml`, `validate_cloacina_compatibility`, `validate_packaged_workflow_presence`), checks `package.toml` existence, then `fidius_core::package::pack_package` produces a `.cloacina` archive (bzip2 tar of source).
3. **Optional sign**: `cloacinactl package pack --sign ./key.pem` — Ed25519 signature stored in `package_signatures` table on upload.
4. **Upload**: `cloacinactl package upload ./my-pkg.cloacina` (server) or drop into a watched directory (daemon).
5. **Compile** (server path only): `cloacina-compiler` claims the row, runs `cargo build --release --lib` on the unpacked source, writes `compiled_data` + marks `success`. Configurable via `--cargo-flag`. Shared `CARGO_TARGET_DIR` allows cross-package transitive-dep reuse.
6. **Load**: reconciler dlopens the cdylib (`fidius_host::loader::load_library` on a temp file), calls plugin methods 0/2/4/5/7 to extract metadata, then registers each primitive into the host runtime via the precedence-ordered loader.
- **Wire-format detail**: per the project memory, debug builds are JSON, release are bincode. Test packages are built in **debug mode** to match the test-binary's wire format (`crates/cloacina/tests/integration/...`).

### W6 — Recovery / restart: server crashes mid-execution → resume in-flight runs

1. State is persisted continuously: `workflow_executions.status`, `task_executions.status`, atomic claim + heartbeat (`task_outbox` rows + per-task heartbeat columns), and every state transition emits a row in `execution_events` for audit.
2. On restart, the new runner runs migrations, seeds the runtime from inventory, then starts background services. The `RegistryReconciler` reloads packages first.
3. The `StaleClaimSweeper` (`crates/cloacina/src/execution_planner/stale_claim_sweeper.rs`) periodically scans for `task_executions` whose claim heartbeat is older than the threshold; it releases the claim by resetting status to `Ready` (and increments a `recovery_event`).
4. The `SchedulerLoop` resumes: any `Pending`/`Ready` tasks become eligible for dispatch; the dispatcher pushes them through the executor.
5. `CronRecoveryService` (`crates/cloacina/src/cron_recovery.rs`) handles cron schedules whose firing window was missed during the outage — it either runs catchup executions or fast-forwards `next_run_at`, governed by `CatchupPolicy` (`crates/cloacina/src/models/schedule.rs`).
6. Computation graphs persisted state through `accumulator_checkpoints` / `accumulator_boundaries` / `reactor_state` / `state_accumulator_buffer`. On restart, `ComputationGraphScheduler::load_reactor` rehydrates each reactor's `InputCache` (`with_dal` path persists every fired snapshot via `persist_reactor_state` at `reactor.rs:670`) and accumulators read their last checkpoint via `CheckpointHandle::load`.
7. The supervision loop (`ComputationGraphScheduler::start_supervision`, `scheduler.rs:1106`) checks every 5s for crashed accumulator/reactor tasks and restarts with exponential backoff (capped at 60s, `BACKOFF_MAX_SECS` line 261), abandoning permanently after `MAX_RECOVERY_ATTEMPTS` (line 255).

### W7 — Reactor + computation graph end-to-end (the recently-shipped path)

1. Author declares a reactor and a CG either in the same crate or across two:
   ```rust
   #[reactor(name = "pricing_rx", accumulators = [orderbook, pricing], criteria = when_any(orderbook, pricing))]
   pub struct PricingRx;

   #[computation_graph(
       name = "score_signals",
       trigger = reactor("pricing_rx"),  // post-I-0102: string, not type path
       graph = { decision(orderbook, pricing) => { Signal -> publish, NoOp -> audit } }
   )]
   mod score_signals { /* ... */ }

   cloacina::package!();
   ```
2. Pack + upload + compile → server reconciler loads the cdylib, walks `inventory::iter::<ReactorEntry>` and `inventory::iter::<ComputationGraphEntry>` via the FFI metadata methods.
3. Loader precedence: reactors register first via `ComputationGraphScheduler::load_reactor` (`scheduler.rs:316`) which creates an `EndpointRegistry` entry per accumulator (`/v1/ws/accumulator/{name}`) plus the reactor command channel (`/v1/ws/reactor/{name}`).
4. The CG (subscriber) loads after, calling `ComputationGraphScheduler::bind_graph_to_reactor("score_signals", "pricing_rx", graph_fn)` (line 510). The scheduler validates the contract via `check_reactor_contract_matches` and rejects on mismatch.
5. External producer connects to `ws://server/v1/ws/accumulator/orderbook?token=<ws_ticket>`. Authentication: the WS ticket is exchanged via `POST /v1/auth/ws-ticket` (single-use, short-TTL) since browsers can't set custom headers on upgrades. After upgrade, raw bytes from the WS are forwarded into the accumulator's socket channel.
6. The accumulator runtime (one of `accumulator_runtime`, `polling_accumulator_runtime`, `batch_accumulator_runtime`, `state_accumulator_runtime` per shape) consumes the bytes, calls `Accumulator::process(event) -> Option<T>`, and on `Some(T)` calls `BoundarySender::send(&boundary)` which serializes via bincode and pushes through the merge channel into the reactor.
7. The reactor's main loop (`Reactor::run`, `reactor.rs:351-666`) updates the `InputCache` for the relevant `SourceName`, sets the `DirtyFlags` entry, and evaluates `ReactionCriteria`. On `WhenAny` with any flag set or `WhenAll` with every flag set, it snapshots the cache, clears dirty flags, and invokes `(graph_fn)(snapshot)`.
8. The compiled graph runs the topology — async nodes communicate via in-memory channels — and returns `GraphResult::Completed { outputs }` or `Error(...)`.
9. Operators can send `ForceFire` / `FireWith(InputCache)` / `Pause` / `Resume` / `GetState` over `/v1/ws/reactor/{name}`; results flow back as `ReactorResponse`.
- **Notable error paths**: accumulator disconnects mark health `Disconnected` (visible at `/v1/health/accumulators`); the reactor enters `Degraded`. Crash recovery runs the supervision loop. Contract mismatch (graph references unknown reactor or wrong accumulator set) hard-errors at load time.

## 6. Public Interface Surface

### REST/HTTP endpoints (`cloacina-server`, all under `/v1` except health/ready/metrics)

Built in `build_router` at `crates/cloacina-server/src/lib.rs:371`. All `/v1/*` routes go through `route_layer(require_auth)` middleware which validates `Authorization: Bearer <key>` against the API-key DAL (with a 30s LRU cache).

**Public** (no auth):
- `GET /health` — liveness, returns `{"status":"ok"}`.
- `GET /ready` — readiness: checks DB pool + crashed-graph list. Returns 503 if DB unreachable or graphs crashed.
- `GET /metrics` — Prometheus text format. Lists `cloacina_workflows_total`, `cloacina_tasks_total`, `cloacina_api_requests_total`, `cloacina_api_request_duration_seconds`, `cloacina_workflow_duration_seconds`, `cloacina_task_duration_seconds`, `cloacina_active_workflows`, `cloacina_active_tasks`.

**Auth** (`crates/cloacina-server/src/routes/keys.rs`):
- `POST /v1/auth/keys` — create API key, body `{name, role: admin|write|read}`. Returns `{key: "clk_...", id, name}`. Plaintext returned **once**; only the hash is persisted.
- `GET /v1/auth/keys` — list keys.
- `DELETE /v1/auth/keys/{key_id}` — revoke key (UUID path; 400 on bad UUID, 404 on missing).
- `POST /v1/auth/ws-ticket` — exchange API key for a single-use, short-TTL WS ticket.

**Tenants** (`routes/tenants.rs`, admin-only):
- `POST /v1/tenants` — create tenant: schema + DB user + run migrations. Body `{schema_name, username, password}`.
- `GET /v1/tenants` — list tenants.
- `DELETE /v1/tenants/{schema_name}` — drop tenant (idempotent).
- `POST /v1/tenants/{tenant_id}/keys` — create tenant-scoped key.

**Workflows** (`routes/workflows.rs`, tenant-scoped):
- `POST /v1/tenants/{tenant_id}/workflows` — multipart upload of `.cloacina` archive, optional signature verification.
- `GET /v1/tenants/{tenant_id}/workflows` — list installed workflow packages.
- `GET /v1/tenants/{tenant_id}/workflows/{name}` — fetch a single workflow's metadata.
- `DELETE /v1/tenants/{tenant_id}/workflows/{name}/{version}` — uninstall.

**Triggers** (`routes/triggers.rs`, tenant-scoped, read-only):
- `GET /v1/tenants/{tenant_id}/triggers` — list cron + custom-poll schedules.
- `GET /v1/tenants/{tenant_id}/triggers/{name}` — schedule details + recent executions.

**Executions** (`routes/executions.rs`, tenant-scoped):
- `POST /v1/tenants/{tenant_id}/workflows/{name}/execute` — trigger a workflow with optional context body.
- `GET /v1/tenants/{tenant_id}/executions` — list executions.
- `GET /v1/tenants/{tenant_id}/executions/{exec_id}` — execution detail.
- `GET /v1/tenants/{tenant_id}/executions/{exec_id}/events` — execution event log.

**Computation graph health** (`routes/health_graphs.rs`, auth-required, **note nesting** — these merge into the router root, not under `/v1`):
- `GET /v1/health/accumulators` — list accumulators with health status.
- `GET /v1/health/graphs` — list loaded graphs with `{name, health, accumulators, paused}`.
- `GET /v1/health/graphs/{name}` — single graph health.

**WebSocket** (`routes/ws.rs`, auth in handler before upgrade):
- `GET /v1/ws/accumulator/{name}` — bidirectional. Producers push raw boundary bytes; the server forwards into the accumulator socket. Auth via `Authorization: Bearer` header **or** `?token=<ws_ticket>` query param (browsers can't send custom upgrade headers).
- `GET /v1/ws/reactor/{name}` — operator commands. JSON message envelope: `ReactorCommand` enum (`ForceFire`, `FireWith`, `GetState`, `Pause`, `Resume`). Server responds with `ReactorResponse` (`Fired`, `State`, `Paused`, `Resumed`, `Error`).

### WebSocket protocol notes

- Auth is one-shot at upgrade; no per-message reauth.
- `WsTicketStore` (`crates/cloacina-server/src/routes/auth.rs`) holds tickets with TTL (default 60s) — single-use; consumed when the upgrade succeeds.
- Accumulator messages: raw bytes, no framing; the server passes them straight to the accumulator's MPSC sender.
- Reactor messages: JSON-encoded `ReactorCommand` / `ReactorResponse`; serialization via serde on the server side.

### Library / public API (the `cloacina` crate)

Re-exports at `crates/cloacina/src/lib.rs:540-588`: `Graph`, `ReactionMode`, `Reactor`, `ReactorConstructor`, `ReactorRegistration`, `ComputationGraphRegistration`, `TriggerlessGraph`, `TriggerlessGraphFn`, `TriggerlessGraphRegistration`, `Context`, `CronError`, `CronEvaluator`, `CronRecoveryConfig`, `CronRecoveryService`, `Scheduler`, `SchedulerConfig`, `AdminError` (postgres), `DatabaseAdmin`, `TenantConfig`, `TenantCredentials`, `UniversalBool`, `UniversalTimestamp`, `UniversalUuid`, `DefaultDispatcher`, `DispatchError`, `Dispatcher`, `ExecutionResult`, `ExecutionStatus`, `ExecutorMetrics`, `RoutingConfig`, `RoutingRule`, `TaskExecutor`, `TaskReadyEvent`, `CheckpointError`, `ContextError`, `ExecutorError`, `RegistrationError`, `SubgraphError`, `TaskError`, `ValidationError`, `WorkflowError`, `TaskScheduler`, `TriggerCondition`, `TriggerRule`, `ValueOperator`, `WorkflowExecution`, `WorkflowExecutionError`, `WorkflowExecutionResult`, `WorkflowExecutor`, `WorkflowStatus`, `ExecutorConfig`, `TaskHandle`, `TaskResult`, `ThreadTaskExecutor`, `DependencyEdge`, `GraphEdge`, `GraphMetadata`, `GraphNode`, `TaskNode`, `WorkflowGraph`, `WorkflowGraphData`, `ComputationGraphEntry`, `ReactorEntry`, `StreamBackendEntry`, `StreamBackendFactoryFn`, `TaskEntry`, `TriggerEntry`, `TriggerlessGraphEntry`, `WorkflowEntry`, `BackoffStrategy`, `RetryCondition`, `RetryPolicy`, `RetryPolicyBuilder`, `DefaultRunnerBuilder`, `DefaultRunner`, `DefaultRunnerConfig`, `Runtime`, `parse_namespace`, `Task`, `TaskNamespace`, `TaskRegistry`, `TaskState`, `Trigger`, `TriggerConfig`, `TriggerError`, `TriggerResult`, `DependencyGraph`, `Workflow`, `WorkflowBuilder`, `WorkflowMetadata`. Macros (feature `macros`): `task`, `workflow`, `trigger`, `reactor`, `computation_graph`, `passthrough_accumulator`, `polling_accumulator`, `batch_accumulator`, `stream_accumulator`.

### CLI commands (`cloacinactl`, mirrors `angreal tree --long` for dev tasks but the CLI is itself the operator surface)

Defined at `crates/cloacinactl/src/main.rs:108`:

```
cloacinactl
├── daemon {start|stop|status|health}
├── server {start|stop|status|health}
├── compiler {start|stop|status|health}
├── package {build|pack|publish|upload|list|inspect|delete}
├── workflow ...
├── graph ...        # CG health (read-only via server API)
├── execution ...
├── tenant ...       # admin only
├── key ...
├── trigger ...
├── status           # composite: daemon + server + compiler
├── config {get|set|list|profile {set|list|use|delete}}
├── admin cleanup-events --older-than 90d [--dry-run]
└── completions {bash|zsh|fish|powershell}
```

Global options: `--verbose`, `--home`, `--profile`, `--server`, `--api-key` (accepts `env:VAR` / `file:PATH`), `--tenant`, `--json` (alias `-o json`), `--output {table|json|yaml|id}`, `--no-color`. Profiles live in `~/.cloacina/config.toml`.

### Python public API (`cloaca` module)

From `crates/cloacina-python/src/lib.rs:87-154` (`#[pymodule] fn cloaca`):
- Decorators: `task`, `trigger`, `reactor`, `passthrough_accumulator`, `stream_accumulator`, `polling_accumulator`, `batch_accumulator`, `node`
- Classes: `Context`, `WorkflowBuilder`, `Workflow`, `DefaultRunner`, `DefaultRunnerConfig`, `WorkflowResult`, `TaskHandle`, `TaskNamespace`, `WorkflowContext`, `RetryPolicy`, `RetryPolicyBuilder`, `BackoffStrategy`, `RetryCondition`, `TriggerResult`, `ComputationGraphBuilder`, `DatabaseAdmin` (postgres), `TenantConfig`, `TenantCredentials`
- Functions: `register_workflow`, `var`, `var_or` (read `CLOACINA_VAR_<NAME>` env vars)
- `var` raises `KeyError` on missing; `var_or(name, default)` falls back

### Config surface

- **Server CLI flags**: `--bind`, `--database-url` (env `DATABASE_URL`), `--bootstrap-key` (env `CLOACINA_BOOTSTRAP_KEY`), `--require-signatures` (env `CLOACINA_REQUIRE_SIGNATURES`), `--reconcile-interval-s`, `--home`, `--verbose`.
- **Daemon CLI flags**: `--watch-dir`, `--poll-interval` (ms), `--verbose`, `--home`. Plus `~/.cloacina/config.toml` (`CloacinaConfig`).
- **Compiler CLI flags**: `--bind`, `--database-url`, `--poll-interval-ms`, `--heartbeat-interval-s`, `--stale-threshold-s`, `--sweep-interval-s`, `--cargo-flag` (repeatable), `--cargo-target-dir`, `--home`, `--verbose`.
- **Telemetry**: `OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME` env vars (server only, gated `feature = "telemetry"`).
- **Config file**: `~/.cloacina/config.toml` — daemon config block (`trigger_poll_interval_ms`, `cron_recovery_interval_s`, `cron_max_catchup`, `watcher_debounce_ms`, `shutdown_timeout_s`, `watch_dirs`), database_url, default_profile, `[profiles.<name>]`.

### Emitted events / metrics

- **Metrics**: `cloacina_workflows_total{status, reason}`, `cloacina_tasks_total{status, reason}`, `cloacina_api_requests_total{method, status}`, `cloacina_api_request_duration_seconds`, `cloacina_workflow_duration_seconds`, `cloacina_task_duration_seconds`, `cloacina_active_workflows`, `cloacina_active_tasks`. Bounded reasons enumerated in the metric description (`task_error`, `timeout`, `validation_failed`, `infrastructure`, `task_not_found`, `claim_lost`, `unknown`).
- **Events** (DB): `execution_events` table — every state transition for workflows and tasks (audit trail). `recovery_events` — stale-claim recoveries, cron catchup events.
- **Logs**: structured tracing via `tracing` + `tracing-subscriber`. Server/daemon write daily-rotated JSON to `~/.cloacina/logs/<service>.log` plus stderr (`tracing-appender::rolling::daily`). Compiler analogous.

## 7. Dependency Graph

### External dependencies (load-bearing)

From `crates/cloacina/Cargo.toml` and the server/compiler/CLI manifests:

**Async + concurrency:** `tokio = "1"` (with `features = ["full"]`), `futures = "0.3"`, `async-trait = "0.1"`, `parking_lot = "0.12"`, `deadpool = "0.12"`.

**Database:** `diesel = "2.1"` (with `chrono`, `serde_json` features; not sqlx — see Open Questions for the prior commentary about sqlx vs diesel), `diesel_migrations = "2.1"`, `deadpool-diesel = "0.6"` (postgres + sqlite), `libsqlite3-sys` (bundled). Postgres-specific: `tokio-postgres = "0.7"` (under `feature = "postgres"`).

**Web (server):** `axum = "0.8"` (`features = ["multipart", "ws"]`), `tower = "0.5"`, `tower-http = "0.6"` (`cors`, `trace`, `limit`), `hyper = "1"`, `lru = "0.12"`, `http-body-util = "0.1"`. Body limit 100MB matching `PackageValidator`.

**FFI / packaging:** `fidius-host = "0.2.1"`, `fidius-core = "0.2.1"`, `fidius = "0.2.1"`, `libloading = "0.8"`, `bzip2 = "0.4"`, `inventory = "0.3"`, `tempfile = "3.2"`.

**Streaming (optional, `feature = "kafka"`):** `rdkafka = "0.39"` (with `tokio` feature).

**Crypto:** `aes-gcm = "0.10"`, `ed25519-dalek = "2.1"`, `pem = "3"`, `sha2 = "0.10"`, `hex = "0.4"`, `base64 = "0.22"`.

**Cron/time:** `chrono = "0.4"` (`serde`), `chrono-tz = "0.10"`, `croner = "2.1"`.

**Observability:** `tracing = "0.1"`, `tracing-subscriber = "0.3"` (`env-filter`, `json`), `tracing-appender = "0.2"`, `metrics = "0.24"`, `metrics-exporter-prometheus = "0.18"`. Optional (server `feature = "telemetry"`): `tracing-opentelemetry = "0.32"`, `opentelemetry = "0.31"`, `opentelemetry-otlp = "0.31"` (grpc-tonic), `opentelemetry_sdk = "0.31"`.

**Misc:** `serde = "1"` (derive), `serde_json = "1"`, `serde_yaml = "0.9"` (cloacinactl), `bincode = "1.3"`, `toml = "0.8"`, `uuid = "1"` (`v4`, `v5`, `serde`), `petgraph = "0.6"`, `regex = "1.10"`, `semver = "1"`, `thiserror = "1"`, `anyhow = "1"`, `clap = "4"` (`derive`, `env`), `clap_complete = "4"`, `dotenvy = "0.15"`, `url = "2.5"`, `urlencoding = "2.1"`, `dirs = "5/6"`, `notify = "7"` (daemon, with `macos_kqueue`), `reqwest = "0.12"` (cloacinactl, `json`/`multipart`/`rustls-tls`/`stream`).

**Python (cloacina-python):** `pyo3 = "0.25"` (`abi3-py39`), `pythonize = "0.25"`.

### Internal crate graph

```
cloacina-workflow ──┐  (minimal: Context/Task/Trigger/retry/error)
                    │
cloacina-computation-graph ──┐  (minimal: InputCache/GraphResult/Reactor trait)
                             │
cloacina-workflow-plugin ──┐   (FFI types + CloacinaPlugin trait + cloacina::package! macro)
       │                   │
       ▼                   │
cloacina-macros            │   (proc-macros emit code referencing cloacina-workflow + -plugin)
                           │
cloacina ──────────────────┴─►  (engine; depends on all four above + macros optional)
   ▲       ▲       ▲      ▲
   │       │       │      │
   │       │       │      └── cloacina-python (pyo3 module + Python loader, also re-exports CG types)
   │       │       │
   │       │       └── cloacina-server  (axum HTTP + WS, depends on cloacina + cloacina-python)
   │       │
   │       └── cloacina-compiler        (build worker, depends on cloacina; deliberately NOT cloacina-python)
   │
   └── cloacinactl                     (CLI client; depends on cloacina + cloacina-workflow-plugin)

cloacina-build       (build-script helper used by server/python/examples)
cloacina-testing     (test helpers; consumed by integration tests)
```

Notable invariant: **cloacina-compiler does not depend on cloacina-python**. The split exists precisely so the build worker doesn't link pyo3 (CLOACI-T-0529 documented this).

### Python dependency surface

The `cloaca` wheel (built from `cloacina-python`) defaults to `features = ["postgres", "sqlite", "macros"]`. `extension-module` is the pyo3 switch that maturin flips on. Tests in `tests/python/` use pytest + scenario files; the `_build_and_install_cloaca_unified` helper in `.angreal/test/_python_utils.py` builds the wheel via maturin and installs it into a venv.

## 8. Build and Deployment

### angreal task tree (canonical surface — `angreal tree --long`)

The user has explicitly requested angreal for all build/test ops; **do not recommend raw `cargo` for build/test** (per `feedback_use_angreal_testing.md` memory and the `task_check.py` task description warnings). Tasks live under `.angreal/`.

```
check
  ├── all-crates                        # cargo check + build on every workspace + standalone crate
  └── crate                             # check a single crate

ci
  ├── fast                              # lint all + test unit (no Docker)  ← .angreal/ci/fast.py
  └── full                              # lint + full tests + coverage (Docker required)  ← .angreal/ci/full.py

demos
  ├── features/{continuous-scheduling, cron-scheduling, deferred-tasks, event-triggers,
  │             multi-tenant, packaged-graph, per-tenant-credentials, python-packaged-graph,
  │             python-workflow, registry-execution}
  └── tutorials/{python/01..11, rust/01..10}            # marked destructive — postgres backend wipes volumes

docs
  ├── build / clean / serve

lint
  ├── all                              # fmt --check + clippy + credential_logging
  ├── clippy
  ├── credential-logging               # scripts/check_credential_logging.py — OPS-03 / T-0443
  └── fmt

performance
  ├── all / quick / simple / parallel / pipeline / computation-graph-bench

services
  ├── up / down / reset / clean / purge       # docker compose lifecycle (purge is the nuke option)

test
  ├── all                              # unit + macros + integration
  ├── auth                             # tenant isolation, roles, god-mode, deny scenarios
  ├── coverage                         # cargo-llvm-cov merged across unit/integration/macros/cloacinactl
  ├── e2e/{cli, compiler, ws}          # against a live server
  ├── integration                      # Rust integration tests + Python pytest scenarios (Docker required)
  ├── macros                           # macro validation system
  ├── metrics-format                   # promtool validation against live /metrics — T-0536
  ├── soak/{daemon, server}            # sustained load
  └── unit                             # crates with --lib only
```

The `unit` task runs `cargo test -p cloacina-workflow --lib` first, then `cargo test -p cloacina --lib --features postgres,sqlite,macros` (`.angreal/test/unit.py:42-62`). The `integration` task pre-builds packaged-workflow fixtures (including the new I-0102 reactor-only/trigger-only fixtures, `.angreal/test/integration.py:54-67`).

### CI mirror tasks

- `angreal ci fast` — lint + unit. No Docker, fast pre-push check.
- `angreal ci full` — lint + full test suite + coverage. Requires Docker (Postgres + Kafka).

### docker-compose layout (`.angreal/docker-compose.yaml`)

- `postgres` — Postgres 16, `cloacina/cloacina` user/pass, `cloacina` DB, `max_connections=500`, port 5432, named volume `cloacina_postgres_data`.
- `kafka` — apache/kafka 3.9 in KRaft mode (no Zookeeper), port 9092, named volume `cloacina_kafka_data`. Used for stream-backed accumulator integration tests.

### Deployment artifacts

- **Binaries**: `cloacina-server`, `cloacina-compiler`, `cloacinactl` — produced by `cargo build --release --bin <name>`. No Dockerfiles ship with the repo.
- **Python wheel**: `cloaca` — built via maturin (the build script lives in `crates/cloacina-python/build.rs`).
- **Source `.cloacina` archives**: end-user deliverables produced by `cloacinactl package pack`.
- **Compiled cdylibs**: stored in `workflow_packages.compiled_data` column (Postgres bytea / SQLite blob); not shipped on disk.

### Test types and locations

- **Unit tests** — embedded `#[cfg(test)] mod tests` in each `src/` module. `cargo test --lib`. Run by `angreal test unit`.
- **Integration tests (Rust)** — `crates/cloacina/tests/integration/` with subdirs `dal/`, `database/`, `executor/`, `models/`, `scheduler/`, `signing/`, `task/`, `workflow/` plus top-level scenario files (`packaging.rs`, `unified_workflow.rs`, `event_dedup.rs`, `fidius_validation.rs`, `primitive_only_packaging.rs`, `test_dlopen_packaged.rs`, etc.). Driven by `tests/fixtures.rs`.
- **Macros tests** — `cloacina-macros` has `trybuild`-based ui tests (`crates/cloacina/dev-dependencies` has `trybuild = "1.0"`).
- **Python integration scenarios** — `tests/python/test_scenario_*.py` (32 scenarios visible) covering basic API, single-task, function-based DAG topology, multi-task, context propagation, error handling, retry, performance, complex deps, trigger rules, versioning, registry management, advanced error handling, shared runner, cron, multi-tenancy, event triggers, callbacks, task handles, task→CG invocation. Runner: `pytest` via `.angreal/test/_python_utils.py::run_pytest_scenarios`.
- **E2E tests** — `.angreal/test/e2e/{cli, compiler, ws}.py` — exercise live `cloacinactl`, `cloacina-compiler`, and the WS endpoints against a started `cloacina-server`.
- **Soak tests** — `.angreal/test/soak/{daemon, server}.py` — sustained-load tests; the `server` soak revealed 5 prior gaps (Python CG routing, cloaca module, rate limiter, runtime snapshots, executor deadlock per memory `project_soak_test_gaps.md`).
- **Cross-language fan-out** — `crates/cloacina-python/tests/cross_language_fan_out.rs` and `python_reactor_library.rs` and `trigger_packaging.rs`.

## 9. Conventions and Implicit Knowledge

### Architectural patterns

- **Constructor closures over instance handles**: every registry stores `Box<dyn Fn() -> X + Send + Sync>` rather than the constructed instance, so a fresh instance can be produced on demand. This is the pattern across `Runtime` (tasks, workflows, triggers, CGs, reactors, stream backends), `EndpointRegistry`, etc.
- **`inventory` linker-section auto-registration**: macros emit `inventory::submit!(crate::inventory_entries::TaskEntry { ... })` and `Runtime::seed_from_inventory` (`crates/cloacina/src/runtime.rs:127`) drains them. The same `inventory` collection works across `dlopen`'d cdylibs on Linux/macOS, but the host's `inventory::iter` does **not** see entries from cdylibs loaded after main has started — that's why packaged metadata is extracted via FFI calls and host-side adapters are registered explicitly. (Quoted from `runtime.rs:122`.)
- **Push-based dispatch (post-T-0509)**: the scheduler used to poll `task_outbox`; today it pushes `TaskReadyEvent`s through a `Dispatcher` that routes to executors. Polling exists only as a fallback in the executor (`enable_claiming` flag).
- **Scoped runtime over process globals**: prior to CLOACI-T-0509 there were process-global static registries; they were deleted. Each `DefaultRunner` owns its own `Arc<Runtime>`, and Python uses a thread-local `runtime_scope` to scope decorator registrations.
- **Service manager pattern**: `DefaultRunner.service_manager` (`crates/cloacina/src/runner/default_runner/service_manager.rs`) holds every background service handle + shutdown signal in one `Arc<RwLock<ServiceManager>>` so `shutdown()` is centralized.
- **Endpoint registry for CG**: `EndpointRegistry` (`crates/cloacina/src/computation_graph/registry.rs`) decouples accumulator/reactor channel endpoints from their consumers; auth policies are stored alongside the channel handles.
- **DB row state machine for packages**: `workflow_packages.build_status` ∈ {`pending`, `building`, `success`, `failed`, `superseded`}; the compiler service heartbeats `building` rows; the sweeper resets stuck rows; new uploads supersede prior active rows.
- **Content-hash artifact reuse**: identical source → reuse compiled cdylib (`crates/cloacina/src/registry/workflow_registry/mod.rs:325` `find_success_by_hash`).

### Error-handling idioms

- `thiserror`-based enums per subsystem: `WorkflowExecutionError`, `TaskError`, `WorkflowError`, `RegistryError`, `ValidationError`, `LoaderError`, `StorageError`, `GraphError`, `AccumulatorError`, `RegistrationError`, `SubgraphError`, `TriggerError`, `CronError`, `DatabaseError`. CLI: `CliError` (`crates/cloacinactl/src/shared/error.rs`).
- `anyhow::Result` at top-level entrypoints (binary `main`, daemon/server `run`).
- Server returns `ApiError` JSON payloads with `error` (slug) + `message`; the typed shape is `crates/cloacina-server/src/routes/error.rs`.
- Best-effort persistence patterns: many CG persistence calls are wrapped in `let _ =` and log on failure (e.g., `set_health`, `persist_boundary`, `persist_reactor_state` in `accumulator.rs`/`reactor.rs`).

### Feature flags

`cloacina` features: `default = ["macros", "postgres", "sqlite", "kafka"]`, plus `auth = ["postgres"]`, individual `postgres`, `sqlite`, `macros`, `kafka`. The `kafka` feature gates `rdkafka` and the `KafkaStreamBackend` impl in `stream_backend.rs:217-340`.

`cloacina-server` features: `default = ["postgres"]`, plus `sqlite`, `kafka`, `telemetry`. Telemetry adds OTLP exporter wiring.

`cloacina-compiler` features: `default = ["postgres"]`, plus `sqlite`. Smaller feature surface than server (no kafka, no telemetry, no auth).

`cloacinactl` features: `default = ["postgres", "sqlite", "kafka"]`.

`cloacina-python` features: `default = ["postgres", "sqlite", "macros"]`, plus `kafka`, `auth`, `extension-module`.

### Async patterns

- Background loops are `tokio::spawn`'d with a `watch::Receiver<bool>` shutdown signal pattern. Cancellation via `tokio::select! { ... = shutdown_rx.changed() => break, ... }`.
- For services that need cancellation that a `watch` doesn't fit (e.g., compiler), `tokio_util::sync::CancellationToken` is used (`crates/cloacina-compiler/src/lib.rs:63`).
- The Python runner wraps a tokio runtime in a dedicated thread because the GIL holder thread can't simultaneously be the tokio worker (`crates/cloacina-python/src/bindings/runner.rs:23-47`).
- Packaged cdylibs each own a `OnceLock<tokio::runtime::Runtime>` for the FFI execute-task / execute-graph paths to avoid creating runtimes per call.

### Naming conventions

- Strict noun-verb in `cloacinactl` — `<noun> <verb>`. The single documented exception is the top-level `status` (composite of daemon + server + compiler).
- Banned phrases per CLOACI-S-0011: "reactive scheduler", "reactive computation graph", "reactive subsystem". Replace with "graph scheduler", "computation graph", "computation graph scheduler" (rendered surfaces: `ComputationGraphScheduler`, `health_graphs.rs`, `/v1/health/graphs`, `cloacinactl graph`).
- `reactor` is **not** a synonym for "computation graph" — it's the noun for the firing primitive bound to accumulators.
- File naming: `task_<name>.py` prefix is required by the angreal loader (`project_angreal_task_loading.md` memory) — that's why `.angreal/task_check.py`, `task_services.py`, `task_project.py`, `task_docs.py` exist. Subgroups (`ci/`, `lint/`, `test/`, `demos/`) are imported by these top-level task files.
- Test file naming: `test_scenario_NN_<name>.py` for Python pytest scenarios.
- Metis IDs: `CLOACI-V-####` (vision), `CLOACI-I-####` (initiative), `CLOACI-T-####` (task), `CLOACI-A-####` (ADR), `CLOACI-S-####` (specification).

### Packaging conventions

- `.cloacina` extension is a bzip2 tarball of source (NOT a compiled artifact).
- `package.toml` `[metadata]` fields use `#[serde(deny_unknown_fields)]` — legacy `package_type` and `[[triggers]]` are hard errors with friendly migration hints.
- Workflow → trigger subscriptions live on `#[workflow(triggers = [...])]` post-I-0102.
- Cross-primitive references use **string names** post-I-0102: `trigger = reactor("name")`, `invokes = computation_graph("name")` — no type paths.
- A package may declare any combination of primitives (workflows, CGs, reactors, triggers); the unified `cloacina::package!()` shell handles all shapes.
- Method indices are pinned: never reorder methods on `CloacinaPlugin`.
- fidius wire format: debug = JSON, release = bincode (per `feedback_fidius_wire_format.md` memory).

### Multi-backend (sqlite/postgres) handling

- Every DAL method has paired `_postgres` and `_sqlite` private fns; the public method dispatches on `dal.backend()` (e.g., `crates/cloacina/src/dal/unified/recovery_event.rs:64`).
- Universal types abstract Uuid (Postgres native vs SQLite TEXT v4-formatted), Timestamp (TIMESTAMPTZ vs TEXT ISO-8601), Bool (BOOL vs INTEGER 0/1), Binary (BYTEA vs BLOB).
- Migrations live in directories per backend (`crates/cloacina/migrations/postgres/` and `crates/cloacina/migrations/sqlite/` — inferred from `diesel_migrations` usage). Per memory `feedback_sqlite_migration_recreate.md`: avoid DROP+CREATE patterns in SQLite migrations; prefer ADD COLUMN + CREATE INDEX.
- `DefaultRunner` chooses backend from URL scheme — no compile-time decision required when both features are enabled (the default).

### Implicit knowledge

- The `cloaca` Python wheel name vs the engine crate `cloacina` name comes from Latin (the README explains: Cloacina = goddess of sewers; Cloaca = the drain itself).
- The `~/.cloacina/` home directory is the canonical state dir for daemon (logs/, packages/, daemon.pid, daemon.sock, cloacina.db, config.toml) and for server (logs/, bootstrap-key).
- Test packages are built in **debug mode** to match the test binary's wire format (otherwise bincode/JSON mismatch causes failures).
- Always use **fresh databases** when testing packaged workflows (memory `feedback_stale_db_testing.md`) — stale pipelines cause misleading failures.
- `serial_test` is a workspace-wide test dep — many DAL/server tests `#[serial]` because of Postgres recorder install / connection pool conflicts.
- Server recorder install: tests use `metrics_exporter_prometheus::PrometheusBuilder::new().install_recorder()` with a fallback to `build_recorder().handle()` if a recorder is already installed (test isolation pattern, `crates/cloacina-server/src/lib.rs:686-693`).
- `PythonTaskWrapper` task execution must be careful around the GIL — `with_gil` blocks the tokio worker, so a single Python task running blocks one of the Python runner's workers.
- The Postgres-only `auth` feature gates the API key DAL implementation — SQLite cannot back the server's auth tables today (visible in features list).

## 10. Open Questions

1. **Workflow registry "schedules" field** — `WorkflowMetadata.schedules` exists (visible in `crates/cloacina/src/registry/workflow_registry/mod.rs:378`) but is initialized to `Vec::new()` and never populated in the path I traced. Is it superseded by the `schedules` table now? Specialist agents reviewing the registry should check.

2. **Reconciler precedence-ordered loader status** — CLOACI-I-0102 specifies a precedence-ordered loader (cron triggers → custom triggers → reactors → trigger-less CGs → reactor-bound CGs → workflows). The code in `crates/cloacina/src/registry/reconciler/loading.rs` shows `step_load_*` helpers exist (`step_load_cron_triggers`, `step_load_custom_triggers`, `step_load_triggerless_cgs`) but I did not verify the full pipeline is wired and gating other branches. The initiative is on tasks T-A through T-E; the branch name suggests we're on T-A or T-B. A specialist should pin the exact state.

3. **Computation graph crate vs computation_graph module overlap** — there's `crates/cloacina-computation-graph` (minimal types) and `crates/cloacina/src/computation_graph/` (runtime). The split mirrors `cloacina-workflow` vs the engine, but the engine crate also re-exports `cloacina_computation_graph::*` at the crate root for use by packaged CGs (`crates/cloacina/src/lib.rs:435`). I did not verify there's no circular dep risk or duplicate type drift.

4. **Stream backend registration timing** — `StreamBackendEntry` is in `inventory_entries.rs` and `Runtime::seed_from_inventory` registers them, but the docs say the only built-in backend is Kafka (gated). How would a user register a custom backend in their own packaged cdylib? Does the `cloacina::package!()` shell include `StreamBackendEntry` walking? (The shell's macro body in `cloacina-workflow-plugin/src/lib.rs:128-470` does not appear to.)

5. **Rate limiter** — memory `project_soak_test_gaps.md` mentions a rate limiter as a soak-test gap. I did not find a rate limiter in the server router (no `tower::limit::RateLimitLayer` or similar). Has it been implemented since? A specialist should check.

6. **OpenAPI / docs.rs** — README links to `docs.rs/cloacina`, but the public API surface is large. There's no top-level `openapi.json` or `docs/api/` HTML I found. Does the server emit OpenAPI? Is it just rustdoc?

7. **OS-specific behavior** — `notify` crate is configured `default-features = false, features = ["macos_kqueue"]` for the daemon (`cloacinactl/Cargo.toml:36`). Linux uses inotify by default; the macOS kqueue choice is explicit. Behavior on Windows is unconfirmed.

8. **Auth roles** — `KeyRole` (`crates/cloacina-server/src/routes/keys.rs:38`) defines `Admin | Write | Read`. The `is_admin` boolean is also a separate column. The interaction between `is_admin` (boolean, set at key creation) and `permissions` (string, "admin"/"write"/"read") is implicit — both seem to gate different operations (admin gates tenant create; write gates workflow upload; tenant-scoped vs cross-tenant). Worth a security review.

9. **Telemetry coverage** — telemetry feature wires OTLP for tracing only; metrics are Prometheus pull. There's no OTLP metric export. Whether traces propagate from CLI → server (W3C `traceparent` header) is unconfirmed.

10. **Computation graph `_ffi` legacy path vs unified shell coexistence** — per CLOACI-I-0102 Detailed Design, T-A keeps per-macro `_ffi` emission alongside the new shell; T-C strips the old path. The current branch (T-A territory) might have both active. If a user's package has `cloacina::package!()` AND `#[computation_graph]`, the linker conflict described in the spec would trip. Specialists working on packaging should verify the gating is exact.

11. **Multi-tenant execution scoping** — `executions::execute_workflow` in the server (`crates/cloacina-server/src/routes/executions.rs:50`) notes "Execution is scheduled through the shared `DefaultRunner`, which uses its own database connection. In per-tenant deployments (recommended), the runner IS scoped to the tenant. In multi-tenant deployments, executions land in the runner's schema." So a single `cloacina-server` running in admin/public schema cannot route workflow execution to tenant-specific runners today — only listing/inspection respects the tenant DB cache. This is a meaningful semantic gap for tenant isolation.

12. **Compiler service security model** — `cloacina-compiler` runs `cargo build` on user-uploaded source. Sandboxing strategy (chroot, container, namespace, uid) was not visible. The README doesn't address this. Risk surface for a security review.

---

End of system overview.
