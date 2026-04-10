---
id: sequential-input-strategy
level: task
title: "Sequential input strategy — implementation, tests, and tutorial"
short_code: "CLOACI-T-0394"
created_at: 2026-04-05T15:12:44.688158+00:00
updated_at: 2026-04-05T15:24:52.690962+00:00
parent: CLOACI-I-0077
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0077
---

# Sequential input strategy — implementation, tests, and tutorial

## Objective

Implement the `Sequential` input strategy. Currently the reactor always uses `Latest` — each boundary overwrites the cache entry for that source, collapsing intermediate values. `Sequential` preserves every boundary in order: the reactor maintains a FIFO queue and processes one boundary per graph execution. No data loss.

Use case: audit trails, ordered event processing, fill-by-fill reconciliation — any scenario where every individual event must be processed and none can be skipped.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Reactor respects `InputStrategy::Sequential` — maintains a `VecDeque<(SourceName, Vec<u8>)>` queue
- [ ] Each boundary received is enqueued (not overwritten)
- [ ] On fire: dequeue one boundary, update cache with that single boundary, execute graph
- [ ] If queue has more items after execution, immediately re-fire (drain queue)
- [ ] Fire count equals number of boundaries pushed (no collapsing)
- [ ] `Latest` strategy continues to work unchanged (regression test)
- [ ] Unit test: push 5 rapid boundaries → graph fires 5 times with Sequential, 1 time with Latest
- [ ] Integration test: sequential reactor preserves order — output values match input order
- [ ] Tutorial example in `examples/tutorials/computation-graphs/library/12-sequential/`
- [ ] All existing tests continue to pass

## Implementation Notes

### Approach
The reactor's receiver task currently does:
```rust
cache.update(source, bytes);
dirty.set(source, true);
```

For Sequential, instead of updating cache directly, push `(source, bytes)` onto a queue. The executor task dequeues one at a time:
```rust
if input_strategy == Sequential {
    while let Some((source, bytes)) = queue.pop_front() {
        cache.update(source, bytes);
        let snapshot = cache.snapshot();
        (graph)(snapshot).await;
    }
}
```

Key design decision: does `Sequential` interact with `WhenAll`? Probably not — `Sequential` + `WhenAll` is contradictory (you can't wait for all sources if you're processing one boundary at a time). `Sequential` should only be used with `WhenAny`.

### Files
- `crates/cloacina/src/computation_graph/reactor.rs` — add queue, modify executor for Sequential
- `crates/cloacina/tests/integration/computation_graph.rs` — integration tests
- `examples/tutorials/computation-graphs/library/12-sequential/` — tutorial

### Dependencies
T-0393 (when_all should be done first so we don't break it)

## Status Updates

- 2026-04-05: Complete. Receiver branches on InputStrategy: Latest updates cache+dirty (unchanged), Sequential pushes to VecDeque. Executor drains queue one at a time for Sequential, uses dirty flags for Latest. Integration test: 5 rapid events → 5 fires with correct ordered outputs (12.0, 14.0, 16.0, 18.0, 20.0). All 9 integration tests pass. Tutorials deferred.
