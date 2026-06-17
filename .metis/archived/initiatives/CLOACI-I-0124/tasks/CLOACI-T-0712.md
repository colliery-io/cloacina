---
id: ws-9-make-the-execution-event-log
level: task
title: "WS-9 — Make the execution event log meaningful (task name + humanized events)"
short_code: "CLOACI-T-0712"
created_at: 2026-06-16T12:13:25.339991+00:00
updated_at: 2026-06-16T12:17:24.869351+00:00
parent: CLOACI-I-0124
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0124
---

# WS-9 — Make the execution event log meaningful (task name + humanized events)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0124]]

## Objective **[REQUIRED]**

The execution-detail **Event log** is not meaningful. For `0bf73958-…` it shows
three rows — `task_marked_ready {}`, `task_completed {}`, `workflow_completed {}`
— with empty `{}` blobs and giant global sequence numbers (9080–9082), and never
says **which task** the event is about (the underlying event row has a
`task_execution_id`, but the server DTO drops it).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria (real)

- [x] Server: `ExecutionEvent` DTO carries the **task name** (resolved from
      `task_execution_id`); OpenAPI + SDK regenerated.
- [x] UI: humanized event labels (not raw snake_case), a **task-name** chip when
      the event is task-scoped, empty `{}` payloads hidden, and a per-execution
      ordinal instead of the global `sequence_num`.
- [x] Verified live against execution `0bf73958-…` (screenshot).

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

### 2026-06-16 — DONE

- **Server:** `ExecutionEvent` DTO gains `task_name: Option<String>`;
  `get_execution_events` resolves it by mapping each event's `task_execution_id`
  against the workflow's task executions (`get_all_tasks_for_workflow`).
  Workflow-scoped events (e.g. `workflow_completed`) stay `null`. OpenAPI +
  `@cloacina/client` regenerated.
- **UI:** new `util/eventLabels.ts` (`describeEvent` → friendly label + status
  color; `meaningfulData` hides empty/`{}` payloads). `EventLog` now renders a
  colored badge ("Task ready" / "Task completed" / "Workflow completed" …), the
  shortened task name when task-scoped, no empty `{}`, and a per-execution
  ordinal (1,2,3) instead of the global `sequence_num` (9080–9082).

Verified live against `0bf73958-…` (`/tmp/cloacina-ui-uat/eventlog/02-fixed.png`):
`1 · TASK READY · demo_cron_step`, `2 · TASK COMPLETED · demo_cron_step`,
`3 · WORKFLOW COMPLETED`. `tsc --noEmit` clean. Committed `b0603d5c` on
`feat/ui-0124-server-read-endpoints`.