---
id: visually-mark-manual-interventions
level: task
title: "Visually mark manual interventions — gold manual pill on workflow runs, reactor fires, accumulator injects"
short_code: "CLOACI-T-0776"
created_at: 2026-06-22T20:35:00.210368+00:00
updated_at: 2026-06-22T21:29:20.926508+00:00
parent: CLOACI-I-0131
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0131
---

# Visually mark manual interventions — gold manual pill on workflow runs, reactor fires, accumulator injects

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0131]]

## Objective **[REQUIRED]**

Operators can't tell at a glance when a workflow/graph ran because of a human, not
a real trigger. Visually mark manual interventions with a gold "manual" pill
across all three surfaces. (User chose: badge inject *action* only — don't
propagate inject→fire; marker = gold pill.)

## Plan

- **Reactor manual fire** (force_fire/fire_with): thread a `manual` flag from the
  ManualCommand path → `FireRecord` → api-types `ReactorFire.manual` → gold pill
  in Recent fires. In-memory, no migration. (Start here.)
- **Accumulator inject**: mark the accumulator's last event as operator-injected
  (runtime flag set by `send_to_accumulator`/inject handler) → `AccumulatorStatus`
  → gold "manual" pill on the accumulator card / last-event. No fire propagation.
- **Workflow manual run**: record trigger origin on the execution (manual when via
  the UI/execute API; cron/trigger/reactor otherwise). Needs an ADD COLUMN
  migration on the executions table + `ExecutionSummary.trigger`/origin → gold
  pill in Executions list + detail. Heaviest — do last.
- **SDK** regen (field additions; gate unaffected). **UI** gold `manual` Pill via
  the existing aurora Pill + TOKEN.gold.

## Status Updates **[REQUIRED]**

- 2026-06-22: Scoped + decided (badge inject action only; gold pill). Building
  reactor → accumulator → workflow.
- 2026-06-22: Reactor + accumulator surfaces DONE + verified (e9bab970 + the
  inject-count fix bhd7430bk). Reactor: StrategySignal::ForceFire → FireRecord.manual
  → ReactorFire.manual → gold pill in Recent fires (verified: forced fire shows the
  pill). Accumulator: note_accumulator_operator_inject() called ONLY from the REST
  inject endpoint → AccumulatorStatus.operator_injects/last_operator_inject_at →
  gold pill + tooltip (verified: orderbook shows manual after a REST inject,
  pricing doesn't; producer's WS feed correctly excluded — was over-counting at 14
  before the fix). SDK regen gate 38/38. REMAINING: workflow-run origin — needs a
  trigger_origin column on pipeline_executions (ADD COLUMN migration) + threading
  through the core runner's execute() (manual via execute API vs cron/trigger/
  reactor) + ExecutionSummary + UI. Heaviest; touches core runner.
- 2026-06-22: ALL THREE DONE + verified (reactor/accumulator e9bab970+bhd7430bk;
  workflow b9o0wfrfw). Workflow: avoided threading origin through the core engine —
  the REST execute handler tags the execution 'manual' via a best-effort UPDATE
  (set_trigger_origin) right after execute_async returns; cron/trigger/reactor stay
  NULL. Migration sqlite 026 / postgres 030 ADD COLUMN trigger_origin; threaded
  through UnifiedWorkflowExecution (the diesel Queryable — NOT WorkflowExecutionRecord,
  which is the domain type built via From) + ExecutionSummary; SDK gate 38/38. UI:
  gold pill in Executions list. Verified live: a POST …/execute shows 'manual' +
  the pill; cron runs unmarked. DONE. Minor follow-up: the execution DETAIL page
  doesn't show the pill (ExecutionDetail api-type lacks trigger_origin — one field
  + handler to add if wanted).

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