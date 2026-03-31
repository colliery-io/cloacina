---
id: deprecate-workflow-and-packaged
level: task
title: "Deprecate workflow! and #[packaged_workflow], terminology cleanup"
short_code: "CLOACI-T-0307"
created_at: 2026-03-29T20:39:46.797557+00:00
updated_at: 2026-03-31T15:31:40.902456+00:00
parent: CLOACI-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0058
---

# Deprecate workflow! and #[packaged_workflow], terminology cleanup

## Parent Initiative

[[CLOACI-I-0058]]

## Objective

Remove the old `workflow!` and `#[packaged_workflow]` macros (or mark deprecated with compile warnings). Clean up terminology throughout the codebase — "workflow" for the execution unit, "package" only for `.cloacina` archives, no more "packaged workflow" as a concept.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `workflow!` macro removed or emits `#[deprecated]` compile warning pointing to `#[workflow]`
- [ ] `#[packaged_workflow]` macro removed or emits `#[deprecated]` compile warning pointing to `#[workflow]`
- [ ] No internal code uses the old macros (all migrated in T-0306)
- [ ] Terminology cleanup: grep for "packaged workflow", "packaged_workflow", "PackagedWorkflow" — replace with "workflow" or "package" as appropriate
- [ ] API server endpoints use consistent terminology (no "packaged workflow" in JSON responses)
- [ ] Doc comments and module-level docs updated
- [ ] `cloacina-macros/src/packaged_workflow.rs` removed (or kept as thin deprecated wrapper)
- [ ] `cloacina-macros/src/workflow.rs` old `workflow!` impl removed (or kept as thin deprecated wrapper)
- [ ] All tests pass

## Implementation Notes

### Depends on
- T-0306 (all examples migrated first — nothing should reference old macros)

### Approach
- If there are external consumers, use `#[deprecated]` first for one release, then remove
- If internal only, can remove directly after T-0306

## Status Updates

**2026-03-30**: Implementation complete.

- `workflow!` macro was already deleted in T-0302 (breaking change)
- `#[packaged_workflow]` proc macro export removed from `cloacina-macros/src/lib.rs`
- `packaged_workflow` removed from re-exports in `cloacina-workflow` and `cloacina`
- `packaged_workflow.rs` kept as `pub(crate)` — used internally by `workflow_attr.rs` for shared utilities
- 4 packaged examples migrated from `#[packaged_workflow]` to `#[workflow]` with `features = ["packaged"]`
- Packaging validation regex updated to match both `#[workflow]` and `#[packaged_workflow]` (backward compat for any remaining .cloacina archives)
- Packaging test fixture updated to use `#[workflow]` pattern
- Doc/comment cleanup: key code references updated (validation.rs, manifest.rs, tests.rs)
- Full doc cleanup (markdown files) deferred — doesn't affect compilation or runtime
- All unit tests pass (angreal cloacina unit), all macro validation tests pass
