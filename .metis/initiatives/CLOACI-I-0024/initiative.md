---
id: watermarks-late-arrival-and
level: initiative
title: "Watermarks, Late Arrival, and Derived Data Sources"
short_code: "CLOACI-I-0024"
created_at: 2026-03-13T02:44:39.399396+00:00
updated_at: 2026-03-15T13:16:53.996919+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: watermarks-late-arrival-and
---

# Watermarks, Late Arrival, and Derived Data Sources

## Context

CLOACI-I-0023 delivers the minimum viable reactive graph — data sources, detectors, accumulators, and continuous tasks working end-to-end with cron-triggered detection and simple accumulation. But without watermarks, the system cannot reason about data completeness, and without `LedgerTrigger`, derived data sources (where one task's output feeds another task's input) require manual cron wiring.

This initiative adds the sophisticated scheduling layer that completes the reactive feedback loop described in CLOACI-S-0001.

**Depends on**: CLOACI-I-0023 (core continuous scheduling)
**Specification**: CLOACI-S-0005 (partial), S-0006, S-0007 (partial)

## Goals & Non-Goals

**Goals:**
- Implement `BoundaryLedger` for source watermark tracking (S-0006)
- Implement `WindowedAccumulator` with watermark awareness (S-0005)
- Implement consumer watermark tracking on accumulators (S-0006)
- Implement `LateArrivalPolicy` — Discard, AccumulateForward, Retrigger, RouteToSideChannel (S-0006)
- Implement `LedgerTrigger` as a `Trigger` impl that watches the `ExecutionLedger` (S-0007)
- Implement `LedgerMatchMode` Any/All for multi-dependency detection (S-0007)
- Extend `ContinuousScheduler` run loop with watermark checks and late arrival routing
- Support `JoinMode::All` for multi-input tasks (S-0008)
- Registration-time validation warning when `WindowedAccumulator` is used on a source with no watermark strategy

**Non-Goals:**
- Accumulator persistence (CLOACI-I-0025)
- Custom boundary schema validation (CLOACI-I-0025)
- TriggerPolicy composition (Any/All) (CLOACI-I-0025)
- Additional TriggerPolicy presets (CLOACI-I-0025)
- Framework-provided DataConnection impls beyond Postgres (CLOACI-I-0025)
- Python/Cloaca support (CLOACI-I-0026)

## Detailed Design

### BoundaryLedger (S-0006)

In-memory source watermark store:

```rust
struct BoundaryLedger {
    watermarks: HashMap<String, ComputationBoundary>,
}
```

- `advance(source, watermark)` — monotonic, rejects backward movement
- `covers(source, boundary) -> bool` — does the watermark cover this boundary?
- `watermark(source) -> Option<&ComputationBoundary>`

Read-heavy (every `WindowedAccumulator.is_ready()` check reads it), writes are infrequent (only on `DetectorOutput::WatermarkAdvance`).

### WindowedAccumulator (S-0005)

Extends the accumulator model with watermark awareness:

```rust
fn is_ready(&self) -> bool {
    if !self.trigger_policy.should_fire(&self.buffer) { return false; }
    match self.watermark_mode {
        WatermarkMode::WaitForWatermark => {
            self.boundary_ledger.covers(&self.source_name, &self.pending_boundary())
        },
        WatermarkMode::BestEffort => true,
    }
}
```

Standard use case: time-windowed aggregation that waits for the source to confirm data completeness before firing.

### Late Arrival Detection & Policy (S-0006)

Before routing a boundary to an accumulator, the scheduler checks the consumer watermark:

```rust
if let Some(consumer_wm) = accumulator.consumer_watermark() {
    if consumer_wm.covers(&boundary) {
        // Already processed — apply late arrival policy
        match edge.late_arrival_policy {
            Discard => { /* drop silently */ },
            AccumulateForward => { accumulator.receive(boundary); },
            Retrigger => { /* re-submit affected boundary */ },
            RouteToSideChannel { task_name } => { /* route to correction task */ },
        }
        return;
    }
}
accumulator.receive(boundary);
```

Each `GraphEdge` configures its own policy. Default is `AccumulateForward` (as established in I-0023).

### LedgerTrigger (S-0007)

Implements the existing `Trigger` trait — watches the `ExecutionLedger` for task completions and fires detector workflows for derived data sources:

```rust
impl Trigger for LedgerTrigger {
    fn name(&self) -> &str { &self.name }
    fn poll_interval(&self) -> Duration { /* sub-second — ledger is in-memory */ }
    fn allow_concurrent(&self) -> bool { false }

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        let new_events = self.ledger.events_since(self.cursor);
        // match against watch_tasks per match_mode
        // Any: fire when any watched task completes
        // All: fire when all watched tasks have completed since last fire
    }
}
```

This completes the reactive feedback loop: task completes → ledger event → LedgerTrigger fires → detector workflow runs → DetectorOutput → boundaries flow downstream.

### JoinMode::All

Multi-input tasks that require all accumulators to be ready before firing:

```rust
// Task readiness check in scheduler
match task_config.join_mode {
    JoinMode::Any => task_config.triggered_edges.iter().any(|e| edges[*e].accumulator.is_ready()),
    JoinMode::All => task_config.triggered_edges.iter().all(|e| edges[*e].accumulator.is_ready()),
}
```

### ContinuousScheduler Run Loop Changes

The run loop from I-0023 is extended with:
- Step 3a: `WatermarkAdvance` → update `BoundaryLedger` (new)
- Step 3b: Late arrival check against consumer watermark before `receive()` (new)
- Step 4: `JoinMode::All` support in readiness check (new)
- `LedgerTrigger` instances registered with existing `TriggerScheduler` (new)

## Alternatives Considered

- **Source watermarks in the database**: Rejected — BoundaryLedger is read-heavy (every accumulator check). Same rationale as in-memory ExecutionLedger. Persistence comes in I-0025 via accumulator persist-on-drain.
- **Late arrival as an accumulator concern**: Rejected — from the accumulator's perspective, a boundary is just data to buffer. The policy decision (discard/retrigger/side-channel) is a routing concern that belongs on the scheduler.
- **Separate derived-source trigger system**: Rejected — `LedgerTrigger` implements the existing `Trigger` trait. No new scheduling infrastructure needed.

## Implementation Plan

### Phase 1: BoundaryLedger & Watermarks
- [ ] `BoundaryLedger` struct with `advance()`, `covers()`, `watermark()`
- [ ] Monotonic watermark enforcement
- [ ] Integration with `ContinuousScheduler` — route `WatermarkAdvance` to ledger
- [ ] Unit tests for watermark advancement and coverage checks

### Phase 2: WindowedAccumulator
- [ ] `WindowedAccumulator` implementation with `WatermarkMode`
- [ ] Integration with `BoundaryLedger` for `WaitForWatermark` readiness
- [ ] Registration-time validation: warn if source has no watermark strategy
- [ ] Unit tests for windowed accumulation with and without watermarks

### Phase 3: Late Arrival
- [ ] `LateArrivalPolicy` enum (Discard, AccumulateForward, Retrigger, RouteToSideChannel)
- [ ] Consumer watermark check in scheduler before `receive()`
- [ ] Policy routing logic for each variant
- [ ] Per-edge policy configuration on `GraphEdge`
- [ ] Unit and integration tests for each policy variant

### Phase 4: LedgerTrigger & Derived Data Sources
- [ ] `LedgerTrigger` struct implementing `Trigger` trait
- [ ] `LedgerMatchMode` Any/All
- [ ] Cursor-based idempotent scanning
- [ ] Registration with existing `TriggerScheduler`
- [ ] Integration test: task completion → LedgerTrigger → detector → downstream accumulator

### Phase 5: JoinMode::All & Integration
- [ ] `JoinMode::All` readiness logic in scheduler
- [ ] End-to-end integration test: multi-source task with All join mode
- [ ] End-to-end test: full reactive feedback loop (source → detect → accumulate → task → derived source → detect → downstream task)
- [ ] Example: derived data source with LedgerTrigger
