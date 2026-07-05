---
id: in-place-python-package-version
level: task
title: "In-place Python package version upgrade silently loses tasks (module re-import is a no-op in the live interpreter)"
short_code: "CLOACI-T-0840"
created_at: 2026-07-05T20:00:01.551787+00:00
updated_at: 2026-07-05T21:12:13.160762+00:00
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

# In-place Python package version upgrade silently loses tasks (module re-import is a no-op in the live interpreter)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Upgrading a Python package IN PLACE on a running server (upload v-next of an already-loaded package name) silently produces an **empty workflow**: the reconciler unloads the old version, imports the new one — but the Python module import is a **no-op** (already in `sys.modules` from the previous version's load), so the `@cloaca.task` decorators never re-run, the fresh scoped Runtime registers 0 tasks, and the loader misclassifies the package as a "reactor library" ("no workflow tasks"). No error anywhere; a server restart fixes it (fresh interpreter → real import).

## Live evidence (2026-07-05, found while verifying T-0754)

Uploading `demo-py-workflow` 0.1.1 to a server that had loaded 0.1.0 at boot:
```
Unloaded package: demo-py-workflow v0.1.0
Reconciler: loading demo-py-workflow v0.1.1
cloacina_python::loader: Python reactor-library package imported: 2 reactors, no workflow tasks (public::demo-py-workflow::demo_py_workflow)
Python package loaded: demo-py-workflow v0.1.1 — 0 tasks, workflow 'demo_py_workflow'
```
The API then serves `tasks: []` for the workflow. After `docker compose restart server`, the same 0.1.1 row loads with all 4 tasks.

## Why

The agent already knows this trap — `cloacina-agent/src/main.rs` (process_work_packet) caches per-digest runtimes precisely because "re-importing a Python module in a live interpreter is a no-op (so the @task decorators wouldn't re-run and the new Runtime would be empty)". The SERVER's reconciler path (`python_runtime::load_workflow_package` → module import) has no such guard: the second load of the same module name reuses the cached module, whose decorators ran against the PREVIOUS load's scoped runtime.

## Fix directions (pick at implementation)

1. **Evict before re-import**: on package unload (or before the new version's import), drop the package's modules from `sys.modules` (the staging dir gives the module names) so the new import re-executes. Watch for stale references held by running tasks — coordinate with the unload lifecycle.
2. **Version-namespaced staging/import**: import the new version under a version-suffixed module path (unique `sys.path` root per (package, version), e.g. via `importlib.util` with an explicit name), so imports never collide across versions.
3. **Detect + refuse loudly** (stopgap): if a Python package load registers 0 tasks for a package whose metadata says it HAS tasks, fail the load with a clear "restart required / module cached" error instead of silently loading an empty workflow.

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (a routine version upgrade silently breaks the workflow until someone restarts the server)

### Impact Assessment
- **Affected Users**: anyone upgrading a Python package version on a live cloacina-server without a restart.
- **Reproduction Steps**:
  1. Seed the demo stack; let a Python package (e.g. demo-py-workflow 0.1.0) load.
  2. Upload the same package as 0.1.1 (any change).
  3. After build+reconcile: `GET /v1/tenants/public/workflows/demo-py-workflow` → `tasks: []`; executes fail/no-op.
- **Expected vs Actual**: expected the new version to load with its tasks; actual silent empty workflow until a server restart.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] Uploading v-next of a loaded Python package results in the new version's tasks registered and executable WITHOUT a server restart.
- [ ] The task graph + docs persist for the new version (same as a fresh load).
- [ ] Regression test covering the same-name re-import upgrade path.
- [ ] If a hard fix is deferred, the stopgap loud-failure (option 3) replaces the silent empty load.

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

### 2026-07-05 — FIXED via option (1) + LIVE-PROVEN, no restart (branch fix/t0840-py-module-reimport)
`import_and_register_python_workflow` now, before the import: (a) evicts the entry module's TOP-LEVEL package subtree from `sys.modules` so the new version's source actually executes; (b) prunes stale `sys.path` roots that can still resolve the top-level module (an old staging dir earlier in sys.path would otherwise win the fresh import and load the previous version's source). In-flight tasks from the old version are unaffected (Arc'd PyObjects live independent of sys.modules).

**Regression test** `python_reimport_after_upgrade_registers_new_tasks`: v1 (task `v1_task`) loads, then v2 at the SAME module path with a DIFFERENT task must register `v2_task` and not leak `v1_task` — fails without the eviction. Both import tests in the binary serialized on a shared key (the process-global workflow-context stack makes concurrent imports interfere; runtime load paths are already sequential).

**LIVE PROOF (demo stack, rebuilt server):** boot loaded 0.1.1 with tasks → uploaded 0.1.2 with a marked docstring → built → reconciled **without any restart** → API serves `version: 0.1.2`, all 4 tasks, and `doc_what: "Stage the demo batch (v0.1.2 upgraded IN PLACE) — …"` — the marker proves the NEW source executed, not a cached module. (Contrast: the identical 0.1.0→0.1.1 upgrade the day before loaded "0 tasks / reactor library" until a restart.) All ACs met; hard fix landed, no stopgap needed. COMPLETE.
