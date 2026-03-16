---
id: continuous-scheduling-production
level: initiative
title: "Continuous Scheduling Production Hardening"
short_code: "CLOACI-I-0025"
created_at: 2026-03-13T02:44:40.269357+00:00
updated_at: 2026-03-16T01:08:22.408438+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: continuous-scheduling-production
---

# Continuous Scheduling Production Hardening

## Context

CLOACI-I-0023 and CLOACI-I-0024 deliver continuous scheduling with the full reactive feedback loop. This initiative hardens the system for production workloads: crash recovery via accumulator persistence, richer out-of-the-box components, custom boundary validation, and operational tooling.

**Depends on**: CLOACI-I-0024 (watermarks, late arrival, derived data sources)
**Specification**: CLOACI-S-0002 (partial), S-0005 (partial)

## Goals & Non-Goals

**Goals:**
- Implement accumulator persist-on-drain (hybrid persistence model from S-0005)
- Implement `PersistedAccumulatorState` schema, DAL, and migrations
- Implement watermark resume on restart — load consumer watermarks from DB
- Implement startup warnings for orphaned persisted state (edges no longer in graph)
- Implement graph prune-state admin command (REST API endpoint + CLI)
- Implement custom boundary schema registration and JSON Schema validation (S-0002)
- Implement `Any`/`All` composition for `TriggerPolicy` (S-0005)
- Implement additional `TriggerPolicy` presets: `WallClockDebounce`, `BoundaryCount`
- Implement framework-provided `DataConnection` impls: `KafkaConnection`, `S3Connection`
- Expose `AccumulatorMetrics` via observability hooks

**Non-Goals:**
- Python/Cloaca support (CLOACI-I-0026)
- Graph hot-reload (decided against — see S-0001 decisions log)
- Active backpressure signaling (observable only — see S-0008 resolved questions)
- Circuit breakers or advanced error handling (deferred — existing retry policy applies)

## Detailed Design

### Accumulator Persistence (S-0005)

Hybrid model: in-memory during operation, persist on drain.

```rust
struct PersistedAccumulatorState {
    edge_id: String,
    consumer_watermark: Option<ComputationBoundary>,
    last_drain_at: DateTime,
    drain_metadata: serde_json::Value,
}
```

**On drain**: After `drain()` produces the context fragment, persist consumer watermark, drain timestamp, and coalescing metadata to DB via DAL.

**On restart**: Load `PersistedAccumulatorState` for all known edge IDs. Initialize accumulators with persisted consumer watermarks. Detectors re-poll from their own persisted `__last_known_state`. Boundaries between last drain and crash are re-detected naturally — coalescing makes this idempotent.

**Schema**: New `accumulator_state` table in the continuous scheduling schema (Postgres only).

### Orphaned State Management

**Startup detection**: On graph assembly, compare persisted edge IDs against current graph edges. Log warnings for any persisted state with no matching edge.

**Prune command**: Administrative action to delete orphaned persisted state.
- REST API: `DELETE /api/v1/continuous/state/orphaned`
- CLI: `cloacina continuous prune-state [--dry-run]`

No auto-deletion — operators control when orphaned state is removed.

### Custom Boundary Schema Validation (S-0002)

```rust
struct CustomBoundarySchema {
    kind: String,
    schema: serde_json::Value,  // JSON Schema
}
```

- Register custom boundary types with JSON Schema at startup
- Validate `Custom` boundary payloads at signal ingestion (reject invalid)
- Re-validate at accumulator `drain()` (catch corruption from custom merge logic)
- Reject unregistered custom boundary kinds at signal ingestion

### TriggerPolicy Composition (S-0005)

```rust
struct Any(Vec<Box<dyn TriggerPolicy>>);  // fire when ANY sub-policy says fire
struct All(Vec<Box<dyn TriggerPolicy>>);  // fire when ALL sub-policies say fire
```

Both implement `TriggerPolicy` and nest arbitrarily. Enables patterns like "every 5 minutes OR 20 boundaries" or "at least 1000 rows AND at least 1 minute since last drain."

### Additional TriggerPolicy Presets

| Policy | Fires when |
|---|---|
| `WallClockDebounce { duration }` | No new boundary received for duration (silence = burst is over) |
| `BoundaryCount { count }` | N boundaries buffered |

### Framework DataConnection Implementations

- `KafkaConnection` { brokers, topic, partition, consumer_group } — descriptor: `kafka`, location: `brokers/topic`
- `S3Connection` { bucket, prefix, region } — descriptor: `s3`, location: `s3://bucket/prefix`

Each implements `DataConnection` with appropriate `descriptor()`, `system_metadata()`, and `connect()`.

### Observability

Expose `AccumulatorMetrics` per edge:
- `buffered_count` — boundaries waiting in buffer
- `oldest_boundary_emitted_at` / `newest_boundary_emitted_at` — buffer age range
- `max_lag` — max(received_at - emitted_at) across buffer
- Drain frequency, coalescing ratio (boundaries received vs drains executed)

Integration with existing metrics infrastructure (Prometheus-compatible if server initiative lands).

## Alternatives Considered

- **Persist on every receive**: Rejected — defeats the purpose of the in-memory hot path. Drain is the natural commit point.
- **Auto-delete orphaned state**: Rejected — removal might be temporary (branch deploy, rolling update). Operator should decide.
- **Custom boundary merge as trait**: Rejected — merge logic belongs on the accumulator (S-0002 design decision). Custom boundaries delegate merge to the accumulator, not to the boundary type.

## Implementation Plan

### Phase 1: Accumulator Persistence
- [ ] `PersistedAccumulatorState` model and schema migration
- [ ] DAL methods: save_accumulator_state, load_accumulator_state, delete_orphaned_state
- [ ] Persist-on-drain integration in accumulator `drain()` path
- [ ] Watermark resume on scheduler startup
- [ ] Integration tests: crash → restart → resume from persisted watermarks

### Phase 2: Orphaned State Management
- [ ] Startup orphan detection and warning log
- [ ] Prune-state admin endpoint (REST API)
- [ ] Prune-state CLI command with `--dry-run`
- [ ] Tests for orphan detection and cleanup

### Phase 3: Custom Boundary Validation
- [ ] `CustomBoundarySchema` registration API
- [ ] JSON Schema validation at signal ingestion
- [ ] JSON Schema re-validation at `drain()`
- [ ] Rejection of unregistered custom boundary kinds
- [ ] Unit tests for valid/invalid custom boundary payloads

### Phase 4: Policy Composition & Presets
- [ ] `Any` and `All` TriggerPolicy combinators
- [ ] `WallClockDebounce` implementation
- [ ] `BoundaryCount` implementation
- [ ] Unit tests for composition and nesting

### Phase 5: DataConnection Impls & Observability
- [ ] `KafkaConnection` implementation
- [ ] `S3Connection` implementation
- [ ] `AccumulatorMetrics` observability integration
- [ ] Integration tests with Kafka and S3 (or mocked equivalents)
