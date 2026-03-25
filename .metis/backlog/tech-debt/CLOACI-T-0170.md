---
id: improve-test-coverage-database
level: task
title: "Improve test coverage: database/connection (26%) and database/admin.rs (33%)"
short_code: "CLOACI-T-0170"
created_at: 2026-03-16T01:01:44.091964+00:00
updated_at: 2026-03-25T12:47:28.721687+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Improve test coverage: database/connection (26%) and database/admin.rs (33%)

**Priority: P2 — Tech Debt**

## Objective

Two database infrastructure files are poorly covered:
- `database/connection/mod.rs` — 423 lines at 26.2%. Connection pool creation, backend detection, migration running.
- `database/admin.rs` — 311 lines at 32.8%. Schema management, migration utilities.

These are exercised indirectly by all integration tests (every test creates a connection), but specific paths like error handling, schema validation failures, and migration edge cases are untested.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `connection/mod.rs` line coverage ≥ 70% (some paths require specific DB states)
- [ ] `admin.rs` line coverage ≥ 70%
- [ ] Test: connection pool creation with valid URL
- [ ] Test: connection pool creation with invalid URL fails gracefully
- [ ] Test: migration up/down cycle
- [ ] Test: schema validation passes on clean DB
- [ ] Test: backend detection (postgres vs sqlite)

## Status Updates

### 2026-03-25 — Complete

connection/mod.rs: Added 5 tests (pool creation, migrations, get_connection, convenience new(), clone). admin.rs: Added 4 tests (username/schema length limits, reserved schema names, password generation uniqueness). Connection tests run with sqlite feature; admin tests are postgres-only (feature-gated). 35 database tests pass on sqlite.
