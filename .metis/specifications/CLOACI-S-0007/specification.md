---
id: executionledger-and-ledgertrigger
level: specification
title: "ExecutionLedger and LedgerTrigger - Operational Backbone"
short_code: "CLOACI-S-0007"
created_at: 2026-03-10T18:18:25.653143+00:00
updated_at: 2026-03-10T18:18:25.653143+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# ExecutionLedger and LedgerTrigger - Operational Backbone

*Component specification for CLOACI-S-0001 (Continuous Reactive Scheduling).*

## Overview

The execution ledger is an in-memory append-only log maintained by the `ContinuousScheduler` that records what is happening in the graph. The `LedgerTrigger` is a specialized `Trigger` implementation that watches the ledger for task completions and fires detector workflows (CLOACI-S-0004) for derived data sources.

Together they form the operational backbone connecting task completions to downstream reactive chains — replacing the need for explicit "completion hints" or output declarations on tasks.

## ExecutionLedger

```rust
struct ExecutionLedger {
    events: Vec<LedgerEvent>,
}

enum LedgerEvent {
    TaskCompleted { task: String, at: DateTime, context: Context<Value> },
    TaskFailed { task: String, at: DateTime, error: String },
    BoundaryEmitted { source: String, boundary: ComputationBoundary },
    AccumulatorDrained { task: String, boundary: ComputationBoundary },
}
```

The scheduler writes to the ledger. `LedgerTrigger` instances read from it. The ledger is the single observation point for all graph activity — no component needs explicit wiring to observe completions.

## LedgerTrigger

A specialized implementation of the existing `Trigger` trait that watches the execution ledger for task completions and fires the associated detector workflow. This is how derived/internal data sources get their detector workflows kicked off.

```rust
struct LedgerTrigger {
    /// Task IDs to watch for completion
    watch_tasks: Vec<String>,
    /// Fire when any watched task completes, or wait for all
    match_mode: LedgerMatchMode,
    /// Reference to the execution ledger
    ledger: Arc<ExecutionLedger>,
    /// Last observed ledger position (so we don't re-trigger on old events)
    cursor: u64,
}

enum LedgerMatchMode {
    /// Fire when any watched task completes
    Any,
    /// Fire when all watched tasks have completed since last fire
    All,
}

impl Trigger for LedgerTrigger {
    fn name(&self) -> &str { &self.name }
    fn poll_interval(&self) -> Duration { /* sub-second — ledger is in-memory */ }
    fn allow_concurrent(&self) -> bool { false }

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        let new_events = self.ledger.events_since(self.cursor);
        // ... match against watch_tasks per match_mode ...
        // TriggerResult::Fire(Some(context)) or TriggerResult::Skip
    }
}
```

The `LedgerTrigger` fits cleanly into the existing trigger infrastructure — it implements the same `Trigger` trait, gets registered the same way, and the scheduler's existing trigger evaluation loop picks it up. The only new piece is that it reads from the `ExecutionLedger` instead of from time or external events.

### Match Modes

**`Any` mode** — fire when any watched task completes. Used when a single upstream task producing output is sufficient to warrant re-detection.

**`All` mode** — fire when all watched tasks have completed since the last fire. Handles multi-dependency detection: a derived data source that depends on multiple upstream tasks completing before it makes sense to check for changes. The trigger tracks which watched tasks have completed since the last fire and only triggers when all have been seen.

### Cursor Semantics

The `cursor` ensures idempotency: the ledger is append-only, so the trigger scans from its cursor forward for matching `TaskCompleted` events. After firing, the cursor advances past all consumed events.

## Example: Derived Data Source

A data source representing "hourly aggregation results" has a detector workflow triggered by a `LedgerTrigger` watching the `aggregate_hourly` task:

```rust
// The data source — detector_workflow points to a regular workflow
DataSource {
    name: "hourly_stats",
    connection: Box::new(PostgresConnection { ... }),
    detector_workflow: "detect_hourly_stats_changes",
    lineage: DataSourceMetadata { ... },
}

// The trigger — watches the ledger for the upstream task
LedgerTrigger {
    watch_tasks: vec!["aggregate_hourly".into()],
    match_mode: LedgerMatchMode::Any,
    ledger: scheduler.execution_ledger.clone(),
    cursor: 0,
}
// Registered via existing trigger system:
// trigger_scheduler.register("detect_hourly_stats_changes", ledger_trigger);
```

**Flow**: `aggregate_hourly` completes → `LedgerEvent::TaskCompleted` written → `LedgerTrigger` fires → `detect_hourly_stats_changes` workflow runs → produces `DetectorOutput` → boundaries flow to downstream accumulators. The task never knew about this data source.

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| Ledger is append-only, in-memory | Read-heavy, writes are cheap. Cursor-based scanning avoids re-processing. |
| LedgerTrigger implements existing Trigger trait | No new scheduling infrastructure. Fits into existing trigger evaluation loop. |
| Any/All match modes | Mirrors TriggerPolicy and JoinMode composition pattern. Consistent semantics across system. |
| Replaces "Internal" ChangeDetector concept | Detectors are workflows triggered by the existing system. LedgerTrigger is just another trigger type, not a special detection mode. |
| Cursor-based idempotency | Simple, efficient. No need for event deduplication or acknowledgment protocol. |
