---
id: wave-3-graph-and-operate-routes
level: task
title: "Wave 3 graph and operate routes — Graphs, Triggers, Operations on pack graph.rs"
short_code: "CLOACI-T-0934"
created_at: 2026-08-30T11:37:58.144384+00:00
updated_at: 2026-08-30T15:19:09.724506+00:00
parent: CLOACI-I-0141
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0141
---

# Wave 3 graph and operate routes — Graphs, Triggers, Operations on pack graph.rs

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0141]]

## Objective **[REQUIRED]**

Port the computation-graph and operate surfaces: Graphs, GraphDetail (the
full CG DAG on `aurora_leptos::graph` — switch that one view to
rust-sugiyama-fed positions only if the layered layout crosses badly),
Triggers, TriggerDetail, Operations. Includes the operator modals with
typed-slot forms: reactor fire/force-fire, accumulator inject
(GraphInjectModal), trigger fire (TriggerFireModal), and the graph health
widgets (GraphHealth/NodeReadiness/InputTable are pack widgets — this wave
supplies cloacina's state vocab as data).

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

- [x] GraphDetail renders a live CG from the demo stack (6 registered
      graphs; topology SVG via pack graph.rs; health coloring). Running-node
      pulse rides ExecutionDetail (Wave 2); GraphDetail pulses via edge
      `active` when firing.
- [x] Inject and fire round-trip LIVE from the UI: accumulator inject →
      "Delivered to N receivers"; trigger fire → "Fired N workflows" (the
      server-side union from T-0929 is what the fire endpoint executes).
- [x] New permanent Playwright spec `wave3-operate.spec.ts` (3 tests) passes
      against the demo stack: graphs+topology+inject, triggers+operations
      live tiles (ops WS snapshot flips the pill to "live" in-browser),
      trigger fire.

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

- 2026-08-30 — ALL FIVE ROUTES PORTED AND COMPILING on
  `feat/i0141-wave3-graph-operate` (535ed1b6): Triggers (type pills,
  raw-cron schedule text — cronstrue humanization deliberately dropped,
  revisit only if the visual gate objects; fire + run-now actions),
  TriggerFireModal (typed pass-through slots via trigger_interface, fan-out
  result list), Graphs (graphs/reactors/accumulators card rows, events/min
  from monotonic fire counters via an app-side Throughput tracker,
  force-fire, inject), GraphInjectModal (typed slots via
  accumulator_interface, raw-JSON textarea fallback), GraphDetail (WS-4
  augmented topology acc→reactor→compute on pack graph.rs, accumulator
  freshness rows + inject, force-fire), Operations (metric cards off the
  warm ops WS + agent roster; the React add-agent modal was a MOCK and
  stays out until a real enrollment API exists).

  Left out relative to React (recorded): GraphNodeModal node drawer,
  fire-activity chart + recent-fires table (Wave-4 chart work),
  DegradedBanner/ReactorReadiness pack-widget wiring (needs state vocab
  mapping — Wave 4 alongside the other widget work).

  NEXT: demo-stack rebuild in flight → live gate (GraphDetail on
  python-stateful-graph, inject round-trip, trigger fire both shapes).