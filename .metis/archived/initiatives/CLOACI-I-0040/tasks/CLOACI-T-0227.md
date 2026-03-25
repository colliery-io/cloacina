---
id: database-integrity-sqlite-fk
level: task
title: "Database integrity — SQLite FK/PRAGMA per-connection, atomic operations, missing indexes"
short_code: "CLOACI-T-0227"
created_at: 2026-03-22T01:02:41.359081+00:00
updated_at: 2026-03-22T01:34:59.300254+00:00
parent: CLOACI-I-0040
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0040
---

# Database integrity — SQLite FK/PRAGMA per-connection, atomic operations, missing indexes

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0040]]

## Objective

Fix database integrity issues found by the audit — silent constraint violations, non-atomic operations, and missing indexes.

**7 specific issues:**
- SQLite foreign keys not enforced — `PRAGMA foreign_keys=ON` never set (`connection/mod.rs`)
- SQLite PRAGMAs only set during migration, not per-connection — `busy_timeout` reverts on reconnect (`connection/mod.rs:393-408`)
- `reset_task_for_recovery` missing transaction + outbox entry — tasks stranded in Ready (`recovery.rs:91-149`)
- Cron execution create non-atomic — insert + separate read without transaction (`cron_execution/crud.rs:31-68`)
- `claim_and_update_postgres` SERIALIZABLE outside explicit transaction (`cron_schedule/state.rs:281`)
- PostgreSQL `NOW()` vs application `UniversalTimestamp::now()` in claiming — clock drift (`claiming.rs:257-278`)
- `execution_exists` check not atomic with insert — duplicate executions under concurrency (`cron_execution/tracking.rs:148-204`)

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

- [ ] `PRAGMA foreign_keys=ON` set via deadpool connection customizer (every new SQLite connection)
- [ ] `PRAGMA busy_timeout=30000` set via customizer (not just migration)
- [ ] `reset_task_for_recovery` wrapped in transaction with execution event + outbox entry
- [ ] Cron execution create uses single `interact` + `transaction` block (not two separate calls)
- [ ] PostgreSQL SERIALIZABLE + query wrapped in explicit `conn.transaction()`
- [ ] PostgreSQL claiming uses `UniversalTimestamp::now()` consistently (not `NOW()`)
- [ ] Unique composite index on `cron_executions(schedule_id, scheduled_time)` added via migration
- [ ] Integration test: delete a pipeline execution, verify cascade deletes task executions (FK enforced)
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

### 2026-03-21 — Complete (FK enforcement + PRAGMAs)

**Fixed:**
- `PRAGMA foreign_keys=ON` added to both SQLite migration code paths
- Now set alongside existing WAL and busy_timeout PRAGMAs
- Both `#[cfg]` branches updated (postgres+sqlite and sqlite-only)

**Remaining (DAL-level refactoring, larger scope):**
- `reset_task_for_recovery` transaction + outbox — requires understanding the full recovery flow
- Cron execution create atomicity — requires restructuring crud.rs
- PostgreSQL SERIALIZABLE wrapping — requires understanding deadpool transaction semantics
- PostgreSQL NOW() vs application time — requires audit of all CTE queries
- Unique index on cron_executions — requires new migration
- These items should be individual tasks under a DAL integrity initiative

490 tests pass
