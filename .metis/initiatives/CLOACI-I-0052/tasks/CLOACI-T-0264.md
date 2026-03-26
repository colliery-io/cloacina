---
id: test-coverage-security-dispatcher
level: task
title: "Test coverage — security, dispatcher, and database modules"
short_code: "CLOACI-T-0264"
created_at: 2026-03-26T14:13:17.009311+00:00
updated_at: 2026-03-26T14:13:17.009311+00:00
parent: CLOACI-I-0052
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0052
---

# Test coverage — security, dispatcher, and database modules

## Parent Initiative

[[CLOACI-I-0052]]

## Objective

Expand unit and integration test coverage for under-tested modules. Focus on security (db_key_manager, package_signer, verification), dispatcher edge cases, database DAL methods, and retry/recovery logic. Use `cloacina-testing` for no-DB workflow tests where applicable.

## Acceptance Criteria

- [ ] Cyclomatic complexity metrics generated for target modules (security, dispatcher, database, retry)
- [ ] Hotspots identified within those modules — functions with complexity > 10 flagged for refactoring
- [ ] Refactor high-complexity functions where it improves testability (not cosmetic cleanup)
- [ ] New tests for `db_key_manager` — key rotation, expiry, concurrent access
- [ ] New tests for `package_signer` — edge cases (empty data, large payloads, corrupted signatures)
- [ ] New tests for `verification` — trust ACL, signature chain validation
- [ ] New tests for dispatcher — routing edge cases, executor registration/deregistration
- [ ] New tests for database DAL — CRUD operations, constraint violations, concurrent writes
- [ ] New tests for retry/recovery — backoff timing, max retries, recovery after transient failures
- [ ] `angreal cloacina all` passes with no regressions
- [ ] Net increase of 30+ new test cases

## Implementation Notes

### Technical Approach
1. Run cyclomatic complexity analysis (e.g. `cargo install cargo-cyclocomplexity` or `rust-code-analysis`) on target modules (security, dispatcher, database, retry)
2. Generate baseline complexity report — identify functions with complexity > 10 as hotspots
3. Refactor hotspot functions for testability where warranted (extract logic, reduce branching, split large functions)
4. Audit current test coverage per module (grep for `#[cfg(test)]` and `#[tokio::test]`)
5. Write tests prioritizing: security > dispatcher > database > retry
6. Use `cloacina-testing::TestRunner` for workflow-level tests that don't need a DB

### Prior Art
Reference: commit `5c4387a` on `archive/cloacina-server-week1` (test coverage improvements)
Reference: commit `88695f3` (test coverage and code quality)

### Dependencies
None — can start immediately.

## Status Updates

*To be added during implementation*
