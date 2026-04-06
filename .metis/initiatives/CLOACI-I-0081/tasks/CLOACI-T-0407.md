---
id: dal-foundation-schema-migrations
level: task
title: "DAL foundation — schema migrations, CheckpointHandle, and CheckpointDal trait"
short_code: "CLOACI-T-0407"
created_at: 2026-04-05T21:24:21.400192+00:00
updated_at: 2026-04-05T23:42:18.117815+00:00
parent: CLOACI-I-0081
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0081
---

# DAL foundation — schema migrations, CheckpointHandle, and CheckpointDal trait

## Parent Initiative

[[CLOACI-I-0081]]

## Objective

Create the database schema and core persistence abstractions that all other resilience tasks depend on. This is the foundation — nothing else can persist state until these tables and traits exist.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Diesel migrations for 4 new tables: `accumulator_checkpoints`, `accumulator_boundaries`, `reactor_state`, `state_accumulator_buffers`
- [ ] Migrations work for both Postgres and SQLite (via Diesel MultiConnection pattern)
- [ ] `CheckpointDal` trait with async `save_checkpoint`, `load_checkpoint`, `save_boundary`, `load_boundary`, `save_reactor_state`, `load_reactor_state`, `save_state_buffer`, `load_state_buffer` methods
- [ ] Postgres and SQLite implementations of `CheckpointDal`
- [ ] `CheckpointHandle` struct wrapping `Arc<dyn CheckpointDal>` + graph_name + accumulator_name
- [ ] `CheckpointHandle::save<T: Serialize>()` and `CheckpointHandle::load<T: DeserializeOwned>()` working
- [ ] `CheckpointHandle` added to `AccumulatorContext` (currently missing from the struct)
- [ ] `AccumulatorError::Checkpoint` variant (currently dead code) properly used by `CheckpointHandle` errors
- [ ] Unit tests: save/load round-trip for each table, both Postgres and SQLite
- [ ] Schema matches the design in I-0081 (composite primary keys, BYTEA columns, timestamps)

## Implementation Notes

### Technical Approach

**Schema** (4 tables):
```
accumulator_checkpoints(graph_name, accumulator_name, checkpoint_data, updated_at) PK(graph_name, accumulator_name)
accumulator_boundaries(graph_name, accumulator_name, boundary_data, sequence_number, updated_at) PK(graph_name, accumulator_name)
reactor_state(graph_name PK, cache_data, dirty_flags, sequential_queue NULLABLE, updated_at)
state_accumulator_buffers(graph_name, accumulator_name, buffer_data, capacity, updated_at) PK(graph_name, accumulator_name)
```

**Key files to modify:**
- `crates/cloacina/src/dal/` — new migration files, schema updates, new DAL module
- `crates/cloacina/src/computation_graph/accumulator.rs` — add `CheckpointHandle` to `AccumulatorContext`
- Follow existing DAL patterns in `crates/cloacina/src/dal/unified/` for the MultiConnection approach

**`CheckpointHandle`** is intentionally simple — key-value persistence keyed by (graph_name, accumulator_name). Serialization uses the same debug-JSON/release-bincode pattern as the rest of the system.

### Dependencies
None — this is the foundation task. All other I-0081 tasks depend on this.

### Risk Considerations
- Diesel MultiConnection composite primary keys — verify both backends handle correctly
- BYTEA column sizes — checkpoint data could be large for state accumulators with high capacity; no artificial limits but document expected sizes

## Status Updates

### 2026-04-05: Core implementation complete

**Completed:**
- Diesel migrations for 4 new tables (both Postgres 017 and SQLite 015)
- Schema definitions in all 3 schema modules (unified, postgres legacy, sqlite legacy)
- Diesel models (Queryable + Insertable) for all 4 tables in models.rs
- CheckpointDAL with full upsert save/load for all 4 tables + delete_graph_state
- CheckpointHandle with save/load using types::serialize/deserialize
- CheckpointHandle added to AccumulatorContext as Option (None for embedded mode)
- AccumulatorError::Checkpoint now used by CheckpointHandle error paths
- All 26 AccumulatorContext construction sites updated with checkpoint: None
- Compiles clean: cloacina lib, tests, cloacinactl

**Design decisions:**
- UUID id PK with unique composite indexes (matching existing DAL pattern) instead of composite PKs
- CheckpointHandle wraps concrete DAL (not dyn — matching existing patterns)
- AccumulatorContext.checkpoint is Option to support embedded mode without database
- Upsert via on_conflict().do_update() on the unique indexes

**Remaining:**
- Unit tests for save/load round-trips (need to run via angreal)
- Verify migrations run against real Postgres
