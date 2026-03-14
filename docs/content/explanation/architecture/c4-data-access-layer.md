---
title: "C4 Level 3 — Data Access Layer"
description: "Component diagram for Cloacina's DAL: facade pattern, domain repositories, dual-backend dispatch, and multi-tenancy"
weight: 31
---

# C4 Level 3 — Data Access Layer Components

This diagram zooms into the DAL portion of the `cloacina` core library from the [Container Diagram]({{< ref "/explanation/architecture/c4-container" >}}). The DAL provides a unified interface over PostgreSQL and SQLite backends using a facade/repository pattern.

## Component Diagram

```mermaid
C4Component
    title Component Diagram — Data Access Layer

    Container_Boundary(dal_sys, "Data Access Layer") {
        Component(dal, "DAL", "Rust facade", "Composes all domain repositories; dispatches to backend-specific implementations at runtime")
        Component(db, "Database", "Rust", "Connection pooling, backend detection, schema management, multi-tenancy isolation")

        Component(ctx_repo, "ContextDAL", "Repository", "CRUD for execution contexts (JSON data passed between tasks)")
        Component(pipe_repo, "PipelineExecutionDAL", "Repository", "Pipeline lifecycle: create, status transitions, completion")
        Component(task_repo, "TaskExecutionDAL", "Repository", "Task lifecycle: claiming, state transitions, retry scheduling, recovery")
        Component(meta_repo, "TaskExecutionMetadataDAL", "Repository", "Task metadata: context links, dependency tracking")
        Component(event_repo, "ExecutionEventDAL", "Repository", "Append-only audit trail of state transitions")
        Component(outbox, "TaskOutboxDAL", "Repository", "Transient work queue for reliable task distribution")
        Component(cron_repo, "CronScheduleDAL", "Repository", "Cron schedule definitions, due-schedule queries, enable/disable")
        Component(trigger_repo, "TriggerScheduleDAL", "Repository", "Event trigger definitions, polling state, enable/disable")
        Component(pkg_repo, "WorkflowPackagesDAL", "Repository", "Package metadata storage and retrieval")
        Component(reg_storage, "UnifiedRegistryStorage", "Repository", "Binary workflow serialization to database")
    }

    ContainerDb(postgres, "PostgreSQL", "Schema-per-tenant isolation")
    ContainerDb(sqlite, "SQLite", "File-per-tenant isolation")

    Rel(dal, db, "Gets connections from")
    Rel(dal, ctx_repo, "Exposes")
    Rel(dal, pipe_repo, "Exposes")
    Rel(dal, task_repo, "Exposes")
    Rel(dal, event_repo, "Exposes")
    Rel(dal, outbox, "Exposes")
    Rel(dal, cron_repo, "Exposes")
    Rel(dal, trigger_repo, "Exposes")
    Rel(dal, pkg_repo, "Exposes")
    Rel(db, postgres, "Diesel ORM + deadpool")
    Rel(db, sqlite, "Diesel ORM + deadpool")
```

## Components

### DAL (Facade)

| | |
|---|---|
| **Location** | `crates/cloacina/src/dal/unified/mod.rs` |
| **Pattern** | Facade + runtime backend dispatch |

The `DAL` struct is the single entry point for all database operations. It composes domain-specific repository DALs and dispatches to PostgreSQL or SQLite implementations at runtime via `backend_dispatch!` and `connection_match!` macros.

Repository DALs are ephemeral — they borrow from the `DAL` instance and are created on each access.

### Database (Connection Management)

| | |
|---|---|
| **Location** | `crates/cloacina/src/database/connection/mod.rs` |
| **Technology** | `deadpool-diesel` for async connection pooling |

**Backend detection** from connection string:
- `postgres://` or `postgresql://` → PostgreSQL (configurable pool size, default 10)
- `sqlite://`, file paths, `:memory:` → SQLite (pool size 1)

**Multi-tenancy:**
- PostgreSQL: `new_with_schema()` sets `search_path` per connection, `setup_schema()` creates schema + runs migrations
- SQLite: file-per-tenant isolation (separate `.db` file per tenant)
- Schema validation prevents SQL injection (alphanumeric + underscores only)

### Universal Types

| | |
|---|---|
| **Location** | `crates/cloacina/src/database/universal_types.rs` |

Cross-backend domain types with Diesel SQL type mappings:

| Domain Type | PostgreSQL | SQLite |
|------------|-----------|--------|
| `UniversalUuid` | Native UUID | BLOB (16 bytes) |
| `UniversalTimestamp` | TIMESTAMP | TEXT (RFC3339) |
| `UniversalBool` | BOOL | INTEGER (0/1) |
| `UniversalBinary` | BYTEA | BLOB |

## Domain Repositories

### TaskExecutionDAL

| | |
|---|---|
| **Location** | `crates/cloacina/src/dal/unified/task_execution/` |
| **Sub-modules** | `crud.rs`, `queries.rs`, `state.rs`, `claiming.rs`, `recovery.rs` |

The most complex repository. Key operations:

- **State transitions** — `mark_ready()`, `mark_completed()`, `mark_failed()` are all transactional: they update status, insert an `ExecutionEvent`, and (for ready) insert an outbox entry in a single transaction
- **Atomic claiming** — `claim_ready_task()` for distributed worker scenarios
- **Retry scheduling** — `schedule_retry()` with backoff delay
- **Recovery** — `get_orphaned_tasks()` detects tasks stuck in Running state

### PipelineExecutionDAL

| | |
|---|---|
| **Location** | `crates/cloacina/src/dal/unified/pipeline_execution.rs` |

Pipeline lifecycle management. Status transitions are transactional (status update + execution event). Provides `get_active_pipelines()` for the scheduler loop.

### ContextDAL

| | |
|---|---|
| **Location** | `crates/cloacina/src/dal/unified/context.rs` |

Stores execution contexts as JSON. `create()` skips empty contexts and returns the UUID for linking to task executions.

### ExecutionEventDAL (Audit Trail)

| | |
|---|---|
| **Location** | `crates/cloacina/src/dal/unified/execution_event.rs` |

**Append-only** — events are never updated or deleted. Provides a complete audit trail of all state transitions for pipelines and tasks.

### TaskOutboxDAL (Work Distribution)

| | |
|---|---|
| **Location** | `crates/cloacina/src/dal/unified/task_outbox.rs` |

Transient work queue. Entries are created atomically with task status transitions and deleted immediately upon claiming. On PostgreSQL, `LISTEN/NOTIFY` enables push-based notification; SQLite uses polling.

### CronScheduleDAL / CronExecutionDAL

| | |
|---|---|
| **Location** | `crates/cloacina/src/dal/unified/cron_schedule/`, `cron_execution/` |

Cron schedule definitions and execution history. `get_due_schedules(now)` returns schedules ready to fire. The execution DAL tracks statistics and supports recovery of lost executions.

### TriggerScheduleDAL / TriggerExecutionDAL

| | |
|---|---|
| **Location** | `crates/cloacina/src/dal/unified/trigger_schedule/`, `trigger_execution/` |

Event trigger definitions and execution tracking. `has_active_execution()` provides deduplication — prevents re-triggering for the same event context.

### WorkflowPackagesDAL / UnifiedRegistryStorage

| | |
|---|---|
| **Location** | `crates/cloacina/src/dal/unified/workflow_packages.rs`, `workflow_registry_storage.rs` |

Package metadata and binary workflow storage. `UnifiedRegistryStorage` implements the `RegistryStorage` trait for serializing/deserializing workflow binaries to the database.

## Database Schema

```mermaid
erDiagram
    pipeline_executions ||--o{ task_executions : contains
    task_executions ||--o| task_execution_metadata : has
    task_executions ||--o{ execution_events : generates
    pipeline_executions ||--o{ execution_events : generates
    task_executions ||--o| task_outbox : queues
    cron_schedules ||--o{ cron_executions : triggers
    trigger_schedules ||--o{ trigger_executions : triggers
    contexts ||--o| task_execution_metadata : referenced_by
```

## Key Patterns

| Pattern | Where | Purpose |
|---------|-------|---------|
| **Transactional Outbox** | `TaskExecutionDAL::mark_ready()` | Status + event + outbox in one transaction |
| **Append-Only Audit** | `ExecutionEventDAL` | Never update/delete — full history |
| **Backend Dispatch** | `backend_dispatch!` macro | Runtime PostgreSQL/SQLite selection |
| **Universal Types** | `UniversalUuid`, etc. | Type-safe cross-backend compatibility |
| **Schema Isolation** | `Database::new_with_schema()` | PostgreSQL multi-tenancy |
