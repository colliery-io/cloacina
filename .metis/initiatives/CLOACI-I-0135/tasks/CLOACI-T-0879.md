---
id: dal-twin-collapse-audit-confirm
level: task
title: "DAL twin-collapse audit — confirm only the genuinely-divergent ops remain as twins"
short_code: "CLOACI-T-0879"
created_at: 2026-07-08T23:03:29.825707+00:00
updated_at: 2026-07-09T00:39:17.084873+00:00
parent: CLOACI-I-0135
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0135
---

# DAL twin-collapse audit — confirm only the genuinely-divergent ops remain as twins

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0135]]

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

### 2026-07-09 — AUDIT PASS + full I-0135 sweep record (commit d8b68a3b; spike 325bda82)

**Sweep complete.** Every backend-agnostic diesel twin in the DAL now routes through `interact_on_backend!` (written once, monomorphized per backend). Areas + line deltas:
- security/db_key_manager.rs (spike): 1835→1216
- registry/workflow_registry/database.rs 3009→2013, dal/workflow_registry_storage.rs 238→113
- dal/task_execution/* (crud/queries/recovery/state/claiming): ~1640 removed, 24 pairs
- dal/schedule + schedule_execution: crud.rs 1564→121, schedule_execution/crud.rs DELETED; ~23 pairs
- dal/unified remainder (10 files): 9263→4835, 87 pairs
- security/{package_signer,secret_store}: 1257→1095, 1035→703; 10 pairs
**Sweep commit d8b68a3b: 23 files, net −9366 lines.** Grand total incl. spike ≈ −9,980.

**AUDIT — the acceptance check.** On the settled tree: `grep 'fn *_postgres|_sqlite'` across dal + security + registry → **exactly 12 twin methods = 6 pairs, and 0 in security, 0 stray.** Those 6 are the genuinely backend-divergent ops, each deliberate:
1. `task_execution/claiming.rs::claim_ready_task` — `FOR UPDATE SKIP LOCKED` (pg) vs `BEGIN IMMEDIATE` + busy-retry (sqlite).
2. `task_execution/state.rs::mark_ready` — pg lets the DB stamp `created_at` (claim filter + write share one clock, avoids skew) vs sqlite app-clock.
3. `schedule/crud.rs::claim_and_update_cron` — `SET TRANSACTION ISOLATION LEVEL SERIALIZABLE` (pg) vs sqlite single-writer.
4. `execution_event.rs::create` — `RETURNING` readback (pg) vs insert-then-select (sqlite).
5. `task_execution_metadata.rs::upsert_task_execution_metadata` — `on_conflict.do_update` (pg) vs check-then-branch (sqlite).
6. `reactor_subscriptions.rs::subscribe` — native upsert (pg) vs try-insert/catch-UniqueViolation (sqlite).

**Verified on the settled combined tree (re-run by orchestrator):** `cargo check -p cloacina` clean for sqlite-only / postgres-only / postgres+sqlite; `cargo test -p cloacina --lib --features sqlite,macros -- dal::unified security:: registry::workflow_registry` → **269 passed, 0 failed**; `cargo fmt --all --check` clean.

**Decomposition gap found + fixed mid-sweep:** the original tasks scoped security/ to only db_key_manager; package_signer.rs + secret_store.rs still had twins. Caught by this audit's twin-count, filed + fixed as [[CLOACI-T-0880]]. Lesson: the twin-count grep is the real completeness check — a per-area task list can miss files.

AUDIT RESULT: **PASS.** Only the 6 intended divergent pairs remain; the backend seam now lives in one macro.