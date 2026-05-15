---
id: port-event-triggers-example-to
level: task
title: "Port event-triggers example to runtime-scoped trigger registry"
short_code: "CLOACI-T-0606"
created_at: 2026-05-15T16:39:00.861852+00:00
updated_at: 2026-05-15T16:39:00.861852+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Port event-triggers example to runtime-scoped trigger registry

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

`examples/features/workflows/event-triggers` no longer builds. It uses the legacy free-function trigger registry — `cloacina::trigger::register_trigger` and `cloacina::trigger::get_trigger` — which were removed when triggers moved to the runtime-scoped registry (`Runtime::register_trigger`). It also imports `cloacina::TriggerError` instead of `cloacina_workflow::TriggerError`.

The demo concept (user-defined poll triggers with context passing, deduplication, audit trails) is still valuable. The README is the spec for what the demo should look like — port the code to current API without changing what it teaches.

## What to do

1. **Imports** — change `src/triggers.rs` to `use cloacina_workflow::{Trigger, TriggerError, TriggerResult, Context};` (or whatever the current canonical path is). Drop the `cloacina::trigger::*` imports.
2. **Registration** — `main.rs` calls `register_trigger(...)` against the old global registry. Replace with `runner.runtime().register_trigger(name, factory)` or whatever the runtime-scoped API is today (check the cron-scheduling example for the current pattern).
3. **Schedule wiring** — the example uses `get_trigger("file_watcher")` to fetch the trigger when registering a `NewSchedule`. Either get the trigger off the runtime or build the schedule without needing a back-reference (the schedule only needs the trigger name).
4. **Smoke test** — `cargo build --all-targets` in the example dir, then `angreal demos features event-triggers` runs the demo end-to-end.

## Acceptance

- [ ] `cd examples/features/workflows/event-triggers && cargo build --all-targets` succeeds
- [ ] `angreal demos features event-triggers` runs and shows triggers firing in the logs
- [ ] README still accurately reflects the demo behaviour

## References

- Trait def: `crates/cloacina-workflow/src/trigger.rs::Trigger`
- Working reference for current API: `examples/features/workflows/cron-scheduling`, `examples/features/workflows/packaged-triggers`
- Removed: `cloacina::trigger::{register_trigger, get_trigger}` (global functions, dropped during I-0096 runtime registry unification)

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

*To be added during implementation*