---
id: investigate-computation-graph
level: task
title: "Investigate computation graph pipeline latency — reduce channel hops for passthrough accumulators"
short_code: "CLOACI-T-0431"
created_at: 2026-04-07T00:50:12.855494+00:00
updated_at: 2026-04-07T00:50:12.855494+00:00
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

# Investigate computation graph pipeline latency — reduce channel hops for passthrough accumulators

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Investigate and reduce event-to-execution latency in the computation graph pipeline. Current p95 is ~7-9ms (both debug and release), dominated by async channel hops and tokio task scheduling — not computation or serialization.

### Current pipeline (2 channel hops per event)
```
sender → [socket channel] → accumulator runtime → [boundary channel] → reactor
```

### Baseline (cg-bench, release build, M3 Mac)
- p50: 2.7ms, p95: 9.2ms, p99: 10.5ms
- Max throughput: ~760 events/sec before channel backup
- Increasing channel buffer sizes made latency WORSE (more queuing delay)

### Investigation areas
1. **Skip passthrough accumulator hop** — passthrough accumulators just forward events unchanged. Short-circuit directly to boundary channel, eliminating one hop (~halve latency)
2. **Avoid double serialization** — sender serializes, BoundarySender re-serializes with source tag. For passthrough, raw bytes could pass straight through
3. **Direct reactor feed mode** — for simple graphs, bypass accumulator runtime entirely and feed reactor from WebSocket/socket
4. **Batch reactor fires** — coalesce multiple boundary updates before firing, reducing per-event overhead at cost of minimum latency
5. **Profile tokio task scheduling** — understand how much time is spent in wakeup/poll cycles vs actual work

### Priority
P3 — current latency is acceptable for most use cases. Optimize when latency-sensitive workloads demand it.

## Status Updates **[REQUIRED]**

*To be added during implementation*
