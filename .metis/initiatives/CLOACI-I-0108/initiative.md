---
id: fix-cloacina-active-tasks-gauge
level: initiative
title: "Fix cloacina_active_tasks gauge leak and extend SQL-derived gauge pattern"
short_code: "CLOACI-I-0108"
created_at: 2026-05-06T11:05:36.453041+00:00
updated_at: 2026-05-14T14:06:00.023834+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: S
initiative_id: fix-cloacina-active-tasks-gauge
---

# Fix cloacina_active_tasks gauge leak and extend SQL-derived gauge pattern Initiative

## Context

CLOACI-T-0534 fixed a gauge leak on `cloacina_active_workflows` by re-seeding the gauge from a SQL count each scheduler tick. The May 2026 review found the same anti-pattern on `cloacina_active_tasks` — naked `metrics::gauge!.increment(1.0)` / `.decrement(1.0)` calls in `crates/cloacina/src/executor/thread_task_executor.rs:840,904`. Any panic between those two lines leaks the count permanently. The COR-04 cdylib panic vector is one reachable path; production deployments observing `cloacina_active_tasks` may see slow drift upward over time.

Additionally, several `let _ = persist_*` calls in the computation-graph runtime swallow errors silently, so persist failures are invisible to operators.

This initiative is small in scope but high in operability value. It should ship as a bounded fix.

## Goals & Non-Goals

**Goals:**
- `cloacina_active_tasks` re-seeds from SQL each scheduler tick; transient panics no longer leak the gauge.
- Persist-failure counters are added on the CG runtime persist paths.
- A reactor whose persist fails 5 consecutive times downgrades to `Degraded` health.

**Non-Goals:**
- General Prometheus surface expansion (tracked under CLOACI-I-0109).
- Reworking the metric schema or label cardinality strategy.
- Reworking persist semantics in the CG runtime.

## Source Findings (May 2026 review)

- **OPS-01 (Major)** — `cloacina_active_tasks` gauge-leak antipattern.
- **OPS-15 (Minor)** — `let _ = persist_*` in CG runtime hides persist failures.

## Discovery Questions

- **Watchdog threshold**: 5 consecutive persist failures → `Degraded` is a guess; is there a baseline failure rate that informs a better number?
- **Persist counter labels**: `kind=cache|dirty|seq_queue|checkpoint` — confirm the full set; are there others?
- **Health propagation**: does `Degraded` need to propagate to readiness probes, or is `/v1/health/graphs/{name}` exposure enough?

## Initial Sketch

- Drop `metrics::gauge!.increment/decrement` at `thread_task_executor.rs:840,904`.
- In the scheduler tick (next to the existing `cloacina_active_workflows` re-seed at `scheduler_loop.rs:166-168`), add `metrics::gauge!("cloacina_active_tasks").set(running_count as f64)` from `task_executions WHERE status = 'Running'`.
- Add `cloacina_reactor_persist_failures_total{reactor, kind}` and `cloacina_accumulator_persist_failures_total{accumulator, kind}` counters on the CG runtime persist paths.
- Add a watchdog: if a reactor logs 5 consecutive persist failures, downgrade `ReactorHealth::Live` → `ReactorHealth::Degraded`; surface via `/v1/health/graphs/{name}`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- After a synthetic panic in `thread_task_executor::execute`, the next scheduler tick re-seeds `cloacina_active_tasks` to the SQL-derived value.
- A reactor whose persist fails 5 times in a row reports `ReactorHealth::Degraded` via the health endpoint.
- New persist-failure counters appear under `angreal test:metrics-format` (promtool-validated).

## References

- `review/06-operability.md` — OPS-01, OPS-15
- `review/10-recommendations.md` — REC-06
- Prior task: CLOACI-T-0534 (Fix `cloacina_active_workflows` gauge leak on crash-recovery, completed) — same pattern.
- Prior task: CLOACI-T-0536 (promtool /metrics format check in CI, completed).
