---
id: python-stream-accumulator-backend
level: task
title: "Python stream accumulator backend wiring and Kafka example"
short_code: "CLOACI-T-0436"
created_at: 2026-04-07T18:44:51.176596+00:00
updated_at: 2026-04-07T18:44:51.176596+00:00
parent: CLOACI-I-0084
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0084
---

# Python stream accumulator backend wiring and Kafka example

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0084]]

## Objective

Wire the Python `@cloaca.stream_accumulator(type="kafka", ...)` decorator to actually create and configure a `KafkaStreamBackend` when the computation graph is loaded. Currently the decorator stores metadata only — the backend is never instantiated.

## Acceptance Criteria

- [ ] Python stream accumulator metadata (type, topic, group) flows through to `AccumulatorFactory` during graph loading
- [ ] `AccumulatorFactory` creates a stream accumulator with `KafkaStreamBackend` when type="kafka"
- [ ] Python accumulator `process()` function receives deserialized Kafka messages
- [ ] Python stateful stream accumulator (`state={...}`) works with Kafka backend
- [ ] Python batch accumulator (`@cloaca.batch_accumulator(type="kafka", ...)`) works with Kafka backend
- [ ] Example: Python Kafka-sourced computation graph (tutorial or example dir)
- [ ] End-to-end test: Python graph consuming from Kafka topic

## Implementation Notes

### Key files
- `crates/cloacina/src/python/computation_graph.rs` — decorator metadata → backend config
- `crates/cloacina/src/computation_graph/packaging_bridge.rs` — AccumulatorFactory for packaged graphs
- `crates/cloacina/src/computation_graph/scheduler.rs` — load_graph with stream-backed accumulators

### Dependencies
- T-0432 (KafkaStreamBackend)
- T-0435 (Docker Kafka for testing)

## Status Updates **[REQUIRED]**

*To be added during implementation*
