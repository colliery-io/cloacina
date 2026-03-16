---
id: improve-test-coverage-security-db
level: task
title: "Improve test coverage: security/db_key_manager.rs (6.8% → 80%+)"
short_code: "CLOACI-T-0166"
created_at: 2026-03-16T01:01:39.245273+00:00
updated_at: 2026-03-16T01:01:39.245273+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Improve test coverage: security/db_key_manager.rs (6.8% → 80%+)

**Priority: P2 — Tech Debt**

## Objective

`security/db_key_manager.rs` has 1,052 lines at 6.8% line coverage. This is the largest untested file in the codebase. It manages cryptographic key storage and retrieval via the database. Most paths are only exercised indirectly through signing integration tests.

## Acceptance Criteria

- [ ] Line coverage ≥ 80% as measured by `cargo llvm-cov`
- [ ] Key generation, storage, retrieval, rotation, and revocation paths tested
- [ ] Error paths tested: missing keys, expired keys, DB failures
- [ ] Needs DB fixture (integration tests in `tests/integration/signing/`)

## Status Updates

*To be added during implementation*
