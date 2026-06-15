---
id: split-workflow-builder-md-move-how
level: task
title: "Split workflow-builder.md: move how-to/best-practices body into a how-to guide"
short_code: "CLOACI-T-0687"
created_at: 2026-06-15T13:06:44.116985+00:00
updated_at: 2026-06-15T13:06:44.116985+00:00
parent: CLOACI-I-0120
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0120
---

# Split workflow-builder.md: move how-to/best-practices body into a how-to guide

## Parent Initiative

[[CLOACI-I-0120]]

## Objective

Split `docs/content/python/api-reference/workflow-builder.md`. Its back half
(roughly the "Complete Workflow Example", "Advanced Patterns", "Validation and
Debugging", "Error Handling", and "Best Practices" sections) is a how-to /
best-practices guide living under a REFERENCE label — a Diátaxis boundary
violation flagged as a blocker during the T-0686 review gate. The reference page
should document only the API surface (constructor, `description`/`tag`,
`add_task`, `build`, context-manager protocol, and the resulting `Workflow`
methods, each as neutral signature/returns entries); the narrated examples,
factory/organization patterns, and Good/Avoid best-practices belong in a how-to
guide under `/python/workflows/how-to-guides`.

## Background

Discovered during the CLOACI-T-0686 adversarial review gate. The
diataxis-compliance-reviewer flagged lines ~178–578 as a multi-section reference
violation; the T-0686 framing only covered the new "Which pattern, and when"
hint block, so the structural split was deferred here rather than expanded into
T-0686. T-0686 already fixed the two concrete factual errors in this file
(`Workflow.can_run_parallel` does not exist; `build()` raises only `ValueError`,
not `KeyError`).

## Acceptance Criteria

- [ ] `workflow-builder.md` reduced to neutral API reference (no narrated tutorials, no Good/Avoid editorializing, no option surveys)
- [ ] How-to content moved to a guide under `/python/workflows/how-to-guides` and cross-linked
- [ ] All examples in the moved content verified against current code
- [ ] diataxis-compliance-reviewer returns zero blockers/majors on both files

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
