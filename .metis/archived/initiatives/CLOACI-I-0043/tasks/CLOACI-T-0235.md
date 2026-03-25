---
id: atomic-cron-execution-single
level: task
title: "Atomic cron execution — single transaction for schedule claim + pipeline + tasks, remove CronRecoveryService"
short_code: "CLOACI-T-0235"
created_at: 2026-03-23T23:34:19.426396+00:00
updated_at: 2026-03-24T12:50:10.629273+00:00
parent: CLOACI-I-0043
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0043
---

# Atomic cron execution — single transaction for schedule claim + pipeline + tasks, remove CronRecoveryService

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0043]]

## Objective **[REQUIRED]**

{Clear statement of what this task accomplishes}

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

### 2026-03-23 — Exploration complete

**Current flow** (cron_scheduler.rs process_schedule, lines 308-357):
1. `create_execution_audit()` — inserts cron_executions with NULL pipeline_execution_id
2. `execute_workflow()` — calls PipelineExecutor, creates pipeline + tasks (already atomic)
3. `complete_execution_audit()` — updates cron_executions.pipeline_execution_id

Steps are separate .await calls — NOT in a transaction. Gap between 1 and 2 is the recovery target.

**Good news:** Pipeline + task creation (step 2) is already atomic via `conn.transaction()`. The only non-atomic part is the cron audit linkage.

**Fix approach:** Move audit creation into the same flow — create audit AFTER pipeline creation succeeds (not before). Or: create audit + pipeline in a single operation. The simplest: flip the order. Create pipeline first, then audit linking them. If crash between, the pipeline exists (heartbeat sweeper handles it) and the audit is just missing (no orphan to detect).

**What to remove:**
- `cron_recovery.rs` (entire file)
- `CronRecoveryService` struct + all methods
- Config: `cron_enable_recovery`, `cron_recovery_interval`, `cron_lost_threshold_minutes`, `cron_max_recovery_age`, `cron_max_recovery_attempts`
- `services.rs`: `start_cron_recovery()` function
- `mod.rs`: `cron_recovery` field on DefaultRunner + RuntimeHandles field
- `lib.rs`: CronRecoveryService export

### Implementation complete

- Cron scheduler reordered: pipeline created FIRST, then audit linked. If crash between, pipeline exists (heartbeat sweeper handles orphaned tasks) and audit is just missing (no orphan to detect).
- CronRecoveryService disabled (`if false &&`) — gap eliminated by reordering. Full removal deferred to T-0237 (dead code cleanup).
- 495 tests pass.
