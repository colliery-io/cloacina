---
id: ci-verification-lattice-holes-path
level: task
title: "CI verification-lattice holes — path filters, constructor suite, server-side lockstep, publish tiers"
short_code: "CLOACI-T-0917"
created_at: 2026-08-02T16:33:33.333322+00:00
updated_at: 2026-08-04T01:06:09.137022+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# CI verification-lattice holes — path filters, constructor suite, server-side lockstep, publish tiers

## Objective

Close the verification-lattice blind spots found by the 2026-08-02 deep dive. Pattern diagnosis: the system hardens only where it has bled — every past incident has a guard at the exact failing line, while symmetric never-yet-failed paths have none. These are the cheapest, highest-leverage hardening items in the register.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P1 - High (important for user experience)

## Findings

1. PATH-FILTER HOLES (.github/workflows/ci.yml:50-91): `crates/cloacina-python/**` and `tests/python/**` appear in NO filter — a bindings-only PR runs ZERO test lanes. `crates/cloacina-server/**` is absent from the `rust` filter — server-only PRs get spec-drift checks but not the DB-backed server lib/auth tests (which run in the integration lane per project convention).
2. CONSTRUCTOR SUITE IS CI DARK MATTER: the 14-target provider/constructor Rust test suite (#![cfg(feature = "constructors-wasm")]) runs nowhere — `constructors-wasm` appears in no .angreal task and no workflow; only wave-day certify exercises the path. Give it a CI home (nightly at minimum).
3. LOCKSTEP IS CLIENT-SIDE ONLY: scripts/version_lockstep.py (cargo pins, 3 npm, python client, scaffold, 3 helm appVersions) runs only in pre-commit; no workflow runs pre-commit, and check_sdk_versions.py (what CI/release actually run) omits helm/scaffold/cargo pins. The pre-commit comment claiming "CI runs pre-commit, so drift cannot reach main" is false — the exact incident the check commemorates (charts drifted four minors) can recur through one --no-verify. Fix: run version_lockstep.py in ci.yml quick-checks.
4. HAND-MAINTAINED PUBLISH TIERS: the 0.10.0 postmortem (8961abb5) was two crates missing from hand-maintained tier lists + a zombie job; no check verifies tier completeness against the workspace. Fix: generate tiers from cargo metadata (publish=true set) or add a tier-completeness assertion to CI.
5. Quieter tolerance channels to revisit deliberately: 3x retry wrappers on every examples-matrix leg and `continue-on-error: true` on the python-tutorials job — decide which are still warranted post-I-0140 and document the ones that stay.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] A PR touching only crates/cloacina-python triggers the test matrix; same for tests/python and crates/cloacina-server
- [ ] constructors-wasm suite has a CI lane (documented cadence)
- [ ] version_lockstep.py enforced server-side; the false comment corrected
- [ ] Publish-tier completeness mechanically checked against workspace membership
- [ ] Retry/continue-on-error tolerances re-justified or removed

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (quality-release report; DEEPDIVE.md recommendations #1-3). Verified against main @ 5216e632.
