---
id: computation-graph-depth-batch
level: initiative
title: "Computation Graph Depth — Batch Accumulators, Python, Soak Tests"
short_code: "CLOACI-I-0073"
created_at: 2026-04-04T17:48:57.173883+00:00
updated_at: 2026-04-04T17:48:57.173883+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: computation-graph-depth-batch
---

# Computation Graph Depth — Batch Accumulators, Python, Soak Tests Initiative

## Context

Fourth implementation initiative for CLOACI-I-0069. Adds depth to the computation graph feature after the MVP (I-0072) is complete: remaining accumulator types, Python bindings, additional reaction criteria and input strategies, soak tests, and performance benchmarks.

This initiative may be decomposed further during planning — it's a collection of additive features that don't change the architecture.

Blocked by: CLOACI-I-0072 (market maker reference implementation / MVP).

## Goals & Non-Goals

**Goals:**
- Implement `#[batch_accumulator]` (dormant until flush, drain source on signal, batch processing)
- Implement `#[polling_accumulator]` (timer-based polling of databases/APIs)
- Implement `when_all` reaction criteria
- Implement `sequential` input strategy
- Implement Python bindings for computation graphs (PyO3 decorators, `spawn_blocking` wrapping)
- Implement Python accumulator decorators (`@stream_accumulator`, `@passthrough_accumulator`, etc.)
- Soak test: market maker running 60+ seconds under sustained load, no memory growth, no accumulator backup
- Performance benchmarks: event-to-graph-execution latency, accumulator throughput, reactor cache update speed
- Benchmarks integrated into CI (CLOACI-I-0054 soak infrastructure)

**Non-Goals:**
- Mixed Rust/Python packages (one language per package)
- Distributed computation graphs across multiple processes
- Complex stream processing (windowed, watermarked — use external processors)

## Acceptance Criteria

- [ ] `#[batch_accumulator]` implemented — dormant, flush signal from reactor, drains source on flush, processes batch, emits one boundary
- [ ] Batch accumulator works with stream backend (Kafka) and socket (passthrough)
- [ ] `#[polling_accumulator]` implemented — timer-based async poll function, `Option<T>` return for "no change"
- [ ] `when_all` reaction criteria — reactor waits until all dirty flags set before firing
- [ ] `when_all` naturally handles initialization — graph doesn't fire until every source has emitted at least once
- [ ] `sequential` input strategy — one boundary per execution, no collapsing, ordered processing
- [ ] Python `@computation_graph` decorator — dict-based topology, tuple returns for routing, class-based graph
- [ ] Python `@stream_accumulator`, `@passthrough_accumulator` decorators — all wrap in `spawn_blocking`
- [ ] Python computation graph loaded via reconciler, runs in server mode
- [ ] Soak test: market maker with mock Kafka producers at alpha=10ms, beta=200ms, gamma sporadic, runs 60+ seconds without memory growth
- [ ] Performance benchmark: measure event-to-execution latency, publish baseline numbers
- [ ] All existing tests continue to pass

## Implementation Plan

This initiative is a collection of independent features. Order by value:

1. **Batch accumulator** — enables "all events since last run" pattern, critical for reconciliation use cases
2. **Polling accumulator** — enables database/API polling, common for config sources
3. **`when_all` + `sequential`** — extends reaction criteria and input strategy
4. **Soak test** — validates stability under sustained load
5. **Performance benchmarks** — establishes baseline latency numbers
6. **Python bindings** — opens computation graphs to Python developers

Each can be implemented and shipped independently.
