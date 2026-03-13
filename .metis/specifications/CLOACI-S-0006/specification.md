---
id: watermark-system-data-completeness
level: specification
title: "Watermark System - Data Completeness and Late Arrival"
short_code: "CLOACI-S-0006"
created_at: 2026-03-10T18:18:24.320547+00:00
updated_at: 2026-03-10T18:18:24.320547+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Watermark System - Data Completeness and Late Arrival

*Component specification for CLOACI-S-0001 (Continuous Reactive Scheduling).*

## Overview

Two distinct watermarks serve different purposes in the system:

- **Source watermark** — per data source: "the producer says nothing earlier than this will arrive." Stored on the `BoundaryLedger`.
- **Consumer watermark** — per accumulator (per edge): "I've processed up to here." Stored on the `SignalAccumulator` (CLOACI-S-0005).

## Source Watermark: BoundaryLedger

The boundary ledger tracks data completeness per data source. It stores assertions from detectors about what data has been fully produced.

```rust
struct BoundaryLedger {
    watermarks: HashMap<String, ComputationBoundary>,  // data source name → source watermark
}

impl BoundaryLedger {
    /// Advance the watermark for a data source (rejects backward movement)
    fn advance(&mut self, source: &str, watermark: ComputationBoundary);

    /// Does the watermark for this source cover the given boundary?
    fn covers(&self, source: &str, boundary: &ComputationBoundary) -> bool;

    /// Get the current watermark for a data source
    fn watermark(&self, source: &str) -> Option<&ComputationBoundary>;
}
```

Three methods. In-memory, read-heavy (every accumulator `is_ready()` check reads it), writes are infrequent (only on `DetectorOutput::WatermarkAdvance`).

## Core Principle: Source Watermarks Are User Assertions

The framework cannot determine that a remote data source has finished producing data for a given boundary. Only the detector — the user's code interfacing with the external system — can make that claim. The boundary ledger accepts, stores, and uses these assertions but never generates them.

Detectors emit watermark advances alongside change boundaries via `DetectorOutput` (CLOACI-S-0004).

How the source watermark is determined depends entirely on the source:

| Source type | Watermark strategy | Who asserts it |
|---|---|---|
| Kafka | Consumer lag = 0 for partition | User's detector workflow checks consumer position |
| Time-partitioned table | Wall clock minus known ingestion delay | User asserts "complete up to `now - max_delay`" |
| S3 landing zone | Presence of a `_SUCCESS` marker file | User's callback checks for the marker |
| CDC stream | Replication slot LSN position | User reads the slot position |
| API poll | "I polled and got everything available" | Implicit — the poll itself is the watermark |
| Manual/webhook push | Unknown until explicitly advanced | User must explicitly emit `WatermarkAdvance` |
| Derived (LedgerTrigger) | Task completion boundary from execution ledger | LedgerTrigger fires detector workflow, which emits watermark based on observed completion |

The common pattern: the watermark is a heuristic with a known lag. "I'm confident data is complete up to `now - max_expected_delay`."

## Consumer Watermark: On the Accumulator

Each accumulator tracks what it has processed via `consumer_watermark()` on the `SignalAccumulator` trait (CLOACI-S-0005).

The consumer watermark advances on each `drain()`. Two accumulators on the same data source may have different consumer watermarks — one might be processing hourly, another daily. They're at different positions.

## How Both Watermarks Are Used

The `WindowedAccumulator` checks the **source watermark** to determine data completeness:

```rust
fn is_ready(&self) -> bool {
    if !self.trigger_policy.should_fire(&self.buffer) { return false; }

    match self.watermark_mode {
        // Safe default: wait for the source to confirm data is complete
        WatermarkMode::WaitForWatermark => {
            self.boundary_ledger.covers(&self.source_name, &self.pending_boundary())
        },
        // Opt-in: fire when trigger policy says so, accept possible incompleteness
        WatermarkMode::BestEffort => true,
    }
}
```

The scheduler checks the **consumer watermark** for late arrival detection:

```rust
// Before routing a boundary to an accumulator:
if let Some(consumer_wm) = accumulator.consumer_watermark() {
    if consumer_wm.covers(&boundary) {
        // This accumulator already processed this range
        apply_late_arrival_policy(...)
    }
}
```

## Late-Arriving Data

When a boundary arrives that falls behind a consumer's watermark, it is "late" for that specific consumer — the data range it covers has already been processed by that accumulator. Late arrival handling is a **scheduler-level concern**, configured per edge:

```rust
enum LateArrivalPolicy {
    /// Drop it silently
    Discard,
    /// Forward to the accumulator normally — becomes part of the next cycle
    AccumulateForward,
    /// Re-submit the affected boundary for re-execution
    Retrigger,
    /// Route to a designated correction task
    RouteToSideChannel { task_name: String },
}
```

## Registration-Time Validation

If a `WindowedAccumulator` is configured on a data source whose detector workflow never emits `WatermarkAdvance`, the framework should surface this clearly — either at registration time ("this data source has no watermark strategy, windowed accumulation may never fire") or at runtime when the accumulator is stuck waiting indefinitely.

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| Two-watermark model (source + consumer) | Two tasks consuming the same data source can be at different positions. Source watermark is global truth; consumer watermark is local progress. |
| Source watermarks are user assertions | Framework cannot determine remote data completeness. Only user code interfacing with the external system can make that claim. |
| Late arrival is scheduler-level, not accumulator-level | From accumulator's perspective a late boundary just goes into the next cycle. The policy decision (discard/retrigger/side-channel) belongs on the scheduler. |
| BoundaryLedger rejects backward movement | Watermarks are monotonic. If a source watermark could go backward, all consumer watermark comparisons become invalid. |
