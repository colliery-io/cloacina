---
id: accumulator-depth-batch-and
level: initiative
title: "Accumulator Depth — Batch and Polling Accumulators"
short_code: "CLOACI-I-0073"
created_at: 2026-04-04T17:48:57.173883+00:00
updated_at: 2026-04-05T14:50:35.873130+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: accumulator-depth-batch-and
---

# Accumulator Depth — Batch and Polling Accumulators

## Context

Adds the remaining accumulator types to the computation graph system. Currently we have passthrough (socket-only) and stream (broker-backed) accumulators. This initiative adds batch and polling accumulators — two common patterns for data sources that don't have a continuous stream.

Blocked by: nothing. All accumulator infrastructure exists from I-0074.

## Goals & Non-Goals

**Goals:**
- Implement `#[batch_accumulator]` — dormant until flush signal, drains source, processes batch, emits one boundary. Critical for "all events since last run" reconciliation patterns.
- Implement `#[polling_accumulator]` — timer-based async poll function that queries databases/APIs on an interval. `Option<T>` return for "no change" (don't emit boundary if nothing changed).
- Macro support for both: `#[batch_accumulator]` and `#[polling_accumulator]` generate structs implementing the `Accumulator` trait
- Tutorial or example demonstrating each new type
- Unit tests for both accumulator types

**Non-Goals:**
- `when_all` / `sequential` reactor features (separate initiative)
- Python accumulator decorators (separate initiative)
- Soak tests / performance benchmarks (separate initiative)
- DAL persistence for accumulator checkpoints — deferred to I-0082 (MVP Resilience Wiring: PERSIST-1 batch buffer crash resilience, PERSIST-2 polling checkpoint restore)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `#[batch_accumulator]` implemented — dormant until flush signal from reactor, drains source on flush, processes batch as a Vec, emits one boundary
- [ ] Batch accumulator works with both stream backend (drain broker) and socket (drain buffered messages)
- [ ] `#[polling_accumulator]` implemented — configurable interval, async poll function, `Option<T>` return semantics
- [ ] Polling accumulator skips boundary emission when poll returns None
- [ ] Macros generate correct `Accumulator` trait implementations
- [ ] Unit tests for batch: buffer events, flush, verify single boundary emitted with batch data
- [ ] Unit tests for polling: timer fires, poll called, boundary emitted on Some, skipped on None
- [ ] Example or tutorial demonstrating each type
- [ ] All existing tests continue to pass

## Implementation Plan

1. **Batch accumulator trait extension** — extend `Accumulator` trait or create `BatchAccumulator` sub-trait with `process_batch(Vec<Event>) -> Option<Output>`
2. **Batch accumulator runtime** — modified runtime that buffers events and drains on flush signal
3. **`#[batch_accumulator]` macro** — generates struct + trait impl, similar to `#[passthrough_accumulator]`
4. **Polling accumulator trait** — `PollingAccumulator` with `async fn poll() -> Option<Output>` + interval config
5. **Polling accumulator runtime** — timer-based loop calling poll, emitting boundary on Some
6. **`#[polling_accumulator]` macro** — generates struct + trait impl
7. **Tests + examples**
