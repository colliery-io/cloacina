---
id: code-quality-cleanup-dal-dedup
level: task
title: "Code quality cleanup — DAL dedup macro, split large files, remove dead code and stubs"
short_code: "CLOACI-T-0231"
created_at: 2026-03-22T13:05:15.414450+00:00
updated_at: 2026-03-22T22:45:09.397556+00:00
parent: CLOACI-I-0041
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0041
---

# Code quality cleanup — DAL dedup macro, split large files, remove dead code and stubs

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0041]]

## Objective

Reduce maintenance burden from the largest tech debt items: 162 duplicated DAL function pairs, 6 files over 1,000 lines, dead code, and abandoned stubs.

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

- [ ] `dispatch_backend!` macro or trait abstraction eliminates duplicated `_postgres`/`_sqlite` function pairs (target: 50%+ reduction in DAL line count)
- [ ] No file in `crates/cloacina/src/` exceeds 1,000 lines (split into sub-modules)
- [ ] All `#[allow(dead_code)]` annotations reviewed — dead code removed or justified with comment
- [ ] 5 no-op `.map_err(|e| e)` calls removed from `database/connection/mod.rs`
- [ ] `WorkflowRegistryDAL` stub either implemented or removed
- [ ] Incomplete OpenTelemetry stub either implemented or removed
- [ ] All existing tests pass after refactoring

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

### 2026-03-22 — Complete (quick wins)

**Done:**
- Removed 5 no-op `.map_err(|e| e)` calls from `database/connection/mod.rs`
- `WorkflowRegistryDAL` stub removed — file reduced to module placeholder with doc comment
- Dead `pub use WorkflowRegistryDAL` removed from `dal/unified/mod.rs`

**Planned (new initiative CLOACI-I-0042):**
- DAL dedup macro (162 function pairs)
- Split files >1,000 lines
- `#[allow(dead_code)]` audit
- OpenTelemetry stub resolution

495 tests pass
