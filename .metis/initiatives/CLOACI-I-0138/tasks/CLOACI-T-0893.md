---
id: operational-surface-coverage
level: task
title: "Operational-surface coverage — reactor fire, accumulator inject, trigger pause/fire, execution events woven into owning examples"
short_code: "CLOACI-T-0893"
created_at: 2026-07-11T22:03:42.162382+00:00
updated_at: 2026-07-11T22:03:42.162382+00:00
parent: CLOACI-I-0138
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0138
---

# Operational-surface coverage — reactor fire, accumulator inject, trigger pause/fire, execution events woven into owning examples

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0138]]

## Objective **[REQUIRED]**

The operational half of the primary interface — the verbs an operator uses on a RUNNING system — is taught nowhere. Surfaces (all shipped, all dark): `cloacinactl reactor force-fire` / `reactor fire <name> <inputs>` (typed), `accumulator inject <name> <event>`, `trigger list/inspect` + server trigger pause/resume/fire routes, `graph list/status/accumulators`, `execution events --follow/--since`, `tenant`/`key` lifecycle, fleet provision/deprovision + tenant limits.

**Approach — weave, don't invent:** each verb belongs in the README of the example that OWNS the feature, as an "Operate it" section after "Run it":
- `packaged-graph` / the T-0891 CG tour → `accumulator inject`, `reactor fire`/`force-fire`, `graph status/accumulators`, the accumulator/reactor WebSocket/UI view
- `event-triggers` (migrated) → `trigger list/inspect`, pause/resume, manual fire
- `simple-packaged` (canonical) → `execution events --follow` (already has list/status)
- `multi-tenant` (migrated) → server-side `tenant create/list` + `key create` + fleet provision/limits — the PRIMARY-interface tenant story (vs the embedded DatabaseAdmin it uses today)

Each "Operate it" section is verified live like the Run-it recipes, and — where cheap — asserted in the example's demos-harness runner so CI exercises the verb (e.g. the CG runner injects an event and polls the reactor fire).

**Acceptance:** every listed verb appears in exactly one owning example's verified "Operate it" section; at least `accumulator inject`, `reactor fire`, and `execution events` are asserted by harness runners in CI. Depends on / sequences with the owning examples' migrations (T-0891, event-triggers + multi-tenant migrations).

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
