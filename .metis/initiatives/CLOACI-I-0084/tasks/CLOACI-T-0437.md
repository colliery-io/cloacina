---
id: package-metadata-for-stream
level: task
title: "Package metadata for stream accumulators and reconciler wiring"
short_code: "CLOACI-T-0437"
created_at: 2026-04-07T18:44:57.753537+00:00
updated_at: 2026-04-07T18:44:57.753537+00:00
parent: CLOACI-I-0084
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0084
---

# Package metadata for stream accumulators and reconciler wiring

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0084]]

## Objective

Extend `package.toml` metadata to declare stream-backed accumulators so the reconciler can create them with the correct backend when loading CG packages. Currently `AccumulatorFactory` in `packaging_bridge.rs` only creates passthrough accumulators.

## Acceptance Criteria

- [ ] `package.toml` supports `[[metadata.accumulators]]` with fields: `name`, `type` (stream/batch/passthrough/polling), `backend` (kafka/mock), `topic`, `group`
- [ ] Optional fields: `flush_interval`, `max_buffer`, `state_type`
- [ ] `CloacinaMetadata` struct in `cloacina-workflow-plugin` extended with accumulator declarations
- [ ] `GraphPackageMetadata` extended with per-accumulator backend config
- [ ] `build_declaration_from_ffi` reads accumulator config and creates appropriate `AccumulatorFactory` (not just passthrough)
- [ ] `StreamAccumulatorFactory` — new factory that creates stream-backed accumulators with configured backend
- [ ] `BatchAccumulatorFactory` — new factory for Kafka-backed batch accumulators
- [ ] Reconciler correctly wires stream backend when loading a package with Kafka accumulators
- [ ] Existing passthrough-only packages unchanged
- [ ] Example package.toml with Kafka-sourced accumulators

## Implementation Notes

### Key files
- `crates/cloacina-workflow-plugin/src/lib.rs` — metadata structs
- `crates/cloacina/src/computation_graph/packaging_bridge.rs` — AccumulatorFactory dispatch
- `crates/cloacina/src/registry/reconciler/loading.rs` — pass metadata through

### Dependencies
- T-0432 (KafkaStreamBackend)
- T-0434 (batch Kafka source)

## Status Updates **[REQUIRED]**

*To be added during implementation*
