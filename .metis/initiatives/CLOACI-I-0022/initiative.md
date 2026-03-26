---
id: execution-events-and-outbox-based
level: initiative
title: "Execution Events and Outbox-Based Task Distribution"
short_code: "CLOACI-I-0022"
created_at: 2026-02-03T19:20:58.505443+00:00
updated_at: 2026-02-07T01:18:09.313755+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
strategy_id: NULL
initiative_id: execution-events-and-outbox-based
---

# Execution Events and Outbox-Based Task Distribution Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context

Implements **CLOACI-A-0002: Execution History and Task Distribution Architecture**.

### Problem 1: Execution History Loss
Current architecture uses mutable rows for task/pipeline state. When a task transitions through states, only the final state is preserved. Answering "why did this task fail?" requires correlating sparse data across multiple sources.

### Problem 2: Task Distribution via Polling
Current task claiming uses `FOR UPDATE SKIP LOCKED` polling with 500ms latency floor and constant DB load even when no work is available.

### Solution
Two new tables with distinct purposes:
- `execution_events` - Append-only history of all state transitions (audit/debugging)
- `task_outbox` - Transient work queue for task distribution (replaces status polling)

## Goals & Non-Goals

**Goals:**
- Complete execution history queryable by task/pipeline ID
- Uniform task distribution semantics for Postgres and SQLite
- Optional LISTEN/NOTIFY boost for Postgres without diverging core logic
- Configurable retention with CLI cleanup tooling

**Non-Goals:**
- External log/stream infrastructure (IGGY, Kafka)
- Real-time event streaming to external consumers
- Changes to user-facing API

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Scheduler                                │
│                                                                  │
│  1. UPDATE task_executions SET status = 'Ready'                 │
│  2. INSERT INTO task_outbox (task_execution_id)                 │
│  3. INSERT INTO execution_events (type = 'task.marked_ready')   │
│  4. [Postgres only] NOTIFY task_ready triggered by outbox       │
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
┌──────────────────┐ ┌──────────────┐ ┌──────────────────┐
│ task_executions  │ │ task_outbox  │ │ execution_events │
│ (state)          │ │ (distribute) │ │ (history)        │
└──────────────────┘ └──────┬───────┘ └──────────────────┘
                           │
              ┌────────────┴────────────┐
              ▼                         ▼
     ┌─────────────────┐       ┌─────────────────┐
     │ Worker (PG)     │       │ Worker (SQLite) │
     │ LISTEN + poll   │       │ poll outbox     │
     │ claim from      │       │ claim from      │
     │ outbox          │       │ outbox          │
     └─────────────────┘       └─────────────────┘
```

### Key Design Points
- `task_executions` = authoritative task state (existing)
- `task_outbox` = work distribution queue (transient, deleted on claim)
- `execution_events` = history/audit (append-only, configurable retention)
- Postgres gets optional NOTIFY boost, SQLite polls - same claiming logic

## Detailed Design

### Schema: execution_events

```sql
CREATE TABLE execution_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_execution_id UUID NOT NULL REFERENCES pipeline_executions(id),
    task_execution_id UUID REFERENCES task_executions(id),
    event_type VARCHAR(50) NOT NULL,
    event_data JSONB NOT NULL DEFAULT '{}',
    worker_id VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sequence_num BIGINT GENERATED ALWAYS AS IDENTITY
);

CREATE INDEX idx_events_pipeline ON execution_events(pipeline_execution_id, sequence_num);
CREATE INDEX idx_events_task ON execution_events(task_execution_id, sequence_num);
CREATE INDEX idx_events_created ON execution_events(created_at DESC);
```

**Event types:**
- `task.created`, `task.marked_ready`, `task.claimed`, `task.started`
- `task.deferred`, `task.resumed`, `task.completed`, `task.failed`
- `task.retry_scheduled`, `task.skipped`, `task.abandoned`
- `pipeline.started`, `pipeline.completed`, `pipeline.failed`

### Schema: task_outbox

```sql
CREATE TABLE task_outbox (
    id BIGSERIAL PRIMARY KEY,
    task_execution_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_outbox_unclaimed ON task_outbox(created_at);
```

### DAL Changes

`mark_ready()` - now also populates outbox + emits event (single transaction)

`claim()` - reads from outbox, deletes row, updates task_executions in one CTE

### Worker Changes

- `WorkDistributor` trait with `wait_for_work()` method
- Postgres: LISTEN/NOTIFY on outbox + 30s poll fallback
- SQLite: 500ms poll on outbox

### CLI

```bash
cloacina admin cleanup-events --older-than 90d
```

### Configuration

```toml
[execution_events]
retention_days = 90
```

## Alternatives Considered

See **CLOACI-A-0002** for full analysis. Summary:

| Option | Rejected Because |
|--------|------------------|
| External log (IGGY, Kafka) | Access pattern is relational, adds infrastructure |
| LISTEN/NOTIFY only | SQLite divergence, parity matters |
| PGMQ extension | Extension dependency |
| Keep polling | Latency floor, DB load |

## Implementation Plan

Internal refactor - no phased migration needed since we own both sides of the DAL.

### Tasks

1. **Schema migrations** - Add `execution_events` and `task_outbox` tables (both backends)
2. **Event emission** - Add event inserts at DAL state transition points
3. **Outbox population** - Modify `mark_ready()` to insert outbox row
4. **Outbox claiming** - Modify `claim()` to read from outbox
5. **WorkDistributor trait** - Abstract wait-for-work with PG/SQLite implementations
6. **LISTEN/NOTIFY trigger** - Postgres trigger on outbox inserts
7. **CLI cleanup command** - `cloacina admin cleanup-events`
8. **Configuration** - Retention settings
9. **Integration tests** - Verify events match state, claiming works correctly
