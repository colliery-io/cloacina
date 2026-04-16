---
id: audit-metrics-endpoints-coverage
level: task
title: "Audit metrics endpoints — coverage, accuracy, and Prometheus compatibility"
short_code: "CLOACI-T-0498"
created_at: 2026-04-16T12:41:49.569237+00:00
updated_at: 2026-04-16T12:41:49.569237+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Audit metrics endpoints — coverage, accuracy, and Prometheus compatibility

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Objective

Deep review of the `/metrics` endpoint and all Prometheus instrumentation. The metrics system was built in I-0088 (CLOACI-T-0453) but has not been validated against real Prometheus scraping, Grafana dashboards, or production load patterns. Need to verify that counters increment correctly, histograms capture meaningful latencies, labels are consistent, and the endpoint is compatible with standard Prometheus tooling.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

### Impact Assessment
- **Affected Users**: Operators relying on metrics for observability
- **Expected vs Actual**: Unknown — metrics were implemented but never validated against real scraping. Some counters may be phantom (registered but never incremented), label cardinality may be too high, CG-specific metrics (graph fires, accumulator throughput, reactor latency) may be missing entirely.

## Acceptance Criteria

- [ ] Audit all registered metrics — verify each counter/histogram/gauge is actually incremented
- [ ] Verify `/metrics` output parses correctly with `promtool check metrics`
- [ ] Validate CG-specific metrics exist: graph fire count, accumulator events/sec, reactor cache age
- [ ] Validate workflow metrics: task throughput, execution latency, claim rate, failure rate
- [ ] Check label cardinality — no unbounded labels (e.g., task IDs as labels)
- [ ] Verify metrics survive across graph reload / package upgrade
- [ ] Document which metrics exist and what they mean

## Implementation Notes

### Areas to audit
- `crates/cloacina/src/metrics/` — metric definitions
- Server handler middleware — request duration histograms
- Reactor fire path — graph execution counters and latencies
- Accumulator runtime — event throughput counters
- Scheduler loop — claim/heartbeat/sweep counters
- WebSocket handler — connection counts, message throughput

## Status Updates

*To be added during implementation*
