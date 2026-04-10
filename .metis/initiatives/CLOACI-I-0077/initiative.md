---
id: reactor-depth-when-all-criteria
level: initiative
title: "Reactor Depth — when_all Criteria and Sequential Input Strategy"
short_code: "CLOACI-I-0077"
created_at: 2026-04-05T14:34:55.023637+00:00
updated_at: 2026-04-06T10:56:11.469983+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: S
initiative_id: reactor-depth-when-all-criteria
---

# Reactor Depth — when_all Criteria and Sequential Input Strategy Initiative

## Context

The Reactor currently supports `WhenAny` criteria and `Latest` input strategy. `WhenAll` is declared in the enum and `DirtyFlags::all_set()` exists, but it hasn't been exercised or tested beyond unit tests. `Sequential` input strategy is declared but not implemented — the reactor always overwrites the cache (Latest behavior).

This initiative implements both, adds tutorials, and validates them end-to-end.

Blocked by: nothing. Reactor infrastructure exists from I-0074.

## Goals & Non-Goals

**Goals:**
- Exercise and validate `when_all` reaction criteria in a real pipeline
- Implement `sequential` input strategy — one boundary per execution, ordered FIFO, no collapsing
- Tutorial demonstrating `when_all` (graph waits until all sources have emitted)
- Tutorial demonstrating `sequential` (process each event individually, no data loss)
- Unit + integration tests for both

**Non-Goals:**
- New accumulator types (I-0073)
- Python bindings (I-0078)
- Soak/perf (I-0079)
- Sequential queue durability and expected_sources scheduler wiring — deferred to I-0082 (MVP Resilience Wiring: PERSIST-3 sequential queue persistence gap, WIRE-1 with_expected_sources)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `when_all` criteria works end-to-end: reactor waits until all registered sources have dirty flags before firing
- [ ] `when_all` handles initialization: graph doesn't fire until every source has emitted at least once
- [ ] `sequential` input strategy: reactor queues boundaries, processes one per execution, preserves order
- [ ] `sequential` doesn't collapse intermediate values — every boundary results in a graph execution
- [ ] Tutorial or example demonstrating `when_all`
- [ ] Tutorial or example demonstrating `sequential`
- [ ] Unit tests for sequential queue behavior
- [ ] Integration test: multi-source `when_all` pipeline
- [ ] All existing tests continue to pass

## Implementation Plan

1. **`when_all` validation** — may already work via `DirtyFlags::all_set()`. Write an integration test + tutorial that proves it.
2. **`sequential` implementation** — modify reactor executor to use a `VecDeque` of boundaries instead of overwriting cache. Each execution dequeues one boundary.
3. **Tests + tutorials**
