---
id: unified-recovery-system-task
level: specification
title: "Unified Recovery System — Task Heartbeating Across All Execution Modes"
short_code: "CLOACI-S-0016"
created_at: 2026-03-23T16:10:42.537033+00:00
updated_at: 2026-03-23T16:10:42.537033+00:00
parent: CLOACI-I-0043
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Unified Recovery System — Task Heartbeating Across All Execution Modes

*This template provides structured sections for system-level design. Delete sections that don't apply to your specification.*

## Overview **[REQUIRED]**

**One recovery mechanism for all execution modes.** The same task heartbeat protocol handles server (PostgreSQL, horizontal scaling), daemon (SQLite, single instance), and continuous scheduling (watermarks, accumulators). Even if this "over-engineers" the daemon, the simplicity of having one system to understand, test, and maintain outweighs the cost.

Supersedes: S-0009 (daemon-specific), S-0010 (server-specific), S-0011 (continuous-specific). Those remain as reference material for mode-specific concerns but the implementation is unified.

## Core Principle

**Only tasks execute.** Pipelines are containers. Recovery operates at the task level only. Pipeline status is derived from task states.

**Task heartbeating is the single detection primitive.** Whether the executor is in-process, on another machine, or on another network — the only way the scheduler knows it's alive is the heartbeat timestamp in the database.

## The Protocol

### 1. Claim

Before executing, the executor atomically claims the task:

```sql
UPDATE task_executions
SET status = 'Running', claimed_by = :executor_id, heartbeat_at = NOW(), started_at = NOW()
WHERE id = :task_id AND status = 'Ready'
RETURNING id
```

If 0 rows returned → another executor claimed it first. Move on.

### 2. Heartbeat

Every 10s while executing:

```sql
UPDATE task_executions SET heartbeat_at = NOW()
WHERE id = :task_id AND claimed_by = :executor_id
```

If 0 rows → task was recovered and reassigned. Stop executing (idempotent tasks mean this is safe).

### 3. Complete / Fail

```sql
UPDATE task_executions
SET status = 'Completed', completed_at = NOW(), heartbeat_at = NULL, claimed_by = NULL
WHERE id = :task_id AND claimed_by = :executor_id
```

### 4. Detect Orphans

Recovery sweeper runs every 30s with two modes:

**Startup mode** (first `startup_grace` seconds, default 120s):
Only recover tasks that were stale BEFORE this instance started — definitely orphaned from a previous session, not tasks that lost heartbeats during the restart window.

```sql
SELECT * FROM task_executions
WHERE status = 'Running'
  AND heartbeat_at < :sweeper_start_time - INTERVAL '60 seconds'
```

**Normal mode** (after grace period):
Real-time orphan detection for tasks that go stale during operation.

```sql
SELECT * FROM task_executions
WHERE status = 'Running'
  AND heartbeat_at < NOW() - INTERVAL '60 seconds'
```

**Why the grace period matters:**
A service might take 60+ seconds to restart (migrations, reconciler, package loading). Without the grace period, the sweeper would immediately recover tasks whose heartbeats are only stale because of the restart gap — not because the executor is dead. The grace period ensures we only recover tasks from the *previous* session during startup, then switch to live detection once the system is stable.

```
Timeline:
  T+0s     crash
  T+0-60s  no heartbeats (process dead)
  T+60s    restart begins
  T+65s    sweeper first tick (startup mode)
           → only recovers tasks with heartbeat < T+0s-60s = T-60s
           → tasks heartbeating at T+0s are NOT recovered (grace window)
  T+180s   grace period ends, switches to normal mode
           → any task with heartbeat > 60s stale is recovered
```

### 5. Recover

For each orphan:
- If `recovery_attempts >= 3` → mark Failed ("Abandoned after 3 recovery attempts")
- Else → reset to Ready, clear claimed_by/heartbeat_at, increment recovery_attempts, insert task_outbox entry

## Schema Changes

```sql
-- task_executions (ALTER existing table)
ALTER TABLE task_executions ADD COLUMN claimed_by TEXT NULL;
ALTER TABLE task_executions ADD COLUMN heartbeat_at TIMESTAMP NULL;

-- Index for orphan scan (partial index, only Running tasks)
CREATE INDEX idx_task_exec_orphan_scan
  ON task_executions(status, heartbeat_at)
  WHERE status = 'Running';

-- Runner instances (for server mode coordination, optional for daemon)
CREATE TABLE runner_instances (
  id TEXT PRIMARY KEY,
  started_at TIMESTAMP NOT NULL,
  last_heartbeat_at TIMESTAMP NOT NULL,
  status TEXT NOT NULL DEFAULT 'active',
  mode TEXT NOT NULL DEFAULT 'all'
);
```

## How Each Mode Uses the Same System

### Server Mode (PostgreSQL, multiple instances)
- Each server registers in `runner_instances` on startup
- Instance heartbeats every 15s (separate from task heartbeats)
- Recovery sweeper runs on scheduler-mode instances
- `FOR UPDATE SKIP LOCKED` on task claiming for zero-contention
- Instance table used for operational visibility (which instances are alive)

### Daemon Mode (SQLite, single instance)
- Single instance — `runner_instances` table exists but has one row
- Same claim → heartbeat → complete protocol
- Recovery sweeper runs in the same process
- SQLite IMMEDIATE transactions for claiming (serialized, no contention)
- On startup: sweeper runs immediately to recover previous session's orphans

### Continuous Scheduling
- When boundaries are drained and tasks dispatched, they enter the same task lifecycle
- Claim → heartbeat → complete, same as cron or on-demand tasks
- Additional continuous-specific recovery (from S-0011):
  - WAL write must happen BEFORE accumulator routing
  - Post-execution persistence batched in single transaction
  - Source watermark persistence for restart resume
- These are complementary to task heartbeating, not alternatives

### Future Distributed Executors
- Remote worker registers in `runner_instances` (or a future `workers` table)
- Same claim/heartbeat/complete protocol, but via API instead of direct DB
- `claimed_by` already stores executor identity — works for any executor type
- Heartbeat writes go through API → database (same column, same detection)

## Cron Recovery Elimination

The existing `CronRecoveryService` detects cron schedules that were claimed but never produced a pipeline. This gap exists because the cron scheduler performs three separate operations:
1. Create audit record
2. Create pipeline + tasks
3. Link audit to pipeline

If the process dies between steps 1 and 2, the audit record exists with no pipeline.

**Fix:** Make steps 1-3 a single atomic transaction. Either all three commit or none do. With correct transaction boundaries, the "claimed but no pipeline" gap cannot occur.

Once atomic, the heartbeat sweeper covers everything after pipeline/task creation. `CronRecoveryService` becomes unnecessary.

```
Before: Two recovery systems for two gaps
  CronRecoveryService:  schedule claimed → pipeline NOT created
  RecoverySweeper:      task claimed     → task NOT completed

After: One recovery system, one gap eliminated by transaction fix
  RecoverySweeper:      task claimed     → task NOT completed
  (cron gap eliminated by atomic schedule claim + pipeline + task creation)
```

## Pipeline Status Derivation

After recovering orphaned tasks, the pipeline status is re-evaluated by the scheduler loop:
- All tasks Completed/Skipped → pipeline Completed
- Any task permanently Failed → pipeline Failed
- Any task still active → pipeline unchanged

The current bug (`complete_pipeline()` ignoring failures) must be fixed as part of this initiative.

## Configuration

All under `DefaultRunnerConfig`:

| Option | Default | Description |
|--------|---------|-------------|
| `task_heartbeat_interval` | 10s | How often executors heartbeat |
| `task_orphan_threshold` | 60s | Stale heartbeat = orphaned |
| `recovery_sweep_interval` | 30s | How often sweeper runs |
| `recovery_startup_grace` | 120s | Grace period after startup before live orphan detection |
| `max_recovery_attempts` | 3 | Before abandoning a task |
| `enable_recovery_sweeper` | true | Enable/disable |
| `instance_heartbeat_interval` | 15s | Runner instance heartbeat (server) |
| `instance_death_threshold` | 60s | Runner instance considered dead (server) |

## What Gets Removed

- `task_scheduler/recovery.rs::RecoveryManager` — dead code, replaced by recovery sweeper
- `cron_recovery.rs::CronRecoveryService` — gap eliminated by atomic cron transaction
- The startup-only recovery call in `TaskScheduler::with_poll_interval()` — replaced by sweeper's startup mode
- All `cron_recovery_*` config options — replaced by unified recovery config
- Separate recovery designs per mode — one system for all

## System Context **[CONDITIONAL: System-Level Spec]**

{Delete for project-level specifications}

### Actors
- **{Actor 1}**: {Role and interaction pattern}
- **{Actor 2}**: {Role and interaction pattern}

### External Systems
- **{System 1}**: {Integration description}
- **{System 2}**: {Integration description}

### Boundaries
{What is inside vs outside the system scope}

## Requirements **[REQUIRED]**

### Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.1.1 | {Requirement description} | {Why this is needed} |
| REQ-1.1.2 | {Requirement description} | {Why this is needed} |
| REQ-1.2.1 | {Requirement description} | {Why this is needed} |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-1.1.1 | {Requirement description} | {Why this is needed} |
| NFR-1.1.2 | {Requirement description} | {Why this is needed} |

## Architecture Framing **[CONDITIONAL: System-Level Spec]**

{Delete for project-level specifications}

### Decision Area: {Area Name}
- **Context**: {What needs to be decided}
- **Constraints**: {Hard constraints that bound the decision}
- **Required Capabilities**: {What the solution must support}
- **ADR**: {Link to ADR when decision is made, e.g., PROJ-A-0001}

## Decision Log **[CONDITIONAL: Has ADRs]**

{Delete if no architectural decisions have been made yet}

| ADR | Title | Status | Summary |
|-----|-------|--------|---------|
| {PROJ-A-0001} | {Decision title} | {decided/superseded} | {One-line summary} |

## Constraints **[CONDITIONAL: Has Constraints]**

{Delete if no hard constraints exist}

### Technical Constraints
- {Constraint 1}
- {Constraint 2}

### Organizational Constraints
- {Constraint 1}

### Regulatory Constraints
- {Constraint 1}

## Changelog **[REQUIRED after publication]**

{Track significant changes after initial publication. Delete this section until the specification is published.}

| Date | Change | Rationale |
|------|--------|-----------|
| {YYYY-MM-DD} | {What changed} | {Why it changed} |
