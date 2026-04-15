---
id: remove-serial-annotations-enable
level: task
title: "Remove #[serial] annotations — enable parallel CG tests"
short_code: "CLOACI-T-0493"
created_at: 2026-04-14T12:38:39.133222+00:00
updated_at: 2026-04-14T14:47:07.198697+00:00
parent: CLOACI-I-0095
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0095
---

# Remove #[serial] annotations — enable parallel CG tests

## Parent Initiative

[[CLOACI-I-0095]]

## Objective

Remove `#[serial]` annotations from tests that only needed them because of global registry contention. Tests use `Runtime::empty()` + local registration for isolation.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Identify which `#[serial]` tests are serial due to global registries vs other reasons (DB, file system)
- [ ] Convert registry-serial tests to use `Runtime::empty()` with local registration
- [ ] Remove `#[serial]` from converted tests
- [ ] Verify parallel execution passes (`angreal cloacina all` multiple times)
- [ ] Reduction of at least 50% of `#[serial]` annotations (currently 159 across 24 files)

## Implementation Notes

### Approach
- Audit each `#[serial]` test to determine why it's serial
- Tests serial due to global registries: convert to `Runtime::empty()` + explicit registration
- Tests serial due to DB (SQLite file contention): leave serial or use unique DB paths
- Tests serial due to file system (package loading): may need unique temp dirs
- Run full suite multiple times to verify no flaky parallel failures

### Dependencies
- T-0491 and T-0492 must be completed first

## Status Updates

- 2026-04-14: Audited all 159 #[serial] annotations across 24 files. Only ~20 are pure REGISTRY (removable). The remaining ~139 are DATABASE (shared fixture singleton) or PROCESS (Python GIL, cargo subprocess). Removing those requires a DB fixture refactor — out of scope for this initiative. Deferring.
