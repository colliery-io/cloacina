---
id: t4-c-un-ignore-signing-trust-chain
level: task
title: "T4 (C): Un-ignore signing trust-chain tests; switch to fixture auto-skip"
short_code: "CLOACI-T-0569"
created_at: 2026-05-06T12:50:52.180361+00:00
updated_at: 2026-05-07T03:52:03.397181+00:00
parent: CLOACI-I-0103
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0103
---

# T4 (C): Un-ignore signing trust-chain tests; switch to fixture auto-skip

## Context

Per CLOACI-I-0103 (and COR-15 in the May 2026 review): the six signing trust-chain tests under `crates/cloacina/tests/integration/signing/` are gated by `#[ignore = "Requires database connection"]` so they don't run in `angreal test`. Switch them to the same auto-skip pattern other integration tests use (`get_all_fixtures()`).

## What to do

- Locate the six signing tests under `crates/cloacina/tests/integration/signing/*`.
- Remove `#[ignore = "Requires database connection"]` annotations.
- Replace the manual ignore with the `get_all_fixtures()` auto-skip pattern that other integration tests use (look at a fixture-using integration test for the canonical example).
- Verify the tests are picked up by `angreal test integration` when fixtures are running and skip cleanly when not.

## Acceptance

- `angreal test integration` runs all six signing tests by default.
- When no DB fixture is up (e.g., bare `cargo test`), the tests skip cleanly without errors.
- All six tests pass when run against a live fixture.

## References

- Parent: CLOACI-I-0103
- Source: `crates/cloacina/tests/integration/signing/*`
- Reference pattern: any integration test in `crates/cloacina/tests/integration/` using `get_all_fixtures()`

## Status Updates

### 2026-05-07 — Discovery: tests are unwritten stubs, not just ignored — needs scope decision

**Found:**
- 6 `#[ignore = "Requires database connection"]` tests across three files:
  - `signing/trust_chain.rs`: `test_direct_trust`, `test_trust_chain_acl`, `test_trust_chain_isolation`, `test_revoke_trust_acl` (4 tests).
  - `signing/security_failures.rs`: `test_revoked_key_rejected` (1 test).
  - `signing/key_rotation.rs`: `test_key_rotation_database_workflow` (1 test).
- **Every body is `todo!("Implement with test database fixture")`.** They aren't tests that were written and gated; they're aspirational placeholders documenting expected behavior.

**Implication for the original task spec:**
The task body assumed the tests existed and just needed un-ignoring. They don't. Removing `#[ignore]` would convert them from silent skips to loud `todo!()` panics. Three options:

1. **Implement all 6 properly** (write the test bodies using the `get_all_fixtures()` pattern + setup data + assertions). Per-test cost ~1-2 hours; total ~1-2 days for the set. Highest value (real coverage of trust-chain ACL, revocation, key rotation paths).

2. **Implement the most-load-bearing 3** and delete the rest. Likely candidates: `test_revoked_key_rejected` (revocation lifecycle), `test_trust_chain_isolation` (cross-org leak prevention), `test_revoke_trust_acl` (ACL revocation). Skip the rest until needed. ~half-day. Trades coverage breadth for cycle time.

3. **Delete all 6 stubs** as misleading dead code; rely on T-0570's contract test for the basic sign→verify path and accept that ACL/revocation paths are uncovered by tests. ~5 minutes. Lowest cost, lowest coverage. **Honest** — removes a misleading test count.

**Recommend option 2** as the balance, but this is your call — the trust-chain tests cover the multi-org SaaS scenarios that are deferred per CLOACI-A-0005 / CLOACI-I-0106 anyway, so they're testing audience that doesn't exist for the platform/enterprise deployment we're targeting first. Option 3 may actually be the most honest near-term answer: don't promise coverage we don't have until we need the multi-org features.

**Status:** task paused pending scope decision. Not progressing further until directed.
