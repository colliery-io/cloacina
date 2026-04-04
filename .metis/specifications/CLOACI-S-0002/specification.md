---
id: computationboundary-accumulators
level: specification
title: "ComputationBoundary & Accumulators"
short_code: "CLOACI-S-0002"
created_at: 2026-04-04T11:45:21.140876+00:00
updated_at: 2026-04-04T11:45:21.140876+00:00
parent: CLOACI-I-0053
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# ComputationBoundary & Accumulators

## Overview

ComputationBoundaries and SignalAccumulators are the data-flow primitives of Cloacina's continuous scheduling system. A `ComputationBoundary` describes *what data changed* — a slice of a data source that a detector observed. A `SignalAccumulator` *buffers and coalesces* boundaries per edge in the DataSourceGraph, deciding when enough data has arrived to fire a downstream task.

Together they solve the core problem of reactive scheduling: multiple independent data sources produce change signals at different rates and with different semantics, and these signals must be correlated into a consistent snapshot for downstream consumers.

### Relationship to other specs

- **CLOACI-S-0001** (Core Architecture) — defines the DataSourceGraph, scheduler loop, and persistence layer that boundaries and accumulators plug into
- **CLOACI-S-0003** (Task Execution Model) — defines how tasks receive accumulated boundaries via context injection

## ComputationBoundary

### Definition

A `ComputationBoundary` describes a contiguous slice of data from a data source. It carries:

- **kind**: One of 5 `BoundaryKind` variants (see below)
- **source_name**: Which data source this boundary came from
- **metadata**: Optional key-value pairs for lineage, tags, or debugging
- **emitted_at**: Timestamp when the detector produced this boundary

### BoundaryKind Variants

| Kind | Fields | Semantics | Typical source |
|------|--------|-----------|----------------|
| `TimeRange` | `start: DateTime`, `end: DateTime` | Airflow-style time intervals. The boundary covers all data in `[start, end)`. | Batch ETL, time-partitioned tables |
| `OffsetRange` | `start: i64`, `end: i64` | Kafka-style partition offsets. The boundary covers messages at offsets `[start, end)`. | Kafka topics, append-only logs |
| `Cursor` | `value: String` | Opaque resume token. The boundary represents "everything since the last cursor." | API pagination, change feeds |
| `FullState` | `value: String` | Hash, version, or commit identifier. The boundary represents a complete snapshot. | Git commits, config versions, S3 ETags |
| `Custom` | `kind: String`, `value: serde_json::Value` | User-defined boundary type with optional schema validation. | Domain-specific change signals |

### Coalescing Rules

When multiple boundaries of the same kind are buffered in an accumulator, they coalesce into a single boundary on drain:

| Kind | Coalescing rule | Result |
|------|----------------|--------|
| `TimeRange` | `min(starts)..max(ends)` | Widest time span covering all buffered boundaries |
| `OffsetRange` | `min(starts)..max(ends)` | Widest offset span covering all buffered boundaries |
| `Cursor` | Latest value wins (by `emitted_at`) | Most recent cursor token |
| `FullState` | Latest value wins (by `emitted_at`) | Most recent state identifier |
| `Custom` | Latest value wins (by `emitted_at`) | Most recent custom payload |

Coalescing is **lossy by design** for range types — intermediate boundaries are merged into the span. For latest-wins types, only the most recent value is retained. This is correct for the target workloads: a decision engine needs the latest state, not the history of intermediate states.

### Custom Boundary Schema

Custom boundaries can optionally declare a `CustomBoundarySchema` — a JSON Schema definition that validates the `value` field. Schema validation runs at boundary creation time (in the detector) and is enforced by a global schema registry. This prevents malformed custom boundaries from propagating through the graph.

### BufferedBoundary

A `BufferedBoundary` wraps a `ComputationBoundary` with a `received_at` timestamp (when the accumulator received it). This enables:

- **Lag tracking**: `received_at - emitted_at` measures ingestion latency per boundary
- **Backpressure detection**: If lag grows over time, the accumulator is falling behind the source

## SignalAccumulator

### Trait Definition

```rust
trait SignalAccumulator: Send + Sync {
    /// Accept a new boundary into the buffer
    fn receive(&mut self, boundary: ComputationBoundary) -> Result<(), AccumulatorError>;

    /// Check if the accumulator is ready to fire (based on TriggerPolicy)
    fn is_ready(&self) -> bool;

    /// Atomically check readiness and drain if ready.
    /// Returns None if not ready, Some(coalesced_boundary) if drained.
    fn try_drain(&mut self) -> Option<CoalescedResult>;

    /// Drain without readiness check (used by scheduler when JoinMode forces drain)
    fn drain(&mut self) -> CoalescedResult;

    /// Current metrics (buffered count, lag, drain count)
    fn metrics(&self) -> AccumulatorMetrics;
}
```

The `try_drain()` method is atomic — it checks readiness and drains in a single call. This prevents race conditions where readiness changes between the check and the drain (important when the scheduler evaluates JoinMode across multiple accumulators).

### CoalescedResult

The result of draining an accumulator:

- **boundary**: The single `ComputationBoundary` produced by coalescing all buffered boundaries
- **signals_coalesced**: How many raw boundaries were merged (useful for metrics)
- **accumulator_lag_ms**: Maximum `received_at - emitted_at` across all coalesced boundaries
- **consumer_watermark**: The new watermark position after this drain

### Implementations

#### SimpleAccumulator

Basic boundary collection with no windowing or watermark gating.

- Receives boundaries into a `Vec<BufferedBoundary>` with bounded capacity (`max_buffer_size`)
- When capacity is reached, oldest boundaries are evicted (FIFO)
- Readiness is determined by the associated `TriggerPolicy`
- On drain: coalesces all buffered boundaries per the coalescing rules, clears the buffer, updates metrics

#### WindowedAccumulator

Adds watermark-aware buffering on top of SimpleAccumulator.

Two modes:
- **WaitForWatermark**: Only becomes ready when the source watermark advances past all buffered boundaries. Ensures completeness — no late arrivals can invalidate the drained result.
- **BestEffort**: Becomes ready per TriggerPolicy regardless of watermark position. Accepts that late arrivals may arrive after drain.

The watermark check uses the `BoundaryLedger` (monotonic source watermark tracker) from the scheduler's watermark subsystem.

#### LatestValue (new)

Retains only the most recent boundary per edge. Designed for the state-materialization pattern where downstream consumers need "the current value," not the history.

- On `receive()`: replaces the single buffered boundary (if any) with the new one. No coalescing needed — there is only ever 0 or 1 boundary in the buffer.
- On `drain()`: returns the single boundary as-is, clears the buffer.
- Readiness: ready whenever a boundary is present (equivalent to `Immediate` trigger policy).
- **Important**: LatestValue discards all intermediate updates. If source_alpha emits 10 boundaries between drain cycles, only the 10th is retained. This is the correct semantic for reactive strategy workloads — the decision engine wants the latest orderbook, not the history of orderbook updates.

Persistence for LatestValue is simpler than other accumulators — only one boundary needs to be stored per edge, not a buffer.

## Trigger Policies

A `TriggerPolicy` determines when an accumulator is "ready" to fire. It is evaluated by `is_ready()` on each scheduler tick.

### Trait Definition

```rust
trait TriggerPolicy: Send + Sync {
    /// Check if the policy conditions are met
    fn check_readiness(&self, buffered_count: usize, oldest_received: Option<Instant>) -> bool;

    /// Called after a successful drain to reset policy state
    fn mark_drained(&mut self);
}
```

### Implementations

| Policy | Fires when | Configuration | Use case |
|--------|-----------|---------------|----------|
| `Immediate` | Any boundary is buffered (`buffered_count > 0`) | None | Low-latency reactive tasks |
| `Count` | `buffered_count >= threshold` | `threshold: usize` | Batch processing (fire every N events) |
| `WallClockWindow` | `now - oldest_received >= duration` | `duration: Duration`, `debounce: Option<Duration>` | Time-based batching |
| `WallClockDebounce` | No new boundary received for `debounce` duration | `debounce: Duration` | Wait for burst to settle before firing |
| `AnyPolicy` | Any child policy fires | `children: Vec<Box<dyn TriggerPolicy>>` | Composite OR |
| `AllPolicy` | All child policies fire | `children: Vec<Box<dyn TriggerPolicy>>` | Composite AND |

### Validation

- `WallClockWindow` validates that `duration` is reasonable (not accidentally set to 52 weeks due to parsing errors)
- `WallClockDebounce` has the same validation
- Composite policies validate that they have at least one child

### Policy + Accumulator Pairing

The trigger policy is injected into the accumulator at construction time. The pairing is typically:

| Accumulator | Default policy | Override scenarios |
|-------------|---------------|-------------------|
| `SimpleAccumulator` | `Immediate` | `Count` for batching, `WallClockWindow` for time-based |
| `WindowedAccumulator` | `Immediate` (gated by watermark) | Same as Simple — watermark provides the primary gating |
| `LatestValue` | `Immediate` (always ready when value present) | Rarely overridden — latest-value semantics imply immediate readiness |

## Accumulator Metrics

Each accumulator tracks incremental metrics (O(1) updates, not O(n) recalculation):

| Metric | Type | Description |
|--------|------|-------------|
| `buffered_count` | `usize` | Current number of boundaries in the buffer |
| `total_received` | `u64` | Lifetime count of boundaries received |
| `total_drained` | `u64` | Lifetime count of drain operations |
| `total_evicted` | `u64` | Lifetime count of boundaries evicted due to capacity |
| `max_lag_ms` | `u64` | Maximum `received_at - emitted_at` across current buffer |
| `consumer_watermark` | `Option<ComputationBoundary>` | Last drained boundary (high watermark) |

These metrics are injected into the task's execution context on drain (see CLOACI-S-0003) and recorded in the ExecutionLedger.

## Persistence

### Accumulator State Persistence

Accumulator state is persisted through the boundary WAL model described in CLOACI-S-0001:

- **Pending boundaries** are written to the `pending_boundaries` table when routed to an accumulator
- **Edge drain cursors** in the `edge_drain_cursors` table track which boundaries have been drained per edge
- On crash recovery, the accumulator is rebuilt by replaying pending boundaries after the edge's drain cursor

For `LatestValue` accumulators, persistence is optimized: only the single latest boundary needs to be stored. The WAL still records all boundaries (for audit), but recovery only needs the most recent entry per edge.

### Watermark Persistence

The `BoundaryLedger` (source watermark tracker) maintains a monotonic watermark per data source. Watermarks are persisted as part of the `detector_state` table — the committed checkpoint includes the watermark position. On recovery, watermarks are restored from detector state.

## Constraints

### Technical Constraints

- Accumulator `receive()` must be O(1) on the hot path (append to buffer or replace for LatestValue). No allocations on the hot path except when buffer grows.
- Coalescing on `drain()` is O(n) where n is buffer size — acceptable because drain happens less frequently than receive.
- The `try_drain()` method must be atomic (check + drain in one call) to prevent TOCTOU issues when the scheduler evaluates multi-edge JoinMode.
- LatestValue accumulator must not retain intermediate values — this is a correctness requirement, not just an optimization. Retaining stale intermediate values could cause a decision engine to act on outdated state.
- Buffer capacity (`max_buffer_size`) must be enforced with FIFO eviction. An unbounded buffer would cause memory growth under sustained load when drain rate falls behind receive rate.
