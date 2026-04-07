---
id: kafka-client-evaluation-and
level: task
title: "Kafka client evaluation and KafkaStreamBackend implementation"
short_code: "CLOACI-T-0432"
created_at: 2026-04-07T18:44:25.633604+00:00
updated_at: 2026-04-07T18:44:25.633604+00:00
parent: CLOACI-I-0084
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0084
---

# Kafka client evaluation and KafkaStreamBackend implementation

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0084]]

## Objective

Evaluate Kafka client libraries (rdkafka, rskafka, kafka-rust) and implement `KafkaStreamBackend` as a concrete `StreamBackend` trait implementation. This is the foundational task — everything else builds on having a working Kafka consumer.

## Acceptance Criteria

- [ ] Kafka client library selected with documented rationale (async-native preferred, C dep tradeoff evaluated)
- [ ] `KafkaStreamBackend` implements `StreamBackend` trait: `connect`, `recv`, `commit`, `current_offset`
- [ ] Registered in global `StreamBackendRegistry` as `"kafka"`
- [ ] `connect(config)` creates consumer, subscribes to topic, uses `StreamConfig` fields (broker_url, topic, group, extra)
- [ ] `recv()` returns `RawMessage` with payload bytes, offset, timestamp
- [ ] `commit()` commits offset to Kafka consumer group
- [ ] Offset tracking integrated with checkpoint DAL — commit to both Kafka and DAL
- [ ] On restart: resume from last committed offset (DAL checkpoint or consumer group, whichever is available)
- [ ] Error handling: connection failures, broker unavailable, deserialization errors surfaced via `StreamError`
- [ ] Unit test with `MockStreamBackend` still passes
- [ ] Manual test against local Kafka broker (T-0435 provides Docker setup)

## Implementation Notes

### Key files
- `crates/cloacina/src/computation_graph/stream_backend.rs` — trait already exists, add `KafkaStreamBackend`
- `crates/cloacina/Cargo.toml` — add Kafka client dependency (behind feature flag `kafka`)

### Client options
1. **rdkafka** — librdkafka wrapper, battle-tested, C dependency
2. **rskafka** — pure Rust, async-native, minimal API
3. **kafka-rust** — pure Rust, sync, older

### Dependencies
- T-0435 (Docker Compose Kafka) for manual testing

## Status Updates **[REQUIRED]**

*To be added during implementation*
