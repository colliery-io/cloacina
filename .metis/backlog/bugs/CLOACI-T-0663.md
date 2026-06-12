---
id: workflow-package-metadata-persists
level: task
title: "Workflow package metadata persists empty tasks/symbols after successful build"
short_code: "CLOACI-T-0663"
created_at: 2026-06-12T01:52:42.764837+00:00
updated_at: 2026-06-12T01:52:42.764837+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Workflow package metadata persists empty tasks/symbols after successful build

## Objective **[REQUIRED]**

A `.cloacina` package that uploads and builds successfully (`build_status: success`)
and executes all of its tasks at runtime nonetheless has an **empty `tasks` and
`symbols` list** in its persisted package metadata. The server therefore returns
`tasks: []` from `GET /workflows/{name}` and the workflows list, so the web UI's
workflow detail shows "Tasks (0) / No tasks" for every package even though the
workflow demonstrably runs its tasks. Whatever stage should extract the task /
symbol list into `workflow_packages.metadata` isn't doing so.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

P2: it degrades the UI's workflow detail/list (a documented CLOACI-T-0652
criterion — "workflow detail shows its tasks") and any API consumer relying on
package task metadata, but it does **not** affect execution — the runtime
introspects the cdylib directly and runs all tasks correctly.

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: All packaged-workflow users / any API or UI consumer of
  workflow task metadata. Appears universal (not fixture-specific).
- **Reproduction Steps**:
  1. Boot `cloacina-server` + `cloacina-compiler` (e.g. `angreal ui up`).
  2. Upload a packaged workflow that defines `#[task]`s (e.g. the
     `demo-slow-rust` fixture, which has 5 chained tasks). Wait for the
     compiler to build it → `build_status: success`.
  3. `GET /v1/tenants/public/workflows/demo-slow-rust` (or the list).
- **Expected vs Actual**:
  - **Expected**: `tasks` lists the 5 tasks (`ingest, validate, transform,
    aggregate, publish`); `symbols` lists the exported task symbols.
  - **Actual**: `"tasks": []`, `"symbols": []`. Confirmed at the source — the
    `workflow_packages.metadata` row stores:
    ```json
    {"package_name":"demo-slow-rust","version":"0.1.0",
     "description":"…","author":null,"tasks":[],"graph_data":null,
     "architecture":"aarch64","symbols":[],"workflow_triggers":[]}
    ```
  - Yet executions of this workflow run all 5 steps to completion (verified
    live via the UI's streaming execution view), so the runtime registry has
    the tasks — only the persisted package metadata is empty.

## Acceptance Criteria **[REQUIRED]**

- [ ] After a successful build, `workflow_packages.metadata.tasks` lists the
      workflow's tasks (ids + dependencies), and `symbols` lists the exported
      task symbols.
- [ ] `GET /workflows/{name}` and the workflows list return the populated
      `tasks` (the UI workflow detail then shows them — no UI change needed).
- [ ] A regression test covers it (upload → build success → assert non-empty
      `tasks`), ideally folded into the compiler e2e or the SDK contract.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Find the stage responsible for populating `metadata.tasks` / `metadata.symbols`
and fix it. Candidates:
- `cloacinactl package pack` — does the manifest it writes into the `.cloacina`
  archive carry the task list, or only `package.toml` `[metadata]` (which has no
  task list)? The task list lives in the Rust `#[task]` macros, so it must come
  from introspecting the built cdylib's exported metadata symbol, not the TOML.
- the **compiler** on `mark_build_success` — it has the freshly-built cdylib in
  hand and could introspect + write the task/symbol list back into the package
  row (mirrors how it writes `compiled_data`).
- the **upload** handler / registry — if the archive manifest *does* carry
  tasks, confirm they aren't being dropped when the metadata row is written.

The runtime clearly extracts tasks from the cdylib at load time (executions
work), so the extraction logic exists — it just isn't persisted to
`metadata.tasks`. Reuse it at build/pack time.

### Risk Considerations
Low — additive metadata population. Verify the `architecture` field (already
populated, `aarch64`) is written at the same stage to locate the right hook.

## Status Updates **[REQUIRED]**

**2026-06-11 — Filed.** Discovered while reviewing the web UI (CLOACI-I-0117):
workflow detail showed "Tasks (0)" for a workflow that runs 5 tasks. Traced
through `GET /workflows/{name}` (`tasks:[]`), the workflows list (`tasks:[]` for
all packages), and finally the `workflow_packages.metadata` DB row (empty
`tasks`/`symbols`). The UI renders the API faithfully — the gap is server/
compiler-side. Separately noted (not part of this bug): the demo seed workload
defines no triggers/reactors/computation graphs, so those UI views are
legitimately empty — a demo-coverage gap for CLOACI-T-0660, not a defect.
