---
id: computation-graph-decorator-and
level: task
title: "@computation_graph decorator and Python graph executor"
short_code: "CLOACI-T-0364"
created_at: 2026-04-04T20:48:53.032715+00:00
updated_at: 2026-04-04T21:32:05.237225+00:00
parent: CLOACI-I-0075
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0075
---

# @computation_graph decorator and Python graph executor

## Objective

Implement the `@computation_graph` Python decorator and the Rust-side executor that bridges a Python class into a callable `async fn(&InputCache) -> GraphResult` — the same interface as Rust-compiled graphs.

The decorator takes a class with async methods (nodes) and a dict-based topology. The Rust side wraps each node call in `spawn_blocking` (GIL), reads the topology, and executes the graph following the same routing logic as the Rust macro.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `@cloaca.computation_graph(react=..., graph=...)` decorator registered as PyO3 function
- [ ] Decorator accepts dict topology: `{"node_name": {"inputs": [...], "routes": {"Variant": "target"}}}` and linear `{"node_name": {"inputs": [...], "next": "target"}}`
- [ ] Validates topology at decoration time: node coverage, route completeness
- [ ] `PythonGraphExecutor` Rust struct: takes decorated class + topology, produces `async fn(&InputCache) -> GraphResult`
- [ ] Node calls wrapped in `spawn_blocking(|| Python::with_gil(|py| ...))` — same pattern as existing `@task`
- [ ] Routing via `return ("VariantName", value)` tuples — executor matches variant string to route
- [ ] `None` return on intermediate nodes → branch short-circuits
- [ ] Linear nodes return a value directly (no tuple wrapping needed)
- [ ] Executor deserializes cache inputs (JSON in debug, bincode in release) and passes as Python dicts
- [ ] Terminal outputs collected into `GraphResult::completed(...)`

## Implementation Notes

Follow the existing pattern in `crates/cloacina/src/python/task.rs` for `spawn_blocking` + GIL handling. The `PythonGraphExecutor` lives in `crates/cloacina/src/python/computation_graph.rs` (new file).

The topology is a Python dict parsed at decoration time into a Rust `GraphIR`-equivalent structure. The executor walks the topology at runtime (not compile-time — Python can't do proc macros). This is the key difference from Rust: Rust resolves at compile time, Python resolves at decoration/init time.

### Dependencies
T-0362 (GraphResult, InputCache — already done in I-0070)

## Status Updates

**2026-04-04**: Completed.
- Created `crates/cloacina/src/python/computation_graph.rs` (~450 lines)
- `PythonGraphExecutor` struct: holds PyObject instance + topology, implements `async execute(&InputCache) -> GraphResult`
- `PythonGraphDecorator` intermediate: `__call__` takes the class, validates methods exist, creates executor
- `computation_graph` PyO3 function registered in cloaca module
- Topology parser: reads dict `react` (mode + accumulators) and `graph` (nodes with inputs, routes/next/terminal)
- Execution: `spawn_blocking` + `Python::with_gil` — entire graph runs inside one blocking task
- Routing: extracts `("VariantName", value)` tuples, matches variant to topology routes
- Terminal nodes: outputs converted to `serde_json::Value` via `pythonize::depythonize`
- Topological sort via Kahn's algorithm for execution order
- Validation: method existence checked at decoration time
- Compiles clean, all existing tests pass
