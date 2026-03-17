---
id: opentelemetry-otlp-tracing
level: task
title: "OpenTelemetry OTLP tracing integration with configurable exporter"
short_code: "CLOACI-T-0209"
created_at: 2026-03-17T01:52:25.443488+00:00
updated_at: 2026-03-17T02:06:52.825037+00:00
parent: CLOACI-I-0035
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0035
---

# OpenTelemetry OTLP tracing integration with configurable exporter

## Parent Initiative

[[CLOACI-I-0035]]

## Objective

Add an `ObservabilitySection` to `ServerConfig` for OpenTelemetry OTLP tracing configuration, and implement a stub/placeholder in the `observability` module that logs when an OTLP endpoint is configured. The actual OTel exporter integration is deferred due to the frequent breaking API changes in the OpenTelemetry Rust crate ecosystem; the config section and stub provide the foundation for future integration without risking version conflicts.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ObservabilitySection` struct added to `config.rs` with `otlp_endpoint` (String, empty = disabled) and `otlp_service_name` (String, default "cloacina")
- [ ] `pub observability: ObservabilitySection` field added to `ServerConfig` with `Default` impl
- [ ] Environment variable overrides: `CLOACINA_OTLP_ENDPOINT`, `CLOACINA_OTLP_SERVICE_NAME`
- [ ] `observability.rs` has `init_opentelemetry(endpoint, service_name)` stub that logs the configured endpoint
- [ ] `serve.rs::run()` calls the OTel init stub when endpoint is non-empty
- [ ] TOML config file parsing accepts `[observability]` section
- [ ] `cargo check -p cloacinactl` passes cleanly (no OTel crate dependencies added unless they compile cleanly)

## Implementation Notes

### Technical Approach

1. Add `ObservabilitySection` to `config.rs` with serde defaults
2. Add `observability` field to `ServerConfig` and its `Default` impl
3. Add env var overrides in `apply_env_overrides()`
4. Add `init_opentelemetry()` to `observability.rs` as a stub that uses `tracing::info!` to log the endpoint
5. Call from `serve.rs::run()` after config is loaded
6. Do NOT add `opentelemetry`/`opentelemetry_sdk`/`opentelemetry-otlp`/`tracing-opentelemetry` dependencies -- these are deferred to a follow-up task when versions stabilize

### Dependencies

- CLOACI-T-0208 (Prometheus metrics endpoint) -- the `observability.rs` module is created there

### Risk Considerations

- OpenTelemetry Rust crate versions are notoriously unstable with breaking API changes between minor versions. The stub approach avoids this entirely while providing the config plumbing.

## Status Updates

*To be added during implementation*
