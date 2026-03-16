---
id: implement-boundaryemitter-and
level: task
title: "Implement BoundaryEmitter and MockDataConnection (feature-gated)"
short_code: "CLOACI-T-0114"
created_at: 2026-03-14T02:59:46.982222+00:00
updated_at: 2026-03-14T03:20:23.021329+00:00
parent: CLOACI-I-0027
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0027
---

# Implement BoundaryEmitter and MockDataConnection (feature-gated)

## Parent Initiative

[[CLOACI-I-0027]]

## Objective

Implement `BoundaryEmitter` and `MockDataConnection<T>` behind the `continuous` feature flag. These types enable testing of `#[continuous_task]` functions (once CLOACI-I-0023 lands) without requiring live external data systems.

**Note**: This task creates the types and their tests, but the actual `#[continuous_task]` trait/types they integrate with don't exist yet. The implementations should be designed against the spec in CLOACI-S-0001 and CLOACI-S-0002, with compile-time validation deferred until the continuous scheduling crates land.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `BoundaryEmitter` struct with builder pattern: `new()`, `emit(boundary)`, `emit_time_range(start, end)`, `emit_offset_range(start, end)`
- [ ] `BoundaryEmitter::into_context()` produces a `Context<serde_json::Value>` matching accumulator drain output format
- [ ] `MockDataConnection<T>` generic struct implementing a connection interface
- [ ] `MockDataConnection::new(handle, descriptor)` constructor
- [ ] Both types gated behind `#[cfg(feature = "continuous")]` in `boundary.rs` and `mock.rs`
- [ ] Unit tests for BoundaryEmitter context generation
- [ ] Unit tests for MockDataConnection handle retrieval
- [ ] `cargo check -p cloacina-testing --features continuous` passes

## Implementation Notes

### Technical Approach
- `BoundaryEmitter` stores `Vec<ComputationBoundary>` (or equivalent JSON representation if the type doesn't exist yet)
- `into_context()` serializes boundaries into the context format that the accumulator's `drain()` would produce
- `MockDataConnection<T: Clone + Send + Sync + 'static>` wraps a user-provided handle and returns it from `connect()`
- If `ComputationBoundary` and `DataConnection` traits don't exist yet, define local placeholder types with `#[cfg]` guards and TODO comments referencing CLOACI-I-0023

### Dependencies
- Depends on CLOACI-T-0111 (crate scaffold)
- Soft dependency on CLOACI-I-0023 (continuous scheduling) — design against specs, validate when types exist
- References: CLOACI-S-0001 (Continuous Reactive Scheduling), CLOACI-S-0002 (ComputationBoundary)

## Status Updates

- BoundaryEmitter and MockDataConnection implemented during T-0111 scaffold
- Added 7 unit tests: 4 for BoundaryEmitter (empty, time_range, offset_range, multiple), 3 for MockDataConnection (connect, descriptor, metadata)
- All tests pass with `--features continuous`
- Using placeholder types for ComputationBoundary (will swap to real types when I-0023 lands)
- `cargo check -p cloacina-testing --features continuous` passes clean
