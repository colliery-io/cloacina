---
id: ci-verification-lattice-holes-path
level: task
title: "CI verification-lattice holes — path filters, constructor suite, server-side lockstep, publish tiers"
short_code: "CLOACI-T-0917"
created_at: 2026-08-02T16:33:33.333322+00:00
updated_at: 2026-08-04T10:15:38.500736+00:00
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

- [x] Test matrix now triggers for cloacina-python, tests/python, cloacina-server, -compiler, -agent, -client, -constructor-contract, clients/python; scripts/** triggers quick-checks
- [x] constructors-wasm suite has a CI home: nightly constructor-suite job (ubuntu, wasm32-wasip2, 60min cap, target-scoped invocation) — release-blocking since unified_release calls nightly
- [x] version_lockstep.py runs in quick-checks (it lives in .angreal/, not scripts/ as filed); false pre-commit comment corrected
- [x] scripts/check_publish_tiers.py asserts exactly-once coverage vs cargo metadata both directions; passes today (15/15)
- [x] python-tutorials continue-on-error removed (post-I-0140); three x3 retry sites kept with justification + removal criteria; repo grep confirms no other allow-failure sites

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (quality-release report; DEEPDIVE.md recommendations #1-3). Verified against main @ 5216e632.
- 2026-08-04: DONE — merged to main in PR #233 (squash). Corrections to the filing: version_lockstep.py is in .angreal/ not scripts/; the "tiers" in unified_release.yml are bare `publish <crate>` shell calls, not YAML lists (the checker regex-parses them and fails loud if that shape changes). Constructor-suite placement decided on measured data: the suite compiles fixture crates inside tests and exceeded the 10-minute per-PR budget mid-run (11/14 green before cutoff, zero constructor-target failures), so nightly — and the invocation MUST be target-scoped because an unscoped `cargo test -p cloacina` drags in the postgres-dependent fixtures target. providers/* are not workspace members (they ship via provider_release waves) and are documented as out of scope in the tier checker.