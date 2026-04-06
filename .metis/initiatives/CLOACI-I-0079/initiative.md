---
id: computation-graph-soak-tests-and
level: initiative
title: "Computation Graph Soak Tests and Performance Benchmarks"
short_code: "CLOACI-I-0079"
created_at: 2026-04-05T14:34:57.420074+00:00
updated_at: 2026-04-05T19:22:09.711202+00:00
parent: CLOACI-V-0001
blocked_by: [CLOACI-I-0082]
archived: false

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: M
initiative_id: computation-graph-soak-tests-and
---

# Computation Graph Soak Tests and Performance Benchmarks Initiative

## Context

The computation graph system has unit and integration tests but no sustained load testing or performance baselines. We need to verify: no memory growth under sustained event flow, no accumulator channel backup, acceptable event-to-execution latency, and stable behavior over 60+ seconds.

Existing soak test infrastructure from I-0054 can be extended. The existing `examples/features/computation-graphs/continuous-scheduling/` example is a starting point.

Blocked by: I-0073 (batch/polling accumulators should exist first to include in soak), I-0082 (resilience wiring — soak testing without connected persistence/health/supervision tests an incomplete system). Can start with passthrough + stream accumulators in embedded mode.

## Goals & Non-Goals

**Goals:**
- Soak test: market maker pipeline running 60+ seconds with sustained event injection at realistic rates (10ms-200ms intervals)
- Verify: no memory growth, no channel backup, no panics
- Performance benchmark: event-to-graph-execution latency (p50, p95, p99)
- Performance benchmark: accumulator throughput (events/sec per accumulator)
- Performance benchmark: reactor cache update + dirty flag cycle time
- Integrate benchmarks into CI via `angreal performance` infrastructure
- Publish baseline numbers in docs

**Non-Goals:**
- New features (covered by I-0073, I-0077, I-0078)
- Server-mode soak testing (requires T-0380 reconciler wiring)
- Distributed performance testing

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Soak test: 2+ accumulators → reactor → routing graph, sustained 60+ seconds, no memory growth
- [ ] Soak test: event injection at 10ms (fast source) and 200ms (slow source) intervals
- [ ] Soak test integrated into `angreal cloacina soak` or similar
- [ ] Latency benchmark: measure event push → graph execution complete, report p50/p95/p99
- [ ] Throughput benchmark: max events/sec before channel backup
- [ ] Benchmarks runnable via `angreal performance` commands
- [ ] Baseline numbers documented
- [ ] All existing tests pass

## Implementation Plan

1. **Soak test harness** — extend Tutorial 10's market maker into a sustained load test with timer-based event injection
2. **Memory monitoring** — track RSS/heap before and after soak, assert no significant growth
3. **Latency instrumentation** — timestamp at event push and graph completion, collect distribution
4. **Throughput test** — ramp up event rate until channel backup detected
5. **CI integration** — angreal tasks for soak + perf
6. **Documentation** — baseline numbers page
