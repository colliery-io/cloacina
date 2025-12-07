---
id: split-task-scheduler-rs-into
level: task
title: "Split task_scheduler.rs into module hierarchy"
short_code: "CLOACI-T-0026"
created_at: 2025-12-07T01:16:44.805554+00:00
updated_at: 2025-12-07T03:32:59.249015+00:00
parent: CLOACI-I-0017
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0017
---

# Split task_scheduler.rs into module hierarchy

## Objective

Split `src/task_scheduler.rs` (1,781 lines) into a module hierarchy with focused, single-responsibility files.

## Current State

The file mixes multiple responsibilities:
- TaskScheduler struct and lifecycle
- Scheduling loop and polling logic
- Task state transitions and dependency checking
- Trigger rule evaluation and context validation
- Recovery event handling and orphan detection
- Context merging for dependencies
- TriggerRule and ValueOperator enums (114 lines of enums)

## Target Structure

```
src/task_scheduler/
  mod.rs              (~200 lines - TaskScheduler struct, public API)
  scheduler_loop.rs   (~400 lines - run_scheduling_loop, polling)
  state_manager.rs    (~350 lines - state transitions, readiness checks)
  recovery.rs         (~250 lines - orphan detection, recovery events)
  trigger_rules.rs    (~300 lines - TriggerRule, TriggerCondition, ValueOperator)
  context_manager.rs  (~200 lines - context loading, merging)
```

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Create `src/task_scheduler/` directory with module files
- [ ] Move scheduling loop logic to `scheduler_loop.rs`
- [ ] Move state transition logic to `state_manager.rs`
- [ ] Move recovery logic to `recovery.rs`
- [ ] Move TriggerRule enums and evaluation to `trigger_rules.rs`
- [ ] Move context merging to `context_manager.rs`
- [ ] Update `mod.rs` with re-exports maintaining public API
- [ ] All existing tests pass
- [ ] `cargo check` passes
- [ ] No public API changes

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
