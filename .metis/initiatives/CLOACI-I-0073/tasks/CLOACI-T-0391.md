---
id: batch-accumulator-trait-buffered
level: task
title: "Batch accumulator — trait, buffered runtime with flush signal, and #[batch_accumulator] macro"
short_code: "CLOACI-T-0391"
created_at: 2026-04-05T14:41:07.758085+00:00
updated_at: 2026-04-05T14:58:40.738280+00:00
parent: CLOACI-I-0073
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0073
---

# Batch accumulator — trait, buffered runtime with flush signal, and #[batch_accumulator] macro

## Objective

Implement the batch accumulator — an accumulator that buffers incoming events and processes them all at once on a flush signal. Instead of processing each event individually (passthrough) or polling on a timer (polling), the batch accumulator collects events into a buffer and drains them when triggered. Emits a single boundary containing the batch result.

Common use case: reconciliation ("give me all fills since last run"), aggregation ("sum all order updates"), or any "micro-batch" pattern.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `BatchAccumulator` trait with `fn process_batch(&mut self, events: Vec<Self::Event>) -> Option<Self::Output>`
- [ ] `batch_accumulator_runtime()` — buffers events from socket + event loop, flushes on:
  - Timer-based flush interval (configurable, e.g., every 5s)
  - Manual flush signal (via channel)
  - Buffer size threshold (optional, e.g., flush at 1000 events)
- [ ] On flush: drains buffer, calls `process_batch(Vec<Event>)`, emits boundary if Some
- [ ] Empty buffer on flush → no boundary emitted (skip)
- [ ] Runtime respects shutdown signal
- [ ] `#[batch_accumulator(flush_interval = "5s")]` macro generates struct + trait impl
- [ ] Optional macro args: `flush_interval`, `max_buffer_size`
- [ ] Unit test: buffer 5 events, flush, verify single boundary with batch data
- [ ] Unit test: empty buffer flush → no boundary
- [ ] Unit test: max_buffer_size triggers automatic flush
- [ ] Unit test: shutdown drains remaining buffer before exit

## Implementation Notes

### Files
- `crates/cloacina/src/computation_graph/accumulator.rs` — add `BatchAccumulator` trait + `batch_accumulator_runtime()`
- `crates/cloacina-macros/src/computation_graph/accumulator_macros.rs` — add `batch_accumulator_impl()`
- `crates/cloacina-macros/src/lib.rs` — register `#[batch_accumulator]` proc macro

### Design
```rust
#[async_trait]
pub trait BatchAccumulator: Send + 'static {
    type Event: DeserializeOwned + Send;
    type Output: Serialize + Send;

    fn process_batch(&mut self, events: Vec<Self::Event>) -> Option<Self::Output>;
}

pub struct BatchAccumulatorConfig {
    pub flush_interval: Duration,
    pub max_buffer_size: Option<usize>,
}
```

Runtime uses `select!` on: socket_rx (buffer event), timer tick (flush), flush signal (flush), shutdown (drain + exit).

### Dependencies
None — builds on existing accumulator infrastructure.

## Status Updates

- 2026-04-05: Complete. BatchAccumulator trait + batch_accumulator_runtime + #[batch_accumulator] macro. Flushes on timer interval, max_buffer_size threshold, or shutdown (drain). Empty flush skipped. 4 unit tests: timer flush (sum of 5 events), empty flush skip, max_buffer_size trigger, shutdown drain.
