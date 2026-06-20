---
id: injectable-interfaces-docs-angreal
level: task
title: "Injectable interfaces — docs + angreal test coverage"
short_code: "CLOACI-T-0761"
created_at: 2026-06-20T16:46:04.794689+00:00
updated_at: 2026-06-20T19:12:32.206998+00:00
parent: CLOACI-I-0128
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0128
---

# Injectable interfaces — docs + angreal test coverage

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0128]]

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

### 2026-06-20 — DONE + VERIFIED (workflow-params scope)

Task G of [[CLOACI-I-0128]] — docs + test coverage for the shipped surface
(workflow params A/B/C + accumulator inject T-0753). The accumulator/reactor
*schema* coverage (D/E) and Python (F) parts are blocked, so this covers what's
landed.

**Tests:**
- Closed B's deferred gap: the `analytics_workflow` example
  (`examples/features/workflows/packaged-workflows`) now declares
  `params(source_id: String, batch_size: u32 = 500)`, exercising the macro's
  *populated* params codegen in packaged context.
- New integration test `fidius_validation::test_input_interface_populated` —
  loads the built cdylib, calls `get_input_interface` (method 9), and asserts the
  workflow entry's slots: `source_id` (required, `string`) + `batch_size`
  (optional, `integer`, default `500`). **Green** in the integration lane
  (313 passed, 0 failed).
- Execute-time validation already unit-tested in T-0757
  (`input_validation_tests`, 5 cases).

**Docs:** new how-to `docs/content/embed/how-to/declare-workflow-inputs.md` —
`#[workflow(params(...))]` authoring (required vs optional+default), how params
surface as `declared_params` `InputSlot`s, the `400 workflow_input_invalid`
execute validation + its v1 scope, and the operator reactor-fire /
accumulator-inject endpoints.

Verified: `angreal test unit` + `angreal test integration` green; OpenAPI in sync.

Follow-ups (not in scope here, tracked on their tasks): accumulator/reactor
schema + validation docs/tests land with D (T-0758) / E (T-0759); Python docs +
tests land with F (T-0760).
