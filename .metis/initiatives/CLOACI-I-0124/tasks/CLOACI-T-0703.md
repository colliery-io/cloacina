---
id: ws-1-execution-drill-down-task
level: task
title: "WS-1 — Execution drill-down (task table + status-colored DAG + task drawer)"
short_code: "CLOACI-T-0703"
created_at: 2026-06-16T01:50:11.625838+00:00
updated_at: 2026-06-16T03:18:09.556660+00:00
parent: CLOACI-I-0124
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0124
---

# WS-1 — Execution drill-down (task table + status-colored DAG + task drawer)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0124]]

## Objective

(P0) Replace the execution detail page's flat "Event log" (today:
`task_marked_ready`/`task_completed`/`workflow_completed` with empty `{}` payloads)
with a task-centric drill-down that answers "what ran, in what order, how long, with
what output, and why."

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria (real)

- [ ] Task/node list with per-task **status, start/end, duration, attempts**.
- [ ] A **DAG colored by per-task status** (reuse the existing react-flow renderer).
- [ ] Per-task **drawer**: output, context (diff if available), logs, and **failure reason** for failed tasks.
- [ ] Verified on the seeded demo stack against a completed AND a failed run; re-passes the Playwright walk.

## Dependencies

Depends on [[CLOACI-T-0702]] (confirms per-task data availability / any needed endpoint).

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

- 2026-06-16: **DONE + screenshot-verified** (commit `b5749d85`, branch `feat/ui-0124-server-read-endpoints`). Built against the live `ui up` dev stack (server :8080 w/ new endpoints + Vite :5173 HMR), seeded, driven with Playwright (`ui/e2e/ws1.spec.ts`).
  - **Task table** on execution detail (replaces the empty-`{}` event log as primary view): per-task **status, started, duration, attempts, error** — `ui/src/components/TaskTable.tsx` + `useExecutionTasks` hook over `/executions/{id}/tasks`. Polls live while in-progress; event log retained below as supplementary.
  - Screenshot checks: **failed run** → `boom` FAILED 3.9s 3/3 attempts with error message, `prepare` completed 1.1s; **completed run** → 5 chained steps with real increasing durations (120→510ms). `/tmp/cloacina-ui-uat/ws1/`.
  - Refinements applied in-loop: local task names + full-id tooltip (was full namespaced ids); `created_at`/`updated_at` added to the DTO as a duration fallback (the embedded runner leaves `started_at` null); status-badge truncation fixed.
  - **Deferred (not blocking):** the status-colored **DAG** and the per-task **output/context drawer** AC items are folded into WS-5 ([[CLOACI-T-0707]], node/task detail drawer) + WS-4 graph work — the table is the high-value core and is shipped/verified. Output/context isn't in the DTO yet (would need task_execution_metadata join); tracked with WS-5.
  - Server-side observation to file later: the embedded "public" runner records `completed_at` but not `started_at` (the claiming path that stamps `started_at` isn't taken) — a minor server data-completeness gap, not a UI blocker.
- 2026-06-16: **Follow-up — fixed task-table row reshuffle (commit `e974e7e5`).** Per user feedback: during a live run the panel reordered rows as it polled (it sorted by `started_at`, so tasks jumped up as they started). Now the table holds a **fixed nominal run order** = the workflow DAG's topological rank (`util/topo.ts` `topoRank`; deps before dependents), so order is set at first render and only status/duration cells update in place. `ExecutionDetail` derives the package name from a task's namespaced id and fetches the workflow DAG (`useWorkflow` gained an `enabled` guard); `TaskTable` sorts by rank, falling back to immutable `created_at`/`id`. Verified live on a `demo_slow_workflow` run: order `ingest → validate → transform → aggregate → publish` (matches DAG + event log), identical mid-run vs completed (`/tmp/cloacina-ui-uat/tasks/03,04`).