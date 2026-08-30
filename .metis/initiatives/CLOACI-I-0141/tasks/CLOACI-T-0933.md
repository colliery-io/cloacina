---
id: wave-2-core-routes-overview
level: task
title: "Wave 2 core routes — Overview, Workflows, Executions + data layer and WS events"
short_code: "CLOACI-T-0933"
created_at: 2026-08-30T11:37:54.182180+00:00
updated_at: 2026-08-30T14:34:43.296674+00:00
parent: CLOACI-I-0141
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] The six routes render live data (seeded ui-e2e stack + demo stack):
      lists, details, and the failed-run debug view against real runs. (The
      upload e2e spec is not @smoke; the upload path gets its live run in
      the Wave-5 full-suite gate.)
- [x] Live updates over the wasm WS stream: the NFR-002 spec — "following an
      in-flight run reaches a terminal state" — passed against the demo
      stack (real 25s run watched to Completed).
- [x] Playwright @smoke specs in Wave-1/2 scope pass (connect, executions
      list + filter, failed-run detail); the 3 failures are Fleet/Accounts/
      Keys — Wave-3/4 stubs by design. Visual re-baselining happens at the
      Wave-5 gate. MERGED as PR #266.

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

- 2026-08-30 (later) — **ALL SIX ROUTES PORTED AND COMPILING** (trunk green),
  pushed to `feat/i0141-wave2-core-routes` (430b683b):
  data.rs (poll/once LocalResources bound to the active connection, transient-
  only single-retry, ClientError→pack-ApiError map), ops.rs (warm
  `ops_metrics:global` WS provider with generation-guarded resubscribe),
  util.rs (format_duration/ago via js_sys::Date), components.rs (RunCircles,
  RunWorkflowModal with typed slot coercion, TagPill), Overview, Workflows
  (pause/resume + run modal), Executions (URL-reflected chips/filter/paging),
  WorkflowUpload (web_sys File → multipart), ExecutionDetail (REST backfill +
  live WS tail merged on sequence_num, pack-graph DAG colored by task rows,
  task table, event log), WorkflowDetail (build badge/error, execute/pause/
  delete, pack-graph DAG, named instances read-only).

  **Deliberate Wave-4 deferrals** (noted in module docs): TaskGantt,
  TaskCodeModal, StatusStrip, RunHeatmap, TaskHealthTable, CombinedTimeline,
  ScheduleCard, InputsCard, and the DAG reliability overlay (fail counts).

  **Leptos gotchas recorded**: non-Copy `use_navigate` in Fn-closures →
  `StoredValue::new(use_navigate())` + `with_value`; multi-line/turbofish
  attr closures need braces in view!; pack `classify` takes the typed
  `ApiError`, not a string.

  **IN FLIGHT**: `angreal test ui-e2e --smoke` (own stack on :18085; first
  exercise of the trunk-based embedded-ui build.rs) + demo-stack rebuild for
  manual verification.