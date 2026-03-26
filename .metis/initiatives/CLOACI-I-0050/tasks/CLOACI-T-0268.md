---
id: remove-bindings-cloaca-backend-and
level: task
title: "Remove bindings/cloaca-backend and update CI"
short_code: "CLOACI-T-0268"
created_at: 2026-03-26T17:33:46.965884+00:00
updated_at: 2026-03-26T21:05:30.185992+00:00
parent: CLOACI-I-0050
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0050
---

# Remove bindings/cloaca-backend and update CI

## Parent Initiative

[[CLOACI-I-0050]]

## Objective

Remove the now-redundant `bindings/cloaca-backend` crate and all associated infrastructure (CI workflows, angreal tasks, release pipeline references). Python is now served natively through cloacina core.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `bindings/cloaca-backend/` directory removed
- [ ] `.github/workflows/cloaca-matrix.yml` removed or reworked to test native Python
- [ ] `ci.yml` python path filters updated (remove `bindings/cloaca-backend` references)
- [ ] `nightly.yml` cloaca-tests job updated or removed
- [ ] `unified_release.yml` cloaca wheel/sdist jobs reworked for native Python packaging
- [ ] Angreal `cloaca` task group updated (smoke, test, package, release, scrub)
- [ ] Root `Cargo.toml` `exclude` list updated (remove `bindings/*` if no longer needed)
- [ ] No remaining references to `cloaca-backend` in codebase
- [ ] `angreal cloacina all` passes

## Implementation Notes

### Technical Approach
1. Delete `bindings/cloaca-backend/` directory
2. Remove or rework `.github/workflows/cloaca-matrix.yml` — Python tests should now run against cloacina core directly
3. Update `ci.yml` path filters — remove `bindings/cloaca-backend` from python filter
4. Update `nightly.yml` — replace cloaca-tests reusable workflow reference
5. Update `unified_release.yml` — wheel builds now come from cloacina core, not cloaca-backend
6. Update angreal tasks in `.angreal/cloaca/` — rework or remove
7. Grep for any remaining `cloaca-backend` references and clean up

### Dependencies
T-0267 (PyO3 move into core) must be completed first.

## Status Updates

### 2026-03-26 — Complete

**Removed:**
- `bindings/cloaca-backend/` — entire directory deleted
- `bindings/` directory removed (was empty after deletion)
- `.github/workflows/cloaca-matrix.yml` — deleted

**CI updates:**
- `ci.yml` — removed `python` path filter output, removed cloaca-tests job, removed `bindings/cloaca-backend` from filters, added `crates/cloacina-build` to rust filter
- `nightly.yml` — removed cloaca-tests job, updated needs lists for notify-failure and close-resolved
- `unified_release.yml` — removed cloaca wheel/sdist build jobs and PyPI publish job, added TODO comments for future native Python packaging

**Other:**
- Root `Cargo.toml` `exclude` updated: removed `bindings/*`
- Angreal `cloaca` task group left in place with note — needs rework once native Python packaging model is defined
- Remaining `cloaca-backend` references are all in `.metis/` historical docs and `.claude/worktrees/` — no functional code references

**Test results:** `angreal cloacina all` — 585 tests, 0 failures
