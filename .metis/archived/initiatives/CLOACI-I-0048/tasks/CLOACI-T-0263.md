---
id: create-cloacina-testing-crate
level: task
title: "Create cloacina-testing crate"
short_code: "CLOACI-T-0263"
created_at: 2026-03-26T05:49:17.619004+00:00
updated_at: 2026-03-26T12:59:54.110044+00:00
parent: CLOACI-I-0048
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0048
---

# Create cloacina-testing crate

## Parent Initiative

[[CLOACI-I-0048]]

## Objective

Create `crates/cloacina-testing` providing `TestRunner`, in-memory DAL, and test DB helpers so workflow logic can be unit-tested without a live database. Port from the proven implementation on the archive branch.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `crates/cloacina-testing` crate exists in workspace
- [ ] `TestRunner` runs tasks in dependency order in-process
- [ ] In-memory DAL and DB stubs provided (no database crates as dependencies)
- [ ] `cloacina-testing` depends only on `cloacina-workflow` (not database crates)
- [ ] Optional `continuous` feature flag gated behind `chrono` for time-dependent helpers
- [ ] `angreal cloacina all` passes

## Implementation Notes

### Technical Approach
1. Port `cloacina-testing` from archive branch (commit `995f8a3`)
2. Key components:
   - `TestRunner` — lightweight in-process executor, runs tasks in dependency order
   - In-memory DAL and DB stubs
   - `continuous` feature flag gated behind `chrono`
3. Add to workspace `members` in root `Cargo.toml`
4. Verify no database crate dependencies leak in

### Prior Art
Reference: commit `995f8a3` on `archive/main-pre-reset`

### Dependencies
CLOACI-T-0262 (CLI rename) should be completed first to avoid merge conflicts.

## Status Updates

### 2026-03-26 — Complete
- Ported all 6 source files from `archive/main-pre-reset` (commit `995f8a3`)
- Components: `TestRunner`, `TestResult`/`TaskOutcome`, assertion helpers, `BoundaryEmitter`, `MockDataConnection`
- Added to workspace members in root `Cargo.toml`
- Depends only on `cloacina-workflow` + lightweight crates (indexmap, petgraph, thiserror, async-trait, serde_json)
- No database crate dependencies (verified via `cargo tree`)
- `continuous` feature flag kept with TODO comments to remove (continuous scheduling is always on)
- 18 crate-specific tests pass; `angreal cloacina all` passes (530 total tests)
