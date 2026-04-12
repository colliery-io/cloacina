---
id: t3-security-module-tests-db-key
level: task
title: "T3: Security module tests (db_key_manager + package_signer)"
short_code: "CLOACI-T-0344"
created_at: 2026-04-03T13:09:22.676306+00:00
updated_at: 2026-04-03T17:33:28.373+00:00
parent: CLOACI-I-0068
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0068
---

# T3: Security module tests (db_key_manager + package_signer)

## Parent Initiative
[[CLOACI-I-0068]] — Tier 1 (~1,120 missed lines)

## Objective
Add tests for the security module. db_key_manager.rs is at 5% (748 missed lines) — the entire key trust chain is untested. package_signer.rs is at 41%, verification.rs at 68%.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] db_key_manager.rs: test key creation, rotation, trust chain operations, key lookup, revocation
- [ ] package_signer.rs: test sign package, verify signature, reject tampered packages
- [ ] verification.rs: test verify_package_signature, trust chain validation, expired key handling
- [ ] security/audit.rs: test audit event creation and querying (65% → >80%)
- [ ] Tests run against real Postgres (key manager is DB-backed)
- [ ] Coverage of security/ moves from ~11% to >50%

## Source Files
- crates/cloacina/src/security/db_key_manager.rs (748 missed, 5%)
- crates/cloacina/src/security/package_signer.rs (258 missed, 41%)
- crates/cloacina/src/security/verification.rs (114 missed, 68%)
- crates/cloacina/src/security/audit.rs (56 missed, 65%)

## Implementation Notes
The existing signing tests in tests/integration/signing/ cover sign_and_verify, trust_chain, security_failures, key_rotation. Check what's already tested before adding new tests. The db_key_manager is the big gap — it's the DB-backed KeyManager trait implementation.

## Status Updates

### 2026-04-03 — Complete (64 new tests, 101 total security tests)

**db_key_manager.rs** (36 tests): PEM encode/decode roundtrips + error cases, create/get/list/revoke signing keys, sign+verify roundtrip, export/import roundtrip, trust ACL grant/revoke/inherited keys, multi-org isolation, wrong master key, revoked key rejection

**audit.rs** (14 tests): all audit log functions — key create/revoke/export, trusted key add/revoke, trust ACL revoke, package sign/sign-failed/load-failure, verification success/failure

**package_signer.rs** (14 tests): sign with raw key, sign with DB key (success/revoked/not-found), store+find signatures, verify detached signature (valid/tampered/wrong-key/wrong-algo), verify via trusted key, no-signature failure

Coverage: db_key_manager 5%→69.6%, audit 65%→99.1%, package_signer 41%→86.7%, key_manager 0%→100%
