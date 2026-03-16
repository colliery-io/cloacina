---
id: improve-test-coverage-dispatcher
level: task
title: "Improve test coverage: dispatcher (default.rs 23%, work_distributor.rs 30%)"
short_code: "CLOACI-T-0168"
created_at: 2026-03-16T01:01:42.157871+00:00
updated_at: 2026-03-16T01:01:42.157871+00:00
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

# Improve test coverage: dispatcher (default.rs 23%, work_distributor.rs 30%)

**Priority: P2 — Tech Debt**

## Objective

Two dispatcher files are poorly covered:
- `dispatcher/default.rs` — 128 lines at 23.4%. The default dispatcher implementation routes tasks to executors.
- `dispatcher/work_distributor.rs` — 184 lines at 30.4%. The work distribution logic including LISTEN/NOTIFY for postgres.

## Acceptance Criteria

- [ ] `default.rs` line coverage ≥ 80%
- [ ] `work_distributor.rs` line coverage ≥ 80%
- [ ] Test: task routing to correct executor
- [ ] Test: work distribution poll interval
- [ ] Test: postgres LISTEN/NOTIFY path (integration test)
- [ ] Test: sqlite fallback polling path
- [ ] Test: dispatcher shutdown handling

## Status Updates

*To be added during implementation*
