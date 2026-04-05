---
id: state-accumulator-dal-vecdeque
level: task
title: "State accumulator DAL — VecDeque persistence on write, load on startup, emit to reactor"
short_code: "CLOACI-T-0409"
created_at: 2026-04-05T21:24:23.557396+00:00
updated_at: 2026-04-05T21:24:23.557396+00:00
parent: CLOACI-I-0081
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0081
---

# State accumulator DAL — VecDeque persistence on write, load on startup, emit to reactor

## Parent Initiative

[[CLOACI-I-0081]]

## Objective

Implement the State accumulator class with full DAL persistence as described in S-0004. The state accumulator holds a bounded `VecDeque<T>` that receives values from the computation graph (collector or mid-graph writes), persists to DAL on every write, and loads from DAL on startup. This enables cyclic state patterns where the graph's output feeds back as input on the next execution.

## Acceptance Criteria

- [ ] `StateAccumulator<T: Boundary>` struct with `VecDeque<T>` buffer and configurable capacity
- [ ] On receive: append to buffer, evict oldest if `len > capacity`, persist full `VecDeque` to DAL via `state_accumulator_buffers` table
- [ ] On startup (`init()`): load `VecDeque` from DAL, emit current list as boundary to reactor immediately
- [ ] Capacity modes: `capacity = N` (bounded), `capacity = -1` (unbounded), no capacity (write-only sink, no history emitted)
- [ ] `#[state_accumulator(capacity = N)]` macro generates `StateAccumulator<T>` with DAL persistence wiring
- [ ] Writes from collector/nodes arrive via the accumulator's receive socket (same channel as external pushes)
- [ ] DAL persistence uses `state_accumulator_buffers` table from T-0407
- [ ] Unit tests: append + evict + persist round-trip, capacity enforcement, load-on-startup
- [ ] Unit tests: capacity = 1 (last output only), capacity = -1 (unbounded growth)
- [ ] Integration test: restart state accumulator, verify VecDeque restored and emitted to reactor

## Implementation Notes

### Technical Approach

The state accumulator is unique — it has no external source and no event loop. It only receives writes from the computation graph itself. The `process()` function appends to the VecDeque, evicts if over capacity, persists, and emits the full list.

```rust
struct StateAccumulator<T: Boundary> {
    buffer: VecDeque<T>,
    capacity: i32,  // -1 = unbounded, 0 = write-only, N = bounded
    checkpoint: CheckpointHandle,
}
```

**Persistence strategy**: persist after every write (append + evict + persist). This is different from other accumulators which checkpoint periodically. State accumulators hold graph-critical cyclic state so every mutation must be durable.

**Macro**: `#[state_accumulator(capacity = 10)]` generates the struct, trait impl, DAL wiring, and the `init()` load path. Follow existing patterns in `accumulator_macros.rs`.

### Key files
- `crates/cloacina/src/computation_graph/accumulator.rs` — `StateAccumulator` struct and runtime
- `crates/cloacina-macros/src/computation_graph/accumulator_macros.rs` — `#[state_accumulator]` macro
- Uses `state_accumulator_buffers` DAL table from T-0407

### Dependencies
- T-0407 (DAL foundation) — needs `state_accumulator_buffers` table and `CheckpointHandle`

### Risk Considerations
- Unbounded mode (`capacity = -1`) can grow without limit — document the risk, don't prevent it
- Persistence on every write could be slow for high-frequency state updates — acceptable for the target use case (cyclic state updated once per graph execution, not per-event)

## Status Updates

*To be added during implementation*
