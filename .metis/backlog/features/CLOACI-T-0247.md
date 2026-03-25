---
id: nightly-release-ci-workflow-server
level: task
title: "Nightly/release CI workflow — server soak (postgres + Docker), extended duration tests"
short_code: "CLOACI-T-0247"
created_at: 2026-03-25T02:21:25.327172+00:00
updated_at: 2026-03-25T02:21:25.327172+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Nightly/release CI workflow — server soak (postgres + Docker), extended duration tests

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Create a nightly/release GitHub Actions workflow for heavy soak and chaos tests that are too slow for every-PR CI. Runs the full containerized server soak (postgres + Docker) and extended-duration daemon/continuous soaks.

## Acceptance Criteria

- [ ] New `.github/workflows/nightly.yml` with `schedule: cron` (nightly) and `workflow_dispatch` (manual)
- [ ] Server soak job: `angreal soak --mode server --duration 5m --profile medium`
- [ ] Extended daemon soak: `angreal soak --mode daemon --duration 5m`
- [ ] Extended continuous soak: 50k boundaries, 4 injectors, 120s duration
- [ ] Slack/email notification on failure (or GitHub issue auto-creation)
- [ ] Badge in README for nightly status
- [ ] Also triggered on release branches / tags

## Status Updates **[REQUIRED]**

*To be added during implementation*
