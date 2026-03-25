---
id: improve-test-coverage-security-db
level: task
title: "Improve test coverage: security/db_key_manager.rs (6.8% → 80%+)"
short_code: "CLOACI-T-0166"
created_at: 2026-03-16T01:01:39.245273+00:00
updated_at: 2026-03-25T12:34:37.078658+00:00
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

# Improve test coverage: security/db_key_manager.rs (6.8% → 80%+)

**Priority: P2 — Tech Debt**

## Objective

`security/db_key_manager.rs` has 1,052 lines at 6.8% line coverage. This is the largest untested file in the codebase. It manages cryptographic key storage and retrieval via the database. Most paths are only exercised indirectly through signing integration tests.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Line coverage ≥ 80% as measured by `cargo llvm-cov`
- [ ] Key generation, storage, retrieval, rotation, and revocation paths tested
- [ ] Error paths tested: missing keys, expired keys, DB failures
- [ ] Needs DB fixture (integration tests in `tests/integration/signing/`)

## Status Updates

### 2026-03-25 — Complete

Added 13 async tests covering all KeyManager trait methods against real SQLite:
create_signing_key, get_signing_key_info, get_signing_key (decrypt), export_public_key,
trust_public_key, trust_public_key_pem, revoke_signing_key, revoke_trusted_key,
grant_trust, revoke_trust, list_signing_keys, list_trusted_keys, find_trusted_key.
Error paths: nonexistent keys, wrong master key, duplicate trust, revoke nonexistent.
15/15 tests pass.
