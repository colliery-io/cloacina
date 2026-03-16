---
id: persistedaccumulatorstate-model
level: task
title: "PersistedAccumulatorState model, diesel migrations, and DAL"
short_code: "CLOACI-T-0139"
created_at: 2026-03-15T13:51:31.863176+00:00
updated_at: 2026-03-15T14:13:22.117147+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# PersistedAccumulatorState model, diesel migrations, and DAL

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0025]]

## Objective

Create the `PersistedAccumulatorState` model, diesel schema migration for an `accumulator_state` table (Postgres only), and DAL methods for save/load/delete operations. This is the storage layer for accumulator persistence.

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

- [ ] `PersistedAccumulatorState` model: edge_id (PK), consumer_watermark (JSONB), last_drain_at (timestamp), drain_metadata (JSONB)
- [ ] Diesel migration creating `accumulator_state` table (Postgres feature-gated)
- [ ] DAL method: `save_accumulator_state(state) -> Result<()>`
- [ ] DAL method: `load_accumulator_state(edge_id) -> Result<Option<PersistedAccumulatorState>>`
- [ ] DAL method: `load_all_accumulator_states() -> Result<Vec<PersistedAccumulatorState>>`
- [ ] DAL method: `delete_accumulator_states(edge_ids: &[String]) -> Result<usize>`
- [ ] Integration tests with Postgres: save, load, delete roundtrip

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

- Created migration: `013_create_accumulator_state` (Postgres) and `012_create_accumulator_state` (SQLite)
- Table: edge_id (PK VARCHAR), consumer_watermark (TEXT/JSON), last_drain_at (TIMESTAMP), drain_metadata (TEXT/JSON), created_at, updated_at
- Added `accumulator_state` table to all 3 schema modules (unified, postgres, sqlite) and all `allow_tables_to_appear_in_same_query!` blocks
- Created `AccumulatorStateRow` (Queryable) and `NewAccumulatorState` (Insertable) models in `dal/unified/models.rs`
- Created `AccumulatorStateDAL` in `dal/unified/accumulator_state.rs` following `dispatch_backend!` pattern
- Methods: `save()` (upsert), `load(edge_id)`, `load_all()`, `delete_by_ids()`
- Both Postgres (`on_conflict do update`) and SQLite (`replace_into`) backends
- `cargo check -p cloacina` passes clean
- Integration tests deferred to T-0140 (requires running DB to validate)
