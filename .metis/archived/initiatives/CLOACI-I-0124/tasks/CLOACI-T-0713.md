---
id: ws-10-at-a-glance-health-on-list
level: task
title: "WS-10 — At-a-glance health on list pages (workflow run circles + CG health)"
short_code: "CLOACI-T-0713"
created_at: 2026-06-16T12:30:10.143342+00:00
updated_at: 2026-06-16T12:39:59.216606+00:00
parent: CLOACI-I-0124
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0124
---

# WS-10 — At-a-glance health on list pages (workflow run circles + CG health)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0124]]

## Objective **[REQUIRED]**

Add Airflow-style at-a-glance health to the list pages so an operator can read
"what's going on" without drilling in: per-workflow recent-run status circles,
and per-CG health. (CG **throughput** — msgs/5s — deferred: needs a runtime
message counter; see follow-up.)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria (real)

- [x] Workflows list + Overview: recent-run **status circles** per workflow
      (color by status, newest-rightmost, tooltip with status+time).
- [x] Graphs list: per-accumulator **health dots** + count alongside the graph
      health badge.
- [x] **Taxonomy fix:** pure computation-graph packages no longer appear in the
      Workflows list — they belong to the Graphs view.
- [ ] (Deferred) CG recent throughput — needs runtime instrumentation.

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

### 2026-06-16 — DONE (throughput deferred)

- `components/RunCircles.tsx` — recent-run status dots (reuses
  `executionStatusColor`); oldest→newest, status+time tooltip. Workflows list &
  Overview tile bucket one `useExecutions({limit:200})` query by `workflow_name`
  (no per-row fetch, no server change).
- `Graphs.tsx` — per-accumulator health dots (`stateColor`: green=live,
  yellow=warming/socket_only, orange=degraded, red=crashed) + count, beside the
  graph health badge (`useAccumulators` name→status map).
- **Taxonomy fix (from user review):** pure CG packages (`tasks.length === 0`)
  filtered out of the Workflows list + Overview workflows tile — they're
  computation graphs (shown in Graphs as `market_pipeline` / `market_maker` /
  `demo_kafka_graph`), not workflows. A CG wrapped in `#[workflow]`+trigger has a
  task and still lists. This reverses WS-7's "graph" badge in the workflow list.
- **Deferred:** CG recent throughput (msgs/5s) — only `accumulator_buffer_depth`
  (gauge, 0 for passthrough) + `component_health` exist; no message counter. User
  chose to defer; needs a `cloacina_*_messages_total` counter on the boundary
  path + an endpoint. (Backlog follow-up.)

Verified live (`/tmp/cloacina-ui-uat/glance/`): Workflows = demo-fail/slow/cron/
poll with run circles, CGs absent; Overview tiles split correctly; Graphs shows
accumulator health dots. `tsc` clean. Committed `bcc9c96e` on
`feat/ui-0124-server-read-endpoints`.