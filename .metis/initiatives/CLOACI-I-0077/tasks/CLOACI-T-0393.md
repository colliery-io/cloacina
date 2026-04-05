---
id: when-all-reaction-criteria
level: task
title: "when_all reaction criteria — validation, integration test, and tutorial"
short_code: "CLOACI-T-0393"
created_at: 2026-04-05T15:12:43.518776+00:00
updated_at: 2026-04-05T15:21:58.531775+00:00
parent: CLOACI-I-0077
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0077
---

# when_all reaction criteria — validation, integration test, and tutorial

## Objective

Validate that `WhenAll` reaction criteria works end-to-end in a real pipeline. The code path exists (`DirtyFlags::all_set()` checked when `ReactionCriteria::WhenAll`), but it has never been tested beyond DirtyFlags unit tests. This task proves it works with real accumulators and a reactor, handles initialization correctly (no fire until all sources emit), and documents it in a tutorial.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Integration test: 2 accumulators + reactor with `WhenAll` — push to source A only → graph does NOT fire
- [ ] Integration test: push to source B → now both dirty → graph fires
- [ ] Integration test: push to A again → only A dirty → graph does NOT fire (WhenAll requires both)
- [ ] Integration test: push to B again → both dirty again → graph fires
- [ ] Verify initialization: graph doesn't fire until every source has emitted at least once
- [ ] Fix any bugs discovered during validation
- [ ] Tutorial example in `examples/tutorials/computation-graphs/library/11-when-all/` demonstrating the behavior
- [ ] All existing tests continue to pass

## Implementation Notes

### Approach
The reactor executor already has the `WhenAll` path:
```rust
ReactionCriteria::WhenAll => d.all_set(),
```

The key question is whether `DirtyFlags` correctly tracks per-source state when the reactor uses `WhenAll`. The `all_set()` method returns false if the flags map is empty, which handles initialization. After firing, `clear_all()` resets all flags — so both sources must emit again before the next fire.

### Potential issue
`DirtyFlags` only tracks sources it has seen. If source B never emits, `all_set()` only checks source A's flag — which would incorrectly return true. The reactor needs to know the *expected* set of sources upfront, not just the sources that have emitted. May need to seed DirtyFlags with expected source names at reactor construction.

### Files
- `crates/cloacina/src/computation_graph/reactor.rs` — may need to seed DirtyFlags
- `crates/cloacina/tests/integration/computation_graph.rs` — integration tests
- `examples/tutorials/computation-graphs/library/11-when-all/` — tutorial

### Dependencies
None.

## Status Updates

- 2026-04-05: Bug found and fixed — DirtyFlags only tracked sources that had emitted, so all_set() returned true with just one source. Added DirtyFlags::with_sources() to pre-seed with expected names. Added Reactor::with_expected_sources() builder. Integration test proves correct WhenAll behavior: A only → no fire, A+B → fire, A only → no fire, A+B → fire. 8 integration tests pass. Tutorial deferred.
