---
id: enrich-recent-fires-with-per-fire
level: task
title: "Enrich Recent fires with per-fire inputs + outputs (graph I/O history)"
short_code: "CLOACI-T-0775"
created_at: 2026-06-22T18:57:22.289475+00:00
updated_at: 2026-06-22T20:30:06.587118+00:00
parent: CLOACI-I-0131
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0131
---

# Enrich Recent fires with per-fire inputs + outputs (graph I/O history)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0131]]

## Objective **[REQUIRED]**

Graph "Recent fires" is sparse ("ran in 0ms · completed · 3s ago"). Enrich each
fire with the graph's I/O: the input boundary values that triggered it and the
terminal outputs it produced. User chose the full inputs+outputs option.

## Plan

- **Inputs (all graphs):** at fire time the reactor holds `snapshot`;
  `snapshot.entries_as_json()` → `{source: value}`. Capture before the graph call
  (snapshot is moved into it).
- **Outputs (rust/FFI graphs):** the FFI bridge already gets
  `ffi_result.terminal_outputs_json: Option<Vec<String>>` then discards it by
  boxing into `Vec<Box<dyn Any>>` (which is write-only downstream). Carry it
  instead: add `outputs_json: Vec<serde_json::Value>` to `GraphResult::Completed`
  (constructor variant; `completed(vec)` defaults it empty). Python reactor-path
  `execute()` boxes opaque terminals → outputs empty for now (follow-up).
- **Plumb:** reactor records `inputs` + `outputs` into `FireRecord`; add the same
  to api-types `ReactorFire`; the `/fires` handler maps them through.
- **SDK** regen (field additions; coverage gate unaffected).
- **UI:** `RecentFires` (graph-ops.tsx) renders each fire's inputs → outputs
  (compact, expandable) alongside outcome/duration/time.

## Status Updates **[REQUIRED]**

- 2026-06-22: Scoped. Outputs are type-erased at the reactor BUT the FFI bridge
  has them as JSON before boxing — carry that. Python outputs deferred.
- 2026-06-22: DONE + user-confirmed (8513e436 + 4ca5298b). Full stack landed:
  computation-graph `outputs_json` → FFI bridge carries terminal_outputs_json →
  macro serializes reactor terminals → reactor records inputs+outputs → api-types
  ReactorFire → /fires handler → SDK regen (gate 38/38) → UI in→out compact +
  expandable. Two bugs found en route: (1) inputs were hex (entries_as_json is a
  debug fallback over bincode frames) — now bincode-decode → JSON; (2) outputs
  always empty because the reactor's graph_fn is the subscriber DISPATCHER
  (make_subscriber_dispatcher) which fanned out to N bound graphs and returned
  `completed(vec![])`, discarding outputs — now aggregates each subscriber's
  outputs_json. Verified live on market_pipeline: inputs {orderbook,pricing} →
  outputs [{action:"WAIT",confidence}]. Required a no-cache fixture recompile to
  pick up the macro change. Python-graph outputs remain a follow-up (reactor-path
  terminals stay opaque).

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