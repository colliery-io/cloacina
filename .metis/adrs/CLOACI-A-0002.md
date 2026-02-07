---
id: 001-execution-history-and-task
level: adr
title: "Execution History and Task Distribution Architecture"
number: 1
short_code: "CLOACI-A-0002"
created_at: 2026-02-03T18:39:56.441450+00:00
updated_at: 2026-02-03T19:16:40.507759+00:00
decision_date:
decision_maker:
parent:
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
strategy_id: NULL
initiative_id: NULL
---

# ADR-1: Execution History and Task Distribution Architecture

*This template includes sections for various types of architectural decisions. Delete sections that don't apply to your specific use case.*

## Context

### Problem 1: Execution History Loss

Current architecture uses mutable rows for task/pipeline state. When a task transitions through states (e.g., `Failed → Ready → Running → Completed`), only the final state is preserved:

- `task_executions.status` shows current state only
- `task_executions.last_error` captures only the most recent error
- Intermediate states and their context are lost

**Impact:** Answering "why did this task fail the first time?" requires correlating sparse `recovery_events`, checking `last_error` (which may have been overwritten), and hoping ephemeral logs still exist.

### Problem 2: Debugging Difficulty

To understand task execution behavior, operators must:
1. Query `task_executions` for current state
2. Query `recovery_events` for recovery operations (sparse)
3. Search application logs (ephemeral, not correlated)
4. Infer timeline from timestamps

Missing: Complete event timeline showing exactly what happened and when.

### Problem 3: Task Distribution via Polling

Current task claiming uses `FOR UPDATE SKIP LOCKED` polling:

```rust
loop {
    let task = query("SELECT ... FOR UPDATE SKIP LOCKED").await?;
    if let Some(task) = task {
        process(task).await;
    } else {
        sleep(Duration::from_millis(500)).await;  // Polling interval
    }
}
```

**Issues:**
- 500ms latency floor (or increased DB load from faster polling)
- Constant queries even when no work available
- Scales poorly with many workers

### Requirements

1. **Complete execution history** - Every state transition captured with context
2. **Queryable by entity** - "Show me everything for task X" or "pipeline Y"
3. **Reduced distribution latency** - Near-instant task pickup when work available
4. **No new infrastructure** - Prefer Postgres-only solutions
5. **Multi-tenant compatible** - Must work with schema-based isolation

## Decision

**Status: Recommended**

Two complementary changes:

1. **Execution Events Table** - Append-only event log in Postgres/SQLite for complete history
2. **Outbox Pattern** - Separate table for work distribution with optional LISTEN/NOTIFY boost for Postgres

### Architecture

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

- **Outbox is source of truth for "what needs claiming"** - same table, same logic, both backends
- **Postgres gets optional NOTIFY boost** - near-instant wakeup, falls back to polling
- **SQLite uses polling on outbox** - identical claiming semantics, just poll-based wakeup
- **execution_events captures full history** - append-only, queryable by task/pipeline

## Alternatives Analysis

### Part A: Execution History

#### Option A1: External Log/Stream (IGGY, Kafka)

| Aspect | Assessment |
|--------|------------|
| Pros | Native replay, distributed replication, consumer groups |
| Cons | New infrastructure, awkward joins to task_id, tenant isolation complexity |
| Risk | Medium - operational complexity |
| Cost | High - new system to operate |

**Rejected:** Access pattern is fundamentally relational (query by task_id, pipeline_id). External log adds infrastructure without solving the core query need.

#### Option A2: Postgres `execution_events` Table

| Aspect | Assessment |
|--------|------------|
| Pros | Natural FK joins, same tenant isolation, full SQL queries, no new infra |
| Cons | Postgres scaling limits (acceptable for this use case) |
| Risk | Low - familiar patterns |
| Cost | Low - new table + event emission |

**Proposed schema:**

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
```

**Event types:**
- `task.created`, `task.marked_ready`, `task.claimed`, `task.started`
- `task.deferred`, `task.resumed`, `task.completed`, `task.failed`
- `task.retry_scheduled`, `task.skipped`, `task.abandoned`
- `pipeline.started`, `pipeline.completed`, `pipeline.failed`

---

### Part B: Task Distribution

#### Option B1: Keep Polling (Status Quo)

| Aspect | Assessment |
|--------|------------|
| Pros | Simple, battle-tested, no changes needed |
| Cons | Latency floor, DB load, scales poorly |
| Risk | Low |
| Cost | None |

#### Option B2: LISTEN/NOTIFY + Polling Fallback

```sql
CREATE FUNCTION notify_task_ready() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status = 'Ready' THEN
        PERFORM pg_notify('task_ready', NEW.id::text);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
```

| Aspect | Assessment |
|--------|------------|
| Pros | Near-instant wakeup, no polling when idle, built into Postgres |
| Cons | Notifications are ephemeral (lost on disconnect), still need claiming logic |
| Risk | Low - fallback polling handles edge cases |
| Cost | Low - trigger + listener code |

#### Option B3: Outbox Pattern

Separate table for work distribution:

```sql
CREATE TABLE task_outbox (
    id BIGSERIAL PRIMARY KEY,
    task_execution_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    claimed_at TIMESTAMPTZ,
    claimed_by VARCHAR(100)
);
```

| Aspect | Assessment |
|--------|------------|
| Pros | Clean separation (state vs distribution), easy cleanup, natural queue metrics |
| Cons | Dual state to maintain, extra table |
| Risk | Low |
| Cost | Low-Medium |

#### Option B4: PGMQ Extension

| Aspect | Assessment |
|--------|------------|
| Pros | Proper queue semantics, visibility timeout, DLQ, blocking reads |
| Cons | Extension dependency, dual state (task_executions + queue) |
| Risk | Medium - extension availability across deployments |
| Cost | Medium |

#### Option B5: Advisory Locks

| Aspect | Assessment |
|--------|------------|
| Pros | Fine-grained locking, no table changes |
| Cons | Lock management complexity, still need notification mechanism |
| Risk | Medium |
| Cost | Low |

---

### Combined Approach Options

| Combination | History | Distribution | Complexity | Status |
|-------------|---------|--------------|------------|--------|
| A2 + B1 | Events table | Keep polling | Lowest | ❌ Latency concerns |
| A2 + B2 | Events table | LISTEN/NOTIFY only | Low | ❌ SQLite divergence |
| **A2 + B3 + B2** | **Events table** | **Outbox + optional NOTIFY** | **Medium** | **✅ Recommended** |
| A2 + B4 | Events table | PGMQ | Medium-High | ❌ Extension dependency |

**Recommended: A2 + B3 + B2**
- `execution_events` table for history
- `task_outbox` table for distribution (both backends)
- Optional LISTEN/NOTIFY on outbox for Postgres latency boost

## Rationale

### Why Outbox over pure LISTEN/NOTIFY?

**Backend parity matters.** Maintaining two fundamentally different distribution mechanisms creates:

| Cost | Impact |
|------|--------|
| Two code paths | Trait keeps interface clean, but internals diverge |
| Testing burden | Must test both paths and their edge cases |
| Behavioral divergence | "Works on my machine" - dev uses SQLite, prod uses Postgres |
| Documentation | Must explain latency differences |

**Outbox provides uniform semantics:**
- Same table schema in both backends
- Same claiming logic (`FOR UPDATE SKIP LOCKED` / `BEGIN IMMEDIATE`)
- Same queue depth queries (`SELECT COUNT(*) WHERE claimed_at IS NULL`)
- Only difference: how workers wake up (NOTIFY vs poll)

### Why not pure polling on task_executions?

The outbox cleanly separates concerns:
- `task_executions` = authoritative task state
- `task_outbox` = work distribution queue (transient)
- `execution_events` = history (append-only)

Direct polling on `task_executions.status = 'Ready'` mixes state queries with distribution, making it harder to reason about each independently.

### Why execution_events in Postgres, not external log?

Access pattern is fundamentally relational:
- "Show me all events for task X" → `WHERE task_execution_id = ?`
- "Failed tasks in last hour" → `WHERE event_type = 'task.failed' AND created_at > ?`

External log (IGGY, Kafka) would require:
- Awkward joins back to task_id
- New infrastructure to operate
- Tenant isolation complexity

Postgres gives us: FK relationships, SQL queries, same tenant schema isolation, no new infra.

## Open Questions

*Resolved during discussion:*

1. ~~**Event granularity?**~~ → Outbox contains only claimable work (transient). Events table captures all state transitions (history). Separate concerns.

2. ~~**Retention policy?**~~ → Configurable. Default TBD.

3. ~~**Outbox cleanup strategy?**~~ → Delete after claim. Retention configurable. Cleanup via CLI task (can be scheduled via Cloacina or run manually).

## Consequences

### Positive

- **Complete debugging** - "show me everything for task X" becomes a simple query
- **Audit trail** - compliance-ready execution history
- **Backend parity** - identical claiming semantics for Postgres and SQLite
- **Clean separation** - state, distribution, and history are distinct concerns
- **Queue observability** - `SELECT COUNT(*) FROM task_outbox WHERE claimed_at IS NULL`
- **No new infrastructure** - all Postgres/SQLite, no external dependencies
- **Postgres latency boost** - optional NOTIFY gives near-instant wakeup without diverging core logic

### Negative

- **Storage growth** - execution_events grows with activity (mitigated by retention policy)
- **Additional writes** - +2 inserts per task ready (outbox + event)
- **Sync complexity** - outbox must stay in sync with task_executions status
- **Event consistency** - must emit events at all state transition points (easy to miss edge cases)

### Neutral

- **Migration effort** - non-trivial but can be phased
- **Polling remains for SQLite** - acceptable given parity goal

## Implementation

Since we own the DAL and the interface stays consistent, no phased migration needed. Consumers call the same methods - internals change.

### Schema Changes

```sql
-- execution_events: append-only history
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

-- task_outbox: transient work queue
CREATE TABLE task_outbox (
    id BIGSERIAL PRIMARY KEY,
    task_execution_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_outbox_unclaimed ON task_outbox(created_at);
```

### DAL Changes

```rust
// mark_ready() - now also populates outbox + emits event
impl TaskExecutionDAL {
    async fn mark_ready(&self, task_id: Uuid) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Update task status (existing)
        sqlx::query("UPDATE task_executions SET status = 'Ready' WHERE id = $1")
            .bind(task_id).execute(&mut *tx).await?;

        // Insert to outbox (new)
        sqlx::query("INSERT INTO task_outbox (task_execution_id) VALUES ($1)")
            .bind(task_id).execute(&mut *tx).await?;

        // Emit event (new)
        sqlx::query("INSERT INTO execution_events (...) VALUES (...)")
            .execute(&mut *tx).await?;

        tx.commit().await
    }
}

// claim() - now reads from outbox
impl TaskExecutionDAL {
    async fn claim(&self, worker_id: &str) -> Result<Option<Task>> {
        // Claim from outbox, delete row, return task
        sqlx::query_as("
            WITH claimed AS (
                DELETE FROM task_outbox
                WHERE id = (
                    SELECT id FROM task_outbox
                    ORDER BY created_at
                    FOR UPDATE SKIP LOCKED
                    LIMIT 1
                )
                RETURNING task_execution_id
            )
            UPDATE task_executions
            SET status = 'Running', started_at = NOW(), worker_id = $1
            FROM claimed
            WHERE task_executions.id = claimed.task_execution_id
            RETURNING task_executions.*
        ")
        .bind(worker_id)
        .fetch_optional(&self.pool)
        .await
    }
}
```

### Worker Changes

```rust
// Postgres: LISTEN/NOTIFY + poll fallback
// SQLite: Poll outbox

impl WorkDistributor for PostgresDistributor {
    async fn wait_for_work(&self) {
        select! {
            _ = self.listener.recv() => {},      // NOTIFY wakeup
            _ = sleep(Duration::from_secs(30)) => {}, // Fallback poll
        }
    }
}

impl WorkDistributor for SqliteDistributor {
    async fn wait_for_work(&self) {
        sleep(Duration::from_millis(500)).await;  // Poll interval
    }
}
```

### Cleanup CLI

```bash
# Manual cleanup
cloacina admin cleanup-events --older-than 90d

# Or schedule via Cloacina itself
# (workflow that runs cleanup task on cron)
```

### Configuration

```toml
[execution_events]
retention_days = 90  # Default, configurable

[task_outbox]
# Rows deleted on claim, no retention needed
```

## Related Decisions

- May influence future distributed execution architecture
- Affects observability/monitoring strategy
- Retention policy needs alignment with compliance requirements
