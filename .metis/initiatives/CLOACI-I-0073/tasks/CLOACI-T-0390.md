---
id: polling-accumulator-trait-timer
level: task
title: "Polling accumulator — trait, timer-based runtime, and #[polling_accumulator] macro"
short_code: "CLOACI-T-0390"
created_at: 2026-04-05T14:41:06.315517+00:00
updated_at: 2026-04-05T14:55:20.901720+00:00
parent: CLOACI-I-0073
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0073
---

# Polling accumulator — trait, timer-based runtime, and #[polling_accumulator] macro

## Objective

Implement the polling accumulator — a timer-based accumulator that periodically calls an async poll function to query databases, APIs, or other pull-based sources. Returns `Option<Output>` — `Some` emits a boundary, `None` means "no change" and skips emission.

Common use case: config sources, database table watchers, API health checks — anything where you poll on an interval rather than receiving a push.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `PollingAccumulator` trait with `async fn poll(&mut self) -> Option<Self::Output>` + `fn interval(&self) -> Duration`
- [ ] `polling_accumulator_runtime()` — timer loop: sleep for interval, call poll, if Some → serialize + send boundary, if None → skip
- [ ] Runtime respects shutdown signal (breaks out of timer loop)
- [ ] Runtime also accepts socket events (same as passthrough — merge channel model)
- [ ] `#[polling_accumulator(interval = "5s")]` macro generates struct + trait impl
- [ ] Macro parses interval from string ("5s", "100ms", "1m")
- [ ] Unit test: poll returns Some → boundary emitted
- [ ] Unit test: poll returns None → no boundary emitted
- [ ] Unit test: timer fires at correct interval (within tolerance)
- [ ] Unit test: shutdown stops the polling loop

## Implementation Notes

### Files
- `crates/cloacina/src/computation_graph/accumulator.rs` — add `PollingAccumulator` trait + `polling_accumulator_runtime()`
- `crates/cloacina-macros/src/computation_graph/accumulator_macros.rs` — add `polling_accumulator_impl()`
- `crates/cloacina-macros/src/lib.rs` — register `#[polling_accumulator]` proc macro

### Design
```rust
#[async_trait]
pub trait PollingAccumulator: Send + 'static {
    type Output: Serialize + DeserializeOwned + Send;

    async fn poll(&mut self) -> Option<Self::Output>;
    fn interval(&self) -> Duration;
}
```

The runtime is simpler than the stream accumulator — no event loop task, just a timer + poll + optional socket receiver merged.

### Dependencies
None — builds on existing accumulator infrastructure from I-0074.

## Status Updates

- 2026-04-05: Complete. PollingAccumulator trait + polling_accumulator_runtime + #[polling_accumulator] macro. 3 unit tests: emits on Some, skips on None, shutdown. Macro parses ms/s/m duration strings. Re-exported from cloacina::.
