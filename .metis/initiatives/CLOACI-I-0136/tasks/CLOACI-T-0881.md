---
id: implement-py-block-on-shared
level: task
title: "Implement py_block_on shared bridge + route context/admin/task, leave runner"
short_code: "CLOACI-T-0881"
created_at: 2026-07-09T02:54:03.444321+00:00
updated_at: 2026-07-09T23:18:50.869456+00:00
parent: CLOACI-I-0136
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0136
---

# Implement py_block_on shared bridge + route context/admin/task, leave runner

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0136]]

## Objective **[REQUIRED]**

Implement the I-0136 design: add `crates/cloacina-python/src/gil.rs::py_block_on(py, fut)` (the one GIL-safe async→sync bridge — releases the GIL via `allow_threads` before blocking, drives on the ambient `Handle` else a transient current-thread runtime; generalized from the proven `context.rs::block_on_secret_access`). Route context.rs (secret/secret_field), admin.rs (create_tenant/remove_tenant — the latent-deadlock fix, add the PyO3-injected `py` param), task.rs (defer_until) through it. Leave runner.rs (dedicated-thread actor loop — documented non-goal).

### Status — 2026-07-09 (commit 72de0777, branch feat/i0135-dal-twin-collapse)
DONE + committed. `gil.rs::py_block_on` added; local `block_on_secret_access` deleted; context/admin/task rewired; runner.rs left with an inline note explaining the exemption.
**Compile-verified:** `cargo check -p cloacina-python` clean (Finished, no errors); the task.rs defer_until future satisfies py_block_on's `Send` bound; the admin `py: Python` param is invisible to Python callers. The change is strictly SAFER by construction: admin.rs was holding the GIL across the await (unsafe shape); it now uses the identical, production-proven pattern the secret path has used.
**RUNTIME verification PENDING:** GIL-safety is a runtime property a compile can't prove. Plan: scenario_31 (defer_until, my rewire) + scenario_30/32/33 (GIL-deadlock history) on sqlite via `uv run angreal test integration --backend sqlite --python-file <name>`, and scenario_28 (admin) on postgres. NOTE: 30/32/33 are the known-flaky GIL scenarios ([[project_scenario32_cg_invocation_deadlock]], xfail-allowlisted in CI). First run attempts blocked by a claude-opus-4-8 classifier outage.

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

*To be added during implementation*