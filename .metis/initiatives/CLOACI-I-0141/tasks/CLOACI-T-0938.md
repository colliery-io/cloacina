---
id: uat-round-1-graphs-operational
level: task
title: "UAT round 1 — graphs operational dashboard, trigger type sections + action columns, workflow list clarity, history vs current-execution views"
short_code: "CLOACI-T-0938"
created_at: 2026-08-31T12:59:12.230586+00:00
updated_at: 2026-09-01T17:56:07.572793+00:00
parent: CLOACI-I-0141
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0141
---

# UAT round 1 — graphs operational dashboard, trigger type sections + action columns, workflow list clarity, history vs current-execution views

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0141]]

## Objective **[REQUIRED]**

Maintainer UAT feedback (2026-08-31), verbatim intent:

1. **/graphs** — fully functional but bare. Add a computation-graphs
   OVERVIEW on the page: make it an "operational dashboard", not a pure
   "list of objects".
2. **/triggers** — separate triggers BY TYPE (cron vs poll — implemented
   the same but behave differently). Polls have a next/last run derived
   from their poll frequency — those cells must not be blank. Action
   column: justify the ⚡/▸ icons LEFT so they align vertically; pick a
   LARGER icon for the run control; add COLUMN HEADERS for both clickables
   (not obvious what they do).
3. **/workflows** — same action-clarity feedback as triggers (aligned,
   larger, labeled controls).
4. **Workflow/graph detail needs TWO views**: the historical "operational
   history" and the "specific execution / current execution" view.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] /graphs opens with an operational overview strip (aggregate state,
      fires, events/min, paused/degraded counts) above the sections.
- [ ] /triggers renders cron and polling triggers as separate sections;
      poll rows show derived next/last run; action columns are headed
      ("Fire", "Run"), left-justified, vertically aligned, larger run icon.
- [ ] /workflows action controls get the same treatment (headed, aligned,
      larger).
- [ ] WorkflowDetail and GraphDetail each expose a History view and a
      Current-execution view, cleanly separated.
- [ ] Verified live on the demo stack + fresh screenshots delivered for
      re-review.

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

- 2026-08-31: ALL four items implemented on feat/i0141-uat-round1 (4 commits):
  1. /graphs: 5-tile operational overview strip (graphs running, total fires, last fire, accumulators live, most active).
  2. /triggers: cron vs polling sections; poll rows derive last/next from new API field `TriggerScheduleSummary.last_poll_at` (+ server route wiring); headed left-justified Fire/Run icon columns (BoltIcon 16, PlayIcon 18).
  3. /workflows: card list → headed table (Package/Version/Tasks/Updated/Recent runs/Pause/Run), same action-column treatment.
  4. Dual views: shared ViewTabs segmented control; WorkflowDetail = Operational history | Current execution (reusable ExecutionView, embedded=true, prefers a live run, falls back to most recent); GraphDetail = Live (topology+accumulators) | Operational history (per-minute fire sparkline via reactor_fire_timeseries + recent-fires table via list_reactor_fires).
- openapi.json regenerated (diff = the new field only). Server + wasm targets compile; ui cargo fmt applied.
- Remaining: demo-stack rebuild (docker volumes pruned — fresh DB), live walk + screenshots for re-review, push + PR.
- 2026-08-31/09-01 UAT rounds 2–4 (all committed on feat/i0141-uat-round1):
  - R2: trigger tables share fixed column widths (aligned across sections); detail pages default to the "now" view (Current execution / Live) with Operational history behind the tab; cron last-run uses the same relative form as poll rows.
  - R2b: 1s wall-clock signal (data.rs Clock) — all relative-time cells (trigger last/next, heartbeats, fire ages, workflows Updated, graphs Last-fire tile) tick live; root cause was keyed <For> rows freezing derived strings.
  - R3: workflow Operational history is real history — run summary strip (runs/success rate/avg wall/failed), per-task exit-type table (completed/failed/skipped/other/retried + avg ± σ), average-timing gantt with ±1σ gold band, aggregated client-side from the last 20 runs' task rows (Memo on run-ID set, join_all fetch); short task names.
  - R4: accumulator dot = AVAILABILITY not activity (core AccumulatorHealth: socket_only = "healthy by definition"; disconnected = degraded/retrying) — util::health_color now maps socket_only→OK; last-event age is neutral info text; /graphs Accumulators-live tile counts availability. Poll-trigger last/next projects the poll-cadence sawtooth ((now−last_poll) mod interval, resets 0s at each boundary, countdown next, "overdue" only past 2 intervals — raw age never read 0s due to fetch latency).