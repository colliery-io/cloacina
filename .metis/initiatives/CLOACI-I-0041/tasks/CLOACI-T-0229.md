---
id: fix-stub-tests-and-add-critical
level: task
title: "Fix stub tests and add critical coverage — cron scheduler, recovery, workflow registry, Python subsystem"
short_code: "CLOACI-T-0229"
created_at: 2026-03-22T13:05:15.300887+00:00
updated_at: 2026-03-22T13:05:15.300887+00:00
parent: CLOACI-I-0041
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0041
---

# Fix stub tests and add critical coverage — cron scheduler, recovery, workflow registry, Python subsystem

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0041]]

## Objective

Replace 5 stub tests with real assertions using `test_dal()`, and add tests for the highest-risk zero-coverage modules. Focus on tests that catch real bugs, not getters.

**Stub tests to fix:** cron_scheduler (3 stubs), cron_recovery (1 stub), workflow_registry (1 stub)
**Zero-test modules:** Python subsystem (1,599 lines), security/db_key_manager (1,200 lines)

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

- [ ] `test_is_schedule_active` has real time-window assertions (not empty)
- [ ] `test_calculate_execution_times_skip_policy` tests Skip returns only current window
- [ ] `test_calculate_execution_times_run_all_policy` tests RunAll returns all missed windows
- [ ] `test_recovery_attempts_tracking` tests actual recovery logic against real DB
- [ ] Workflow registry test: register package → retrieve → verify metadata
- [ ] Python: `ensure_cloaca_module` registers module in sys.modules
- [ ] Python: task decorator registers task in global registry (existing test in mod.rs extended)
- [ ] Python: `validate_no_stdlib_shadowing` rejects package with os.py
- [ ] Security: db_key_manager create + retrieve + revoke lifecycle against real SQLite
- [ ] Zero stub tests remain (grep for "would need" / empty test bodies)
- [ ] All existing tests still pass

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
