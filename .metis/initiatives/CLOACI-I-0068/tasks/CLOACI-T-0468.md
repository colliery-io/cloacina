---
id: integration-test-python-cg-package
level: task
title: "Integration test: Python CG package reconciler routing (workflow vs CG path)"
short_code: "CLOACI-T-0468"
created_at: 2026-04-10T12:45:32.135745+00:00
updated_at: 2026-04-10T12:45:32.135745+00:00
parent: CLOACI-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0068
---

# Integration test: Python CG package reconciler routing (workflow vs CG path)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0068]]

## Objective

Reconciler must route Python CG packages through the CG loading path, not the workflow import path. The workflow path expects `@task` decorators and fails with "No tasks registered" for CG packages.

**Bug:** `loading.rs` sent all Python packages through `import_and_register_python_workflow` before CG routing. Python CG packages failed before reaching step 7.
**Fix:** Guard with `!has_computation_graph()`. Python CG packages now skip to CG route → `import_python_computation_graph` → `build_python_graph_declaration` → `load_graph`.

## Acceptance Criteria

- [ ] Python CG package loads via reconciler without "No tasks registered" error
- [ ] Python workflow package still loads correctly (regression check)
- [ ] Python CG graph registers in ReactiveScheduler and fires on WS events
- [ ] Package with both `package_type = ["computation_graph"]` and `language = "python"` takes the CG path

## Files

- `crates/cloacina/src/registry/reconciler/loading.rs` — routing logic
- `crates/cloacina/src/python/computation_graph.rs` — `build_python_graph_declaration`

## Status Updates

*To be added during implementation*
