# Changelog — cloacina-provider-kafka

All notable changes to this provider. Providers version independently of
cloacina core (ADR A-0010); config-schema changes are breaking changes.

## [0.1.1] - UNRELEASED

### Changed

- Edition 2024 and `rust-version = "1.99"`; `cargo fix --edition` and the 2024
  rustfmt style. No behavior or config-schema change (CLOACINA-T-0954).

## [0.1.0] - UNRELEASED

### Added

- `kafka_source` — native (non-WASM) stream accumulator: a synchronous rdkafka
  `BaseConsumer` poll loop on fidius's dedicated pump thread, emitting message
  payloads to the accumulator boundary channel. Config: `broker`, `topic`,
  `group`. Ships rdkafka inside the provider — cloacina core carries no Kafka
  dependency (CLOACI-I-0139).
