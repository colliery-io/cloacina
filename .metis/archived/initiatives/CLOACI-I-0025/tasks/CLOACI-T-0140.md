---
id: persist-on-drain-integration-and
level: task
title: "Persist-on-drain integration and watermark resume on startup"
short_code: "CLOACI-T-0140"
created_at: 2026-03-15T13:51:32.716057+00:00
updated_at: 2026-03-15T14:18:59.775227+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# Persist-on-drain integration and watermark resume on startup

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0025]]

## Objective

Wire persist-on-drain into the accumulator drain path and implement watermark resume on scheduler startup. After `drain()`, the scheduler persists the consumer watermark and metadata to DB. On startup, the scheduler loads persisted state and initializes accumulators with their last-known consumer watermarks.

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

## Acceptance Criteria

- [ ] After `drain()` in `check_readiness()`, scheduler calls `save_accumulator_state()` with edge ID, consumer watermark, drain timestamp
- [ ] `ContinuousScheduler::new()` accepts optional DAL reference for persistence
- [ ] On startup with DAL: loads all persisted states, matches by edge ID, initializes accumulator consumer watermarks
- [ ] Unmatched persisted states (no edge in graph) are logged as warnings (orphan detection)
- [ ] Without DAL: scheduler works as before (pure in-memory, no persistence)
- [ ] Integration test: populate state → restart scheduler → verify watermarks restored

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

- Added optional `dal: Option<Arc<DAL>>` field to `ContinuousScheduler`
- `with_dal()` builder method enables persistence
- `restore_from_persisted_state()` async method: loads all persisted states, matches edge IDs, logs orphans as warnings
- Run loop: after drain, collects accumulator state (scoped locks), then persists batch async (no locks across await)
- Fixed `Send` issue: all `MutexGuard` and `RwLockWriteGuard` scoped before `.await` points
- Without DAL: works exactly as before (pure in-memory)
- 105 unit + 8 integration tests passing
- DB integration test for full persist→restart→resume cycle deferred (needs running Postgres)
