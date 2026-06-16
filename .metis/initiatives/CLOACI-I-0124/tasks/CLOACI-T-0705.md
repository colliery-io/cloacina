---
id: ws-3-overview-as-paginated
level: task
title: "WS-3 — Overview as paginated workflows + graphs lists"
short_code: "CLOACI-T-0705"
created_at: 2026-06-16T01:50:14.367320+00:00
updated_at: 2026-06-16T03:25:09.858667+00:00
parent: CLOACI-I-0124
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0124
---

# WS-3 — Overview as paginated workflows + graphs lists

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0124]]

## Objective

(P0) Replace the Overview's summary-count cards (today "Recent status: COMPLETED 5",
"3 loaded" — which read as totals and hide a WARMING graph) with paginated, faceted
**workflows** and **graphs** lists.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria (real)

- [ ] Overview shows a paginated **workflows** list and a paginated **graphs** list (not summary squares).
- [ ] Columns include **type** (workflow vs computation graph), **health**, and **last-run**.
- [ ] Misleading count cards removed (or replaced with a correctly-labeled rollup).
- [ ] Re-passes the Playwright walk; a count can't be misread as a total.

## Dependencies

Uses existing list endpoints (independent of [[CLOACI-T-0702]]); coordinate the
type/"Tasks: 0" column with [[CLOACI-T-0709]].

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

## Status Updates

- 2026-06-16: **DONE + screenshot-verified** (commit after WS-2, branch `feat/ui-0124-server-read-endpoints`). Screenshot `/tmp/cloacina-ui-uat/ws3/overview.png`.
  - Rewrote `Overview.tsx`: removed the misleading count cards ("Recent status: COMPLETED 5" / "3 loaded"); now shows a real **Workflows** list (package/version/tasks, navigable) + **Computation graphs** list (name/health via `GraphHealth`/accumulator count, navigable), each previewing the first 8 with an "All N" link to the full paginated page; Recent executions retained.
  - Verified: workflows list shows demo-fail-rust/demo-slow-rust with task counts; graphs list shows correct empty state ("No computation graphs loaded" — this seed had no CG packages); recent executions show completed/failed with timestamps. No count can be misread as a total.
  - **Scoping notes:** per-row **last-run** column omitted — joining executions→workflows is unreliable (executions key on `workflow_name`, packages on `package_name`, which drift). **type** (workflow vs CG) is handled by separate lists + WS-7's "Tasks:0→type" fix ([[CLOACI-T-0709]]); a CG graph-package still appears in the workflows list (data reality — CG packages are workflow packages), acceptable. Full pagination lives on `/workflows` + `/graphs`; the overview previews + links (Airflow-home pattern).