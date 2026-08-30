---
id: wave-2-core-routes-overview
level: task
title: "Wave 2 core routes — Overview, Workflows, Executions + data layer and WS events"
short_code: "CLOACI-T-0933"
created_at: 2026-08-30T11:37:54.182180+00:00
updated_at: 2026-08-30T12:49:46.842417+00:00
parent: CLOACI-I-0141
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0141
---

# Wave 2 core routes — Overview, Workflows, Executions + data layer and WS events

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0141]]

## Objective **[REQUIRED]**

Port the core operator surface to Leptos at strict parity: Overview,
Workflows, WorkflowDetail, WorkflowUpload (multipart), Executions,
ExecutionDetail — plus the shared data layer they establish for later waves:
leptos resources with polling intervals (react-query parity where the UI
relied on stale-while-revalidate), the wasm WS execution-event stream
driving EventLog/ActiveRunCard/StatusStrip, and the run/upload modals.
Workflow DAG summaries (MiniDag) come from `aurora_leptos::graph`.

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

- [ ] The six routes render live data against the demo stack: list, detail,
      upload → compile → reconcile → run → watch to Completed, all from the
      Leptos UI.
- [ ] Live updates arrive over the wasm WS stream (not just polling) on
      ExecutionDetail/EventLog.
- [ ] Playwright e2e specs covering these routes pass; visual specs
      re-baselined only where output legitimately differs.

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

- 2026-08-30 — STARTED on branch `feat/i0141-wave2-core-routes` (stacked on
  the Wave-1 branch; PR after #265 merges). Porting map from the React
  sources (readable at `git show origin/main:ui/src/...` until #265 lands;
  after that use the pre-merge sha).

  **Data layer** (`ui/src/data.rs`, new): wasm client futures are !Send →
  `LocalResource`. Polling = tick `RwSignal<u32>` bumped by a gloo-timers
  interval; resource reads the tick. React parity: staleTime 10s; retry
  ONCE and only transient (server/network) per aurora `classify`. Query
  keys were tenant-scoped — in Leptos, resources derive from
  `auth.connection()` so a tenant switch re-fetches naturally.

  **Reference surfaces** (origin/main `ui/src/api/*`): workflows.ts
  (list/get/upload/execute/delete), executions.ts (list/get livePoll/
  events/live-events-WS/tasks/task-runtimes), operations.tsx
  (useServerHealth + OpsMetricsProvider app-level WS, T-0774), health.ts
  (graphs). WS = cloacina-client `follow_execution_events` /
  `subscribe_delivery` (wasm-capable since Wave 1).

  **Routes** (origin/main sizes): Overview 265, Workflows 156,
  WorkflowDetail 291, WorkflowUpload 116, Executions 168, ExecutionDetail
  292 lines TSX. Components: RunWorkflowModal, StatusStrip, ActiveRunCard,
  RecentTasksCell, EventLog, MiniDag (pack graph.rs), ScheduleCard,
  InputsCard, TaskTable. Heavy chart pieces (TaskGantt etc.) are Wave 4 —
  ExecutionDetail's Gantt section stubs until then.

  **Upload**: multipart via cloacina-client (reqwest multipart works on
  wasm); file bytes from web_sys::File.

  **Acceptance**: demo stack live upload→run→Completed + the
  walk/scenarios Playwright subsets that need only Waves 1–2 surface.