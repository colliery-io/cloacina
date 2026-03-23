---
id: server-mode-recovery-system
level: specification
title: "Server Mode Recovery System — Instance Heartbeats, Task Claiming, Orphan Detection"
short_code: "CLOACI-S-0010"
created_at: 2026-03-23T02:16:46.043269+00:00
updated_at: 2026-03-23T02:16:46.043269+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Server Mode Recovery System — Instance Heartbeats, Task Claiming, Orphan Detection

## Overview

This specification defines the recovery subsystem for Cloacina's server mode (`cloacinactl serve`). The system must detect crashed server instances, recover their orphaned work, and prevent double-execution in a horizontally-scaled deployment. It replaces the current startup-only `RecoveryManager` with a continuous, distributed-safe recovery mechanism.

### Problem Statement

The current architecture has several gaps that prevent reliable server-mode operation:

1. **No instance identity**: When multiple server instances share a PostgreSQL database, there is no way to distinguish which instance owns which work. If an instance crashes, no other instance can determine what was in-flight.

2. **No "Running" state for tasks**: The push-based dispatcher (`DefaultDispatcher`) receives a `TaskReadyEvent` and calls `ThreadTaskExecutor::execute()` inline. The task moves from `Ready` directly to `Completed` or `Failed`. There is no durable "Running" marker in the database, so a crash leaves tasks stuck in `Ready` (which will be re-dispatched) or in an ambiguous state where partial side effects have occurred.

3. **Startup-only recovery**: `RecoveryManager::recover_orphaned_tasks()` runs once at startup. In a multi-instance deployment, this means recovery only happens when a new instance starts, not when a running instance detects a peer failure.

4. **No heartbeat**: No mechanism exists for instances to prove liveness to each other.

### Design Goals

- Detect instance death within a configurable time window (default: 60 seconds)
- Recover orphaned tasks and pipelines without double-execution
- Support horizontal scaling where any instance can recover any other instance's work
- Prepare the data model for distributed executors (remote workers claiming tasks from a queue)
- Integrate cleanly with existing cron recovery (`CronRecoveryService`)
- Remain backward-compatible: embedded-mode (single process, SQLite) must continue working without requiring heartbeats

## System Context

### Actors

- **Server Instance (ServeMode::All / Worker / Scheduler)**: A running `cloacinactl serve` process that registers itself, heartbeats, and may execute tasks.
- **Recovery Sweeper**: A background service within each Scheduler-mode or All-mode instance that scans for dead peers and recovers their work.
- **HTTP Client**: External caller that triggers pipeline executions via `POST /executions`. Not directly involved in recovery, but its executions must survive server crashes.
- **Cron Scheduler**: Fires pipelines on a time-based schedule. Its existing `CronRecoveryService` handles cron-specific gaps; the new recovery system handles the broader task-level gaps.

### External Systems

- **PostgreSQL**: Primary state store. All recovery coordination is PostgreSQL-native (advisory locks, row-level locking, heartbeat rows). No external coordination service (etcd, ZooKeeper) is required.

### Boundaries

**In scope:**
- Instance registration, heartbeat, and death detection
- Task-level state lifecycle changes (adding a durable "Running" state)
- Orphaned task recovery (reset to Ready) with idempotency guarantees
- Orphaned pipeline recovery (mark failed or re-queue)
- Integration point with `CronRecoveryService`
- Database schema additions for `server_instances` table and `task_executions` column additions
- Configuration surface for heartbeat intervals, death thresholds, and recovery limits

**Out of scope:**
- Distributed executor protocol (remote worker claiming over gRPC/HTTP) -- this spec defines the data model that enables it, but not the protocol
- Task checkpointing / partial progress recovery -- tasks restart from scratch (idempotency requirement is unchanged)
- Exactly-once execution guarantees for non-idempotent tasks

---

## 1. Instance Registration and Heartbeat

### 1.1 Instance Identity

Each server process generates a unique instance ID at startup (UUID v4). This ID is stable for the lifetime of the process and is recorded in every task claim.

```
instance_id: UUID  -- generated at DefaultRunner::with_config() time
instance_name: String  -- human-readable, from config or hostname+pid
```

The `DefaultRunnerConfig` already has optional `runner_id` and `runner_name` fields (used in tracing spans). These will be promoted to required fields populated automatically if not configured explicitly.

### 1.2 Registration

On startup, each instance inserts a row into `server_instances`:

```sql
INSERT INTO server_instances (id, instance_name, mode, started_at, last_heartbeat_at, status, metadata)
VALUES ($1, $2, $3, NOW(), NOW(), 'active', $4)
ON CONFLICT (id) DO UPDATE
  SET status = 'active', started_at = NOW(), last_heartbeat_at = NOW(), metadata = $4;
```

The `ON CONFLICT` clause handles the (unlikely) case of UUID collision or a process restarting with a persisted ID.

### 1.3 Heartbeat Loop

A background task updates `last_heartbeat_at` at a configurable interval:

```
heartbeat_interval: Duration  -- default 15 seconds
death_threshold: Duration     -- default 60 seconds (4 missed heartbeats)
```

The heartbeat is a single UPDATE:

```sql
UPDATE server_instances
SET last_heartbeat_at = NOW(), metadata = $2
WHERE id = $1 AND status = 'active';
```

The `metadata` column carries ephemeral runtime info (active task count, memory usage, executor capacity) that is useful for monitoring and load-aware routing but not required for correctness.

### 1.4 Graceful Shutdown

On graceful shutdown (`DefaultRunner::shutdown()`), the instance sets its own status to `stopped`:

```sql
UPDATE server_instances
SET status = 'stopped', stopped_at = NOW()
WHERE id = $1;
```

This allows the recovery sweeper to skip graceful shutdowns and focus on actual crashes (where `status` remains `active` but `last_heartbeat_at` is stale).

### 1.5 Embedded Mode Behavior

When running in embedded mode (SQLite backend, single process), no `server_instances` table is created. The heartbeat service is not started. Recovery falls back to the existing startup-only `RecoveryManager` logic, which checks for `Running` tasks on boot. This is safe because there is only one process.

---

## 2. Task State Lifecycle

### 2.1 Current States

Today the task lifecycle is:

```
Pending --> Ready --> Completed
                 \--> Failed
                 \--> Skipped
```

The gap: there is no durable "Running" state. The dispatcher calls `executor.execute(event)` and the task runs in-memory. If the process dies, the task is stuck in `Ready` (it was never transitioned) and will be re-dispatched on the next scheduler cycle -- which is actually safe for idempotent tasks but not observable. Worse, if the executor partially completed (wrote context, but crashed before `mark_completed`), the task metadata exists without a status update.

### 2.2 New State: "Running"

Add a durable `Running` transition before execution begins:

```
Pending --> Ready --> Running --> Completed
                            \--> Failed
                            \--> Skipped
                    \--> Ready (recovery reset)
```

The transition to `Running` is performed atomically by the executor immediately after acquiring the semaphore permit but before calling `task.execute()`. This transition also records the `claimed_by_instance_id`:

```sql
UPDATE task_executions
SET status = 'Running',
    started_at = NOW(),
    claimed_by_instance_id = $1,
    updated_at = NOW()
WHERE id = $2 AND status = 'Ready';
```

The `WHERE status = 'Ready'` guard makes the transition idempotent and prevents double-claiming in a race between dispatcher and recovery.

### 2.3 State Transition Table

| From    | To        | Trigger                        | Who               |
|---------|-----------|-------------------------------|-------------------|
| Pending | Ready     | Dependencies satisfied         | Scheduler         |
| Ready   | Running   | Executor claims task           | Executor (new)    |
| Running | Completed | Task succeeds                  | Executor          |
| Running | Failed    | Task fails, no retries left    | Executor          |
| Running | Ready     | Recovery resets orphaned task   | Recovery Sweeper  |
| Ready   | Skipped   | Trigger rule evaluation        | Scheduler         |
| Running | Ready     | Retry scheduled (backoff)      | Executor          |

### 2.4 Claiming Semantics

The `claimed_by_instance_id` column on `task_executions` records which instance is responsible for a task. This replaces the current approach where no instance ownership exists.

For the current in-process executor, the claim happens inside `ThreadTaskExecutor::execute()`. For future distributed executors, the claim will happen when a remote worker pulls a task from the queue -- the same column and same semantics apply.

---

## 3. Orphan Detection (Recovery Sweeper)

### 3.1 Death Detection

The Recovery Sweeper is a periodic background service (like `CronRecoveryService`) that runs on every Scheduler-mode and All-mode instance. It scans `server_instances` for dead peers:

```sql
SELECT id FROM server_instances
WHERE status = 'active'
  AND last_heartbeat_at < NOW() - INTERVAL '$death_threshold seconds';
```

When a dead instance is found, the sweeper:
1. Atomically marks it as `dead` using `SELECT ... FOR UPDATE SKIP LOCKED` to prevent multiple sweepers from processing the same dead instance simultaneously
2. Queries for all tasks `WHERE claimed_by_instance_id = $dead_id AND status = 'Running'`
3. Recovers each task (see section 4)
4. Marks the instance as `recovered`

```sql
-- Step 1: Claim the dead instance for recovery (only one sweeper wins)
UPDATE server_instances
SET status = 'recovering', recovered_by_instance_id = $my_id
WHERE id = $dead_id AND status = 'dead'
RETURNING id;

-- Step 2: Find orphaned tasks
SELECT * FROM task_executions
WHERE claimed_by_instance_id = $dead_id AND status = 'Running';

-- Step 3: (Per-task recovery, see section 4)

-- Step 4: Mark recovery complete
UPDATE server_instances
SET status = 'recovered', recovered_at = NOW()
WHERE id = $dead_id AND recovered_by_instance_id = $my_id;
```

Note: The two-phase approach (mark dead, then mark recovering) prevents a race where two sweepers both detect the same dead instance. The actual implementation uses `UPDATE ... WHERE status = 'active' AND last_heartbeat_at < threshold RETURNING id` as a single atomic step, followed by `UPDATE ... SET status = 'recovering' WHERE id = $dead AND status = 'dead'` -- only one sweeper's UPDATE will match.

### 3.2 Sweeper Configuration

```
recovery_sweep_interval: Duration  -- default 30 seconds
death_threshold: Duration          -- default 60 seconds
max_recovery_batch_size: usize     -- default 100
```

### 3.3 Self-Recovery on Startup

When a server instance starts, before entering its normal loop, it checks if its own instance ID (from a previous run) has orphaned tasks. This covers the case where a process restarts with a persisted instance ID (which is not the default but is configurable). It also runs the general recovery sweep to pick up any other dead instances.

This replaces the current `RecoveryManager::recover_orphaned_tasks()` call, which today simply finds all `Running` tasks globally. The new logic is scoped to specific dead instances.

---

## 4. Orphaned Work Recovery

### 4.1 Task Recovery

For each orphaned task (status = `Running`, `claimed_by_instance_id` = dead instance):

1. **Check recovery limit**: If `recovery_attempts >= max_recovery_attempts` (default 3), mark the task as `Failed` with error `"Exceeded recovery attempts after instance crash"` and record a `RecoveryEvent` with type `TaskAbandoned`.

2. **Check workflow availability**: Query the global workflow registry. If the workflow is no longer registered, mark the task as `Failed` with `WorkflowUnavailable` and record a `RecoveryEvent`.

3. **Reset to Ready**: Otherwise, reset the task for re-execution:
   ```sql
   UPDATE task_executions
   SET status = 'Ready',
       started_at = NULL,
       claimed_by_instance_id = NULL,
       recovery_attempts = recovery_attempts + 1,
       last_recovery_at = NOW(),
       updated_at = NOW()
   WHERE id = $1 AND status = 'Running';
   ```
   The `WHERE status = 'Running'` guard ensures idempotency -- if another sweeper already reset this task, the UPDATE is a no-op.

4. **Record event**: Insert a `RecoveryEvent` and an `execution_event` for observability.

### 4.2 Pipeline Recovery

After recovering all tasks for a dead instance, check each affected pipeline:

- If all tasks are now `Completed`, `Failed`, or `Skipped` -- mark the pipeline `Completed` or `Failed` as appropriate.
- If some tasks were reset to `Ready` -- the pipeline remains `Running` and the scheduler loop will re-evaluate task readiness on its next tick, which will dispatch the reset tasks via the outbox.
- The pipeline outbox entry should be re-inserted to ensure the scheduler picks it up promptly rather than waiting for the fallback scan.

### 4.3 Idempotency Guarantee

Recovery is idempotent because:
- The `UPDATE ... WHERE status = 'Running'` guard prevents double-reset
- Tasks are required to be idempotent by the Cloacina contract (documented in `cloacina-workflow` trait)
- Context is rebuilt from dependencies on re-execution (no stale partial context is reused)
- The `recovery_attempts` counter monotonically increases, providing a hard stop

### 4.4 Retry vs Recovery

These are distinct mechanisms:
- **Retry**: Task failed with a retryable error. The *executor* schedules a retry with backoff (`retry_at` timestamp). The task status is set to `Ready` with a future `retry_at`. The scheduler dispatches it when `retry_at` has passed.
- **Recovery**: Instance died while task was `Running`. The *recovery sweeper* resets the task to `Ready` with `retry_at = NULL` (immediate). The `recovery_attempts` counter increments (separate from `attempt`).

Both share the `Ready` state as their re-entry point, which means the existing dispatcher logic handles both without modification.

---

## 5. Integration with Cron Recovery

The existing `CronRecoveryService` handles a specific gap: cron executions that were claimed (have a `cron_executions` audit row) but never produced a `pipeline_execution_id`. This gap occurs when the cron scheduler claims a schedule and creates the audit row, but crashes before successfully handing off to the pipeline executor.

The new recovery sweeper handles a different gap: pipeline executions and tasks that were created and started but not completed due to an instance crash.

These two systems are complementary:

| Scenario | Which Recovery Handles It |
|----------|--------------------------|
| Cron fires, audit row created, pipeline handoff fails | CronRecoveryService |
| Cron fires, pipeline created, task starts, instance crashes | Recovery Sweeper |
| HTTP-triggered pipeline, task starts, instance crashes | Recovery Sweeper |
| Trigger-fired pipeline, task starts, instance crashes | Recovery Sweeper |

**Integration point**: When the Recovery Sweeper marks an instance as `dead`, it can optionally notify the `CronRecoveryService` to run an immediate check cycle rather than waiting for its next polling interval. This is a minor optimization, not a correctness requirement.

The `CronRecoveryService` does not need to be modified. It already handles its specific gap correctly. The only change is that its `lost_threshold_minutes` should be configured to be greater than `death_threshold` to avoid racing with the sweeper on tasks that are merely slow.

---

## 6. Preparing for Distributed Executors

The design intentionally creates a data model that supports distributed (remote) executors with minimal future changes:

### 6.1 What This Spec Provides

- **`claimed_by_instance_id`**: Today this is a server instance UUID. When distributed executors arrive, it becomes the worker UUID. The recovery logic is identical: if the claiming entity stops heartbeating, its work is recovered.
- **`server_instances` table**: Generalizes to a `worker_instances` table (or the same table with a `role` column). Remote workers register and heartbeat using the same protocol.
- **`Running` state**: Provides a durable marker that a task is in-flight. This is essential for distributed executors where network partitions can cause ambiguity.

### 6.2 What Distributed Executors Will Add

- **Task queue**: Today the dispatcher pushes tasks directly to in-process executors. Distributed executors will pull tasks from a queue (the `task_outbox` table already exists and can serve this role, or a dedicated queue table can be added).
- **Lease-based claiming**: Instead of the current `UPDATE ... SET status = 'Running'`, distributed workers will use `SELECT ... FOR UPDATE SKIP LOCKED` with a lease expiry. If the lease expires without completion, the task is automatically available for re-claiming.
- **Result reporting**: Remote workers will POST results back to the server, which updates task state. The server-side recovery sweeper remains responsible for detecting dead workers.

The key insight is that `claimed_by_instance_id` + heartbeat-based death detection is the same pattern whether the executor is in-process or remote. This spec builds the foundation.

---

## 7. Database Schema Changes

### 7.1 New Table: `server_instances`

```sql
CREATE TABLE server_instances (
    id                      UUID PRIMARY KEY,
    instance_name           VARCHAR NOT NULL,
    mode                    VARCHAR NOT NULL,        -- 'all', 'api', 'worker', 'scheduler'
    started_at              TIMESTAMP NOT NULL DEFAULT NOW(),
    last_heartbeat_at       TIMESTAMP NOT NULL DEFAULT NOW(),
    stopped_at              TIMESTAMP,
    recovered_at            TIMESTAMP,
    recovered_by_instance_id UUID,
    status                  VARCHAR NOT NULL DEFAULT 'active',
        -- 'active', 'stopped', 'dead', 'recovering', 'recovered'
    metadata                TEXT,                    -- JSON: capacity, version, etc.
    created_at              TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_server_instances_status ON server_instances (status);
CREATE INDEX idx_server_instances_heartbeat ON server_instances (status, last_heartbeat_at)
    WHERE status = 'active';
```

### 7.2 Column Addition: `task_executions`

```sql
ALTER TABLE task_executions
ADD COLUMN claimed_by_instance_id UUID;

CREATE INDEX idx_task_executions_claimed_by ON task_executions (claimed_by_instance_id)
    WHERE status = 'Running';
```

This is a nullable column. For embedded-mode (SQLite), it is always NULL. For server-mode, it is set when a task transitions to `Running`.

### 7.3 Column Addition: `pipeline_executions`

```sql
ALTER TABLE pipeline_executions
ADD COLUMN claimed_by_instance_id UUID;
```

Records which instance created and is managing the pipeline execution. Used for diagnostics and optional pipeline-level recovery.

### 7.4 Diesel Schema Addition

Add to `unified_schema`:

```rust
diesel::table! {
    server_instances (id) {
        id -> DbUuid,
        instance_name -> Text,
        mode -> Text,
        started_at -> DbTimestamp,
        last_heartbeat_at -> DbTimestamp,
        stopped_at -> Nullable<DbTimestamp>,
        recovered_at -> Nullable<DbTimestamp>,
        recovered_by_instance_id -> Nullable<DbUuid>,
        status -> Text,
        metadata -> Nullable<Text>,
        created_at -> DbTimestamp,
        updated_at -> DbTimestamp,
    }
}
```

---

## 8. Configuration

All recovery configuration is nested under `[recovery]` in the server TOML config and exposed as `DefaultRunnerConfig` fields:

```toml
[recovery]
# Enable the recovery sweeper (default: true for server mode, false for embedded)
enabled = true

# How often to check for dead instances and orphaned tasks
sweep_interval_seconds = 30

# How long after last heartbeat before an instance is considered dead
death_threshold_seconds = 60

# How often to send heartbeats
heartbeat_interval_seconds = 15

# Maximum number of recovery attempts per task before abandoning
max_recovery_attempts = 3

# Maximum number of orphaned tasks to recover in a single sweep
max_recovery_batch_size = 100

# Whether to run a recovery sweep on startup (covers self-recovery and peer recovery)
sweep_on_startup = true
```

### 8.1 ServeMode Mapping

| Mode      | Heartbeat | Recovery Sweeper | Task Execution | Notes |
|-----------|-----------|-----------------|----------------|-------|
| All       | Yes       | Yes             | Yes            | Full recovery participant |
| Worker    | Yes       | No              | Yes            | Heartbeats so others can detect its death |
| Scheduler | No        | Yes             | No             | Runs sweeper to recover dead workers |
| Api       | No        | No              | No             | Stateless, no recovery role |

---

## Requirements

### Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.1 | Each server instance registers itself with a unique ID on startup | Enables instance-scoped recovery |
| REQ-1.2 | Instances heartbeat at a configurable interval | Enables death detection |
| REQ-1.3 | Graceful shutdown sets instance status to 'stopped' | Avoids unnecessary recovery processing |
| REQ-2.1 | Tasks transition to 'Running' with instance claim before execution begins | Creates a durable marker for orphan detection |
| REQ-2.2 | The Running transition is atomic with a status guard | Prevents double-claiming |
| REQ-3.1 | Recovery sweeper detects instances whose heartbeat exceeds the death threshold | Enables automated failure detection |
| REQ-3.2 | Only one sweeper processes a given dead instance | Prevents duplicate recovery |
| REQ-3.3 | Orphaned Running tasks are reset to Ready with recovery_attempts incremented | Enables re-execution |
| REQ-3.4 | Tasks exceeding max recovery attempts are marked Failed | Prevents infinite recovery loops |
| REQ-4.1 | Recovery is idempotent -- running it twice produces the same result | Required for distributed safety |
| REQ-4.2 | Recovery integrates with existing CronRecoveryService without duplication | Clean separation of concerns |
| REQ-5.1 | Embedded mode (SQLite, single process) continues working without heartbeats | Backward compatibility |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-1.1 | Death detection within 60 seconds (configurable) | Limits the window of unrecoverable work |
| NFR-1.2 | Heartbeat overhead less than 1 query per 15 seconds per instance | Minimal database load |
| NFR-2.1 | Recovery of 100 orphaned tasks completes within 5 seconds | Bounded recovery latency |
| NFR-3.1 | No external coordination service required (PostgreSQL only) | Operational simplicity |

## Constraints

### Technical Constraints

- Must work with PostgreSQL 14+ (the minimum supported version for Cloacina server mode)
- Must use Diesel ORM for all database operations (consistent with the rest of the codebase)
- Schema changes must be delivered as Diesel migrations
- The `task_executions` table is high-traffic; index additions must be partial indexes to minimize write amplification
- SQLite backend does not support `FOR UPDATE SKIP LOCKED`; embedded mode must use a simpler code path

### Organizational Constraints

- The `cloacina-workflow` crate's `Task` trait cannot be modified (it is the public API for task authors)
- Tasks are required to be idempotent; this spec relies on that invariant for correctness
