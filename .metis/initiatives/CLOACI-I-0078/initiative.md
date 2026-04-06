---
id: python-computation-graph-depth
level: initiative
title: "Python Computation Graph Depth — Accumulator Decorators and Tutorials"
short_code: "CLOACI-I-0078"
created_at: 2026-04-05T14:34:56.356757+00:00
updated_at: 2026-04-06T10:56:13.927045+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: python-computation-graph-depth
---

# Python Computation Graph Depth — Accumulator Decorators and Tutorials Initiative

## Context

Python computation graph bindings exist (I-0075): `ComputationGraphBuilder` context manager + `@cloaca.node` decorator + `PythonGraphExecutor`. But Python accumulator decorators (`@stream_accumulator`, `@passthrough_accumulator`) are not yet implemented (T-0365 is todo). Python tutorials for computation graphs don't exist yet.

This initiative completes the Python DX for computation graphs: accumulator decorators, tutorials mirroring Rust tutorials 07-10, and end-to-end tests.

Blocked by: nothing. Python graph bindings exist from I-0075.

## Goals & Non-Goals

**Goals:**
- Implement Python accumulator decorators: `@passthrough_accumulator`, `@stream_accumulator`
- Python tutorials mirroring Rust tutorials 07-10 (in `tutorials/python/computation-graphs/`)
- End-to-end Python tests proving decorator → graph executor → output
- Complete T-0365 (Python accumulator decorators)

**Non-Goals:**
- Rust accumulator types (I-0073)
- Reactor features (I-0077)
- Soak/perf (I-0079)
- Server mode / packaging for Python graphs

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `@passthrough_accumulator` decorator implemented — wraps Python function into Accumulator trait via PyO3
- [ ] `@stream_accumulator` decorator implemented — wraps Python function with stream config
- [ ] Python tutorial: define computation graph with builder + @node (mirrors Tutorial 07)
- [ ] Python tutorial: add accumulator + reactor (mirrors Tutorial 08)
- [ ] Python tutorial: routing with tuple returns (mirrors Tutorial 10)
- [ ] All Python decorators run in `spawn_blocking` (GIL safety)
- [ ] Tests in `computation_graph_tests.rs` for Python accumulators
- [ ] All existing tests pass

## Implementation Plan

1. **Python accumulator decorators** (T-0365) — `@passthrough_accumulator`, `@stream_accumulator` in `cloaca` module
2. **Python tutorials** — 3 tutorials in `tutorials/python/computation-graphs/`
3. **Tests**
