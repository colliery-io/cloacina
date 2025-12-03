---
id: eliminate-feature-flags-from-tests
level: task
title: "Eliminate Feature Flags from Tests - Use Connection Strings for Backend Selection"
short_code: "CLOACI-T-0011"
created_at: 2025-11-30T18:31:16.252316+00:00
updated_at: 2025-12-03T23:35:58.920649+00:00
parent: CLOACI-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0001
---

# Eliminate Feature Flags from Tests - Use Connection Strings for Backend Selection

## Parent Initiative

[[CLOACI-I-0001]]

## Objective

Refactor all integration tests to be backend-agnostic, using connection strings for runtime backend selection instead of compile-time `#[cfg(feature = "...")]` attributes. This aligns with the initiative goal of runtime database backend selection and eliminates the need for feature flags in test code.

## Context

Currently, tests use `#[cfg(feature = "postgres")]` and `#[cfg(feature = "sqlite")]` attributes to conditionally compile backend-specific test code. This is inconsistent with the runtime backend selection design goal. Since we will NOT support "smaller binary without X backend" builds, both backends will always be compiled in, and tests should select their backend via connection string only.

### Current Problems

1. **Leaky abstractions**: Backend-specific types (`PgWorkflowPackage`, `SqliteWorkflowPackage`) and schemas (`schema::postgres::*`, `schema::sqlite::*`) leak into test code
2. **Feature flag complexity**: Tests require `#[cfg]` attributes creating maintenance burden
3. **Inconsistent with production**: Production code will use runtime selection, but tests use compile-time selection
4. **Duplicate test code**: Many tests are duplicated for each backend with only schema imports differing

### Files Requiring Refactoring

- `cloacina/tests/integration/context.rs` - Has separate `postgres_tests` and `sqlite_tests` modules
- `cloacina/tests/integration/scheduler/recovery.rs` - Uses cfg-gated schema imports
- `cloacina/tests/integration/registry_workflow_registry_tests.rs` - Has cfg-gated `create_test_storage` variants
- `cloacina/src/executor/thread_task_executor.rs` - Has cfg-gated `UniversalUuid` wrapping
- `cloacina/src/runner/default_runner.rs` - Has cfg-gated storage creation
- `cloacina/src/registry/workflow_registry.rs` - Has cfg-gated legacy module imports

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All `#[cfg(feature = "postgres")]` and `#[cfg(feature = "sqlite")]` removed from test files
- [ ] Tests use connection strings (via environment variable or fixture configuration) to select backend
- [ ] Test fixture provides backend-agnostic API for database operations
- [ ] DAL layer properly abstracts backend-specific types so they don't leak to tests
- [ ] All 164 tests pass when run with `angreal cloacina unit` (both backends)
- [ ] Single test implementation runs against both backends (no duplicate test functions)

## Implementation Notes

### Technical Approach

1. **Enhance DAL abstraction**: Ensure DAL operations return backend-agnostic types
2. **Update test fixture**: Modify `TestFixture` to provide connection based on environment/config
3. **Refactor tests**: Remove cfg attributes, use fixture's backend-agnostic API
4. **Add parameterized tests**: Consider test parameterization to run same test against both backends

### Key Design Principle

From user direction: "we will not be supporting 'i want a smaller binary without X backend' so we should only end up with 'tests use connection strings to select backend'"

### Dependencies

- Requires DAL layer to provide backend-agnostic abstractions
- May require updates to `UniversalUuid` and other wrapper types

## Status Updates

- **2025-11-30**: Task created based on architectural feedback during test suite fixes
