---
id: python-accumulator-decorators-all
level: task
title: "Python accumulator decorators — all 4 types (passthrough, stream, polling, batch)"
short_code: "CLOACI-T-0395"
created_at: 2026-04-05T15:27:20.969974+00:00
updated_at: 2026-04-05T15:39:06.682977+00:00
parent: CLOACI-I-0078
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0078
---

# Python accumulator decorators — @passthrough_accumulator and @stream_accumulator

## Objective

Implement Python decorators for accumulators, mirroring the Rust `#[passthrough_accumulator]` and `#[stream_accumulator]` macros. These decorators wrap Python functions into accumulator instances that can be wired into a computation graph pipeline. Completes T-0365.

The pattern mirrors existing `@cloaca.node` — function decorators, not classes.

## Python DX

```python
@cloaca.passthrough_accumulator
def pricing(event: PricingUpdate) -> PricingData:
    return PricingData(estimate=event.mid_price)

@cloaca.stream_accumulator(type="kafka", topic="market.orderbook")
def orderbook(event: OrderBookUpdate) -> OrderBookData:
    return OrderBookData(top_high=event.best_ask, top_low=event.best_bid)

@cloaca.polling_accumulator(interval="5s")
async def config_source() -> Optional[ConfigData]:
    data = await fetch_config()
    return ConfigData(value=data) if data.changed else None

@cloaca.batch_accumulator(flush_interval="1s")
def aggregate_fills(events: list[FillEvent]) -> Optional[AggregatedFills]:
    if not events:
        return None
    return AggregatedFills(total=len(events), volume=sum(e.qty for e in events))
```

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `@cloaca.passthrough_accumulator` decorator — wraps Event → Output function
- [ ] `@cloaca.stream_accumulator(type=..., topic=...)` decorator — wraps with stream config
- [ ] `@cloaca.polling_accumulator(interval=...)` decorator — wraps async poll function returning Optional
- [ ] `@cloaca.batch_accumulator(flush_interval=..., max_buffer_size=...)` decorator — wraps batch processing function
- [ ] Decorated functions registered in a global accumulator registry (parallel to node registry)
- [ ] Each decorator stores: function reference, accumulator name, type, config
- [ ] All accumulator functions called via `spawn_blocking` + `Python::with_gil` (GIL safety)
- [ ] Registered in PyO3 module as `cloaca.passthrough_accumulator`, `cloaca.stream_accumulator`, `cloaca.polling_accumulator`, `cloaca.batch_accumulator`
- [ ] Unit test: decorate each type, verify registered with correct metadata
- [ ] Unit test: call decorated function's process logic, verify output

## Implementation Notes

### Files
- `crates/cloacina/src/python/computation_graph.rs` — add decorator functions + accumulator registry
- `crates/cloacina/src/lib.rs` — register decorators in `cloaca` PyO3 module

### Design
Follow the `@cloaca.node` pattern from I-0075. The decorator:
1. Stores the Python function in a global registry keyed by name
2. Returns the original function (transparent decorator)
3. The `ComputationGraphBuilder.__exit__` can look up accumulators by name

The accumulator registry stores `(name, function, type, config)` tuples.

### Dependencies
None — Python graph bindings already exist from I-0075.

## Status Updates

- 2026-04-05: Complete. All 4 decorators implemented: passthrough (direct decorator), stream/polling/batch (parameterized via PyCFunction closures). Global ACCUMULATOR_REGISTRY with PyAccumulatorRegistration metadata. All registered in cloaca PyO3 module with Python-friendly names. Compiles clean. T-0365 complete.
