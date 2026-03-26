---
id: nightly-ci-workflow-with-failure
level: task
title: "Nightly CI workflow with failure alerting"
short_code: "CLOACI-T-0266"
created_at: 2026-03-26T14:13:19.353379+00:00
updated_at: 2026-03-26T15:21:07.142549+00:00
parent: CLOACI-I-0052
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0052
---

# Nightly CI workflow with failure alerting

## Parent Initiative

[[CLOACI-I-0052]]

## Objective

Create a nightly GitHub Actions workflow that runs the extended test suite (slow jobs moved out of PR CI) and auto-creates a GitHub issue when any job fails. Supports manual dispatch for on-demand runs.

## Acceptance Criteria

## Acceptance Criteria

- [ ] New `.github/workflows/nightly.yml` workflow
- [ ] Scheduled at 3am UTC via cron
- [ ] Manual dispatch support (`workflow_dispatch`)
- [ ] Jobs: macOS integration, examples-docs validation, full test suite
- [ ] Auto-creates a GitHub issue on any job failure (with job name, run link, and failure details)
- [ ] Does not duplicate — checks for existing open nightly-failure issue before creating
- [ ] Successful run closes any open nightly-failure issue

## Implementation Notes

### Technical Approach
1. Create `nightly.yml` with `schedule` (cron `0 3 * * *`) and `workflow_dispatch` triggers
2. Reuse existing reusable workflows: `cloacina.yml`, `examples-docs.yml`, `performance.yml`
3. Add a final `notify` job that runs `if: failure()` and uses `gh issue create` to file a bug
4. Add label `nightly-failure` to auto-created issues for easy filtering
5. Add a `close-resolved` job that runs `if: success()` and closes any open `nightly-failure` issues

### Prior Art
Reference: commit `5c4387a` on `archive/cloacina-server-week1` (CI nightly setup)

### Dependencies
None — this task should be completed before T-0265 (CI restructure) so the nightly workflow exists as a home for slow jobs being removed from PR CI.

## Status Updates

### 2026-03-26 — Complete
- Created `.github/workflows/nightly.yml`
- Triggers: cron `0 3 * * *`, `workflow_dispatch`, and `workflow_call` (for release pipeline)
- Jobs: cloacina-tests, cloaca-tests, examples-docs, performance, macOS integration (SQLite unit tests)
- `notify-failure` job: checks for existing open `nightly-failure` issue before creating; adds comment if already open
- `close-resolved` job: auto-closes open `nightly-failure` issues on success with link to passing run
- All reusable workflows verified to support `workflow_call`: cloacina.yml, cloaca-matrix.yml, examples-docs.yml, performance.yml
- Soak/daemon/server jobs intentionally excluded (deferred to I-0054)
