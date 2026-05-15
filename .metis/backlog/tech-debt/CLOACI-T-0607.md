---
id: delete-or-rewrite-continuous
level: task
title: "Delete or rewrite continuous-scheduling example (speculative future API)"
short_code: "CLOACI-T-0607"
created_at: 2026-05-15T16:39:01.909253+00:00
updated_at: 2026-05-15T16:39:01.909253+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Delete or rewrite continuous-scheduling example (speculative future API)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

`examples/features/computation-graphs/continuous-scheduling` imports `cloacina::continuous::*` modules (`boundary`, `datasource`, `detector`, `graph`, `ledger`, `scheduler`). None of those exist — they were a speculative future-state API name from CLOACI-S-0001/CLOACI-S-0003 that never landed. The actual feature shipped under the `computation_graph` + reactor + accumulator API instead. The example has been broken since before the I-0099 batch began.

Two options:

### Option A — delete (recommended)

The example duplicates concepts already covered by:
- `examples/features/computation-graphs/packaged-graph` (CG basics, packaging)
- `examples/tutorials/computation-graphs/library/{07,08,09,10}` (reactor + accumulators, full pipelines, routing)

Nothing here is uniquely valuable that the current CG tutorials don't cover. Delete the dir, update any sibling READMEs that reference it (if any), and remove it from the angreal demo registry.

### Option B — rewrite as a current-API CG demo

If "continuous scheduling" as a demo name still has a story (e.g., showing the reactor running indefinitely against a polling accumulator with a routing decision engine), rewrite it against the current API. Mostly: rename `cloacina::continuous::scheduler::ContinuousScheduler` → reactor; `boundary` → accumulator boundary; `datasource` → polling accumulator; `detector` → reactor with WhenAny criteria. ~1 day of work, plus a README rewrite.

## What to do

1. Decide A vs B (recommend A — duplicates existing tutorials).
2. **A**: `git rm -r examples/features/computation-graphs/continuous-scheduling`; remove from `.angreal/demos/*` if registered.
3. **B**: rebuild the example against the current `computation_graph` macro + reactor + accumulator runtime. Run `angreal demos features continuous-scheduling` end-to-end.

## Acceptance

- [ ] Decision recorded (delete vs rewrite).
- [ ] `angreal check all-crates` no longer reports continuous-scheduling as broken.
- [ ] If rewritten: README accurately describes what the demo teaches and the demo runs cleanly.

## References

- Spec lineage: CLOACI-S-0001 (Continuous Reactive Scheduling), CLOACI-S-0003 (Continuous Task Execution Model) — both still in `discovery` phase.
- Working substitutes: `examples/features/computation-graphs/packaged-graph`, `examples/tutorials/computation-graphs/library/*`.

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

*To be added during implementation*