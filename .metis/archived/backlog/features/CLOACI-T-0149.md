---
id: end-to-end-continuous-scheduling
level: task
title: "End-to-end continuous scheduling performance test with real Postgres data"
short_code: "CLOACI-T-0149"
created_at: 2026-03-15T16:27:17.115908+00:00
updated_at: 2026-03-25T13:43:22.261146+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: NULL
---

# End-to-end continuous scheduling performance test with real Postgres data

## Objective

Build a performance test that exercises the full continuous scheduling pipeline against a real Postgres database. Real rows written, real detector queries, real task execution, real I/O — not simulated ledger events.

Requires: `angreal services up` (Postgres)

## What It Exercises

The complete data path, end to end:

1. **Background writer** inserts rows into `raw_events` table at configurable rate
2. **Detector task** polls `SELECT max(id) FROM raw_events WHERE id > $last_known`, emits `OffsetRange` boundaries
3. **Scheduler** routes boundaries through accumulator, applies trigger policy
4. **Continuous task** executes: `SELECT count(*) FROM raw_events WHERE id BETWEEN $start AND $end`, writes result
5. **Ledger** records TaskCompleted, LedgerTrigger could chain downstream

### What We Measure

| Metric | What It Tells Us |
|--------|-----------------|
| End-to-end latency | Time from row insertion to task completion |
| Detection latency | Time from row insertion to boundary emission |
| Scheduling latency | Time from boundary received to task fired |
| Execution throughput | Tasks completed per second |
| Coalescing ratio | Raw boundaries received / drains executed |
| Buffer depth over time | Accumulator pressure under load |
| Ledger growth | Events per second, memory footprint |

### Test Scenarios

**Scenario 1: Steady state** — rows arrive at constant rate (e.g., 100/s), detector polls every 1s, Immediate policy. Expect: 1 task execution per detector poll, low latency, coalescing ratio ~1.

**Scenario 2: Burst** — 10,000 rows inserted in 1 second, then silence. WallClockDebounce(2s) policy. Expect: boundaries accumulate during burst, single drain after silence, coalescing ratio high.

**Scenario 3: Slow consumer** — rows arrive at 100/s, task takes 2s to execute. Expect: boundaries accumulate while task runs, next drain covers all accumulated, coalescing absorbs backpressure.

**Scenario 4: Multi-source fan-in** — 3 source tables, 1 task with JoinMode::Any. Boundaries arrive at different rates. Expect: task fires on fastest source, others accumulate.

**Scenario 5: Long run** — 5 minutes of steady 50/s insertion. Measure: memory growth, timing stability, no degradation.

**Scenario 6: Complex graph (20-30 nodes)** — a realistic multi-level DAG with fan-out, fan-in, and compounding derivation. Tests graph assembly, routing to many accumulators, and cascading execution.

Example topology:
```
Layer 0 (sources):     raw_clicks, raw_impressions, raw_conversions, raw_pageviews, config
Layer 1 (clean):       clean_clicks, clean_impressions, clean_conversions, clean_pageviews
                       (each reads one raw source, filters/validates)
Layer 2 (join):        user_sessions (clicks + pageviews fan-in)
                       ad_performance (impressions + clicks + conversions fan-in)
                       engagement_metrics (clicks + pageviews fan-in)
Layer 3 (aggregate):   hourly_sessions, hourly_ad_perf, hourly_engagement
                       (each reads one L2 source, does time-windowed rollup)
Layer 4 (derive):      campaign_roi (hourly_ad_perf + config fan-in)
                       user_scoring (hourly_sessions + hourly_engagement fan-in)
                       anomaly_flags (all 3 hourly sources fan-in)
Layer 5 (output):      dashboard_cache (campaign_roi + user_scoring + anomaly_flags)
                       alert_dispatch (anomaly_flags only)
```

~5 source tables, ~20 continuous tasks, ~25 edges. Data flows from raw → clean → join → aggregate → derive → output. Each layer compounds on the previous. A single row insertion into `raw_clicks` should cascade through clean_clicks → user_sessions + ad_performance → hourly rollups → campaign_roi + user_scoring → dashboard_cache.

Measures for this scenario:
- Cascade latency: time from source insertion to final dashboard_cache task completion
- Parallelism: how many L1 tasks fire simultaneously from a single source change
- Fan-in correctness: L2 tasks with JoinMode::Any fire on partial input vs JoinMode::All waits
- Graph assembly time for 20+ nodes
- Scheduler overhead: poll interval impact with 25 edges to check

## Schema

```sql
-- Test table (created by the harness, dropped after)
CREATE TABLE IF NOT EXISTS perf_raw_events (
    id SERIAL PRIMARY KEY,
    value TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Output table for the continuous task
CREATE TABLE IF NOT EXISTS perf_aggregations (
    id SERIAL PRIMARY KEY,
    boundary_start BIGINT,
    boundary_end BIGINT,
    row_count BIGINT,
    aggregated_at TIMESTAMP DEFAULT NOW()
);
```

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Harness binary at `examples/features/continuous-scheduling-perf/`
- [ ] CLI args: `--duration`, `--insert-rate`, `--task-delay`, `--policy`, `--detector-interval`
- [ ] Background writer inserts real rows into Postgres at configured rate
- [ ] Detector task queries Postgres for new rows, emits OffsetRange boundaries
- [ ] Continuous task queries Postgres for count within boundary range, writes to output table
- [ ] Reports: end-to-end latency (avg/p50/p95/p99), throughput, coalescing ratio, buffer depth
- [ ] All 5 scenarios documented with expected vs actual results
- [ ] Runnable via `angreal performance continuous` (add angreal task)
- [ ] Cleans up test tables after run

## Implementation Notes

### Architecture

```
┌─────────────────┐     INSERT      ┌──────────────────┐
│ Background      │ ──────────────► │ perf_raw_events  │
│ Writer (tokio)  │                 │ (Postgres table) │
└─────────────────┘                 └────────┬─────────┘
                                             │
                                    SELECT max(id)
                                             │
                                    ┌────────▼─────────┐
                                    │ Detector Task     │
                                    │ (registered task) │
                                    └────────┬─────────┘
                                             │
                                    DetectorOutput::Change
                                             │
                                    ┌────────▼─────────┐
                                    │ ContinuousScheduler│
                                    │ + Accumulator     │
                                    └────────┬─────────┘
                                             │
                                    execute() with boundary
                                             │
                                    ┌────────▼─────────┐
                                    │ Aggregate Task    │
                                    │ SELECT count(*)   │
                                    │ INSERT INTO agg   │
                                    └──────────────────┘
```

### Key Design Points

- Writer and scheduler run as separate tokio tasks
- Detector is a registered continuous task that the scheduler calls (not a cron workflow — we're testing the scheduler directly)
- Actually, the detector needs to be triggered on an interval. Two options:
  - **Option A**: Separate tokio::interval that writes DetectorOutput to ledger (simulates cron trigger)
  - **Option B**: Run detector as part of the scheduler loop with a polling interval
- Option A is more realistic — the detector is triggered externally, the scheduler just observes completions
- PostgresConnection provides the connection URL; tasks create their own connection pools
- Timing: stamp each row with insertion time, compare against task completion time for e2e latency

### Dependencies

- `angreal services up` (Postgres running)
- All continuous scheduling code from I-0023/I-0024/I-0025
- `deadpool-diesel` or `tokio-postgres` for direct DB access in writer/tasks

## Status Updates

### 2026-03-25 — Blocked: scope too large for backlog task

This is initiative-sized work: a full performance harness binary with 6 scenarios, background Postgres writer, detector tasks, custom tables, and metric collection. Should be decomposed into an initiative with multiple tasks (harness scaffolding, scenario 1-2 basic, scenario 3-6 advanced, angreal integration, CI integration). Blocking to properly scope later.
