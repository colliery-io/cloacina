---
id: daemon-crash-recovery-in-flight
level: task
title: "Daemon crash recovery — in-flight pipeline executions not re-executed after restart"
short_code: "CLOACI-T-0232"
created_at: 2026-03-22T22:40:38.509272+00:00
updated_at: 2026-03-22T22:40:38.509272+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Daemon crash recovery — in-flight pipeline executions not re-executed after restart

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

**ESCALATED TO INITIATIVE SCOPE.** What started as a daemon crash recovery bug revealed systemic issues in the entire recovery system.

The recovery system was designed for a pull-based execution model (executor polls outbox and claims tasks). The architecture shifted to push-based (dispatcher sends events directly to executor). This broke fundamental assumptions:

1. **RecoveryManager is dead code** — only runs once at startup, never again
2. **Executor never marks tasks "Running"** — push model skips the claiming step entirely
3. **Outbox entry not recreated on recovery** — recovered tasks invisible to dispatcher
4. **Pipeline marked Completed even if tasks failed**
5. **Same bug affects server mode** — not daemon-specific

### Priority
- [x] P1 - High (blocks production reliability)

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

### 2026-03-22 — Deep audit, escalated to initiative

**Investigation path:**
1. Found RecoveryManager dead code — wired into scheduler loop
2. Discovered tasks never reach "Running" state in push model — recovery finds nothing
3. Tried pipeline re-queuing on startup — user correctly identified pipelines aren't queued
4. Deep audit revealed architecture mismatch: pull-based recovery vs push-based execution
5. Same issue affects server mode (not just daemon)

**Three design agents launched for unified recovery redesign:**
- Server mode (multi-instance, PostgreSQL, horizontal scaling, heartbeat)
- Daemon mode (single-instance, SQLite, startup recovery)
- Continuous scheduling (watermarks, exactly-once, boundary ledger)

All designing with future distributed executors in mind.

**Critical design requirement (from user):**
Future distributed executors are NOT pulling from a queue — they're autonomous processes on potentially different networks entirely. The scheduler has no direct access to the executor process. Therefore:
- **Task-level heartbeating is required** — executor periodically writes "still alive" for each task it's executing
- **Stale heartbeat = orphaned** — no heartbeat for N seconds means task is abandoned (crash, network partition, executor killed)
- **No co-location assumptions** — can't check PID, share memory, or send signals
- Recovery must work purely via database state (heartbeat timestamps)

This is the foundational primitive: all recovery (server, daemon, continuous) should be built on task heartbeat expiry.

**Status:** Waiting for design agents, then synthesizing into initiative specification with heartbeat-based recovery as the core primitive.
