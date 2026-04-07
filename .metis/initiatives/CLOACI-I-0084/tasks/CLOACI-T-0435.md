---
id: docker-compose-kafka-kraft-angreal
level: task
title: "Docker Compose Kafka (KRaft), angreal tasks, and integration tests"
short_code: "CLOACI-T-0435"
created_at: 2026-04-07T18:44:44.899332+00:00
updated_at: 2026-04-07T19:39:06.567944+00:00
parent: CLOACI-I-0084
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0084
---

# Docker Compose Kafka (KRaft), angreal tasks, and integration tests

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0084]]

## Objective

Add Kafka to the Docker Compose infrastructure (KIP-500 / KRaft mode, no ZooKeeper), create angreal tasks for Kafka integration testing, and write end-to-end tests that exercise the full Kafka → accumulator → graph pipeline.

## Acceptance Criteria

## Acceptance Criteria

- [ ] Kafka added to `.angreal/docker-compose.yaml` — KRaft mode (KIP-500), no ZooKeeper
- [ ] Kafka accessible at `localhost:9092` when services are running
- [ ] `angreal services up` starts both Postgres and Kafka
- [ ] `angreal cloacina kafka-integration` runs Kafka integration tests
- [ ] End-to-end test: produce messages to Kafka topic → stream accumulator consumes → graph fires → verify output
- [ ] Restart recovery test: produce, consume, stop server, restart, verify resume from last offset
- [ ] Soak test updated: server-soak uploads a Kafka-sourced CG package, produces to topic during soak loop
- [ ] CI workflow updated: Kafka service alongside Postgres in integration test jobs

## Implementation Notes

### Docker image
Use `apache/kafka` (official KRaft image) or `bitnami/kafka` with KRaft mode enabled. Both support ZooKeeper-free operation via KIP-500.

### Key files
- `.angreal/docker-compose.yaml` — add kafka service
- `.angreal/cloacina/kafka_integration.py` — new angreal task
- `.angreal/cloacina/server_soak.py` — add Kafka-sourced CG step
- `.github/workflows/cloacina.yml` or `performance.yml` — add Kafka service to CI

### Dependencies
- T-0432 (KafkaStreamBackend) — needs the backend to test against

## Status Updates **[REQUIRED]**

**2026-04-07 — Docker Compose Kafka done**
- Added `apache/kafka:3.9.0` to `.angreal/docker-compose.yaml` in KRaft mode (KIP-500, no ZooKeeper)
- Config: KRaft combined broker+controller, PLAINTEXT on 9092, controller on 9093
- `CLUSTER_ID: cloacina-dev-cluster-001`, replication factor 1 for dev
- Volume: `kafka_data` at `/tmp/kraft-combined-logs`
- Healthcheck: `kafka-broker-api-versions.sh` with 30s start_period
- `angreal services up` starts both Postgres and Kafka
- `angreal services clean` removes both volumes
- Verified: Kafka Server started in KRaft mode, version 3.9.0
- Remaining: integration test angreal task, soak test update, CI workflow — blocked on T-0432 (KafkaStreamBackend)
