---
id: ci-restructure-fast-pr-checks-move
level: task
title: "CI restructure — fast PR checks, move slow jobs to nightly"
short_code: "CLOACI-T-0265"
created_at: 2026-03-26T14:13:18.338992+00:00
updated_at: 2026-03-26T16:57:55.549425+00:00
parent: CLOACI-I-0052
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0052
---

# CI restructure — fast PR checks, move slow jobs to nightly

## Parent Initiative

[[CLOACI-I-0052]]

## Objective

Restructure the CI pipeline so PR checks complete fast (under 10 minutes) by moving slow jobs (macOS integration, performance benchmarks, examples validation) out of the PR workflow. Currently `ci.yml` runs everything sequentially including examples-docs and performance on main.

## Current State

Existing workflows in `.github/workflows/`:
- `ci.yml` — main PR/push pipeline (quick-checks → cloacina-tests → cloaca-tests → examples-docs → performance)
- `cloacina.yml` — reusable Cloacina test workflow (feature builds + tests)
- `cloaca-matrix.yml` — Python bindings matrix
- `examples-docs.yml` — examples and docs validation
- `performance.yml` — performance tests (currently only on main or `run-perf` label)
- `docs.yml` — docs build
- `unified_release.yml` — release workflow

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### PR CI
- [ ] PR CI runs only: quick-checks (fmt, clippy, cargo check) + cloacina-tests (unit, integration, macros)
- [ ] PR CI completes under 10 minutes
- [ ] Examples-docs validation moved to nightly (not PR)
- [ ] Performance tests moved to nightly (not PR)
- [ ] Cloaca tests run on any rust or python file changes (bindings consume cloacina core)
- [ ] Update `cloacinactl` path references (was `cloacina-cli`)
- [ ] All PR-blocking jobs still catch real breakage

### Release Pipeline
- [ ] Nightly/long-running jobs triggered as part of the release cycle (not just nightly cron)
- [ ] Release builds produce artifacts uploaded to GHA artifact registry (cargo packages, wheels, etc.)
- [ ] Manual approval gate before publishing to cargo/PyPI — `environment: production` with required reviewers
- [ ] No automatic push to cargo or PyPI — artifacts wait for human "send it" button press
- [ ] Rework `unified_release.yml` to: build → upload artifacts → wait for approval → publish

## Implementation Notes

### Technical Approach
1. Trim `ci.yml` PR jobs: keep quick-checks + cloacina-tests + cloaca-tests (conditional)
2. Remove examples-docs and performance from PR pipeline
3. Update path filters in `ci.yml` to include `crates/cloacinactl/**` and `crates/cloacina-testing/**`
4. Ensure nightly workflow (T-0266) picks up the slow jobs
5. Rework release workflow:
   - Trigger nightly suite as a prerequisite for release
   - Build step produces cargo packages and wheels, uploads as GHA artifacts
   - Gate publish step behind a GitHub environment with required reviewers
   - Publish step downloads artifacts and pushes to cargo/PyPI only after approval

### Dependencies
T-0266 (nightly workflow) must be completed first — slow jobs need a home before being removed from PR CI.

## Status Updates

### 2026-03-26 — Complete

**ci.yml changes:**
- Removed `examples-docs` and `performance` jobs from PR pipeline
- Removed `examples` path filter output (no longer used in PR CI)
- Updated rust path filters: added `crates/cloacinactl/**` and `crates/cloacina-testing/**`
- Updated python path filters: fixed to `bindings/cloaca-backend/**` (was stale `crates/cloaca*` refs), added `crates/cloacina-workflow/**`
- Cloaca tests now trigger on rust OR python changes (not just python)
- PR CI is now: changes → quick-checks → cloacina-tests → cloaca-tests (3 jobs, no sequential slow jobs)

**unified_release.yml changes:**
- Added nightly suite as first step (release prerequisite via `workflow_call`)
- Split into phases: nightly validation → version verify → build artifacts → approval gate → publish
- Build steps upload all artifacts to GHA artifact registry (`.crate` files, wheels, sdist) with 14-day retention
- Both `publish-cargo` and `publish-pypi` jobs gated behind `environment: production` (requires manual approval)
- No automatic push to cargo or PyPI — human must approve in GitHub environment UI
