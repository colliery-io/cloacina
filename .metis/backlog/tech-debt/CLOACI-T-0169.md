---
id: improve-test-coverage-security
level: task
title: "Improve test coverage: security/package_signer.rs (29%) and verification.rs (39%)"
short_code: "CLOACI-T-0169"
created_at: 2026-03-16T01:01:43.196718+00:00
updated_at: 2026-03-16T01:01:43.196718+00:00
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

# Improve test coverage: security/package_signer.rs (29%) and verification.rs (39%)

**Priority: P2 — Tech Debt**

## Objective

Two security files are poorly covered:
- `security/package_signer.rs` — 480 lines at 29.2%. Signs workflow packages with ed25519.
- `security/verification.rs` — 295 lines at 39.3%. Verifies package signatures against trust chain.

Existing `tests/integration/signing/` tests cover happy paths but miss error paths and edge cases.

## Acceptance Criteria

- [ ] `package_signer.rs` line coverage ≥ 80%
- [ ] `verification.rs` line coverage ≥ 80%
- [ ] Test: sign and verify roundtrip (happy path — may already exist)
- [ ] Test: verification with tampered package fails
- [ ] Test: verification with expired key fails
- [ ] Test: verification with revoked key fails
- [ ] Test: verification with untrusted signer fails
- [ ] Test: signing with missing private key fails

## Status Updates

*To be added during implementation*
