---
id: daemon-recovery-system-design
level: specification
title: "Daemon Recovery System Design"
short_code: "CLOACI-S-0009"
created_at: 2026-03-23T02:16:25.987072+00:00
updated_at: 2026-03-23T02:16:25.987072+00:00
parent: CLOACI-I-0040
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Daemon Recovery System Design

Recovery system for `cloacinactl daemon` mode -- the single-instance background process that watches a directory for `.cloacina` packages, registers them, schedules cron executions, and runs workflows locally using SQLite.

## Overview

The daemon is Cloacina's "just works" deployment mode. When a daemon is killed (SIGKILL, OOM, power loss) mid-execution, it must recover cleanly on restart and maintain data integrity during operation. Today, recovery has two gaps:

1. **RecoveryManager runs once at startup, never again.** It finds tasks stuck in `Running` state and resets them to `Ready`, but only during `TaskScheduler::with_poll_interval()` construction. If the daemon runs for days and a task executor thread panics or a timeout is missed, those orphaned tasks are never recovered.

2. **CronRecoveryService handles cron-claim-to-pipeline gaps, but not task-level orphans.** It detects `cron_executions` records where `pipeline_execution_id` is NULL (claimed but never handed off). It does not address tasks that were dispatched but never completed.

These two recovery paths are independent and uncoordinated. This specification defines a unified daemon recovery system that addresses both startup and periodic recovery for SQLite-backed daemon mode.

## Current State Analysis

### What exists today

| Component | Location | Scope | When it runs | Gap |
|-----------|----------|-------|-------------|-----|
| `RecoveryManager` | `task_scheduler/recovery.rs` | Tasks stuck in `Running` | Once at `TaskScheduler` construction | Never runs again after startup |
| `CronRecoveryService` | `cron_recovery.rs` | Cron executions with NULL `pipeline_execution_id` | Periodic (default 5min) | Does not recover task-level orphans |
| `ThreadTaskExecutor` timeout | `executor/thread_task_executor.rs` | Individual task timeout via `tokio::time::timeout` | Per-task execution | If executor itself crashes, timeout never fires |
| Graceful shutdown | `daemon.rs` / `DefaultRunner::shutdown()` | Clean state on SIGTERM/Ctrl+C | On signal | No effect on SIGKILL/OOM |

### Task state lifecycle

```
Pending --> Ready --> Running --> Completed
                        |
                        +--> Failed (permanent, after retries exhausted)
                        +--> Abandoned (recovery limit exceeded)
```

The critical gap: a task in `Running` state when the daemon dies will remain `Running` forever unless recovery runs. Today, recovery only runs at startup.

### How recovery currently integrates

```
TaskScheduler::with_poll_interval()
  |
  +-- RecoveryManager::recover_orphaned_tasks()  <-- runs ONCE
  |     |
  |     +-- find tasks WHERE status = 'Running'
  |     +-- reset to 'Ready' (if workflow exists) or 'Abandoned'
  |
  +-- returns TaskScheduler (recovery never runs again)

CronRecoveryService::run_recovery_loop()  <-- runs periodically
  |
  +-- find cron_executions WHERE pipeline_execution_id IS NULL
  +-- AND created_at < (now - threshold)
  +-- re-execute workflow via PipelineExecutor
```

## Design

### 1. Startup Recovery Sequence

On daemon startup, before the scheduler loop begins processing new work, execute the following sequence synchronously:

```
daemon start
  |
  1. SQLite WAL checkpoint (integrity)
  |
  2. Write daemon instance marker (pid + start_time)
  |
  3. Recover orphaned tasks (existing RecoveryManager logic)
  |     - Tasks in 'Running' --> 'Ready' (if workflow exists, recovery_attempts < 3)
  |     - Tasks in 'Running' --> 'Abandoned' (if workflow gone or attempts exceeded)
  |     - Pipelines with all-abandoned tasks --> 'Failed'
  |
  4. Recover stale pipelines
  |     - Pipelines in 'Running' with no 'Running' or 'Ready' tasks --> 'Failed'
  |     - Pipelines in 'Pending' older than pipeline_timeout --> 'Failed'
  |
  5. Cron recovery (existing CronRecoveryService.check_and_recover_lost_executions)
  |     - cron_executions with NULL pipeline_execution_id --> re-execute
  |
  6. Start background services (scheduler loop, cron scheduler, cron recovery, etc.)
```

**Step 1: SQLite WAL checkpoint.** After a crash, SQLite's WAL file may contain uncommitted pages. SQLite handles this automatically on connection open, but we should force a `PRAGMA wal_checkpoint(TRUNCATE)` to ensure the WAL is clean and the database file is self-contained. This also reclaims disk space from the WAL.

**Step 2: Daemon instance marker.** Write a record to a `daemon_instance` table (or a file-based lock) with the current PID and start timestamp. On next startup, the previous marker tells us the daemon crashed (PID no longer running). This is informational for logging and future distributed-executor use -- it is not required for correctness in single-instance mode.

**Step 3-4: Task and pipeline recovery.** This is the existing `RecoveryManager::recover_orphaned_tasks()` logic, extended with stale pipeline detection. The key addition is pipelines that are stuck in `Running` or `Pending` but have no actionable tasks remaining.

**Step 5: Cron recovery.** Run the existing `CronRecoveryService::check_and_recover_lost_executions()` once synchronously before the periodic loop starts. This ensures that cron executions lost during a crash are recovered immediately rather than waiting for the first periodic check (default 5 minutes).

### 2. Periodic Recovery

Add a new `DaemonRecoveryService` that runs alongside the existing `CronRecoveryService`. The two services have distinct, non-overlapping responsibilities:

| Service | Detects | Recovers by | Interval |
|---------|---------|-------------|----------|
| `DaemonRecoveryService` (NEW) | Tasks stuck in `Running` longer than `task_stuck_threshold` | Reset to `Ready` (same as startup RecoveryManager) | `recovery_check_interval` (default: 60s) |
| `DaemonRecoveryService` (NEW) | Pipelines stuck in `Running`/`Pending` with no actionable tasks | Mark `Failed` | Same interval |
| `CronRecoveryService` (EXISTS) | `cron_executions` with NULL `pipeline_execution_id` | Re-execute workflow | `cron_recovery_interval` (default: 300s) |

**No overlap:** `DaemonRecoveryService` operates on `task_executions` and `pipeline_executions` tables. `CronRecoveryService` operates on `cron_executions` table. They recover different failure modes.

#### DaemonRecoveryService design

```rust
pub struct DaemonRecoveryService {
    dal: Arc<DAL>,
    config: DaemonRecoveryConfig,
    shutdown: watch::Receiver<bool>,
}

pub struct DaemonRecoveryConfig {
    /// How often to check for stuck tasks/pipelines
    pub check_interval: Duration,          // default: 60s
    /// Tasks in 'Running' longer than this are considered stuck
    pub task_stuck_threshold: Duration,     // default: task_timeout * 2
    /// Pipelines in 'Running'/'Pending' longer than this with no progress
    pub pipeline_stuck_threshold: Duration, // default: pipeline_timeout or 2h
    /// Maximum recovery attempts per task before abandoning
    pub max_recovery_attempts: i32,         // default: 3 (matches existing)
}
```

The periodic check:

```
DaemonRecoveryService::check_cycle()
  |
  1. Find tasks WHERE status = 'Running'
  |    AND updated_at < (now - task_stuck_threshold)
  |
  2. For each stuck task:
  |    - If recovery_attempts < max: reset to 'Ready', increment recovery_attempts
  |    - If recovery_attempts >= max: mark 'Abandoned', record recovery_event
  |
  3. Find pipelines WHERE status IN ('Running', 'Pending')
  |    AND no tasks in 'Running' or 'Ready' or 'Pending' state
  |    AND updated_at < (now - pipeline_stuck_threshold)
  |
  4. For each stuck pipeline: mark 'Failed', record recovery_event
```

**Why `task_stuck_threshold` defaults to `task_timeout * 2`:** The executor already applies `tokio::time::timeout` at `task_timeout`. If a task is still `Running` after 2x that duration, the executor that owned it is gone. Using 2x avoids false positives where a task is legitimately running near its timeout boundary.

### 3. Task State Lifecycle Fixes

#### 3a. Add `started_at` timestamp to task_executions

Currently, tasks transition to `Running` but we only have `updated_at` to detect staleness. Add a `started_at` column that is set when the executor marks a task as `Running`. This gives a precise anchor for stuck-task detection:

```sql
ALTER TABLE task_executions ADD COLUMN started_at TIMESTAMP NULL;
```

The stuck-task query becomes:
```sql
SELECT * FROM task_executions
WHERE status = 'Running'
  AND started_at < datetime('now', '-' || ? || ' seconds')
```

Without `started_at`, we must rely on `updated_at`, which is acceptable but less precise since other metadata updates can bump `updated_at`.

#### 3b. Mark tasks Running atomically with started_at

In `ThreadTaskExecutor::execute()`, before executing the task, update the task status to `Running` with `started_at = now()`. This is not currently done explicitly -- the dispatcher pushes the task event and execution begins immediately. The DAL should provide:

```rust
pub async fn mark_running(&self, task_id: UniversalUuid) -> Result<(), ValidationError>
```

This sets `status = 'Running'`, `started_at = now()`, `updated_at = now()`.

#### 3c. Heartbeat for long-running tasks (future consideration)

For the initial implementation, the stuck threshold is sufficient. For future distributed executors, tasks should periodically update a `last_heartbeat` column. This is deferred to keep the daemon implementation simple.

### 4. Integration with Existing Cron Recovery

The two recovery services are complementary and must not interfere:

**CronRecoveryService responsibility boundary:**
- Input: `cron_executions` rows where `pipeline_execution_id IS NULL` and `created_at < threshold`
- Action: Re-execute the workflow (creates new pipeline_execution)
- Output: Updates `cron_executions.pipeline_execution_id` to link to the new execution

**DaemonRecoveryService responsibility boundary:**
- Input: `task_executions` rows where `status = 'Running'` and `started_at < threshold`
- Input: `pipeline_executions` rows where `status IN ('Running', 'Pending')` and no live tasks
- Action: Reset tasks to `Ready` or `Abandoned`; mark pipelines `Failed`
- Output: Updates `task_executions.status`, `pipeline_executions.status`

**Ordering guarantee at startup:** The daemon recovery (tasks/pipelines) runs BEFORE cron recovery. This ensures that any pipelines from the previous daemon instance are resolved before cron recovery potentially creates new ones for the same schedule.

**No shared state:** The two services do not share mutexes, rate limiters, or attempt counters. `CronRecoveryService` tracks its own `recovery_attempts` HashMap per `cron_execution.id`. `DaemonRecoveryService` uses the existing `task_executions.recovery_attempts` column.

### 5. SQLite-Specific Considerations

#### 5a. WAL mode and crash recovery

SQLite with WAL mode (which Cloacina uses) provides automatic crash recovery: on the next connection open, SQLite replays or discards the WAL. The daemon should:

1. **Force WAL checkpoint at startup** via `PRAGMA wal_checkpoint(TRUNCATE)`. This ensures the main database file is up to date and reclaims WAL disk space. After a crash, the WAL may be large.

2. **Run `PRAGMA integrity_check` optionally.** This is slow on large databases but can be enabled via a `--check-integrity` flag for paranoid mode. Not run by default.

3. **Set `PRAGMA journal_size_limit`** to bound WAL growth during operation (e.g., 64MB). Prevents unbounded WAL growth if checkpointing falls behind.

#### 5b. Single-writer simplification

SQLite allows only one writer at a time (WAL mode allows concurrent readers). Since the daemon is single-instance with a small connection pool (4 connections), writer contention is minimal. The recovery service does not need distributed locking or instance-aware claiming -- it can safely assume it is the only writer.

This means:
- No need for `FOR UPDATE` / `SKIP LOCKED` patterns (Postgres-isms)
- No need for instance_id in recovery queries
- `claim_and_update` on cron_schedules still works correctly (single writer = no contention)

#### 5c. Busy timeout

Set `PRAGMA busy_timeout = 5000` (5 seconds) to handle brief writer contention between the daemon's own connection pool threads. This prevents `SQLITE_BUSY` errors during concurrent recovery + scheduling operations.

### 6. Preparation for Distributed Executors

The design intentionally separates "what needs recovery" from "who performs recovery":

**Current (single-instance daemon):**
- Recovery assumes all `Running` tasks are orphaned (the only executor is this process, and it just started)
- No instance tracking needed

**Future (daemon + remote executors):**
- `Running` tasks may belong to a live remote executor -- cannot blindly reset them
- Need: `executor_instance_id` column on `task_executions` (which executor claimed this task)
- Need: executor heartbeat table (`executor_instances` with `last_heartbeat`)
- Recovery query becomes: tasks WHERE `status = 'Running'` AND executor's `last_heartbeat` is stale

**What we build now that helps later:**
- `started_at` column -- already needed for distributed stuck detection
- `recovery_events` table -- already exists, provides audit trail
- `DaemonRecoveryService` as a separate service -- can be extended to check executor liveness
- `DaemonRecoveryConfig` with configurable thresholds -- distributed mode needs different defaults

**What we explicitly defer:**
- `executor_instance_id` column (no remote executors yet)
- Executor heartbeat protocol
- Instance-aware recovery queries
- Distributed locking for recovery coordination

### 7. Configuration

New configuration options added to `DefaultRunnerConfig`:

```rust
// In DefaultRunnerConfigBuilder:

/// Enable periodic daemon recovery (task/pipeline level)
pub fn enable_daemon_recovery(mut self, value: bool) -> Self

/// How often to check for stuck tasks/pipelines (default: 60s)
pub fn daemon_recovery_interval(mut self, value: Duration) -> Self

/// How long a Running task must be stuck before recovery (default: task_timeout * 2)
pub fn daemon_task_stuck_threshold(mut self, value: Duration) -> Self

/// How long a stuck pipeline must be idle before marking Failed (default: pipeline_timeout or 7200s)
pub fn daemon_pipeline_stuck_threshold(mut self, value: Duration) -> Self

/// Force WAL checkpoint on startup (default: true)
pub fn sqlite_wal_checkpoint_on_startup(mut self, value: bool) -> Self
```

CLI flags on `cloacinactl daemon`:

```
--recovery-interval <SECONDS>    How often to check for stuck tasks (default: 60)
--task-stuck-threshold <SECONDS> Mark Running tasks as stuck after this duration (default: 600)
--no-recovery                    Disable periodic daemon recovery
--check-integrity                Run PRAGMA integrity_check on startup (slow)
```

### 8. Changes from Current Behavior

| Area | Before | After |
|------|--------|-------|
| Startup task recovery | Runs once in `TaskScheduler::with_poll_interval()` | Same logic, but also runs pipeline-level recovery and WAL checkpoint |
| Startup cron recovery | First check after `cron_recovery_interval` (5min) | Immediate synchronous check before loop starts |
| Periodic task recovery | Never | Every `daemon_recovery_interval` (60s) |
| Periodic pipeline recovery | Never | Every `daemon_recovery_interval` (60s) |
| Periodic cron recovery | Every `cron_recovery_interval` (5min) | Unchanged |
| Task stuck detection | Based on `updated_at` | Based on new `started_at` column (falls back to `updated_at`) |
| WAL management | Automatic (SQLite default) | Explicit checkpoint on startup + journal_size_limit |
| Recovery events | Logged for task resets | Extended to pipeline failures and stuck detection |

### 9. Implementation Tasks

1. **Add `started_at` migration** -- new column on `task_executions`, set in `mark_running()`
2. **Add `mark_running()` to DAL** -- sets status + started_at atomically
3. **Call `mark_running()` from ThreadTaskExecutor** -- before executing the task
4. **Create `DaemonRecoveryService`** -- periodic stuck-task and stuck-pipeline detection
5. **Add startup WAL checkpoint** -- `PRAGMA wal_checkpoint(TRUNCATE)` before recovery
6. **Add startup cron recovery** -- synchronous `check_and_recover_lost_executions()` call
7. **Wire into DefaultRunner** -- new background service handle, config options
8. **Wire into daemon.rs** -- CLI flags mapped to config
9. **Add daemon instance marker** -- table or lock file with PID/timestamp
10. **Tests** -- recovery scenarios with SQLite (crash simulation via unclean shutdown)

## Constraints

### Technical Constraints
- Must work with SQLite (daemon's primary and only backend)
- Single-instance process -- no distributed coordination required
- Recovery must complete before scheduler loop starts accepting new work
- Must not break existing Postgres-based server recovery (feature-flag separation)
- `task_timeout` default is 300s; recovery threshold must be safely above this

### Operational Constraints
- Daemon is the "just works" mode -- recovery should require zero configuration
- Defaults must be safe for common workloads (cron jobs running every few minutes)
- Recovery logging must be clear enough for users to understand what happened post-crash
