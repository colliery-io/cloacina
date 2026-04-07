---
id: connection-variable-registry
level: task
title: "Connection/variable registry — CLOACINA_VAR_{NAME} env convention for external connections"
short_code: "CLOACI-T-0438"
created_at: 2026-04-07T22:20:44.213845+00:00
updated_at: 2026-04-07T22:20:44.213845+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Connection/variable registry — CLOACINA_VAR_{NAME} env convention for external connections

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Type-agnostic variable/connection registry using `CLOACINA_VAR_{NAME}` env var convention. Similar to Airflow's connection/variable system — external connections, secrets, and config values are referenced by name in package metadata and resolved from env vars at runtime.

## Design

### Convention
```
CLOACINA_VAR_{NAME}=value
```

Values are opaque strings. The runtime resolves them by name. Interpretation depends on context:
- Kafka broker URL: `CLOACINA_VAR_KAFKA_BROKER=localhost:9092`
- Database connection: `CLOACINA_VAR_ANALYTICS_DB=postgres://user:pass@host/db`
- API key: `CLOACINA_VAR_MARKET_DATA_KEY=abc123`
- Any config: `CLOACINA_VAR_MODEL_THRESHOLD=0.85`

### Usage in package metadata
```toml
[[metadata.accumulators]]
name = "orderbook"
type = "stream"
topic = "market.orderbook"
group = "cloacina-mm"
broker = "{{ KAFKA_BROKER }}"   # Resolved from CLOACINA_VAR_KAFKA_BROKER
```

### Usage in code
```rust
// Rust
let broker = cloacina::var("KAFKA_BROKER")?;

// Python
broker = cloaca.var("KAFKA_BROKER")
```

### Resolution order
1. `CLOACINA_VAR_{NAME}` env var
2. Server-level config file (future — `.cloacina/vars.toml` or similar)
3. Error if not found

### Interaction with current code
- `StreamBackendAccumulatorFactory` currently reads `KAFKA_BROKER_URL` env var directly
- After this task: reads `CLOACINA_VAR_KAFKA_BROKER` (or whatever name is in the accumulator config)
- Package metadata references variables by name, runtime resolves them
- Nodes can access variables for external API calls, credentials, config

## Acceptance Criteria

- [ ] `cloacina::var(name)` function resolves `CLOACINA_VAR_{NAME}` from env
- [ ] Python `cloaca.var(name)` equivalent
- [ ] Template syntax in package metadata (`{{ VAR_NAME }}`) resolved at load time
- [ ] Error reporting when a required variable is missing
- [ ] `cloacina::var_or(name, default)` for optional variables with defaults
- [ ] Documentation: env var naming convention, usage in metadata, usage in code

## Status Updates **[REQUIRED]**

*To be added during implementation*
