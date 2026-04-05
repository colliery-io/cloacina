---
id: emission-tracking-sequence-numbers
level: task
title: "Emission tracking — sequence numbers on boundaries, persisted counters, reactor-side deduplication"
short_code: "CLOACI-T-0413"
created_at: 2026-04-05T21:24:28.172337+00:00
updated_at: 2026-04-05T21:24:28.172337+00:00
parent: CLOACI-I-0081
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0081
---

# Emission tracking — sequence numbers on boundaries, persisted counters, reactor-side deduplication

## Parent Initiative

[[CLOACI-I-0081]]

## Objective

Add sequence numbers to boundaries flowing between accumulators and reactors. This enables deduplication at the reactor (skip boundaries already processed after a restart), ordering guarantees for the Sequential strategy, and observability of emission rates.

## Acceptance Criteria

- [ ] `BoundaryEnvelope` wrapper struct: `{ source: SourceName, data: Vec<u8>, sequence: u64 }`
- [ ] Each accumulator maintains a monotonically increasing sequence counter (starts at 0, increments on each emit)
- [ ] Sequence counter persisted to `accumulator_boundaries.sequence_number` column (from T-0407 schema)
- [ ] On accumulator restart: load last sequence number from DAL, resume from `last + 1`
- [ ] Reactor tracks last-processed sequence number per source
- [ ] Reactor skips boundaries with `sequence <= last_processed[source]` (deduplication)
- [ ] Last-processed sequence numbers persisted alongside `InputCache` in `reactor_state`
- [ ] `BoundarySender::send()` now wraps output in `BoundaryEnvelope` (internal change, accumulator authors don't see it)
- [ ] Channel type changes from `mpsc::Sender<(SourceName, Vec<u8>)>` to `mpsc::Sender<BoundaryEnvelope>`
- [ ] Sequential strategy uses sequence numbers for ordering verification (log warning if out-of-order boundary received)
- [ ] Unit tests: sequence number increment, persistence round-trip, deduplication logic
- [ ] Unit tests: restart accumulator, verify sequence resumes from persisted value
- [ ] Integration test: restart accumulator mid-stream, verify reactor deduplicates re-emitted boundaries

## Implementation Notes

### Technical Approach

The change is mostly internal to `BoundarySender` and the reactor receiver. Accumulator authors never interact with sequence numbers directly — the `BoundarySender::send()` method wraps the boundary in a `BoundaryEnvelope` before putting it on the channel.

```rust
struct BoundaryEnvelope {
    source: SourceName,
    data: Vec<u8>,
    sequence: u64,
}
```

**Deduplication**: When the reactor receives a boundary, it checks `envelope.sequence > last_processed[envelope.source]`. If not, it's a duplicate (from a restarted accumulator re-emitting) and is dropped. This is especially important after accumulator restart — the accumulator may re-process events it already handled before the crash.

**Ordering**: For Sequential strategy, boundaries should arrive in sequence order per source. If a boundary arrives out of order (sequence gap), log a warning but process it anyway — this shouldn't happen in normal operation but could indicate a bug.

### Key files
- `crates/cloacina/src/computation_graph/accumulator.rs` — `BoundaryEnvelope`, sequence counter in `BoundarySender`
- `crates/cloacina/src/computation_graph/reactor.rs` — deduplication logic in receiver, sequence tracking

### Dependencies
- T-0407 (DAL foundation) — uses `accumulator_boundaries.sequence_number` column
- T-0408 (accumulator checkpoint) — sequence counter persistence uses same DAL path

### Risk Considerations
- Channel type change (`(SourceName, Vec<u8>)` -> `BoundaryEnvelope`) is a breaking internal change. All code that creates or reads from boundary channels must be updated. Grep for the old tuple type.
- u64 sequence counter won't overflow in practice (2^64 emissions at 1M/sec = 584,000 years)

## Status Updates

*To be added during implementation*
