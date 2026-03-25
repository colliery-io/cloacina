---
id: performance-harness-scaffolding
level: task
title: "Performance harness scaffolding — binary, Postgres setup, metrics, reporting"
short_code: "CLOACI-T-0250"
created_at: 2026-03-25T20:45:14.234121+00:00
updated_at: 2026-03-25T20:59:28.218188+00:00
parent: CLOACI-I-0045
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0045
---

# Performance harness scaffolding — binary, Postgres setup, metrics, reporting

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0045]]

## Objective

Shared infrastructure that all benchmark tasks build on. Binary skeleton with CLI dispatch, Postgres connection management, test table setup/teardown, latency histogram collection, and json/table output.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Binary at `examples/features/perf-harness/` with clap CLI dispatching to subcommands
- [ ] Postgres connection helper: connect, create test tables, teardown on exit
- [ ] Metric collector: latency histogram (p50/p95/p99), throughput counter, memory sampler
- [ ] Reporter: `--output json` and `--output table` formatting
- [ ] Common `--duration` flag respected by all subcommands
- [ ] A "smoke" scenario that submits 1 workflow and reports metrics (proves the harness works end-to-end)
- [ ] Runnable via `angreal performance smoke` (add angreal task)

## Status Updates **[REQUIRED]**

### 2026-03-25 — Complete

Created `examples/performance/scheduler-bench/`:
- Binary with clap CLI: smoke, trigger, continuous, execution, hybrid, all subcommands
- MetricCollector: latency histogram (p50/p95/p99), throughput, success/fail counters
- Reporter: table + json output formats
- Scenario stubs for trigger/continuous/execution/hybrid (to be implemented in T-0251..T-0254)
- Smoke scenario: 10 workflows end-to-end, ~500ms/op, proves harness works
- angreal commands: `performance smoke/trigger/continuous/execution/hybrid`
- PyO3 rpath handled via build.rs
