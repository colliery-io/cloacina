---
id: foundation-types-computationboundar
level: task
title: "Foundation types: ComputationBoundary, BoundaryKind, and coalescing"
short_code: "CLOACI-T-0117"
created_at: 2026-03-15T11:46:24.847500+00:00
updated_at: 2026-03-15T11:57:20.383387+00:00
parent: CLOACI-I-0023
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0023
---

# Foundation types: ComputationBoundary, BoundaryKind, and coalescing

## Parent Initiative

[[CLOACI-I-0023]]

## Objective

Implement `ComputationBoundary`, `BoundaryKind` enum, and coalescing logic as specified in CLOACI-S-0002. These are the foundational data types that describe what slice of data a signal covers. All other continuous scheduling components depend on these types.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ComputationBoundary` struct with `kind: BoundaryKind`, `metadata: Option<Value>`, `emitted_at: DateTime<Utc>`
- [ ] `BoundaryKind` enum with variants: `TimeRange`, `OffsetRange`, `Cursor`, `FullState`, `Custom`
- [ ] `coalesce()` method implementing per-variant merge rules (TimeRange/OffsetRange: min/max, Cursor/FullState: latest-wins, Custom: unmerged)
- [ ] `Serialize`/`Deserialize` derives for all types (carried through context)
- [ ] `CustomBoundarySchema` struct and `register_custom_boundary()` registration function
- [ ] Schema validation on Custom boundary creation
- [ ] Unit tests: coalescing for each variant, custom schema validation, edge cases (empty merge, single boundary)
- [ ] Types live in a new `continuous` module in `cloacina` crate

## Implementation Notes

### Technical Approach
- New module: `crates/cloacina/src/continuous/mod.rs` with sub-modules `boundary.rs`, `types.rs`
- Coalescing is a free function or method on `Vec<ComputationBoundary>` — not on the boundary itself (accumulator calls it)
- Custom schema validation uses `jsonschema` crate or manual validation against `serde_json::Value` schema
- `emitted_at` is stamped by the detector at emission time

### Key Design Constraints (from S-0002)
- Boundary is advisory data, not enforced contract
- Struct + enum, NOT a trait
- Custom requires registered JSON Schema — unregistered kinds rejected

### Dependencies
- No task dependencies — this is the first task in the chain

## Status Updates

- Created `crates/cloacina/src/continuous/mod.rs` and `boundary.rs`
- Added `pub mod continuous;` to lib.rs
- Implemented: `ComputationBoundary`, `BoundaryKind` (5 variants), `BufferedBoundary`, `CustomBoundarySchema`
- Coalescing: `coalesce()` function with TimeRange/OffsetRange min/max, Cursor/FullState latest-wins, Custom latest-wins
- Custom schema: `register_custom_boundary()`, `validate_custom_boundary()` with basic JSON schema validation (type, required, properties)
- All types have Serialize/Deserialize derives; BoundaryKind uses `#[serde(tag = "type")]` for tagged serialization
- 13 passing tests covering: coalescing (empty, single, time ranges, offsets, cursors, fullstate), serialization roundtrip, tagged serialization, buffered boundary lag, custom schema (valid, missing field, unregistered, wrong type)
- `cargo check -p cloacina` passes clean
