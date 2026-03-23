---
id: recovery-system-redesign-task
level: initiative
title: "Recovery System Redesign — Task Heartbeating, Orphan Detection, Crash Recovery"
short_code: "CLOACI-I-0043"
created_at: 2026-03-23T02:22:02.573302+00:00
updated_at: 2026-03-23T02:23:29.587804+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/design"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: recovery-system-redesign-task
---

# Recovery System Redesign — Task Heartbeating, Orphan Detection, Crash Recovery Initiative

## Context

Deep audit of the recovery system (triggered by T-0232 chaos test failure) revealed the recovery system is **fundamentally broken** due to an architecture shift from pull-based to push-based task execution. The system was designed for executors that poll a database outbox; it now uses a push-based dispatcher that sends events directly to in-process executors. This breaks every assumption the recovery system makes.

**Key findings:**
1. `RecoveryManager` is dead code — runs once at startup, never again
2. Executor never marks tasks "Running" — push model skips the claiming step
3. Outbox entries not recreated on recovery — recovered tasks invisible to dispatcher
4. Pipeline marked "Completed" even if tasks failed
5. Same bugs affect server, daemon, and continuous scheduling modes
6. No mechanism to detect orphaned tasks in the current push model

**Critical constraint:** Future distributed executors will be autonomous processes on potentially different networks. The scheduler has no direct access to executor processes. Recovery must work purely via database state — specifically, **task-level heartbeating**.

Absorbs: T-0232 (daemon crash recovery bug)

Reference specifications:
- **CLOACI-S-0016** — Unified Recovery System (supersedes S-0009/S-0010/S-0011)
- CLOACI-S-0012 — Task Heartbeat Protocol (detail spec)
- CLOACI-S-0013 — Recovery Sweeper Service (detail spec)
- CLOACI-S-0014 — Pipeline Status Derivation (detail spec)

Archived (reference material only): S-0009 (daemon), S-0010 (server), S-0011 (continuous)

## Goals & Non-Goals

**Goals:**
- Every task execution has a durable "Running" state with a heartbeat timestamp
- Orphaned tasks (stale heartbeat) are automatically detected and recovered
- Recovery works identically across server, daemon, and continuous scheduling
- Design accommodates future distributed executors (remote processes, different networks)
- Chaos soak test passes: kill process mid-execution → restart → all tasks eventually complete
- Recovery is idempotent — running it twice doesn't cause double-execution

**Non-Goals:**
- Implementing distributed executors (just preparing the primitives)
- Exactly-once task execution guarantees (at-least-once is sufficient; tasks must be idempotent)
- Changing the cron recovery service (it handles a different layer — schedule-to-pipeline handoff)

## Architecture

### Core Primitive: Task Heartbeating

Only tasks are executed — pipelines are containers that derive status from their tasks. Recovery operates at the task level only.

```
Task Lifecycle (new):

  NotStarted → Pending → Ready → Running → Completed
                                    ↑  ↓         ↓
                                    │  Failed → (retry → Ready)
                                    │
                              heartbeat_at updated
                              every N seconds while running
```

### Task Claiming (the missing step)

Current (broken):
```
Scheduler: mark_ready() → dispatch(event) → Executor: execute inline → mark_completed()
                                             (no Running state, no claiming)
```

Fixed:
```
Scheduler: mark_ready()
Executor:  claim_task(task_id, executor_id) → sets Running + heartbeat_at + claimed_by
Executor:  heartbeat(task_id) every 10s while executing
Executor:  mark_completed(task_id) or mark_failed(task_id)
```

The `claim_task` step is atomic: `UPDATE task_executions SET status='Running', claimed_by=?, heartbeat_at=NOW() WHERE id=? AND status='Ready'`. If two executors race, only one succeeds.

### Orphan Detection

A **Recovery Sweeper** background service runs periodically (every 30s):

```sql
SELECT * FROM task_executions
WHERE status = 'Running'
  AND heartbeat_at < NOW() - INTERVAL '60 seconds'
```

For each orphaned task:
1. Increment `recovery_attempts`
2. If `recovery_attempts >= max_recovery_attempts` → mark Failed ("abandoned after N recovery attempts")
3. Otherwise → reset to Ready, clear `claimed_by`, re-insert task_outbox entry

### Executor Identity

| Mode | Executor identity | Heartbeat mechanism |
|------|------------------|---------------------|
| **Daemon** (in-process) | Instance UUID | Direct DB write every 10s |
| **Server** (in-process) | Instance UUID from `server_instances` table | Direct DB write every 10s |
| **Future distributed** | Remote worker UUID | API call or direct DB write every 10s |

The `claimed_by` column on `task_executions` stores the executor identity. For in-process executors, this is the runner's instance UUID. For future remote workers, this will be the worker's UUID.

### Pipeline Status Derivation

Pipelines don't need recovery — their status is derived:
- All tasks Completed → pipeline Completed
- Any task Failed (permanently) → pipeline Failed
- Any task still active → pipeline still active

The scheduler loop already does this in `check_pipeline_completion()`. The fix: ensure "Failed" is included as terminal and check for permanent failures.

### Schema Changes

```sql
-- Add to task_executions
ALTER TABLE task_executions ADD COLUMN claimed_by TEXT NULL;     -- executor/instance UUID
ALTER TABLE task_executions ADD COLUMN heartbeat_at TIMESTAMP NULL;  -- last heartbeat

-- Server instances (multi-instance coordination)
CREATE TABLE IF NOT EXISTS runner_instances (
    id TEXT PRIMARY KEY,              -- UUID
    started_at TIMESTAMP NOT NULL,
    last_heartbeat_at TIMESTAMP NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',  -- active, stopped, dead
    mode TEXT NOT NULL DEFAULT 'all'        -- all, scheduler, worker, api
);

-- Index for recovery sweeper
CREATE INDEX idx_task_exec_running_heartbeat
    ON task_executions(status, heartbeat_at)
    WHERE status = 'Running';
```

### Continuous Scheduling Integration

Continuous scheduling has its own persistence layer (WAL, watermarks, accumulators). The task heartbeat model applies to continuous tasks too — when a boundary is drained and tasks are dispatched, they follow the same claim → heartbeat → complete lifecycle.

Additional fixes from CLOACI-S-0011:
- WAL write must happen BEFORE accumulator routing (crash between them loses boundaries)
- Post-execution persistence (accumulator state, drain cursors, detector state) should be batched in a single transaction
- Source watermark persistence needed for restart resume

## Alternatives Considered

**Instance-level heartbeat only (no per-task heartbeat):** Simpler but can't detect individual task hangs. A live instance with one stuck task would never be recovered. Rejected — per-task heartbeat is needed for distributed executors anyway.

**Timeout-based detection (no heartbeat, just age):** Use `started_at` + timeout threshold. Simpler but can't distinguish slow tasks from dead executors. A 30-minute analytical task would be incorrectly recovered with a 60s threshold. Rejected — heartbeat is more precise.

**Separate recovery per mode (server vs daemon vs continuous):** Each mode gets custom recovery logic. Rejected — leads to divergence and bugs. One recovery sweeper with the same heartbeat primitive works everywhere.

## Implementation Plan

### Phase 1: Task lifecycle fix (foundation)
- Add `claimed_by` and `heartbeat_at` columns to `task_executions` (migration)
- Add `mark_running(task_id, executor_id)` DAL method — atomic claim with heartbeat
- Update `ThreadTaskExecutor` to call `mark_running()` before execution
- Update `ThreadTaskExecutor` to call heartbeat every 10s during execution
- Remove dead `RecoveryManager` code (startup-only recovery)

### Phase 2: Recovery sweeper
- Create `RecoverySweeper` background service (replaces dead RecoveryManager)
- Query for stale heartbeats every 30s
- Reset orphaned tasks to Ready + re-insert task_outbox
- Respect `max_recovery_attempts` limit
- Wire into `DefaultRunner::start_background_services()`

### Phase 3: Instance registration (server mode)
- Create `runner_instances` table (migration)
- Register instance on startup, update heartbeat
- Recovery sweeper uses instance liveness for faster detection
- Graceful shutdown marks instance as "stopped"

### Phase 4: Pipeline status fix
- Fix `check_pipeline_completion()` to detect task failures
- Pipeline marked "Failed" if any task permanently failed
- Pipeline marked "Completed" only if ALL tasks completed

### Phase 5: Continuous scheduling fixes
- Move WAL write before accumulator routing
- Batch post-execution persistence in single transaction
- Add source watermark persistence table

### Phase 6: Chaos testing
- Daemon chaos soak passes (--chaos flag)
- Server chaos soak passes (kill mid-execution)
- Continuous scheduling crash recovery test
- All existing soak tests still pass
