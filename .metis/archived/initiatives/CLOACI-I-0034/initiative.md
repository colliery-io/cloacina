---
id: server-phase-6-pipeline-claiming
level: initiative
title: "Server Phase 6: Pipeline Claiming — Scheduler Horizontal Scaling"
short_code: "CLOACI-I-0034"
created_at: 2026-03-16T01:32:38.179956+00:00
updated_at: 2026-03-17T01:51:04.219486+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: server-phase-6-pipeline-claiming
---

# Server Phase 6: Pipeline Claiming — Scheduler Horizontal Scaling Initiative

**Parent tracker**: [[CLOACI-I-0018]]
**Depends on**: CLOACI-I-0029 (Foundation — scheduler loop must be running)
**Blocks**: None (can parallel with API phases)

## Context

Currently only task-level claiming exists (`FOR UPDATE SKIP LOCKED` in `task_execution/claiming.rs`). For horizontal scheduler scaling, multiple scheduler instances need to claim **batches of pipelines** so they process non-overlapping work. Without this, running 2+ scheduler instances causes duplicate scheduling.

## Goals

- Add `last_scheduled_at`, `last_scheduled_by` columns to `pipeline_executions`
- Implement `claim_pipeline_batch(scheduler_id, limit)` DAL method
- Modify scheduler loop to claim pipelines in batches instead of scanning all
- Multiple schedulers claim non-overlapping batches (true throughput scaling, not just HA)
- Existing task claiming continues to work within the scope of claimed pipelines

## Detailed Design

### Approach: Pipeline Outbox (matching task claiming pattern)

The existing task claiming uses a `task_outbox` table with `FOR UPDATE SKIP LOCKED` — pipelines should use the same proven pattern rather than adding columns to `pipeline_executions`.

**Current flow** (single scheduler):
```
scheduler_loop.tick()
  → get_active_executions()  // scans ALL Pending|Running pipelines
  → process_pipelines_batch(all)
```

**New flow** (multi-scheduler):
```
scheduler_loop.tick()
  → claim_pipeline_batch(scheduler_id, limit=100)  // FOR UPDATE SKIP LOCKED on pipeline_outbox
  → process_pipelines_batch(claimed_only)
```

### New Table: `pipeline_outbox`

```sql
CREATE TABLE pipeline_outbox (
    id BIGSERIAL PRIMARY KEY,
    pipeline_execution_id UUID NOT NULL REFERENCES pipeline_executions(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE INDEX idx_pipeline_outbox_created ON pipeline_outbox(created_at);
```

Entries are inserted when a pipeline is created (`schedule_workflow_execution()`), and deleted when claimed by a scheduler. The scheduler processes the claimed pipelines, and completed pipelines are not re-inserted.

For pipelines that need continued processing (Running state, multiple poll cycles), the scheduler re-inserts them into the outbox after each processing cycle if they're not yet complete.

### Claiming SQL (Postgres)

```sql
WITH claimed AS (
    DELETE FROM pipeline_outbox
    WHERE id IN (
        SELECT id FROM pipeline_outbox
        ORDER BY created_at ASC
        LIMIT $1
        FOR UPDATE SKIP LOCKED
    )
    RETURNING pipeline_execution_id
)
SELECT pe.* FROM pipeline_executions pe
INNER JOIN claimed c ON pe.id = c.pipeline_execution_id
WHERE pe.status IN ('Pending', 'Running');
```

### Backward Compatibility

When only one scheduler is running, this is equivalent to the current all-scan — the outbox drains to the single scheduler. No configuration change needed for single-instance deployments.

## Implementation Plan

- [ ] Migration: `pipeline_outbox` table (Postgres + SQLite)
- [ ] Schema + model: diesel table declaration, PipelineOutboxRow, NewPipelineOutbox
- [ ] DAL: `claim_pipeline_batch()`, `insert_pipeline_outbox()`, `requeue_pipeline()`
- [ ] Modify `schedule_workflow_execution()` to insert outbox entry on pipeline creation
- [ ] Modify `scheduler_loop` to use `claim_pipeline_batch()` instead of `get_active_executions()`
- [ ] Re-insert Running pipelines into outbox after processing (if not completed)
- [ ] Integration test: two claim calls return non-overlapping batches
