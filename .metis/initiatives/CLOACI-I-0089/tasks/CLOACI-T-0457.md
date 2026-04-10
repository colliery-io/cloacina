---
id: clean-up-duplicate-error-variants
level: task
title: "Clean up duplicate error variants and unused dispatch macros (LEG-04, LEG-03)"
short_code: "CLOACI-T-0457"
created_at: 2026-04-09T13:51:27.788094+00:00
updated_at: 2026-04-09T14:01:13.794661+00:00
parent: CLOACI-I-0089
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0089
---

# Clean up duplicate error variants and unused dispatch macros (LEG-04, LEG-03)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0089]]

## Objective

Duplicate error variants force contributors to choose between near-identical options and make pattern matching verbose. Three dispatch macros with overlapping names cause confusion. Clean up both.

**Effort**: 2-3 hours

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `MissingDependencyOld` removed from `ValidationError`, its 3 call sites migrated to `MissingDependency`
- [ ] Either `CyclicDependency` or `CircularDependency` removed (pick one), all references updated
- [ ] `WorkflowError::CyclicDependency` uses the same variant name as `ValidationError`
- [ ] `backend_dispatch!` macro removed (inline its single use site)
- [ ] `connection_match!` macro removed (inline its single use site)
- [ ] `dispatch_backend!` (132 uses) remains as the canonical dispatch macro
- [ ] All tests pass with no broken match arms

## Implementation Notes

### Technical Approach

1. Search for `MissingDependencyOld` — find 3 call sites, change to `MissingDependency`. Remove the variant.
2. Search for `CircularDependency` and `CyclicDependency` — pick `CyclicDependency` (matches the graph theory term). Update all references. Remove the other.
3. Find the single use of `backend_dispatch!` — inline the macro body at the call site. Delete the macro definition.
4. Find the single use of `connection_match!` — inline similarly. Delete.
5. Run tests after each change to catch broken match arms early.

Mechanical refactoring — use find-and-replace.

### Dependencies
None.

## Status Updates

- **2026-04-09**: Removed `MissingDependencyOld` variant, migrated 2 call sites in task.rs to `MissingDependency` (field renamed task_id->task). Removed `CircularDependency` variant, migrated 2 call sites in task.rs to `CyclicDependency` (wrapped String in vec![]). Removed `backend_dispatch!` and `connection_match!` macro definitions (zero usage). `dispatch_backend!` (132 uses) retained as canonical. Compiles clean on both backends.
