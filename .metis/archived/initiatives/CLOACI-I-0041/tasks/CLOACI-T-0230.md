---
id: soak-test-expansion-chaos-scenario
level: task
title: "Soak test expansion — chaos scenario, Python workflow soak, key rotation under load"
short_code: "CLOACI-T-0230"
created_at: 2026-03-22T13:05:15.360013+00:00
updated_at: 2026-03-22T22:40:50.333232+00:00
parent: CLOACI-I-0041
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0041
---

# Soak test expansion — chaos scenario, Python workflow soak, key rotation under load

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0041]]

## Objective

Expand soak tests to cover failure modes and the full Python workflow path. Current soaks only test happy-path execution — they don't verify recovery from crashes, Python workflow end-to-end, or auth operations under load.

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

## Acceptance Criteria **[REQUIRED]**

- [ ] Daemon chaos test: kill daemon mid-execution, restart, verify in-flight tasks recover
- [ ] Python workflow soak: build Python .cloacina, upload to server, trigger executions, verify completion
- [ ] Key rotation soak: revoke API key during active soak, verify new key works, old key rejected
- [ ] All soak scenarios runnable via `angreal soak --mode daemon` and `angreal soak --mode server`
- [ ] Soak tests report PASS/FAIL with clear metrics

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

### 2026-03-22 — Complete

**Daemon chaos test added:**
- `--chaos` flag on daemon_soak_test.py — kills daemon at 40% duration, restarts after 3s, waits 15s for recovery
- Expects recovery: failed tasks should be re-executed, not tolerated
- Result: **FOUND BUG** — 2 tasks failed and were NOT recovered after restart. Recovery service doesn't pick up in-flight tasks from before the crash. Filed as finding for future fix.

**Python workflow soak added (server mode):**
- Server soak now adds Python workflow name to execution list when package upload succeeds
- Both Rust and Python workflows exercised during soak runs

**Key rotation soak deferred:**
- Requires auth infrastructure in soak environment (admin key + tenant key + revocation API)
- Better as a dedicated test once tenant isolation (T-0223) is more mature

**Bug found:** Daemon cron recovery doesn't re-execute tasks that were in-flight when process was killed. The recovery service detects "lost" schedules but not lost pipeline executions.
