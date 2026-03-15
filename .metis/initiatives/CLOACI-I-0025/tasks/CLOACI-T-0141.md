---
id: orphaned-state-detection-and-prune
level: task
title: "Orphaned state detection and prune-state admin command"
short_code: "CLOACI-T-0141"
created_at: 2026-03-15T13:51:33.535283+00:00
updated_at: 2026-03-15T14:21:32.298466+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# Orphaned state detection and prune-state admin command

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0025]]

## Objective

Implement orphaned state detection at startup and a prune-state admin command. Orphaned state = persisted accumulator states with edge IDs that no longer match the current graph. The prune command deletes them.

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

- [ ] Startup: compare persisted edge IDs vs current graph edges, log `warn!` for each orphan
- [ ] `list_orphaned_states(graph, dal) -> Vec<String>` returns orphaned edge IDs
- [ ] `prune_orphaned_states(graph, dal) -> Result<usize>` deletes orphans, returns count
- [ ] CLI command: `cloacinactl continuous prune-state [--dry-run]`
- [ ] `--dry-run` lists orphans without deleting
- [ ] Integration test: create state for edges not in graph → detect orphans → prune → verify deleted

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

- Created `continuous/state_management.rs` with `list_orphaned_states()` and `prune_orphaned_states()`
- Both functions take `&DataSourceGraph` + `&DAL`, compare persisted edge IDs against graph edges
- Added `AdminCommands::ContinuousPruneState { dry_run }` to cloacinactl
- `--dry-run` lists all persisted states without deleting
- Without `--dry-run`, deletes orphaned states and reports count
- Orphan detection at startup already in scheduler's `restore_from_persisted_state()` (T-0140)
- `cargo check --workspace` passes clean
- Integration test with real DB deferred (requires Postgres + migrations run)
