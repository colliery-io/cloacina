---
id: authoring-dx-follow-ups-package
level: task
title: "Authoring DX follow-ups — package new --kind graph|cron, deeper validate lint, first-package how-to"
short_code: "CLOACI-T-0680"
created_at: 2026-06-14T16:37:35.268775+00:00
updated_at: 2026-06-14T16:37:35.268775+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Authoring DX follow-ups — package new --kind graph|cron, deeper validate lint, first-package how-to

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Objective **[REQUIRED]**

Follow-ups descoped from CLOACI-I-0119 (closed for the core authoring loop). The
`new → validate → pack → upload` DX ships and is verified; these polish the
remaining exit-criteria items:

1. **`package new --kind graph|cron`** — today `package new` scaffolds the
   `workflow` kind only (Rust + Python). Add computation-graph and cron-trigger
   templates (e.g. Rust `#[computation_graph]` split-reactor + `#[trigger(cron)]`;
   Python CG with `graph_name`/reactor).
2. **Deeper `validate` footgun lint** — current `package validate` checks the
   closed `[metadata]` schema + language layout. Add static checks for the known
   footguns: a cron trigger listed in `#[workflow(triggers=[…])]`, an unrewritten
   `__WORKSPACE__` placeholder, a CG package missing `graph_name`.
3. **"Create your first package" how-to** built around `package new` (Diataxis
   tutorial/how-to), linked from the packaging docs.
4. **e2e coverage** for `new → publish` of Rust / computation-graph / cron — the
   I-0119 e2e proves the Python workflow path; Rust depends on the `cloacina-*`
   crates being published to crates.io (gate this scenario on that).

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (nice to have)

Core authoring DX (I-0119) is done; these are polish/coverage, not blocking.

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