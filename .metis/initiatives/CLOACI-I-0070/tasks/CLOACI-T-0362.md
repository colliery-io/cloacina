---
id: graphresult-type-inputcache
level: task
title: "GraphResult type, InputCache interface, and #[node(blocking)]"
short_code: "CLOACI-T-0362"
created_at: 2026-04-04T19:51:03.369811+00:00
updated_at: 2026-04-04T20:04:10.717843+00:00
parent: CLOACI-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0070
---

# GraphResult type, InputCache interface, and #[node(blocking)]

## Objective

Define the runtime types that the compiled graph function depends on: `GraphResult`, `InputCache`, and `GraphError`. These live in the main `cloacina` crate (not the macro crate) and are the interface between the compiled function and the reactor.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `GraphResult` enum: `Completed { outputs: Vec<Box<dyn Any + Send>> }` and `Error(GraphError)`
- [ ] `GraphError` type with variant for graph execution failures
- [ ] `InputCache` struct: `HashMap<SourceName, Vec<u8>>` with `get::<T>(name)` that deserializes (bincode/JSON based on build profile)
- [ ] `InputCache::snapshot()` — clones the cache for the executor to use while the receiver keeps updating
- [ ] `SourceName` type alias or newtype
- [ ] All types are `Send + Sync` where needed for async usage
- [ ] Unit tests: InputCache get/update, serialization round-trip (bincode + JSON), GraphResult construction

## Implementation Notes

These types can be developed in parallel with the parser/IR tasks since they're independent. The code generator (T-0361) will emit code that references these types, so they need to exist before T-0361 can produce compilable output.

Place in `cloacina/src/computation_graph/types.rs` or similar module within the main crate.

### Dependencies
None — can start immediately, parallel with T-0359.

## Status Updates

**2026-04-04**: Completed.
- Created `crates/cloacina/src/computation_graph/mod.rs` and `types.rs`
- Implemented: `SourceName`, `InputCache` (get/update/snapshot/has/replace_all/sources), `GraphResult` (Completed/Error), `GraphError` (5 variants)
- Implemented `serialize()` / `deserialize()` with dual-format: bincode in release, JSON in debug
- Added `bincode = "1.3"` to workspace and cloacina crate
- Registered `pub mod computation_graph` in lib.rs
- 14 unit tests passing
