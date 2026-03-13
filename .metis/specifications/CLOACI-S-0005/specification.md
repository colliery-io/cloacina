---
id: signalaccumulator-and
level: specification
title: "SignalAccumulator and TriggerPolicy - Buffering and Firing"
short_code: "CLOACI-S-0005"
created_at: 2026-03-10T18:18:23.012854+00:00
updated_at: 2026-03-10T18:18:23.012854+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# SignalAccumulator and TriggerPolicy - Buffering and Firing

*Component specification for CLOACI-S-0001 (Continuous Reactive Scheduling).*

## Overview

### SignalAccumulator

Per-edge stateful component that buffers boundaries (CLOACI-S-0002), coalesces them, and decides when to fire the downstream task.

The accumulator exists because **a boundary arriving is not the same as a task needing to run.** Without it, every boundary triggers immediate execution. The accumulator answers three questions:
1. **When** should the task run? (trigger policy)
2. **With what scope?** (coalesce buffered boundaries)
3. **What context?** (produce a partial context fragment for the execution layer)

### TriggerPolicy

Controls when the accumulator fires. Policies are inherently domain-dependent — "how many" and "how long" are interpretations of boundary data, and meaningful implementations need to understand the boundaries they evaluate.

## SignalAccumulator Trait

```rust
trait SignalAccumulator: Send + Sync {
    /// Buffer a boundary event (internally timestamped on receipt for backpressure)
    fn receive(&mut self, boundary: ComputationBoundary);

    /// Should the downstream task run now?
    fn is_ready(&self) -> bool;

    /// Coalesce buffered boundaries and produce a partial context fragment.
    /// This fragment is merged with upstream task output context by the ContextManager
    /// at the execution layer. Clears the buffer.
    fn drain(&mut self) -> Context<Value>;

    /// Observable state for monitoring and backpressure detection
    fn metrics(&self) -> AccumulatorMetrics;

    /// What boundary has this accumulator processed up to?
    /// Updated on each drain(). Used by the scheduler for late arrival detection.
    fn consumer_watermark(&self) -> Option<&ComputationBoundary>;
}

struct AccumulatorMetrics {
    buffered_count: usize,
    oldest_boundary_emitted_at: Option<DateTime>,
    newest_boundary_emitted_at: Option<DateTime>,
    max_lag: Option<Duration>,               // max(received_at - emitted_at) across buffer
}
```

The accumulator is purely a scheduling component — it decides **when** to fire and **what boundary** to process. It does not handle upstream task context; that is the `ContextManager`'s responsibility at the execution layer. The accumulator's `drain()` returns a partial context fragment (boundary info, coalescing metadata) that gets merged with upstream context by the existing execution pipeline.

A default `drain()` produces context keys like:
```json
{
    "__boundary": { "kind": "time_range", "start": "...", "end": "..." },
    "__signals_coalesced": 47,
    "__accumulator_lag_ms": 230
}
```

`TriggerPolicy` and `WatermarkMode` are construction-time configuration for the built-in accumulator presets, not part of the trait itself. Custom accumulators may implement readiness logic that doesn't decompose into policy + watermark mode.

## TriggerPolicy Trait

A trigger policy is a function of the current buffer state → `bool`.

```rust
trait TriggerPolicy: Send + Sync {
    fn should_fire(&self, buffer: &[BufferedBoundary]) -> bool;
}
```

### Composition

Policies are composable via `Any` (OR) and `All` (AND) combinators:

```rust
struct Any(Vec<Box<dyn TriggerPolicy>>);  // fire when ANY sub-policy says fire
struct All(Vec<Box<dyn TriggerPolicy>>);  // fire when ALL sub-policies say fire
```

Both implement `TriggerPolicy` and nest arbitrarily. Examples:

```rust
// "every 5 minutes OR 20 boundaries"
Any(vec![
    Box::new(WallClockWindow { duration: minutes(5) }),
    Box::new(BoundaryCount { count: 20 }),
])

// "at least 1000 rows AND at least 1 minute since last drain"
All(vec![
    Box::new(RowCountThreshold { threshold: 1000 }),
    Box::new(WallClockWindow { duration: minutes(1) }),
])
```

### Framework-Provided Implementations

Common building blocks, explicit about what they measure:

| Policy | Fires when | Assumption |
|---|---|---|
| `Immediate` | Every boundary | None — always fires |
| `WallClockWindow { duration }` | Wall clock time since last drain exceeds duration | Uses wall clock, not data time |
| `WallClockDebounce { duration }` | No new boundary received for duration (wall clock) | Silence = burst is over |
| `BoundaryCount { count }` | N boundaries buffered | All boundaries are equal weight |

These are honest about their implicit assumptions. Users who need data-time-aware policies (e.g., "fire when 6 hours of data time have accumulated") implement the trait and read boundary data directly.

### User-Defined Example

```rust
struct RowCountThreshold { threshold: u64 }

impl TriggerPolicy for RowCountThreshold {
    fn should_fire(&self, buffer: &[BufferedBoundary]) -> bool {
        let total_rows: u64 = buffer.iter()
            .filter_map(|b| b.boundary.metadata.as_ref())
            .filter_map(|m| m["row_count"].as_u64())
            .sum();
        total_rows >= self.threshold
    }
}
```

### Per-Edge Policy Examples

Different tasks watching the same data source can have different trigger policies:
- `[raw_events] → [realtime_alerts]` — `Immediate`
- `[raw_events] → [aggregate_hourly]` — `WallClockWindow(1h)`
- `[raw_events] → [ml_retrain]` — `Any(BoundaryCount(100_000), WallClockWindow(24h))`

## Multi-Input Task Semantics

When a task has multiple accumulators (from multiple input data sources), the task-level readiness is determined by composing accumulator readiness with the same `Any`/`All` pattern used in `TriggerPolicy`:

- **Any** — fire when any accumulator is ready (use latest-known context for others)
- **All** — fire when all accumulators are ready

This is configured at the graph wiring level (CLOACI-S-0008), not on the task itself. Data sources that should be available to the task but should not participate in scheduling (e.g., config lookups) simply don't have accumulators — they're wired as referenced data sources without accumulation.

## Framework-Provided Accumulator Presets

| Accumulator preset | Source watermark behavior | Use case |
|---|---|---|
| `SimpleAccumulator` | No watermark awareness | Config changes, full-state data, non-temporal data |
| `WindowedAccumulator` | Waits for source watermark before firing | Standard time-windowed aggregation |
| Custom `impl SignalAccumulator` | Developer controls | Custom readiness logic, domain-specific coalescing |

## Accumulator Persistence (Hybrid Model)

Accumulators are **in-memory during operation** but **persist on drain**:

- **`receive()`** — pure in-memory, no DB writes. This is the hot path.
- **`drain()`** — after producing the context fragment, persists: consumer watermark, drain timestamp, coalesced boundary metadata. This is the commit point.
- **On restart** — consumer watermarks are loaded from DB. Detectors re-poll from their own persisted state (detector workflows store `__last_known_state` in context). Boundaries between last drain and crash are re-detected naturally.

The re-processing window after a crash is bounded by the trigger policy interval (e.g., if drains happen hourly, worst case is re-detecting up to 1 hour of boundaries). Coalescing ensures re-detected boundaries produce the same result — at-least-once semantics, not exactly-once.

```rust
struct PersistedAccumulatorState {
    edge_id: String,                           // identifies the graph edge
    consumer_watermark: Option<ComputationBoundary>,
    last_drain_at: DateTime,
    drain_metadata: serde_json::Value,         // coalescing stats, boundary summary
}
```

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| TriggerPolicy as trait, not enum | Policies are inherently interpretive and domain-dependent. Can't enumerate them. |
| Persist on drain, not on receive | Hot path (receive) stays in-memory. Drain is the natural commit point — infrequent, already produces output. Re-detection between drains is cheap and coalescing makes it idempotent. |
| Any/All composition | Mirrors JoinMode and LedgerMatchMode — consistent pattern. Supports "every 5 minutes or 20 boundaries." |
| `drain()` returns `Context<Value>` not `ComputationBoundary` | Drain returns a partial context fragment for the execution layer, not just boundary info. |
| `on_late_arrival` removed from accumulator | Late arrival isn't the accumulator's concern — from its perspective a boundary just goes into the next cycle. Moved to scheduler (CLOACI-S-0006). |
| Accumulator doesn't take upstream context | Upstream context is an execution layer concern (ContextManager), not a scheduling concern. |
