---
id: graphdetail-operational-view-full
level: task
title: "GraphDetail operational view — full UI recompose on the new instrumentation"
short_code: "CLOACI-T-0767"
created_at: 2026-06-21T19:21:15.136288+00:00
updated_at: 2026-06-21T20:15:36.814839+00:00
parent: CLOACI-I-0131
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0131
---

# GraphDetail operational view — full UI recompose on the new instrumentation

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0131]]

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

### 2026-06-21 — DONE (7097dc90)
GraphDetail recomposed per the `graph_operational_view` pack on real
instrumentation (no PROPOSED stubs for the data tier):
- New `graph-ops.tsx`: DegradedBanner, GraphStatusStrip (health / throughput /
  last fire / total fires / sources-healthy / fire-failures), FireActivity
  (per-minute heatmap off T-0766 timeseries), ReactorReadiness (criteria +
  per-source fresh/stale chips + verdict), AccumulatorTable (state / last-event /
  rate / freshness bar / inline error off T-0765 fields), RecentFires (outcome +
  duration off T-0766 fires log).
- `controls.ts`: `useReactorFires` + `useReactorFireTimeseries` (5s poll).
- `GraphDetail.tsx`: header w/ Fire (useFireReactor) + Pause; degraded-source
  gold overlay on the topology FullDag (`failByNode` keyed `acc:{name}`).
- Per-source events/min derived via `useGraphThroughput` over `events_total`
  (same delta-over-polls pattern as fires/min); staleness from last_event_at age
  (>30s) or disconnected state.

Green: ui typecheck + build + unit tests; SDK types resolve the new fields.

Verification note: live screenshot vs the 3 reference shots needs the demo stack
rebuilt with these changes (`angreal ui up --build`) — the demo's live CG
producer (Kafka accumulators) populates the view; the currently-running stack is
the pre-change build so its /v1/health/graphs is empty.