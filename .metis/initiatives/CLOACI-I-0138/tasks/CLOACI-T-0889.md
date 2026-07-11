---
id: gold-path-example-parameterized
level: task
title: "Gold-path example: parameterized workflow instances (I-0116) — params, named instances, schedules"
short_code: "CLOACI-T-0889"
created_at: 2026-07-11T22:03:08.051490+00:00
updated_at: 2026-07-11T22:03:08.051490+00:00
parent: CLOACI-I-0138
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0138
---

# Gold-path example: parameterized workflow instances (I-0116) — params, named instances, schedules

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0138]]

## Objective **[REQUIRED]**

I-0116 (parameterized workflow instances — named, scheduled, param-bound) shipped with ZERO user-facing example coverage; only test fixtures (`demo-py-cron`, `demo-py-workflow`) use `workflow_params`. Build the gold-path example that demonstrates it end-to-end through the primary interface.

**Surface to exercise (grounded):**
- Rust authoring: `#[workflow] params( name: Type [= default], … )` (workflow_attr.rs:281)
- Python authoring: `@cloaca.workflow_params(**kwargs)` (lib.rs:135, workflow.rs:451)
- Instance registration: named instances with bound params + optional schedule (I-0116; runner `register_workflow_instance` runner.rs:1015 is the embedded API — find and use the SERVER-side equivalent: how do packaged workflows declare/bind instances via upload + API? The compiler parses `declared_params` from package source at build (build.rs run_build) — trace how the server exposes param binding + scheduled instances, and demo THAT)
- Run with per-execution param values via `cloacinactl workflow run` (context/params input)

**Shape (per the T-0886 standard):** `examples/features/workflows/parameterized-instances/` — package.toml + version-dep Cargo.toml + `#[workflow]` with `params(...)` + gold-path README (pack → upload → build → bind instance(s) → run with values → observe) + a bespoke or auto `demos features` runner (auto-joins CI via `demos matrix`).

**Acceptance:** example builds on the demo stack; two named instances with different param bindings both reach Completed; README verified command-by-command; CI runs it.

**Loud-failure clause (I-0137 lesson):** if the server path can't express something I-0116 promised (e.g. schedule binding for packaged workflows), that's a FINDING to surface, not to paper over.

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
