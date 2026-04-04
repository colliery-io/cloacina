---
id: python-computation-graph-example
level: task
title: "Python computation graph example and tests"
short_code: "CLOACI-T-0366"
created_at: 2026-04-04T20:48:55.052484+00:00
updated_at: 2026-04-04T21:32:34.285239+00:00
parent: CLOACI-I-0075
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0075
---

# Python computation graph example and tests

## Objective

Working Python computation graph example (mirrors the Rust example from I-0070) and tests that verify routing, conditional propagation, and spawn_blocking wrapping.

## Acceptance Criteria

## Acceptance Criteria

- [ ] Python example in `examples/tutorials/python/` or `examples/features/python-computation-graph/`: class-based graph with dict topology, routing, terminal nodes
- [ ] Example mirrors the Rust linear chain + routing structure (decision → signal/no-action paths)
- [ ] Example callable with test data — constructs InputCache from Python, calls executor, gets GraphResult
- [ ] Test: linear chain — values pass through nodes correctly
- [ ] Test: routing — correct branch taken based on return tuple variant
- [ ] Test: None return → branch short-circuits
- [ ] Test: all node calls go through spawn_blocking (verify no GIL deadlocks under concurrent load)
- [ ] Test: error handling — node raises exception → GraphResult::Error
- [ ] Tests runnable via `angreal cloaca test` or similar
- [ ] All existing tests continue to pass

## Implementation Notes

Tests use the `cloaca` wheel (built via `angreal cloaca package`). The test pattern follows existing Python tutorials — build wheel, create test venv, run pytest.

The example should look like:
```python
@cloaca.computation_graph(
    react={"mode": "when_any", "accumulators": ["alpha", "beta"]},
    graph={
        "decision": {"inputs": ["alpha", "beta"], "routes": {
            "Signal": "output_handler",
            "NoAction": "audit_logger",
        }},
    }
)
class MyStrategy:
    async def decision(self, alpha, beta):
        if alpha["value"] + beta["estimate"] > 10:
            return ("Signal", {"output": alpha["value"] + beta["estimate"]})
        else:
            return ("NoAction", {"reason": "below threshold"})

    async def output_handler(self, signal):
        return {"published": True, "value": signal["output"]}

    async def audit_logger(self, reason):
        return {"logged": True}
```

### Dependencies
T-0364 (decorator + executor), T-0365 (accumulator decorators — optional for this task, graph can be tested without accumulators)

## Status Updates

*To be added during implementation*
