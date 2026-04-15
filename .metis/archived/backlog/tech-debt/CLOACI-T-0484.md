---
id: extend-runtime-to-cover-all-global
level: task
title: "Extend Runtime to cover all global registries"
short_code: "CLOACI-T-0484"
created_at: 2026-04-11T14:49:52.454121+00:00
updated_at: 2026-04-13T01:52:08.289621+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Extend Runtime to cover all global registries

## Objective

Extend `Runtime` to encompass computation graph, stream backend, and Python registries. Make `#[ctor]` auto-registration opt-in to eliminate remaining `#[serial]` test requirements.

## Review Finding References

EVO-002, EVO-008 (REC-014). Pragmatic first step toward eliminating global mutable state without crate splitting.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (nice to have)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `Runtime` struct includes computation graph and stream backend registry fields
- [ ] `ReactiveScheduler` uses `Runtime` instead of global statics
- [ ] `#[ctor]` registration is opt-in (tests can bypass)
- [ ] Reduction in `#[serial]` annotations across test files

## Implementation Notes

### Key Files
- `crates/cloacina/src/runtime.rs`
- `crates/cloacina/src/computation_graph/` (registry references)
- `crates/cloacina/src/reactive/` (scheduler references)

### Dependencies
None. Independent.

## Status Updates

*To be added during implementation*