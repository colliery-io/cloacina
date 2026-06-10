---
id: cross-sdk-release-angreal-sdk
level: task
title: "Cross-SDK release — angreal sdk-contract matrix, Diataxis docs, lockstep release tooling"
short_code: "CLOACI-T-0648"
created_at: 2026-06-10T01:30:42.525161+00:00
updated_at: 2026-06-10T01:30:42.525161+00:00
parent: CLOACI-I-0113
blocked_by: ["CLOACI-T-0645", "CLOACI-T-0646", "CLOACI-T-0647"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0113
---

# Cross-SDK release — angreal sdk-contract matrix, Diataxis docs, lockstep release tooling

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0113]]

## Objective **[REQUIRED]**

Aggregate and ship: an `angreal test sdk-contract` matrix running all three SDK contract suites against a live server in CI, Diataxis docs per language (tutorial, how-to, reference), version-lockstep release tooling so SDK versions stamp from the workspace version, and the first tagged SDK release riding the next cloacina release (REQ-006/REQ-008). This phase is an aggregation — each SDK's contract suite was already green when its own task exited.

## Acceptance Criteria **[REQUIRED]**

- [ ] `angreal test sdk-contract` (plus per-language variants) boots `cloacina-server` via existing compose and runs all three SDK contract suites; wired into CI/nightly
- [ ] Coverage rule enforced: every spec endpoint exercised per SDK; every documented WS message variant round-tripped at least once
- [ ] Diataxis docs per language: tutorial (small consumer), how-to (auth, pagination, WS subscribe), reference generated from the spec (REQ-006)
- [ ] Lockstep release tooling: SDK versions stamped from the workspace version in `unified_release.yml`; npm/PyPI/crates.io publish steps idempotent (REQ-008)
- [ ] First tagged SDK release ships with the next cloacina release

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
New angreal task group `test sdk-contract` (+ `sdk-contract-rust`/`-python`/`-ts`) reusing the existing e2e compose to boot the server, then running each suite. Coverage rule enforced by a script that diffs exercised endpoints/WS variants against the spec. Docs go into the existing Diataxis tree. Version stamping: SDK package versions derive from `workspace.package.version` at release time in `unified_release.yml`.

### Dependencies
Blocked by CLOACI-T-0645, CLOACI-T-0646, CLOACI-T-0647 — this phase aggregates; each SDK's suite was green at its own task exit.

### Risk Considerations
Release-workflow permissions: the v0.7.0 release hit two rounds of GitHub Actions permission ceilings (nested reusable-workflow perms, missing `contents:write`) — new npm/PyPI publish jobs need explicit job-level `permissions` from the start. CI wall-time for three live-server suites — consider full matrix on nightly with a smoke subset on PR.

## Status Updates **[REQUIRED]**

*To be added during implementation*
