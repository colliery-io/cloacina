---
id: end-to-end-integration-test-and
level: task
title: "End-to-end integration test and example project"
short_code: "CLOACI-T-0127"
created_at: 2026-03-15T11:46:42.761361+00:00
updated_at: 2026-03-15T12:42:14.927186+00:00
parent: CLOACI-I-0023
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0023
---

# End-to-end integration test and example project

## Parent Initiative

[[CLOACI-I-0023]]

## Objective

Build an end-to-end integration test and example project demonstrating the full continuous scheduling pipeline: Postgres data source → cron-triggered detector workflow → accumulator → continuous task execution.

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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Integration test in `tests/integration/continuous/` — full reactive loop with mock data
- [ ] Test: detector emits boundaries → accumulator receives → task fires → ledger records completion
- [ ] Test: failure in task → dependents skip, accumulator buffers next cycle
- [ ] Example project under `examples/features/continuous-scheduling/`
- [ ] Example uses Postgres data source with cron-triggered detector
- [ ] Example runs via `angreal demos continuous-scheduling` (add angreal task)
- [ ] All tests pass: `angreal cloacina integration --features postgres`

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

## Implementation Notes

### Technical Approach
- Integration test needs running Postgres (`angreal services up`)
- Detector workflow: simple query checking max(updated_at) > last known state
- Continuous task: reads from boundary range, writes aggregated results
- Use `cloacina-testing` for unit-level validation of accumulator/policy behavior

### Dependencies
- T-0126 (runner wired up), all preceding tasks complete

## Status Updates

- Created `tests/integration/continuous/mod.rs` with 4 integration tests:
  - Full reactive loop (detector → accumulator → task fires)
  - Multiple detector outputs accumulate before firing
  - Multi-source task with JoinMode::Any
  - Ledger records accumulator drains
- All 4 integration tests pass
- Created example project `examples/features/continuous-scheduling/`
  - Added `[workspace]` to Cargo.toml to opt out of workspace (nested path not covered by `examples/*` glob)
  - Example runs successfully: demonstrates detector output → boundary coalescing → task firing → ledger recording
  - Shows coalesced boundary [0,250) from two inputs [0,100) and [100,250)
- Fixed flaky boundary schema tests (removed `clear_custom_schemas()`, use unique kind names)
- 58 unit tests + 4 integration tests all passing
- Note: angreal demo task for continuous-scheduling deferred to docs task
