---
id: doc-sweep-i-0129-i-0131-t-0763-t
level: task
title: "Doc sweep — I-0129/I-0131/T-0763/T-0768 (health API, fires, injectors, trigger-rule parity, operational UI)"
short_code: "CLOACI-T-0769"
created_at: 2026-06-21T22:22:37.788242+00:00
updated_at: 2026-06-21T22:29:37.391522+00:00
parent: CLOACI-I-0131
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0131
---

# Doc sweep — I-0129/I-0131/T-0763/T-0768 (health API, fires, injectors, trigger-rule parity, operational UI)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0131]]

## Objective **[REQUIRED]**

Heavy documentation sweep for everything that landed on the
`feat/i0129-operational-dag` branch: the I-0131 instrumentation (accumulator
freshness, reactor fires log + timeseries, operational views), T-0768 declared
injectors (workflow params, CG boundary schemas, inject/fire), and T-0763 Python
trigger-rule parity. Discovery (3 Explore agents) found the *conceptual* docs
largely already exist + current (`engine/explanation/trigger-rules.md`,
`embed/how-to/declare-workflow-inputs.md`); the real gaps were the API reference
(entirely new endpoints/fields) + a few reference tables.

## Status Updates **[REQUIRED]**

### 2026-06-21 — DONE (d388fb57)
Edited, verified against code + live demo (not the agents' speculation — e.g. no
fictional `cloacina_accumulator_last_event_at` metric was added):
- `reference/http-api.md`: AccumulatorStatus freshness fields (state/
  last_event_at/events_total/error, `{items,total}` envelope) + NEW reactor
  `/fires`, `/fires/timeseries` endpoints + the fire/inject/`/interface` REST
  surface.
- `reference/macros.md`: `#[workflow]` `params(...)` + `triggers` rows +
  "Declared params" section (required vs defaulted, schemars slots).
- `reference/python-api/task.md`: `trigger_rules` param + cloaca rule builders +
  Skipped semantics.
- `engine/computation-graphs/how-to/computation-graph-health.md`: accumulator
  freshness reading + "Inspecting reactor fires" how-to.
- `engine/computation-graphs/boundary.md`: typed inject/fire interface — opt in
  via `schemars::JsonSchema`.

Cross-linked to the existing current conceptual docs rather than duplicating.

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