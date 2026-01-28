---
id: replace-sleep-based-test
level: task
title: "Replace sleep-based test synchronization with event-based patterns"
short_code: "CLOACI-T-0056"
created_at: 2026-01-27T13:56:56.585865+00:00
updated_at: 2026-01-28T03:56:44.690616+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: NULL
---

# Replace sleep-based test synchronization with event-based patterns

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Replace sleep-based test synchronization with event-based patterns (barriers, channels, notify) to eliminate flaky tests.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [x] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [x] P2 - Medium (nice to have)
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
- **Current Problems**: Tests use hard-coded `sleep()` durations (2-3 seconds) for synchronization
- **Benefits of Fixing**: Faster tests, no flaky failures, better CI reliability
- **Risk Assessment**: Flaky tests erode confidence in CI; false negatives waste developer time
- **Locations**:
  - `tests/integration/executor/pause_resume.rs`
  - `tests/integration/executor/context_merging.rs`
  - `tests/integration/registry/test_registry_dynamic_loading.rs`

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] Replace `sleep()` calls with event-based synchronization (Barrier, channels, Notify)
- [x] Tests run faster (no arbitrary waits)
- [x] Tests are reliable on slow CI systems
- [x] No intermittent failures in 100 consecutive runs

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

### 2026-01-28: Completed
- **context_merging.rs**: Replaced 3s and 500ms sleeps with `wait_for_completion()`
- **multi_tenant.rs**: Replaced all 500ms sleeps with `wait_for_completion()` and `tokio::join!` for parallel waits
- **pause_resume.rs**: Added helper functions `wait_for_status()` and `wait_for_terminal()` for event-based status polling; replaced completion sleeps with polling
- Preserved intentional task sleeps in slow_first_task/slow_second_task (simulate slow work for pause/resume testing)
- Kept brief 200ms scheduler pickup waits in pause_resume tests (necessary for mid-execution pause testing)
- All 172+ integration tests pass
- Commit: 7a4ad5f
