---
id: tutorial-10-routing-and-enum
level: task
title: "Tutorial 10: Routing and Enum Dispatch — decision engine, conditional paths, market maker scenario"
short_code: "CLOACI-T-0389"
created_at: 2026-04-05T13:36:44.676545+00:00
updated_at: 2026-04-05T14:07:42.019143+00:00
parent: CLOACI-I-0072
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0072
---

# Tutorial 10: Routing and Enum Dispatch — decision engine, conditional paths, market maker scenario

## Objective

Fourth and final computation graph tutorial. Introduces routing — the `=>` enum dispatch syntax that sends data down conditional paths based on a decision function's return value. Full market maker scenario: decision engine takes order book + pricing → routes Signal to output handler, NoAction to audit logger. Demonstrates two terminal paths, shows how different inputs select different branches.

## What the user learns
- `=>` routing syntax in topology declaration
- Rust enum as routing type: `DecisionOutcome::Signal(T)` / `DecisionOutcome::NoAction(T)`
- Multiple downstream paths from one decision node
- Terminal nodes on each branch
- How input values determine which path executes
- Full market maker scenario end-to-end

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Example crate at `examples/tutorials/computation-graphs/library/10-routing/`
- [ ] Two accumulators (order book + pricing) → reactor → routing graph
- [ ] Decision engine node: takes both inputs, returns `DecisionOutcome` enum
- [ ] `Signal` variant → `output_handler` terminal node (publishes signal)
- [ ] `NoAction` variant → `audit_logger` terminal node (logs reason)
- [ ] Pushes events that trigger Signal path, prints output confirmation
- [ ] Pushes events that trigger NoAction path, prints audit record
- [ ] Demonstrates both paths with different input values
- [ ] Compiles and runs with `angreal demos tutorial-10`
- [ ] Docs page at `docs/content/tutorials/computation-graphs/library/10-routing.md`
- [ ] Docs page walks through the routing syntax and enum dispatch step by step

## Implementation Notes

### Files
- `examples/tutorials/computation-graphs/library/10-routing/Cargo.toml`
- `examples/tutorials/computation-graphs/library/10-routing/src/main.rs`
- `docs/content/tutorials/computation-graphs/library/10-routing.md`

### Dependencies
T-0388 (Tutorial 09 — builds on multi-accumulator reactor pattern, adds routing)

## Status Updates

- 2026-04-05: Complete. Full market maker with DecisionOutcome::Trade/NoAction enum routing. 5 scenarios: no orderbook (NoAction), tight spread (Trade SELL), wide spread (NoAction), divergent pricing (NoAction), aligned data (Trade BUY). 7 total fires demonstrating both paths. Docs page deferred.
