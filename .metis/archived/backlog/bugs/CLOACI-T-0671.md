---
id: ui-execute-button-fails-executes
level: task
title: "UI Execute button fails — executes by package_name but server resolves by workflow name (package/workflow naming drift)"
short_code: "CLOACI-T-0671"
created_at: 2026-06-13T12:23:44.022531+00:00
updated_at: 2026-06-13T13:12:48.406447+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# UI Execute button fails — executes by package_name but server resolves by workflow name (package/workflow naming drift)

## Objective **[REQUIRED]**

Clicking **Execute** on a workflow in the UI fails for any package whose package
name differs from its workflow name — which is the **standard convention**
(`demo-slow-rust` package → `demo_slow_workflow` workflow, like
`fleet-slow-rust` → `fleet_slow_workflow`). So Execute is broken for essentially
every real package. UC-1 (execute → watch it run) is unreachable from the UI.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

P1: Execute is a primary UI action (CLOACI-T-0657 / UC-1) and it fails for the
normal naming convention.

### Impact Assessment **[CONDITIONAL: Bug]**
- **Reproduction** (verified in a real Chromium drive of the demo, 2026-06-13):
  1. Open `/workflows/demo-slow-rust`, click **Execute**, submit.
  2. UI POSTs `/v1/tenants/public/workflows/demo-slow-rust/execute`.
  3. Server → **400**:
     `{"error":"Workflow execution failed: Failed to schedule workflow: Workflow
     not found in registry: demo-slow-rust","code":"execution_failed"}`
- **Expected**: the run starts and the UI navigates to its execution detail.
- **Root cause**: the **package name** (`demo-slow-rust`) is what the UI has and
  routes/executes by (`Workflows` list → `navigate(/workflows/:package_name)` →
  `executeWorkflow(name)`), but the server's execute route resolves the **runner
  registry by workflow name** (`execute_async(&name)` → registry holds
  `demo_slow_workflow`). The two namespaces differ → not found. The UI never even
  has the workflow name: `getWorkflow` returns `package_name` + `tasks` (and
  `tasks` is empty too — CLOACI-T-0663) but no workflow name.

### Note
This is the long-suspected "package/workflow naming drift." The harness sidesteps
it by executing via the **workflow** name (`demo_slow_workflow`); the UI can't,
because the API surface it has is package-name-keyed.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] Executing a workflow from the UI starts a run and navigates to its
      execution detail, for packages where package name ≠ workflow name.
      (Verified in Chromium: demo-slow-rust detail → Execute → navigated to
      /executions/93f2e32b-…, zero failed /v1 calls.)
- [x] One coherent identifier story: the list/detail API now exposes
      `workflow_name` and the UI executes by it. list/detail/execute agree.
- [x] Regression test through the server: `POST .../workflows/demo_slow_workflow/execute`
      → 200 `{status:"scheduled"}`; the old `.../demo-slow-rust/execute` still 400s,
      confirming the UI now sends the right identifier.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### 2026-06-13 — Decision: Option A (persist + expose). Implementation map (deeper than first estimated)

`workflow_name` is **not modeled or persisted anywhere reachable**:
- The registry `WorkflowMetadata` struct (`registry/workflow_registry/database.rs`,
  built at `get_package_metadata_by_id` etc.) has **no `workflow_name` field** —
  only id/registry_id/package_name/version/description/author/tasks/schedules/dates.
- The stored `PackageMetadata` (`registry/loader/package_loader.rs`) the row
  deserializes from has **empty `tasks`** and no workflow_name (the T-0663 root).
- The authoritative source is the cdylib's `PackageTasksMetadata`
  (`workflow_name` + tasks), which the reconciler reads at load (and `PackageState`
  caches) but never writes back to the row.

So Option A is the same persistence work as T-0663, threaded through layers:
1. **Capture + persist** the cdylib's `workflow_name` (+ `tasks`) into the stored
   `PackageMetadata` — at compiler `mark_build_success` (it has the cdylib) or at
   reconciler load (it extracts them). Needs a metadata-update path (none today).
2. **Registry `WorkflowMetadata`**: add `workflow_name`, populate from the stored
   `PackageMetadata` at all read sites (database.rs ~367/418/571/636/942).
3. **api-types `WorkflowDetail`** (+ `WorkflowSummary`): add `workflow_name`;
   `routes/workflows.rs` sets it from `ins.metadata.workflow_name`.
4. **OpenAPI emit + SDK regen** (TS `generated/types.ts`).
5. **UI**: `WorkflowDetail.tsx` executes via `data.workflow_name` (fallback to the
   package name); `useExecuteWorkflow` keyed by workflow name.
This fixes T-0671 **and** T-0663. It is a multi-crate change + SDK regen + UI +
full rebuild — track as a focused task (or fold into T-0663/T-0670), not a quick
roll-in.

### Alternative (smaller, server-only — Option B, deferred per decision)
In `routes/executions.rs::execute_workflow`, on "not found in registry", resolve
`name` as a package via the reconciler's in-memory `loaded_packages`
(`PackageState` has both `package_name` and `workflow_name`) and retry. Needs a
`DefaultRunner`→reconciler resolver accessor. No UI/SDK/persistence; doesn't fix
T-0663. Coordinate with the
naming-drift cleanup.

### Dependencies / related
- CLOACI-T-0663 (workflow detail metadata empty — same API surface).
- CLOACI-T-0670 (package-authoring DX) — naming conventions are part of that.

## Status Updates **[REQUIRED]**

**2026-06-13 — DONE. Verified end-to-end on a fresh demo stack.**
Container rebuild compiled clean across cloacina + api-types + server. Fresh DB
(tore down volumes — stale rows predating the persistence change would have
hit the package_name fallback) re-uploaded + rebuilt all six fixtures through the
new `mark_build_success`. `GET /workflows` now returns distinct `workflow_name`
(demo-slow-rust → demo_slow_workflow, demo-fail-rust → demo_fail_workflow,
demo-cron-rust → demo_cron_workflow, demo-py-workflow → demo_py_workflow,
mixed-rust → mixed_workflow; demo-py-graph falls back to package name — it's a
CG, no #[workflow]). Rust workflows also show populated `tasks` now (**T-0663
fixed**). Execute-by-workflow_name → 200 scheduled; execute-by-package_name →
the original 400. Chromium drive of the UI Execute button navigated to the new
execution detail with zero failed /v1 calls.

**2026-06-13 — Implementing Option A (engine + API layers done; SDK/UI pending).**
Threaded `workflow_name` end-to-end:
- `package_loader::PackageMetadata` gained `#[serde(default)] workflow_name`,
  populated from the cdylib's `PackageTasksMetadata` in `convert_plugin_metadata_to_rust`.
- `mark_build_success` now extracts metadata from the freshly compiled cdylib and
  **persists** the authoritative `workflow_name` + tasks + graph_data into the row's
  `metadata` JSON (new `extract_and_merge_build_metadata` helper; best-effort —
  build success is still recorded if extraction fails). This fixes T-0663 too.
- Registry `WorkflowMetadata` gained `workflow_name`, populated at every DB read
  site (list_all postgres/sqlite, get_package_metadata_by_id both, inspect) with
  fallback to `package_name` for legacy rows; plus filesystem registry (from
  manifest) and the upload path (mod.rs).
- api-types `WorkflowSummary` + `WorkflowDetail` gained `workflow_name`; routes set it.
- All test constructions updated.
**Remaining:** regenerate OpenAPI spec + TS SDK types, update UI WorkflowDetail to
execute by `workflow_name` (fallback package_name), then container rebuild + verify
Execute end-to-end.

**2026-06-13 — Filed.** Found by driving the full demo UI in Chromium
(every view + action). Everything else passed — all read views, live WS
execution stream, API key create/revoke, upload (new + rejected) — the only
hard failure was Execute, with the 400 above. Confirms the package/workflow
naming drift end-to-end from the UI.