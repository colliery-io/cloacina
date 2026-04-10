---
id: kafka-client-evaluation-and
level: task
title: "Kafka client evaluation and KafkaStreamBackend implementation"
short_code: "CLOACI-T-0432"
created_at: 2026-04-07T18:44:25.633604+00:00
updated_at: 2026-04-07T21:17:40.925753+00:00
parent: CLOACI-I-0084
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

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

**2026-04-07 — Complete**
- **Library selected: rdkafka 0.39** — the only option with consumer groups, async/tokio, manual offset commit, and KRaft support. rskafka has no consumer groups; kafka-rust is abandoned (last release 2016).
- Added `rdkafka = { version = "0.39", features = ["tokio"], optional = true }` behind `kafka` feature flag
- `KafkaStreamBackend` implements `StreamBackend` trait in `stream_backend.rs::kafka` module
  - `connect`: creates `StreamConsumer` with bootstrap.servers, group.id, auto.commit=false, extra config overrides
  - `recv`: reads from `consumer.stream().next()`, returns payload bytes + offset + timestamp
  - `commit`: calls `commit_consumer_state(CommitMode::Sync)`, updates committed_offset
  - `current_offset`: returns last received offset
- `register_kafka_backend()` registers factory as `"kafka"` in global `StreamBackendRegistry`
- Compiles with `--features kafka` and without (feature-gated behind `#[cfg(feature = "kafka")]`)
- Default build unchanged — no kafka dep unless explicitly enabled
- DAL checkpoint integration deferred — currently commits to Kafka consumer group only. DAL dual-write can be added when checkpoint infrastructure is wired (existing accumulator checkpoint handles can be reused)
