---
id: cache-accumulator-metrics-and-add
level: task
title: "Cache accumulator metrics and add incremental update on receive/drain"
short_code: "CLOACI-T-0159"
created_at: 2026-03-15T18:24:35.463233+00:00
updated_at: 2026-03-15T19:29:14.108662+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# Cache accumulator metrics and add incremental update on receive/drain

**Priority: P2 — MEDIUM**
**Parent**: [[CLOACI-I-0025]]

## Objective

`accumulator.rs:157-170` (and similarly `288-300` for `WindowedAccumulator`): `metrics()` iterates the entire buffer for min/max/lag on every call — O(n) with no caching. Under high throughput with monitoring polling metrics frequently, this becomes a hot-path bottleneck.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `AccumulatorMetrics` fields (`oldest_emitted_at`, `newest_emitted_at`, `max_lag`, `buffered_count`) are maintained incrementally
- [ ] `receive()` updates cached min/max/lag on each boundary insertion
- [ ] `drain()` resets cached metrics (or recomputes from remaining buffer)
- [ ] `metrics()` returns cached values in O(1)
- [ ] Unit test: metrics are accurate after interleaved receive/drain operations
- [ ] Benchmark: metrics call is constant-time regardless of buffer size

## Implementation Notes

- Add `cached_oldest: Option<DateTime>`, `cached_newest: Option<DateTime>`, `cached_max_lag: Option<Duration>` fields to accumulator structs
- On `receive()`: update newest unconditionally, update oldest only if None or earlier, update max_lag if new boundary lag is larger
- On `drain()`: if buffer is empty, clear all caches. If partial drain (windowed), recompute from remaining buffer (one-time O(n) is acceptable on drain)
- Alternative: use a `BTreeMap` keyed by emitted_at for O(log n) min/max, but this adds complexity

## Status Updates

### 2026-03-15 — Completed
- Added `cached_oldest`, `cached_newest`, `cached_max_lag` fields to both `SimpleAccumulator` and `WindowedAccumulator`
- `receive()` updates caches incrementally: newest unconditionally, oldest if earlier, max_lag if larger
- On buffer-full drop, `cached_oldest` is recomputed from remaining buffer head (O(1))
- `drain()` clears all caches
- `metrics()` returns cached values in O(1) — no more O(n) iteration
- All existing metrics tests pass unchanged (same semantics, faster implementation)
- All 412 unit tests pass
