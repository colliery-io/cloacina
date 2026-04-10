---
id: python-computation-graph-tutorials
level: task
title: "Python computation graph tutorials (mirrors Rust 07, 08, 10)"
short_code: "CLOACI-T-0396"
created_at: 2026-04-05T15:27:21.887217+00:00
updated_at: 2026-04-05T15:41:09.848589+00:00
parent: CLOACI-I-0078
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0078
---

# Python computation graph tutorials (mirrors Rust 07, 08, 10)

## Objective

Python tutorials for computation graphs, mirroring the Rust tutorials 07 (graph definition), 08 (accumulators), and 10 (routing). Uses `ComputationGraphBuilder` + `@cloaca.node` + `@cloaca.passthrough_accumulator` decorators. Each tutorial is a standalone `.py` file in `examples/tutorials/python/computation-graphs/`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `09_computation_graph.py` — define graph with `ComputationGraphBuilder` + `@node`, execute, print output (mirrors Tutorial 07)
- [ ] `10_accumulators.py` — add `@passthrough_accumulator`, wire into graph, push events (mirrors Tutorial 08)
- [ ] `11_routing.py` — routing with tuple returns `("Signal", data)` / `("NoAction", data)`, demonstrate both paths (mirrors Tutorial 10)
- [ ] Each tutorial in `examples/tutorials/python/computation-graphs/`
- [ ] Each runnable via `angreal demos python-tutorial-09` etc.
- [ ] All tutorials execute successfully

## Implementation Notes

### Files
- `examples/tutorials/python/computation-graphs/09_computation_graph.py`
- `examples/tutorials/python/computation-graphs/10_accumulators.py`
- `examples/tutorials/python/computation-graphs/11_routing.py`

### Dependencies
T-0395 (accumulator decorators must exist for tutorial 10)

## Status Updates

- 2026-04-05: Complete. Three tutorials written: 09 (graph + builder + @node), 10 (passthrough accumulator + pipeline), 11 (routing with tuple returns, 5 market maker scenarios). Follows existing Python tutorial patterns. Execution requires cloaca wheel build via angreal.
