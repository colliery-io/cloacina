---
id: e2e-continuous-scheduling
level: initiative
title: "E2E Continuous Scheduling Performance Harness — Real Postgres, Multi-Scenario Benchmarking"
short_code: "CLOACI-I-0045"
created_at: 2026-03-25T13:42:23.223681+00:00
updated_at: 2026-03-25T13:44:56.843517+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/design"


exit_criteria_met: false
estimated_complexity: M
initiative_id: e2e-continuous-scheduling
---

# Full Scheduler Performance Harness — Task, Cron, Continuous, Trigger Benchmarking Against Real Postgres

Absorbs: T-0149 (backlog task, blocked due to scope).

## Context

We have extensive in-process soak tests (continuous scheduler against SQLite) and containerized daemon/server soaks. What's missing is a **real Postgres performance benchmark** that exercises the full continuous scheduling pipeline end-to-end: real rows inserted, real detector queries, real task execution, real I/O — not simulated ledger events.

The existing soak tests prove correctness under load. This initiative proves **performance characteristics** — latency, throughput, coalescing efficiency, and resource usage over time.

## Goals & Non-Goals

**Goals:**
- Harness that exercises the **full scheduling feature set** against real Postgres:
  - **Trigger scheduler** (including cron as a special case): trigger poll → fire → workflow execution → dedup; cron schedule → pipeline creation → execution → audit
  - **Continuous scheduler**: data arrival → detector → accumulator → boundary-driven execution → ledger
  - **Task execution engine** (internal, exercised by both): DAG resolution → task claiming → execution → completion
- Configurable scenarios per scheduler type
- Metrics: e2e latency (p50/p95/p99), throughput, coalescing ratio, buffer depth, memory
- Runnable via `angreal performance` subcommands and in nightly CI
- Baseline numbers documented for regression detection

**Non-Goals:**
- Replacing existing soak/chaos tests (complementary, not replacement)
- Automated regression detection with thresholds (future work on top of baseline)

## Detailed Design

### Scheduler Coverage

#### Trigger Scheduler (includes cron)

Exercises event-driven workflow activation — both custom triggers and cron (which is a first-class trigger with time-based semantics).

**Trigger scenarios:**
- **High-frequency poll** — trigger with 100ms interval, measure poll-to-execution latency
- **Concurrent triggers** — 50 triggers registered, measure scheduler loop overhead
- **Dedup under load** — rapid-fire trigger with allow_concurrent=false, verify no duplicates
- **Mixed trigger types** — webhook + http_poll + file_watch + python triggers all active

**Cron scenarios (special-case trigger):**
- **Sub-second cron** — `*/2 * * * * *` (every 2s), measure poll-to-execution latency
- **Burst catchup** — pause scheduler, accumulate missed ticks, resume, measure catchup throughput
- **Many schedules** — 100 concurrent cron schedules, measure overhead

**Task execution engine (exercised by all trigger-fired workflows):**
- **Simple pipeline** — linear 5-task chain, measure per-task overhead
- **Wide fan-out** — 1 root → 20 parallel tasks → 1 join, measure parallelism
- **Deep DAG** — 10-level chain with branching, measure dependency resolution
- **Concurrent submissions** — N workflows submitted simultaneously, measure contention
- **Claiming contention** — multiple claim_ready_task calls racing, measure fairness

#### Hybrid (all schedulers running simultaneously)

Exercises the system as it runs in production — triggers firing workflows, cron on schedule, continuous reacting to data, all sharing the same executor pool, database connections, and task claiming pipeline.

- **Steady mixed load** — 5 cron schedules + 3 triggers + 2 continuous sources, all active for 5 min. Measure: total throughput, per-scheduler latency, no starvation.
- **Burst under mixed load** — continuous steady state + cron running, then inject a trigger burst (100 rapid fires). Measure: does the burst starve cron/continuous?
- **Resource contention** — saturate the executor pool (max_concurrent_tasks) with continuous tasks, verify triggers and cron still make progress when slots free up.
- **Long soak** — all schedulers active for 15+ min at moderate load. Measure: memory growth, connection pool health, no degradation over time.

#### Continuous Scheduler

Full data-driven pipeline against real Postgres.

- **Steady state** — constant 100/s insertion, Immediate policy
- **Burst** — 10k rows in 1s then silence, WallClockDebounce(2s)
- **Slow consumer** — 100/s arrival, 2s task execution, accumulator absorption
- **Multi-source fan-in** — 3 sources, JoinMode::Any
- **Long run** — 5 min steady 50/s, stability and memory
- **Complex graph (20-30 nodes)** — multi-level DAG with cascading execution

### Continuous Scheduler Data Path

```
Background Writer → perf_raw_events (Postgres)
                          ↓
                    Detector Task (SELECT max(id) > last_known)
                          ↓
                    ContinuousScheduler + Accumulator
                          ↓
                    Aggregate Task (SELECT count(*) WHERE id BETWEEN start AND end)
                          ↓
                    perf_aggregations (Postgres) + ExecutionLedger
```

### CLI Interface

```
angreal performance trigger           [--scenario high-freq|concurrent|dedup|mixed|cron-sub-second|cron-catchup|cron-many]
angreal performance continuous        [--scenario steady|burst|slow-consumer|multi-source|long-run|complex]
angreal performance execution         [--scenario simple|fan-out|deep-dag|concurrent|contention]
angreal performance hybrid            [--scenario mixed-load|burst-under-load|contention|long-soak]
angreal performance all               # run all with default scenarios

Common flags:
  --duration 60s
  --output json|table
```

## Alternatives Considered

**Extend existing soak tests:** Rejected — soak tests use in-memory SQLite with simulated events. Perf testing needs real Postgres I/O to measure actual latency.

**Use external benchmarking tool (criterion):** Rejected — criterion is for micro-benchmarks. This is a system-level throughput test with I/O.

## Implementation Plan

### Phase 1: Harness scaffolding + shared infrastructure
- Harness binary with CLI dispatch (angreal subcommands)
- Postgres connection management, test table setup/teardown
- Shared metric collection: latency histograms, throughput counters, memory sampling
- Shared reporting: json + table output

### Phase 2: Trigger scheduler benchmarks (including cron)
- Trigger registration, poll-to-execution latency
- Cron schedule scenarios (sub-second, catchup, many-schedules)
- Concurrent triggers, dedup, mixed trigger types

### Phase 3: Continuous scheduler benchmarks
- Background writer, detector task, accumulator wiring
- Steady state, burst, slow consumer, multi-source, long run, complex graph
- Coalescing ratio and buffer depth metrics

### Phase 4: Execution engine benchmarks
- Workflow generation (linear, fan-out, deep DAG)
- Submit → claim → execute → complete cycle against Postgres
- Concurrent submission + claiming contention scenarios

### Phase 5: Hybrid benchmarks
- Stand up full DefaultRunner with all schedulers enabled
- Mixed load: cron + triggers + continuous all active simultaneously
- Burst-under-load and resource contention scenarios
- Long soak (15+ min) measuring stability and resource health

### Phase 6: CI integration + baselines
- `angreal performance` subcommands
- Add to nightly workflow
- Document baseline numbers, expected ranges
