---
id: latearrivalpolicy-tests-for
level: task
title: "LateArrivalPolicy tests for Discard, Retrigger, and RouteToSideChannel"
short_code: "CLOACI-T-0146"
created_at: 2026-03-15T14:39:33.771511+00:00
updated_at: 2026-03-15T14:48:26.089682+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# LateArrivalPolicy tests for Discard, Retrigger, and RouteToSideChannel

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

The scheduler has `LateArrivalPolicy` routing code for all 4 variants, but only `AccumulateForward` (the default) is exercised in any test. `Discard`, `Retrigger`, and `RouteToSideChannel` are untested code paths. Need unit tests that configure edges with each policy and verify correct behavior when a late boundary arrives.

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

## Acceptance Criteria

- [ ] Test: Discard policy — late boundary dropped, accumulator buffer unchanged
- [ ] Test: AccumulateForward — late boundary forwarded to accumulator (current behavior, verify explicitly)
- [ ] Test: Retrigger — late boundary forwarded to accumulator for re-processing
- [ ] Test: RouteToSideChannel — late boundary routed to designated task (or side-channel buffer)
- [ ] Tests must create an accumulator that has already drained (consumer watermark set), then send a boundary that falls behind it

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

- Added `setup_scheduler_with_watermark(policy)` helper: creates graph, drains once to set consumer watermark, sets boundary ledger coverage
- 5 new tests in scheduler:
  - `test_late_arrival_discard_drops_boundary` — boundary dropped, task doesn't fire
  - `test_late_arrival_accumulate_forward` — boundary forwarded, task fires
  - `test_late_arrival_retrigger` — boundary forwarded for re-execution
  - `test_late_arrival_route_to_side_channel` — boundary NOT in accumulator
  - `test_non_late_boundary_passes_through_regardless_of_policy` — non-late boundary unaffected by Discard policy
- Tests set GraphEdge.late_arrival_policy after assembly (field is pub)
- 10 total scheduler tests passing
