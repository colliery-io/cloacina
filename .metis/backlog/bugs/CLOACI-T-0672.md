---
id: python-workflow-task-metadata
level: task
title: "Python workflow task metadata persists empty — no tasks/DAG shown for Python packages"
short_code: "CLOACI-T-0672"
created_at: 2026-06-13T14:21:46.204672+00:00
updated_at: 2026-06-13T14:33:27.499554+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Python workflow task metadata persists empty — no tasks/DAG shown for Python packages

## Objective **[REQUIRED]**

Python packaged workflows build + execute correctly but persist an **empty
`tasks`** list, so `GET /workflows/{name}` returns `tasks: []` / `task_graph: []`
and the UI workflow detail shows no tasks/DAG (e.g. `demo-py-workflow`, which has
`prepare → finish`). Rust packages now show their task DAG (T-0663/T-0671); Python
must reach parity.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

P2: degrades the UI workflow detail for all Python packages; execution is
unaffected (runtime introspects tasks directly).

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: all Python packaged-workflow users / UI + API consumers.
- **Reproduction**: upload `demo-py-workflow.cloacina`, wait for build success,
  `GET /v1/tenants/public/workflows/demo-py-workflow` → `tasks: []`,
  `task_graph: []`. Yet executions run `prepare` then `finish`.
- **Expected vs Actual**: expected `tasks: [prepare, finish]` with
  `finish` depending on `prepare`; actual empty.

### Root cause (investigated 2026-06-13)
- The compiler emits **empty bytes** for Python packages (no cargo build):
  `crates/cloacina-compiler/src/build.rs:216-222`.
- `mark_build_success` → `extract_and_merge_build_metadata`
  (`crates/cloacina/src/registry/workflow_registry/database.rs:1342`) **returns
  early when `compiled.is_empty()`** — so Python rows never get task metadata.
- The Python load path returns only `LoadedPythonWorkflow { task_namespaces,
  workflow_name }` (`crates/cloacina/src/python_runtime.rs:38-45`) — task **ids
  only, no dependencies**. Dependencies ARE captured in
  `PythonTaskWrapper.dependencies` (`crates/cloacina-python/src/task.rs:125-134`,
  populated at decorator time ~518-524) but only inside the scoped Runtime, which
  is dropped right after import (`reconciler/loading.rs:~398-411`); never surfaced.
- The Python view builder intentionally returns empty tasks today
  (`reconciler/loading.rs:~1257-1262`).

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

- [x] After a Python package builds successfully, its persisted `PackageMetadata.tasks`
      lists the workflow's tasks with dependencies.
- [x] `GET /workflows/{name}` returns populated `tasks` + `task_graph` for Python
      packages; the UI renders the same interactive DAG used for Rust.
- [x] Verified end-to-end on the demo stack with `demo-py-workflow`
      (`prepare → finish`).

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
Capture Python task dependencies host-side while the scoped Runtime is alive,
thread them back, and persist them — reusing the registry read path + `task_graph`
DAG already built for Rust (T-0663).
1. In the Python loader (`cloacina-python/src/loader.rs`, before the scoped
   Runtime drops), for each registered task read `task.dependencies()` and build
   a `Vec<(task_id, Vec<dep_local_id>)>` (strip namespaces to local ids).
2. Extend `LoadedPythonWorkflow` (`python_runtime.rs:38-45`) with this task graph.
3. Reconciler: on Python load, persist the task list (ids + deps) into the row's
   `PackageMetadata` via a metadata-update path (new small method, or reuse the
   merge logic from `extract_and_merge_build_metadata` with a non-cdylib source).
4. No api-types/UI change needed — the existing `tasks`/`task_graph` read sites
   and `WorkflowGraph` component pick it up once the row is populated.

### Dependencies
- Builds on T-0663/T-0671 (task_graph plumbing + WorkflowGraph UI).

### Risk Considerations
- Must read deps BEFORE the scoped Runtime is dropped (synchronous hook).
- Python local-id vs namespaced-id: normalize to local ids to match Rust.

## Status Updates **[REQUIRED]**

**2026-06-13 — DONE.** Captured Python task dependencies host-side and persist
them at reconciler load:
- `LoadedPythonWorkflow` gained `tasks: Vec<PythonTaskNode>` (id + dependency
  local ids); populated in `cloacina-python/src/runtime_impl.rs` by reading each
  task's `.dependencies()` off the scoped Runtime (still alive post-import).
- New `WorkflowRegistry::persist_task_graph` trait method (default no-op),
  implemented on the DB registry (`persist_task_graph_db`) — reads the row,
  writes `PackageMetadata.tasks` (ids + deps), preserving identity/workflow_name.
- Reconciler Python branch calls it after load (best-effort).
- No api-types/UI change — existing `task_graph` read path + WorkflowGraph render it.
Verified on fresh demo stack: `GET /workflows/demo-py-workflow` →
`tasks: [prepare, finish]`, `task_graph` with `finish` depending on `prepare`;
UI renders the DAG (2 nodes / 1 edge, zero console errors).
