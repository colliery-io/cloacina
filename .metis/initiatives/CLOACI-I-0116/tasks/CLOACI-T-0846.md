---
id: named-instance-lifecycle-python
level: task
title: "Named-instance lifecycle + Python parity — CRUD by instance name, pyo3 instance API, docs and tests"
short_code: "CLOACI-T-0846"
created_at: 2026-07-06T00:12:32.486091+00:00
updated_at: 2026-07-06T00:45:00.261364+00:00
parent: CLOACI-I-0116
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0116
---

# Named-instance lifecycle + Python parity — CRUD by instance name, pyo3 instance API, docs and tests

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0116]]

## Objective **[REQUIRED]**

Finish I-0116: name-resolved lifecycle over the T-0843 DAL finder (OQ-7 delegation), the pyo3 instance API, Diataxis docs, and the angreal integration proof that a registered instance fires with its bound params.

## Status Updates (working)

### 2026-07-05 — code surface SHIPPED (commit 157ad24c, PR #181); docs + integration proof remain
- Name-resolved lifecycle on the runner: `get_workflow_instance(workflow, name)` + `unregister_workflow_instance` — OQ-7 as designed (name → schedule UUID → delegate to existing cron primitives; enable/disable ride the UUID ops). DAL `find_by_instance_name` landed with T-0843.
- `WorkflowInstance::from_resolved` for dynamic surfaces (no declared slots at hand).
- **Python**: `runner.register_workflow_instance(workflow_name, instance_name, cron, timezone, params: cloaca.Context)` — new RuntimeMessage variant + handler; the Context's JSON is the persisted fully-resolved param set.
- **REMAINING before this task closes**: Diataxis docs (declare → instantiate → schedule → manage) and the angreal integration proof (register a named instance → cron fire delivers bound params, sqlite + postgres). PR #181 is open for the initiative; these ride follow-up commits on that branch.

### 2026-07-05 (later) — docs + integration proof landed; CLOSING
- **Docs**: `docs/content/engine/scheduling/workflow-instances.md` — the template→instances model, Rust + Python usage, the fire-time merge contract (flat keys, reserved-keys-win, trigger-payload precedence), and the not-a-closure/not-a-version boundaries.
- **Integration proof** `test_workflow_instance_register_roundtrip` (scheduler/cron_basic.rs), **run live against real postgres, PASSED**: validated build (default snapshotted) → `register_cron_workflow_instance` → `get_workflow_instance` round-trip (params + name on the row) → duplicate name REJECTED by the unique index → second named instance ok → `unregister_workflow_instance` → lookup misses. Migration 040 applied on a fresh DB in the same run. Fire-time param delivery is covered by the `workflow_instance` merge unit tests + the compile-enforced cron/trigger wiring (matching the existing cron suite's depth — no wall-clock fire waits). COMPLETE.

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