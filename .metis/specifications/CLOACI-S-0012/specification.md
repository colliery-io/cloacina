---
id: task-heartbeat-protocol-claiming
level: specification
title: "Task Heartbeat Protocol — Claiming, Heartbeating, and Orphan Detection"
short_code: "CLOACI-S-0012"
created_at: 2026-03-23T02:23:39.054318+00:00
updated_at: 2026-03-23T02:23:39.054318+00:00
parent: CLOACI-I-0043
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Task Heartbeat Protocol — Claiming, Heartbeating, and Orphan Detection

*This template provides structured sections for system-level design. Delete sections that don't apply to your specification.*

## Overview **[REQUIRED]**

The protocol by which executors claim tasks, maintain heartbeats during execution, and how the system detects orphaned tasks. This is the foundational primitive for all recovery — every mode (server, daemon, continuous) and every executor type (in-process, future distributed) uses this same protocol.

Only tasks are executed. Pipelines are containers that derive status from their tasks.

## Task Claiming

Atomic claim prevents double-execution:

```sql
UPDATE task_executions
SET status = 'Running',
    claimed_by = :executor_id,
    heartbeat_at = NOW(),
    started_at = NOW()
WHERE id = :task_id AND status = 'Ready'
```

`claimed_by` is an opaque string: runner instance UUID (in-process) or worker UUID (future distributed).

## Heartbeat Protocol

Executor updates `heartbeat_at` every 10s while running:

```sql
UPDATE task_executions
SET heartbeat_at = NOW()
WHERE id = :task_id AND claimed_by = :executor_id
```

The `AND claimed_by` guard ensures a recovered-and-reassigned task silently rejects the old executor's heartbeat (0 rows affected).

## Orphan Detection

A task is orphaned when `status = 'Running' AND heartbeat_at < NOW() - 60s`.

The 60s threshold must be > 2x the heartbeat interval to tolerate network jitter and GC pauses.

## Schema Changes

```sql
ALTER TABLE task_executions ADD COLUMN claimed_by TEXT NULL;
ALTER TABLE task_executions ADD COLUMN heartbeat_at TIMESTAMP NULL;
CREATE INDEX idx_task_exec_orphan_scan ON task_executions(status, heartbeat_at) WHERE status = 'Running';
```

## In-Process Executor

For `ThreadTaskExecutor`, heartbeating is a background tokio task spawned alongside the task execution. Aborted when the task completes or fails.

## Distributed Executor Interface (future)

Same protocol via API: `POST /tasks/{id}/claim`, `PUT /tasks/{id}/heartbeat`, `POST /tasks/{id}/complete`. Database operations are identical — the API is a thin wrapper.

## Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1 | Atomic task claiming (WHERE status='Ready') | Prevents double-execution |
| REQ-2 | Heartbeat every 10s (configurable) | Liveness detection |
| REQ-3 | Orphan threshold 60s (configurable, > 2x heartbeat) | Avoid false positives |
| REQ-4 | `claimed_by` guard on heartbeat and completion | Fencing against stale executors |
| REQ-5 | Works with both PostgreSQL and SQLite | Daemon + server |
| NFR-1 | Heartbeat write < 5ms p99 | Must not slow down task execution |
| NFR-2 | Orphan scan < 10ms for 10k tasks | Must not slow down recovery sweeper |
