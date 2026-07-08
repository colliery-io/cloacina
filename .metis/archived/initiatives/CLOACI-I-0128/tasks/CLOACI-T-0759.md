---
id: typed-injection-validation-reactor
level: task
title: "Typed injection validation — reactor fire + accumulator inject against derived schema"
short_code: "CLOACI-T-0759"
created_at: 2026-06-20T16:46:02.725645+00:00
updated_at: 2026-06-21T00:24:03.112211+00:00
parent: CLOACI-I-0128
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0128
---

# Typed injection validation — reactor fire + accumulator inject against derived schema

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

### 2026-06-20 — BLOCKED on T-0758 (D)

Task E of [[CLOACI-I-0128]] — validate reactor-fire / accumulator-inject payloads
against the surfaces' **derived schemas**. Those schemas come from D
([[CLOACI-T-0758]]), which is **blocked** on the boundary-`JsonSchema` authoring
decision. Without derived accumulator/reactor schemas there is nothing to
validate against.

The injection surfaces themselves exist and accept **untyped** JSON today
(reactor fire = T-0751; accumulator inject = [[CLOACI-T-0753]], done). The
validation helper pattern is already proven for workflows
(`validate_declared_params` in `routes/executions.rs`, T-0757) and will port
directly to the CG-health handlers once D supplies the schemas. Unblock D, then
this is a small, mechanical follow-on.

### 2026-06-20 — DONE + VERIFIED (landed with T-0758 part 2)

D unblocked (opt-in typed surfaces). Validation wired into the operator inject
handlers (`routes/health_graphs.rs`), reusing the T-0757 validators:
- **`fire_reactor`** (`fire_with`): validates the `inputs` map against the
  reactor's per-source declared slots (`find_surface_input_slots("reactor", …)`)
  → **`400 reactor_input_invalid`** with per-field messages.
- **`inject_accumulator`**: validates the event against the accumulator's
  boundary slot (`find_accumulator_input_slot(…)`) via a new single-value
  validator (`validate_value_against_schema`) → **`400 accumulator_input_invalid`**.
- Untyped/unknown surfaces accept free-form input (permissive `{}` schema);
  registry errors fail open.

Verified: unit (single-value validator cases) + integration green (314+100+6,
0 failed). Committed with the T-0758 part-2 change.