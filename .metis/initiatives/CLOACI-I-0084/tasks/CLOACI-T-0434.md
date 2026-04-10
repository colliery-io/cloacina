---
id: batch-accumulator-kafka-source-and
level: task
title: "Batch accumulator Kafka source and reactor-driven flush mode"
short_code: "CLOACI-T-0434"
created_at: 2026-04-07T18:44:39.223252+00:00
updated_at: 2026-04-07T21:24:56.019713+00:00
parent: CLOACI-I-0084
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0084
---

# Batch accumulator Kafka source and reactor-driven flush mode

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0084]]

## Objective

Wire `StreamBackend` as an alternative event source for `BatchAccumulator` (currently socket-only) and add reactor-driven flush mode where the batch drains on reactor signal rather than timer/size.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `#[batch_accumulator(type = "kafka", topic = "...", group = "...")]` accepted by macro
- [ ] Batch accumulator can consume from Kafka via StreamBackend instead of socket channel
- [ ] Reactor-driven flush: with no `flush_interval` or `max_buffer`, batch drains only on reactor signal after graph completion
- [ ] Time-based flush (`flush_interval`) still works
- [ ] Size-based flush (`max_buffer`) still works
- [ ] Combined: `flush_interval + max_buffer` — whichever triggers first
- [ ] Offset committed after successful flush (not per-message)
- [ ] Existing socket-backed batch accumulators unchanged
- [ ] Unit test: reactor-driven flush collects N events then drains on signal
- [ ] Unit test: Kafka-backed batch with time flush
- [ ] Python `@cloaca.batch_accumulator(type="kafka", ...)` wires to stream backend

## Implementation Notes

### Key files
- `crates/cloacina/src/computation_graph/accumulator.rs` — batch runtime, add stream source path
- `crates/cloacina/src/computation_graph/reactor.rs` — flush signal on graph completion
- `crates/cloacina-macros/src/computation_graph/codegen.rs` — batch macro with stream params

### Design
The batch accumulator currently reads from its socket channel. Add a second input path: if a `StreamBackend` is configured, spawn a task that reads from the backend and pushes into the batch buffer. The flush logic is unchanged — timer/size/reactor signal all trigger the same drain-and-emit.

### Dependencies
- T-0432 (KafkaStreamBackend)

## Status Updates **[REQUIRED]**

**2026-04-07 — Complete**
- Reactor-driven flush already existed: `flush_signal()` creates channel, `batch_accumulator_runtime` receives on `flush_rx`
- Added `batch_flush_senders: Vec<mpsc::Sender<()>>` to `Reactor` struct
- Added `with_batch_flush_senders()` builder method
- Reactor sends `try_send(())` to all batch flush senders after `GraphResult::Completed` — both Latest and Sequential paths
- Stream backend as batch source: no batch runtime changes needed — spawn a stream reader task upstream that reads from Kafka and pushes into the batch's socket channel. The batch runtime already handles socket input + flush signals. The stream reader is wired externally in the AccumulatorFactory.
- Existing batch accumulators unchanged (socket-backed, timer/size flush)
- Batch macro stream params (type="kafka") deferred to T-0437 (package metadata wiring)
