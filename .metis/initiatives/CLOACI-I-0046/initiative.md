---
id: scheduler-bench-v2-real-daemon
level: initiative
title: "Scheduler Bench v2 — Real Daemon/Server Performance Harness"
short_code: "CLOACI-I-0046"
created_at: 2026-03-26T02:30:44.820896+00:00
updated_at: 2026-03-26T02:40:14.013092+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: M
initiative_id: scheduler-bench-v2-real-daemon
---

# Scheduler Bench v2 — Real Daemon/Server Performance Harness Initiative

## Context

The scheduler-bench (I-0045) was built to benchmark scheduler performance but programs directly against the library API — it creates `DefaultRunner`, `TriggerScheduler`, `ContinuousScheduler` instances in code and calls methods on them. It does not exercise the real daemon (`cloacinactl daemon`) or server (`cloacinactl serve`) code paths that users actually hit.

Additionally, several scenarios record fake latency values (e.g. `Duration::from_millis(100)`) instead of measuring real submit-to-complete times, making the p50/p95/p99 stats meaningless.

This initiative redesigns the bench to use the real deployment architecture.

## Goals & Non-Goals

**Goals:**
- Benchmark through the same code paths users hit (daemon and server)
- Measure real end-to-end latency from submit to pipeline completion
- Cover all scheduler types: pipeline execution, triggers, cron, continuous
- Support both SQLite (daemon mode) and Postgres (server mode via HTTP)
- Include warmup phase to exclude JIT/cache warming from results
- Exercise the continuous scheduler through DefaultRunner integration

**Non-Goals:**
- Replacing the soak tests (those test sustained reliability, not perf)
- Benchmarking Python/PyO3 task execution specifically
- Multi-tenant performance testing
- Automated regression detection (future initiative)

## Detailed Design

### Two Modes

**Daemon mode** — In-process DefaultRunner with all services enabled (same code path as `cloacinactl daemon`). Creates a full runner with cron, triggers, continuous scheduler, recovery sweeper, registry reconciler. Registers workflows via macros. Measures real pipeline completion latency via `runner.execute()`.

**Server mode** — HTTP client against a running `cloacinactl serve` instance. Uses reqwest to POST /executions, poll GET /executions/{id} until terminal state. Requires `angreal services up` + running server. Measures real HTTP round-trip latency including serialization, network, auth, and server-side dispatch.

### Scenarios

#### Pipeline (daemon)
| Scenario | Description |
|---|---|
| `single-task` | 1-task workflow, baseline submit → complete latency |
| `five-stage` | 5-task serial pipeline, DAG resolution + sequential dispatch overhead |
| `fan-out` | 1 → N parallel → 1 join, parallel dispatch efficiency |
| `concurrent` | N pipelines submitted simultaneously, contention under load |

#### Trigger (daemon)
| Scenario | Description |
|---|---|
| `fire-to-complete` | Register trigger, fire it, measure trigger-poll → pipeline-complete |
| `many-triggers` | 50 triggers firing, scheduler loop overhead under fan-out |
| `dedup` | Rapid fire with `allow_concurrent=false`, dedup correctness + throughput |

#### Cron (daemon)
| Scenario | Description |
|---|---|
| `schedule-to-complete` | Sub-minute cron, measure schedule-fire → pipeline-complete |

#### Continuous (daemon)
| Scenario | Description |
|---|---|
| `steady-injection` | Constant boundary injection via DefaultRunner, inject → task-execute latency |
| `burst` | 10k boundaries at once, coalescing ratio + processing time |
| `multi-source` | 3 data sources feeding one scheduler, per-source throughput |

#### Mixed (daemon)
| Scenario | Description |
|---|---|
| `all-schedulers` | Cron + triggers + continuous + direct submit all active simultaneously |

#### Server (HTTP)
| Scenario | Description |
|---|---|
| `execute` | POST /executions → poll GET /executions/{id} until complete |
| `execute-concurrent` | N concurrent HTTP submissions, HTTP round-trip under load |
| `cron-via-api` | POST /schedules, wait for cron fire, measure schedule → complete |

### CLI

```
scheduler-bench daemon [--scenario <name>] [--duration 60s] [--database-url <url>]
scheduler-bench server [--scenario <name>] [--duration 60s] [--base-url http://localhost:8080]
scheduler-bench all    [--duration 60s]
```

### Implementation

Python script at `tests/performance/scheduler_bench.py`. Zero external dependencies (stdlib only, like the soak tests). Reuses the daemon soak test patterns for package building, registration, daemon lifecycle management, and execution monitoring.

The bench is an **external orchestrator** — it builds `.cloacina` packages, spawns `cloacinactl daemon` or connects to a running `cloacinactl serve`, registers packages, sets up schedules/triggers, monitors execution completion, and measures latencies.

### File Structure

```
tests/performance/
  scheduler_bench.py    — Main bench script (daemon + server modes)
```

### Key Implementation Details

1. **Real e2e latency** — Every scenario measures wall-clock time from action (submit, cron fire, trigger fire) to pipeline completion as reported by daemon status or server API
2. **Package-based** — Builds real `.cloacina` packages from `examples/features/simple-packaged`, registers them through the normal CLI/API path
3. **Daemon mode** — Spawns `cloacinactl daemon` as subprocess, registers packages via `cloacinactl daemon register`, sets cron via `cloacinactl daemon schedule set`, monitors via `cloacinactl daemon status`
4. **Server mode** — HTTP client (stdlib urllib) against running `cloacinactl serve`, uploads packages, triggers executions via POST /executions, polls GET /executions/{id}
5. **Warmup** — First 3 executions discarded from stats
6. **Cleanup** — Temp directories, SQLite files, and daemon processes cleaned up after each run
7. **Reporting** — Table and JSON output matching the old Rust harness format

## Alternatives Considered

1. **Rust binary with in-process DefaultRunner (v1 approach)** — Programs against the library API directly, bypassing the real daemon/server code paths. Rejected because it doesn't test what users actually experience — package loading, reconciliation, process lifecycle.

2. **Rust binary that spawns cloacinactl** — More realistic but adds compile overhead for every bench iteration and subprocess management is clunkier in Rust than Python. Rejected in favor of Python.

3. **Keep v1 and patch** — v1's fake latency recording and unused `--database-url` flag make it fundamentally broken. A clean rewrite is less work than patching.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `scheduler-bench daemon` runs all daemon scenarios with real latency measurement
- [ ] `scheduler-bench server` runs HTTP scenarios against a running server
- [ ] Continuous scheduler scenarios exercise DefaultRunner's continuous scheduling integration
- [ ] All p50/p95/p99 values reflect actual measured latencies
- [ ] Warmup phase excludes cold-start from measurements
- [ ] `--database-url` flag works for both SQLite and Postgres in daemon mode
- [ ] JSON output mode produces machine-parseable results for CI
- [ ] angreal performance commands updated to match new CLI
- [ ] Old scheduler-bench v1 code removed (scenarios/, fake latency recording, unused DB param)
- [ ] Old `examples/performance/{simple,pipeline,parallel}` evaluated — consolidate or remove if redundant
