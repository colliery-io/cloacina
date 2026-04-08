---
id: fix-lossy-error-conversion-add
level: task
title: "Fix lossy error conversion — add Database/ConnectionPool variants to ContextError (COR-04, LEG-05)"
short_code: "CLOACI-T-0446"
created_at: 2026-04-08T23:30:11.069003+00:00
updated_at: 2026-04-08T23:41:17.747504+00:00
parent: CLOACI-I-0086
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0086
---

# Fix lossy error conversion — add Database/ConnectionPool variants to ContextError (COR-04, LEG-05)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0086]]

## Objective

The `From<ContextError> for TaskError` conversion maps database connectivity failures (`Database`, `ConnectionPool`) to `ContextError::KeyNotFound`, producing misleading error messages like "Key not found: Database error: connection refused." This breaks retry classification (infrastructure errors should be retryable, key-not-found should not) and confuses debugging.

**Effort**: 2-3 hours

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `cloacina_workflow::ContextError` has new variants: `Database(String)` and `ConnectionPool(String)`
- [ ] `From<ContextError> for TaskError` in `cloacina/src/error.rs` maps these variants correctly (not to `KeyNotFound`)
- [ ] A database connectivity error surfaces as `ContextError::Database(...)`, not `KeyNotFound`
- [ ] A pool exhaustion error surfaces as `ContextError::ConnectionPool(...)`, not `KeyNotFound`
- [ ] Existing tests pass — no match arms broken by the new variants
- [ ] Unit test verifies the conversion produces the correct variant

## Implementation Notes

### Technical Approach

1. In `crates/cloacina-workflow/src/error.rs`, add two new variants to `ContextError`:
   ```rust
   Database(String),
   ConnectionPool(String),
   ```
2. In `crates/cloacina/src/error.rs`, update the `From<ContextError> for TaskError` impl (lines ~354-378) to map:
   - `ContextError::Database(msg)` -> `TaskError::DatabaseError { ... }` (or a new appropriate variant)
   - `ContextError::ConnectionPool(msg)` -> same
3. Update any match arms on `ContextError` that use `_` wildcards to handle the new variants explicitly.

This is a cross-crate change (cloacina-workflow + cloacina) but small — two new enum variants + updated match arms.

### Dependencies
Independent of T-0444 and T-0445. Can run in parallel.

## Status Updates

- **2026-04-08**: Added `Database(String)` and `ConnectionPool(String)` variants to `cloacina_workflow::ContextError`. Updated `From<ContextError> for TaskError` in core crate to map `Database` -> `workflow::Database`, `ConnectionPool` -> `workflow::ConnectionPool` instead of `KeyNotFound`. Updated reverse conversion (`From<workflow::ContextError> for ContextError`) to handle new variants. No wildcard matches broken. Compiles clean on both backends.
