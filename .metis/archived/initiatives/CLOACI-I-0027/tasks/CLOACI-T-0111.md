---
id: scaffold-cloacina-testing-crate
level: task
title: "Scaffold cloacina-testing crate and workspace integration"
short_code: "CLOACI-T-0111"
created_at: 2026-03-14T02:59:43.355776+00:00
updated_at: 2026-03-14T03:17:22.316918+00:00
parent: CLOACI-I-0027
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0027
---

# Scaffold cloacina-testing crate and workspace integration

## Parent Initiative

[[CLOACI-I-0027]]

## Objective

Create the `cloacina-testing` crate directory structure, Cargo.toml with correct dependencies, add it to the workspace, and establish the module layout. This is the foundation all other tasks build on.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `crates/cloacina-testing/` directory exists with proper structure
- [ ] `Cargo.toml` declares dependencies on `cloacina-workflow` (required) and `cloacina` (for graph/topo sort)
- [ ] Feature flag `continuous` defined (empty for now, gated modules added in T-0114)
- [ ] Added to root `Cargo.toml` workspace members
- [ ] `src/lib.rs` exists with module declarations: `runner`, `result`, `assertions`, plus feature-gated `boundary` and `mock`
- [ ] `cargo check -p cloacina-testing` passes
- [ ] `angreal check all-crates` passes with the new crate included

## Implementation Notes

### Crate Structure
```
crates/cloacina-testing/
  Cargo.toml
  src/
    lib.rs            # Re-exports, module declarations
    runner.rs         # TestRunner (placeholder)
    result.rs         # TestResult, TaskOutcome (placeholder)
    assertions.rs     # Assertion helpers (placeholder)
    boundary.rs       # BoundaryEmitter (feature-gated, placeholder)
    mock.rs           # MockDataConnection (feature-gated, placeholder)
```

### Dependencies
- `cloacina-workflow` — for `Task`, `Context<T>`, `TaskError`, `TaskNamespace`
- `cloacina` — for `DependencyGraph` topological sort (from `cloacina::workflow::graph`)
- `indexmap` — for ordered task outcomes
- `async-trait` — for Task trait usage
- `serde_json` — for `Context<serde_json::Value>`

### Key Decisions
- Separate crate, not a feature flag on `cloacina`, so consumers add it as `[dev-dependencies]`
- Module files created with placeholder structs/types so downstream tasks can fill in implementations independently

## Status Updates

- Created `crates/cloacina-testing/` with full module structure
- Cargo.toml with dependencies: `cloacina-workflow`, `async-trait`, `serde_json`, `indexmap`, `petgraph`, `thiserror`; optional `chrono` for `continuous` feature
- Decided against depending on `cloacina` crate (too heavy — pulls diesel, DB drivers); instead using `petgraph` directly for topological sort
- Added to workspace members in root Cargo.toml
- All modules created: `lib.rs`, `runner.rs`, `result.rs`, `assertions.rs`, `boundary.rs` (feature-gated), `mock.rs` (feature-gated)
- `cargo check -p cloacina-testing` passes clean (no warnings)
- `cargo check -p cloacina-testing --features continuous` passes clean
