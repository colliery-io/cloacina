---
id: unified-scheduling-infrastructure
level: initiative
title: "Unified scheduling infrastructure — collapse cron/trigger into single scheduler and schema"
short_code: "CLOACI-I-0059"
created_at: 2026-03-29T20:21:35.665158+00:00
updated_at: 2026-03-29T20:36:53.163699+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/ready"


exit_criteria_met: false
estimated_complexity: L
initiative_id: unified-scheduling-infrastructure
---

# Unified scheduling infrastructure — collapse cron/trigger into single scheduler and schema Initiative

## Context

Cloacina has separate infrastructure for cron scheduling and trigger scheduling that is ~80% duplicated:

- **Two schedulers**: `CronScheduler` and `TriggerScheduler` — same loop/claim/execute/audit pattern, different implementations
- **Four database tables**: `cron_schedules` + `cron_executions` and `trigger_schedules` + `trigger_executions` — parallel schemas
- **Four DAL modules**: `cron_schedule`, `cron_execution`, `trigger_schedule`, `trigger_execution` — duplicated CRUD
- **Two config structs**: `CronSchedulerConfig` and `TriggerSchedulerConfig`

This duplication exists because cron and triggers were built as separate features. With I-0058 (unified macro system) treating cron as just a trigger with a built-in poll function, the infrastructure should follow suit.

**Relationship to I-0058**: I-0058 unifies the *authoring* side (`#[workflow]` + `#[trigger]`). This initiative unifies the *runtime* side (scheduler, storage, DAL). Do I-0058 first, then this.

## Goals & Non-Goals

**Goals:**
- Single `Scheduler` that handles both cron and custom triggers via a strategy pattern
- Collapse 4 tables into 2: `schedules` (with `schedule_type` discriminator) + `schedule_executions`
- Single DAL module for schedule operations
- Single config struct with schedule-type-specific fields
- Unified registration entry point — one path into the scheduler regardless of trigger type
- ~2000-3000 lines of duplicate code eliminated

**Non-Goals:**
- Changing the `Trigger` trait (stays as-is for custom poll logic)
- Changing the execution engine (already unified — both types use `PipelineExecutor`)
- Unifying embedded vs packaged workflow storage (different concern — embedded is memory-only by design)

## Detailed Design

### Unified schema

```sql
CREATE TABLE schedules (
    id UUID PRIMARY KEY,
    schedule_type TEXT NOT NULL CHECK (schedule_type IN ('cron', 'trigger')),
    workflow_name TEXT NOT NULL,
    enabled BOOLEAN DEFAULT true,

    -- Cron-specific
    cron_expression TEXT,
    timezone TEXT DEFAULT 'UTC',
    catchup_policy TEXT,
    start_date TIMESTAMP,
    end_date TIMESTAMP,

    -- Trigger-specific
    trigger_name TEXT,
    poll_interval_ms INTEGER,
    allow_concurrent BOOLEAN DEFAULT false,

    -- Shared timing
    next_run_at TIMESTAMP,
    last_run_at TIMESTAMP,
    last_poll_at TIMESTAMP,

    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);

CREATE TABLE schedule_executions (
    id UUID PRIMARY KEY,
    schedule_id UUID NOT NULL REFERENCES schedules(id),
    pipeline_execution_id UUID REFERENCES pipeline_executions(id),

    -- Cron-specific
    scheduled_time TIMESTAMP,
    claimed_at TIMESTAMP,

    -- Trigger-specific
    context_hash TEXT,

    started_at TIMESTAMP NOT NULL,
    completed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);
```

### Unified scheduler

```rust
pub struct Scheduler {
    dal: Arc<DAL>,
    executor: Arc<dyn PipelineExecutor>,
    config: SchedulerConfig,
    shutdown: watch::Receiver<bool>,
}

impl Scheduler {
    async fn run_loop(&mut self) {
        // Single loop handles both cron and trigger schedules
        // Cron: check next_run_at against now
        // Trigger: call registered poll function at poll_interval
        // Both: hand off to PipelineExecutor, record in schedule_executions
    }
}
```

### Migration path

1. Create new unified tables
2. Migrate data from old tables
3. Rewrite DAL to target new tables
4. Replace CronScheduler + TriggerScheduler with unified Scheduler
5. Drop old tables in a later migration

## Alternatives Considered

1. **Keep separate tables, unify only the scheduler code** — Reduces code duplication but leaves schema mess. Harder to query "all schedules for a workflow" across types.
2. **Abstract base class with inheritance** — Rust doesn't have inheritance. Trait + strategy pattern achieves the same goal idiomatically.
3. **Fully collapse cron into trigger at the data model level** — Considered making cron just a trigger row with `cron_expression` filled in. Decided to keep `schedule_type` discriminator for clarity and query efficiency.

## Implementation Plan

Phase 1: New unified schema + migration (create tables, migrate data)
Phase 2: Unified schedule DAL (replaces 4 DAL modules)
Phase 3: Unified Scheduler (replaces CronScheduler + TriggerScheduler)
Phase 4: Drop old tables + old DAL modules
Phase 5: Update runner config and startup to use single Scheduler
