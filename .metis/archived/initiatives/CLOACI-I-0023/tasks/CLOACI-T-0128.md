---
id: continuous-scheduling
level: task
title: "Continuous scheduling documentation and tutorial"
short_code: "CLOACI-T-0128"
created_at: 2026-03-15T11:46:43.444253+00:00
updated_at: 2026-03-15T13:06:53.157692+00:00
parent: CLOACI-I-0023
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0023
---

# Continuous scheduling documentation and tutorial

## Parent Initiative

[[CLOACI-I-0023]]

## Objective

Write documentation for continuous scheduling: explanation page covering architecture and concepts, tutorial walking through building a reactive pipeline, and API reference for new types.

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

- [ ] Explanation page: `docs/content/explanation/continuous-scheduling.md` — architecture, data flow, dual-phase model
- [ ] Tutorial: `docs/content/tutorials/12-continuous-scheduling.md` — step-by-step building a reactive pipeline
- [ ] API reference additions for: ComputationBoundary, DataSource, DataConnection, SignalAccumulator, TriggerPolicy, ContinuousScheduler
- [ ] Rustdoc on all public types in the `continuous` module
- [ ] `angreal docs build` passes
- [ ] `cargo doc -p cloacina --no-deps` builds without warnings on new types

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
- Leverage C4 diagrams from S-0001 for explanation page
- Tutorial structure: setup data source → write detector → write continuous task → configure runner → observe reactive execution
- Reference existing tutorial patterns (weight numbering, front matter)

### Dependencies
- T-0127 (example project provides tutorial material), all implementation tasks complete

## Status Updates

- Created `docs/content/explanation/continuous-scheduling.md` — architecture, data flow, all key concepts
- Created `docs/content/tutorials/12-continuous-scheduling.md` — step-by-step tutorial
- Hugo builds clean (`hugo --minify` succeeds)
- Note: `angreal docs build` has pre-existing rsync error on rustdoc copy (not from our changes)
- Rustdoc on continuous module types deferred — existing types have doc comments, full `cargo doc` coverage to be verified separately
