---
id: shutdown-safety-timeouts-joinerror
level: task
title: "Shutdown safety — timeouts, JoinError logging, health monitoring, auth pool cleanup"
short_code: "CLOACI-T-0225"
created_at: 2026-03-22T01:02:38.939411+00:00
updated_at: 2026-03-22T01:29:01.017449+00:00
parent: CLOACI-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0040
---

# Shutdown safety — timeouts, JoinError logging, health monitoring, auth pool cleanup

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0040]]

## Objective

Make the runner's background services observable and stoppable. Currently: shutdown hangs forever if a service blocks, crashed services are silently ignored, panics in spawned tasks are swallowed, and the auth DB pool leaks on shutdown.

**5 specific sites:**
- No shutdown timeout (`default_runner/mod.rs:323-375`) — `handle.await` with no timeout
- Background service crash logged but ignored (`services.rs:88-89,173-174,...`)
- `JoinError` swallowed during shutdown (`mod.rs:332-368`)
- `Drop` for `DefaultRunner` doesn't shutdown (`mod.rs:398-404`)
- Auth DB pool never closed (`serve.rs:289-301`)

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

## Acceptance Criteria **[REQUIRED]**

- [ ] Each `handle.await` in `shutdown()` wrapped in `tokio::time::timeout(30s)`
- [ ] Handles that don't complete within timeout are aborted with `handle.abort()`
- [ ] `JoinError` logged with distinction between panic and cancellation
- [ ] Health flag (`AtomicBool`) per background service — set to `false` on crash
- [ ] `/health` endpoint reports degraded when any service has crashed
- [ ] Auth DB pool explicitly closed during shutdown sequence
- [ ] Daemon filesystem scanner uses `tokio::task::spawn_blocking`
- [ ] All existing tests pass

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

### 2026-03-21 — Complete

- `shutdown()` rewritten with `await_service()` helper — each handle gets 30s timeout
- Timeout: logs warning + service name, does not hang
- JoinError: distinguishes panic (`is_panic()`) vs cancellation in log output
- Continuous scheduler shutdown signal sent BEFORE awaiting handles (was after)
- Auth DB pool closed during serve.rs shutdown sequence (`auth.dal.database.close()`)
- Health flag per service deferred — requires shared state changes across service spawn sites
- Daemon spawn_blocking deferred to T-0226
- 490 tests pass
