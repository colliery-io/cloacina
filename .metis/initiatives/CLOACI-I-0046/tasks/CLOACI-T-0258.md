---
id: daemon-mode-pipeline-trigger-and
level: task
title: "Daemon mode — pipeline, trigger, and cron scenarios with real latency"
short_code: "CLOACI-T-0258"
created_at: 2026-03-26T02:36:45.975890+00:00
updated_at: 2026-03-26T03:16:59.128310+00:00
parent: CLOACI-I-0046
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0046
---

# Daemon mode — pipeline, trigger, and cron scenarios with real latency

## Parent Initiative

[[CLOACI-I-0046]]

## Objective

Create `tests/performance/scheduler_bench.py` — the Python bench script with daemon-mode scenarios. Builds `.cloacina` packages, spawns `cloacinactl daemon`, registers packages, sets up cron schedules, and measures real e2e execution latency. Also remove the old Rust scheduler-bench entirely.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `tests/performance/scheduler_bench.py` created with CLI: `--mode daemon|server`, `--scenario`, `--duration`, `--build`
- [ ] Package build helper: builds `.cloacina` from `examples/features/simple-packaged`
- [ ] Stats collector: tracks latency samples, computes p50/p95/p99/min/max/mean, warmup support
- [ ] Table + JSON reporting output
- [ ] `cron-execution` scenario: register package, set cron, start daemon, measure schedule-to-complete
- [ ] `cron-throughput` scenario: sustained ops/s over duration
- [ ] Daemon lifecycle: spawn, health check, graceful shutdown, cleanup temp dirs
- [ ] Old `examples/performance/scheduler-bench/` Rust binary deleted

## Implementation Notes

### Pattern
- Follow `tests/soak/daemon_soak_test.py` for package build, daemon spawn, cron setup, monitoring
- Zero external dependencies (stdlib only)
- CLI: `cloacinactl daemon register`, `cloacinactl daemon schedule set`, `cloacinactl daemon status`

### Dependencies
- Built cloacinactl binary

## Status Updates

- 2026-03-26: Created `tests/performance/scheduler_bench.py` with daemon mode (cron-execution, cron-throughput scenarios), stats collector with warmup, table+JSON reporting. Deleted `examples/performance/scheduler-bench/` Rust binary entirely. Script syntax-checks and CLI help works.
