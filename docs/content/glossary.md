---
title: "Glossary"
description: "Definitions of key terms and concepts in the Cloacina ecosystem"
weight: 55
---

# Glossary

Definitions of key terms and concepts used throughout the Cloacina ecosystem.

---

### Accumulator

A long-lived component that connects to an external data source, receives events, and buffers them for consumption by computation graphs. Accumulators transform raw events into typed boundary values and push them to the reactor. See [Accumulator Design]({{< ref "/computation-graphs/explanation/accumulator-design" >}}).

### Backoff Strategy

The algorithm that determines the delay between retry attempts for a failed task. Cloacina supports three strategies: Fixed (constant delay), Linear (delay increases by a fixed increment), and Exponential (delay doubles each attempt). Configured as part of a Retry Policy.

### Boundary

A data change event emitted by a detector, representing new data available for processing. Boundaries are recorded in the ledger and signal the reactor that fresh input is ready for a computation graph to consume.

### Claim

A database-level lock mechanism that prevents multiple runners from executing the same task simultaneously. When a runner picks up a task, it writes a claim row with its identity and a heartbeat timestamp. Stale claims from crashed runners are swept by the scheduler. See [Guaranteed Cron Scheduling]({{< ref "/workflows/explanation/guaranteed-execution-architecture" >}}).

### Cloaca

The Python bindings package for Cloacina. Cloaca exposes Cloacina's workflow orchestration, computation graphs, and runner management to Python applications via PyO3. See [Python Bindings]({{< ref "/python/" >}}).

### Cloacina

The core Rust workflow orchestration and computation graph engine. Cloacina provides DAG-based workflow execution, reactive computation graphs, cron scheduling, multi-tenancy, and a packaging system for distributable workflows. See [Architecture Overview]({{< ref "/workflows/explanation/architecture-overview" >}}).

### Computation Graph

A reactive, streaming computation model where nodes process data flowing through directed edges. Unlike workflows, computation graphs are event-driven, in-process, and compiled. They fire when reaction criteria are met, processing correlated data from multiple sources in a single execution. See [Computation Graph Architecture]({{< ref "/computation-graphs/explanation/architecture" >}}).

### Context

A key-value data container that flows between tasks in a workflow, accumulating results as tasks complete. Each task receives the context from its predecessors and can insert new values for downstream tasks. Context is persisted to the database between task executions. See [Context System]({{< ref "/workflows/explanation/context-management" >}}).

### Cron Schedule

A time-based trigger using cron expressions to execute workflows on a recurring basis. Cloacina's cron system supports guaranteed execution semantics, ensuring missed schedules are caught up after outages. See [Cron Scheduling Architecture]({{< ref "/workflows/explanation/cron-scheduling" >}}).

### DAG

Directed Acyclic Graph. The dependency structure of tasks in a workflow, where edges represent execution order constraints. A task runs only after all its upstream dependencies have completed (subject to trigger rules). This structure guarantees no circular dependencies exist.

### DAL

Data Access Layer. The abstraction over database backends that allows Cloacina to operate against either PostgreSQL or SQLite without changing application code. The DAL handles differences in UUID representation, schema management, and query syntax. See [Database Backends]({{< ref "/platform/explanation/database-backends" >}}).

### DefaultRunner

The primary implementation of the WorkflowExecutor trait. DefaultRunner manages background services (scheduler, dispatcher, executor, reconciler) and provides the main entry point for executing workflows. It handles recovery of orphaned executions on startup and supports graceful shutdown.

### DefaultRunnerConfig

The configuration struct for DefaultRunner, using a builder pattern for construction. It specifies the database URL, concurrency limits, heartbeat intervals, stale claim thresholds, and other operational parameters. See [Configuration Reference]({{< ref "/platform/reference/configuration" >}}).

### Detector

A component that monitors data sources and emits boundaries when new data arrives. Detectors are used in trigger-based scheduling to notify the system that a workflow should execute based on external state changes.

### Dispatcher

The component that routes ready tasks from the scheduler to executor slots. The dispatcher bridges scheduling decisions and actual execution, ensuring tasks are only dispatched when concurrency permits are available. See [Dispatcher Architecture]({{< ref "/workflows/explanation/dispatcher-architecture" >}}).

### Entry Node

A computation graph node that reads from the InputCache rather than receiving output from another node. Entry nodes receive optional data — in Rust: `Option<&T>`, in Python: the value or `None` — where the absent case indicates no data has been received yet for that source. They serve as the ingress points for external data into the graph.

### Execution

A single run of a workflow, identified by a unique UUID. An execution tracks the lifecycle of all tasks in the workflow from Pending through to a terminal state (Completed, Failed, or Cancelled). Multiple executions of the same workflow can exist simultaneously.

### Executor

The background service that spawns async tasks and manages concurrency limits using a semaphore-based slot system. The executor receives dispatched tasks, runs them to completion, and reports results back to the scheduler for state transitions.

### FFI

Foreign Function Interface. The mechanism for loading packaged workflows as dynamic libraries at runtime. Cloacina uses a C ABI (Application Binary Interface) boundary via fidius to call into compiled workflow packages without requiring the host and plugin to share the same Rust compiler version. See [FFI System]({{< ref "/platform/explanation/ffi-system" >}}).

### fidius

The binary serialization and packaging library used internally by Cloacina. fidius transforms Rust traits into stable C ABI plugins, handling serialization across the FFI boundary. It uses JSON encoding in debug builds for readability and bincode (a compact binary format) in release builds for performance. See [FFI System]({{< ref "/platform/explanation/ffi-system" >}}).

### GraphResult

The output of a computation graph execution. It is an enum with two variants: `Completed { outputs }` containing the serialized outputs from all terminal nodes, or `Error` indicating the graph function failed during execution.

### InputCache

A map from SourceName to serialized bytes that feeds entry nodes in computation graphs. The reactor updates the InputCache each time an accumulator pushes a new boundary value. When the graph fires, entry nodes deserialize their input from this cache.

### Ledger

A persistent record of execution events used by the continuous scheduling system. The ledger tracks boundaries received, graph completions, and failures, enabling the reactor to make informed decisions about when to fire and providing an audit trail.

### Multi-tenancy

Running isolated workflow environments within a single Cloacina deployment. Implemented using PostgreSQL schema separation, where each tenant's data (workflows, executions, contexts) lives in a dedicated schema, preventing accidental cross-tenant access. See [Multi-Tenancy Architecture]({{< ref "/platform/explanation/multi-tenancy" >}}).

### Node

A function within a computation graph that transforms data from its inputs to an output. Nodes are connected by directed edges forming a topology. Each node executes once per graph firing, receiving the outputs of its predecessors as input parameters.

### Pipeline

In some code paths, metric names (e.g., `cloacina_pipelines_total`), and internal APIs, "pipeline" is used as a synonym for a workflow execution. The documentation uses "workflow" as the standard term. If you encounter "pipeline" in logs, metrics, or configuration fields, read it as "workflow execution."

### Package (.cloacina)

A distributable workflow artifact containing compiled code (as a platform-specific shared library — .so on Linux, .dylib on macOS) and metadata. Packages are uploaded to the runner's registry and loaded at runtime by the reconciler. They enable shipping workflows independently of the host application. See [Package Format]({{< ref "/platform/explanation/package-format" >}}).

### Reactor

The runtime orchestrator for computation graphs. The reactor owns the InputCache, evaluates reaction criteria after each accumulator update, and fires the compiled graph function when criteria are met. It manages the lifecycle of all computation graphs registered with a runner.

### Reconciler

A background service that periodically scans the package registry, loads newly registered packages, and makes their workflows available for execution. The reconciler handles the FFI loading of dynamic libraries and wires up the packaged workflows into the runner. See [Packaging & FFI]({{< ref "/computation-graphs/explanation/packaging" >}}).

### Recovery

The automatic restart of orphaned or lost executions after a runner restart. On startup, the runner queries for executions that were in-progress when the previous instance terminated, reclaims them, and resumes execution from the last known state.

### Retry Policy

Configuration for automatic task retry on failure. A retry policy specifies the maximum number of attempts, the initial delay between retries, the backoff strategy (Fixed, Linear, or Exponential), and optional conditions that determine which errors are retryable.

### Runner

An instance of DefaultRunner managing a set of workflows against a database. A runner encapsulates the scheduler, dispatcher, executor, and reconciler services, providing a single entry point for workflow execution and lifecycle management.

### Scheduler

The background service that evaluates task readiness and transitions tasks from Pending to Running state. The scheduler polls the database for tasks whose dependencies are satisfied, respects trigger rules, sweeps stale claims from crashed runners, and feeds ready tasks to the dispatcher.

### Schema Isolation

The PostgreSQL multi-tenancy strategy where each tenant uses a separate database schema. All tables (workflows, executions, tasks, contexts) are duplicated per schema, providing strong data isolation while sharing a single database connection pool. See [Multi-Tenancy Architecture]({{< ref "/platform/explanation/multi-tenancy" >}}).

### Slot

A concurrency permit managed by the executor's semaphore. Tasks consume a slot when they begin execution and release it on completion (or failure). The total number of slots bounds the maximum concurrent task count for a runner.

### SourceName

A string identifier for a data source feeding a computation graph's InputCache. Each accumulator is associated with a SourceName, and entry nodes declare which SourceName they read from. This mapping connects external data producers to internal graph consumers.

### Stale Claim

A task claim that was not properly released, typically because the owning runner crashed or lost connectivity. The scheduler periodically sweeps for claims whose heartbeat timestamp exceeds a configurable threshold and releases them, allowing another runner to pick up the task.

### Task

An atomic unit of work in a workflow. A task is an async function with declared dependencies, an optional retry policy, and optional trigger rules. Tasks receive a Context and optionally a TaskHandle, and return either a modified Context or a TaskError.

### TaskError

The error type returned by task functions when execution fails. Variants include `ValidationFailed` (input preconditions not met), `ExecutionFailed` (runtime error during processing), and others that influence whether the scheduler will retry the task.

### TaskHandle

An optional second parameter on task functions that provides execution control capabilities. The primary use is `defer_until`, which releases the task's concurrency slot while waiting for an external condition, then reacquires it when ready to resume. See [Task Deferral]({{< ref "/workflows/explanation/task-deferral" >}}).

### Terminal Node

A computation graph node whose output appears in the GraphResult. Terminal nodes have no downstream consumers within the graph; their outputs are collected and returned to the caller as the result of graph execution.

### Topology

The directed graph structure connecting nodes in a computation graph. The topology defines which nodes feed into which others, determining execution order and data flow. It is validated at compile time to ensure no cycles exist.

### Trigger

A polling-based mechanism that fires workflows when external conditions are met. Unlike cron schedules which are time-based, triggers evaluate arbitrary predicates (such as file existence, queue depth, or API state) and initiate workflow execution when the predicate returns true.

### Trigger Rule

Conditional execution criteria for individual tasks within a workflow. Trigger rules override the default behavior (run when all dependencies succeed) with alternatives such as `task_success` (specific task succeeded), `task_failed` (run on failure), or `context_value` (run when context contains a specific key/value). See [Trigger Rules]({{< ref "/workflows/explanation/trigger-rules" >}}).

### UniversalUuid

A cross-database UUID type that abstracts over backend differences. In PostgreSQL it maps to the native `UUID` column type; in SQLite it is stored as `TEXT`. This allows the same Rust code to work against both backends without conditional compilation.

### Workflow

A collection of tasks with defined dependencies forming a DAG. A workflow is registered with a runner by name and can be executed multiple times concurrently. Each execution produces an independent Context and tracks its own task states. See [Architecture Overview]({{< ref "/workflows/explanation/architecture-overview" >}}).

### WorkflowExecutor

The trait providing `execute()`, `shutdown()`, and lifecycle management for workflow runners. DefaultRunner is the primary implementation. The trait abstracts the execution interface, enabling testing with mock implementations and future alternative executors.

### WorkflowStatus

An enum representing the lifecycle state of a workflow execution. Variants are: `Pending` (not yet started), `Running` (at least one task active), `Completed` (all tasks succeeded), `Failed` (a task failed without remaining retries), `Cancelled` (manually stopped), and `Paused` (temporarily suspended).
