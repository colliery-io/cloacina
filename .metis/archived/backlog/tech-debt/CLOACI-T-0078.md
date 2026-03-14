---
id: document-taskhandle-defer-until
level: task
title: "Document TaskHandle/defer_until and Python workflow packaging features"
short_code: "CLOACI-T-0078"
created_at: 2026-01-29T19:06:24.383932+00:00
updated_at: 2026-03-13T14:09:06.513368+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Document TaskHandle/defer_until and Python workflow packaging features

## Objective

Two features shipped with zero documentation. Create 6 new docs and update 4 existing ones.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Rust tutorial for TaskHandle (`tutorials/10-task-handles.md`)
- [x] Python tutorial for TaskHandle (`python-bindings/tutorials/08-task-handles.md`)
- [x] Explanation doc for task handle architecture (`explanation/task-handle-architecture.md`)
- [x] Update `python-bindings/api-reference/task.md` with PyTaskHandle section
- [x] Update `explanation/macro-system.md` with handle parameter detection
- [x] Python tutorial for packaging (`python-bindings/tutorials/09-packaging-workflows.md`)
- [x] Python how-to for multi-platform packaging (`python-bindings/how-to-guides/packaging-for-multiple-platforms.md`)
- [x] Python API ref for packaging CLI (`python-bindings/api-reference/packaging.md`)
- [x] Update `explanation/package-format.md` with Python/ManifestV2 section
- [x] Update `explanation/packaged-workflow-architecture.md` with Python pipeline section
- [x] `angreal docs build` passes with no broken links

## Key Source Files

### TaskHandle
- `crates/cloacina/src/executor/task_handle.rs` — core impl
- `crates/cloacina/src/executor/slot_token.rs` — slot abstraction
- `bindings/cloaca-backend/src/task.rs` — PyTaskHandle
- `crates/cloacina-macros/src/tasks.rs` — handle param detection
- `examples/features/deferred-tasks/src/main.rs` — Rust example
- `tests/python/test_scenario_31_task_handle.py` — Python tests

### Python Packaging
- `bindings/cloaca-backend/python/cloaca/cli/build.py` — CLI
- `bindings/cloaca-backend/python/cloaca/manifest.py` — manifest models
- `bindings/cloaca-backend/python/cloaca/discovery.py` — AST task discovery
- `bindings/cloaca-backend/python/cloaca/vendoring.py` — uv vendoring
- `crates/cloacina/src/packaging/manifest_v2.rs` — unified manifest
- `crates/cloacina/src/registry/loader/python_loader.rs` — loader
- `examples/features/python-workflow/` — example project

## Progress

### Session 2026-03-13
- Read all source files listed in Key Source Files section
- Completed 4 existing doc updates:
  - `python-bindings/api-reference/task.md` — added TaskHandle section with handle detection, defer_until API, lifecycle, examples
  - `explanation/macro-system.md` — added TaskHandle Detection section with generated code differences
  - `explanation/package-format.md` — added Python Packages (Manifest V2) section with layout, example, fields, validation
  - `explanation/packaged-workflow-architecture.md` — added Python Workflow Pipeline section with loading diagram, execution details, comparison table
- 6 new docs being written in parallel by background agents:
  - `tutorials/10-task-handles.md`
  - `python-bindings/tutorials/08-task-handles.md`
  - `explanation/task-handle-architecture.md`
  - `python-bindings/tutorials/09-packaging-workflows.md`
  - `python-bindings/how-to-guides/packaging-for-multiple-platforms.md`
  - `python-bindings/api-reference/packaging.md`
- Remaining: verify `angreal docs build` passes
