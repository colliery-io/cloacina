---
id: manual-trigger-fire-push-a-typed
level: task
title: "Manual trigger fire — push a typed event to a trigger, fan out to all subscribed workflows"
short_code: "CLOACI-T-0777"
created_at: 2026-06-22T22:42:45.829027+00:00
updated_at: 2026-06-22T22:44:31.923844+00:00
parent: CLOACI-I-0131
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0131
---

# Manual trigger fire — push a typed event to a trigger, fan out to all subscribed workflows

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0131]]

## Objective **[REQUIRED]**

Operators can manually run workflows, but not *triggers*. A trigger (by name) can
be subscribed by multiple workflows (multiple `#[trigger(name=N, on=W)]` decls
share a name → one schedule per workflow). "Fire the trigger" should push a typed
event to it and fan out to every subscribed workflow — one action instead of N.
User wants the event typed via our declared-params/surface schema (the trigger
declares what it passes through); demo needs a manual-only trigger bound to 2
workflows.

## Plan (phased)

**P1 — Fire mechanics + fan-out (backend):** `POST /v1/tenants/{tenant}/triggers/
{name}/fire` (optional event body). Query enabled `schedule_type='trigger'`
schedules with `trigger_name=name`; execute each workflow with context =
{trigger_name, triggered_at, ...event}, mark execution manual (T-0776). Return
{trigger, fired:N, executions}. Reuse execute_trigger_workflow context shape.

**P2 — Typed trigger params:** `#[trigger]` gains `params(name: Type [= def])`.
Trigger emits a declared surface (kind="trigger"). `GET /…/triggers/{name}/interface`
→ DeclaredSurface (like get_reactor_interface). Fire validates event vs the slots.

**P3 — UI:** "fire" button on Triggers; typed fire modal from the interface (like
reactor/accumulator inject); show fan-out result.

**P4 — Demo:** manual-only trigger (poll→false) w/ declared params, shared name
bound to 2 existing workflows (demo_slow + demo_branch). Fixture + rebuild.

**SDK** regen for fire_trigger + get_trigger_interface (wrap in all 3).

## Status Updates **[REQUIRED]**

- 2026-06-22: Scoped. Typed event via params schema; demo = manual-only trigger →
  2 workflows. Building P1→P4.

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