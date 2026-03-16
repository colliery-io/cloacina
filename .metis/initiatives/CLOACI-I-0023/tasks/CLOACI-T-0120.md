---
id: signalaccumulator-trait
level: task
title: "SignalAccumulator trait, SimpleAccumulator, and AccumulatorMetrics"
short_code: "CLOACI-T-0120"
created_at: 2026-03-15T11:46:28.471591+00:00
updated_at: 2026-03-15T12:04:57.863938+00:00
parent: CLOACI-I-0023
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0023
---

# SignalAccumulator trait, SimpleAccumulator, and AccumulatorMetrics

## Parent Initiative

[[CLOACI-I-0023]]

## Objective

Implement the `SignalAccumulator` trait, `SimpleAccumulator` preset, and `AccumulatorMetrics` as specified in CLOACI-S-0005. The accumulator is the per-edge stateful component that buffers boundaries, coalesces them, and decides when to fire the downstream task.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `SignalAccumulator` trait with methods: `receive()`, `is_ready()`, `drain()`, `metrics()`, `consumer_watermark()`
- [ ] `AccumulatorMetrics` struct: `buffered_count`, `oldest_boundary_emitted_at`, `newest_boundary_emitted_at`, `max_lag`
- [ ] `SimpleAccumulator` implementation — no watermark awareness, fires based on injected `TriggerPolicy`
- [ ] `SimpleAccumulator::receive()` wraps boundary in `BufferedBoundary` with `received_at` timestamp
- [ ] `SimpleAccumulator::drain()` returns partial `Context<Value>` with `__boundary`, `__signals_coalesced`, `__accumulator_lag_ms` keys
- [ ] `SimpleAccumulator::drain()` calls coalesce on buffered boundaries per variant rules, then clears buffer
- [ ] `consumer_watermark()` returns the last drained boundary (updated on each drain)
- [ ] Unit tests: receive + drain cycle, coalescing through accumulator, metrics accuracy, empty drain behavior

## Implementation Notes

### Technical Approach
- Trait and impls in `continuous/accumulator.rs`
- `SimpleAccumulator` holds: `buffer: Vec<BufferedBoundary>`, `policy: Box<dyn TriggerPolicy>`, `consumer_watermark: Option<ComputationBoundary>`
- `is_ready()` delegates to `self.policy.should_fire(&self.buffer)`
- `drain()` coalesces all buffered boundaries, produces context fragment, clears buffer, updates consumer watermark
- Accumulator is `Send + Sync` (stored behind `Arc<Mutex<>>` on the graph edge)

### Dependencies
- T-0117 (ComputationBoundary, coalescing), T-0121 (TriggerPolicy — can stub with `Immediate` initially)

## Status Updates

- Created `continuous/accumulator.rs`
- `SignalAccumulator` trait: `receive()`, `is_ready()`, `drain()`, `metrics()`, `consumer_watermark()`
- `AccumulatorMetrics` struct with buffered_count, oldest/newest emitted_at, max_lag
- `SimpleAccumulator`: buffers as `Vec<BufferedBoundary>`, delegates `is_ready()` to injected `TriggerPolicy`
- `drain()` coalesces boundaries, produces context with `__boundary`, `__signals_coalesced`, `__accumulator_lag_ms`
- Consumer watermark updated on each drain
- 7 passing tests: receive+drain, coalescing, watermark update, empty drain, metrics, lag tracking, multiple cycles
