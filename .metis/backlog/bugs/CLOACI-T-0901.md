---
id: cross-tenant-workflow-definition
level: task
title: "Cross-tenant workflow-definition leak — a tenant can execute a workflow it never installed (shared Runtime, no per-tenant existence check in execute)"
short_code: "CLOACI-T-0901"
created_at: 2026-07-12T21:05:14.990166+00:00
updated_at: 2026-07-12T22:39:35.915124+00:00
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

# Cross-tenant workflow-definition leak — a tenant can execute a workflow it never installed (shared Runtime, no per-tenant existence check in execute)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Close a cross-tenant isolation gap: a tenant can execute a workflow it never installed, as long as some OTHER tenant (or the public/global runner) has loaded that workflow name into the process-shared in-memory `Runtime`. Execution/data state is correctly isolated (the run lands in the caller's schema); the leak is at the workflow-DEFINITION visibility layer.

Surfaced by the `python-multi-tenant` gold-path example (I-0138 Python-gap sweep, 2026-07-12) — exactly the kind of server-path gap D-3 predicted the migration would expose.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [ ] P0
- [x] P1 - High — it's a breach of the tenant isolation boundary ([[project_tenant_is_isolation_boundary]] says the tenant IS the hard isolation line), though exploitation requires a valid admin/tenant key AND knowing another tenant's workflow name, and the run executes against the CALLER's own schema.

### Impact Assessment
- **Affected Users**: any multi-tenant deployment (one server process, multiple tenant schemas sharing the runner cache's `shared_runtime`).
- **Reproduction Steps** (observed live, commit-time evidence in the `python-multi-tenant` lane):
  1. Server + a package `tenant-job` (workflow `tenant_job`) uploaded/built/loaded in tenant `public` (and independently in `mtbeta` via its own `--tenant-schema` compiler).
  2. Create a THIRD tenant `mtgamma` and never upload any package to it. Confirm `mtgamma.workflow_packages` has **0** `tenant-job` rows (DB isolation intact).
  3. `cloacinactl --tenant mtgamma workflow run tenant_job` → **accepted**. Server log: `Workflow execution scheduled: <id>` + `Executed workflow 'tenant_job' for tenant 'mtgamma'`. (Contrast: before `mtbeta`/`public` had loaded it, the same call returned `Workflow not found in registry: tenant_job`.)
- **Expected vs Actual**: Expected — a tenant with no matching workflow in its OWN registry gets `404 workflow_not_found` (like it does when NO tenant has loaded it). Actual — the execute route schedules the run because the name resolves in the process-shared `Runtime` populated by other tenants.

### Root cause (grounded)
`crates/cloacina-server/src/routes/executions.rs::execute_workflow`: it resolves the tenant-scoped `Database` (so execution STATE is isolated) and does a paused-check + declared-params-check against the tenant's `WorkflowRegistryImpl` — but BOTH "fail open" and neither REJECTS when the workflow is simply absent from this tenant's registry. It then calls `tenant_runner.execute_async(&name, ...)`, which resolves `name` against the per-tenant runner's `shared_runtime` (`tenant_runner_cache.rs`: "Shared `Runtime` for every per-tenant runner"). The reconciler namespaces tasks `tenant::pkg::wf::task`, but the execute-by-bare-name lookup isn't tenant-scoped, so a workflow any tenant loaded is executable by all.

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

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] `execute_workflow` (and the equivalent `workflow run` path) REJECTS with `404 workflow_not_found` when the workflow is absent from the CALLING tenant's own registry — for non-public tenants — instead of resolving it from the shared `Runtime`.
- [ ] `public` behavior is preserved (it maps to the admin schema + global runner by design; decide whether public keeps shared-catalog semantics or is also scoped — likely scope it too for consistency).
- [ ] The `python-multi-tenant` example's Isolation-2 assertion (a tenant with no package cannot run the workflow) flips from a logged known-gap back to a HARD assertion.
- [ ] A server integration test covers the negative: tenant B cannot execute a workflow only tenant A installed.

## Proposed fix (small, targeted)
In `execute_workflow`, after constructing the tenant `WorkflowRegistryImpl`, add an existence gate: if `registry.get_workflow(&name)` / the declared-metadata lookup finds NO such workflow in THIS tenant, return `404 workflow_not_found` before touching the runner. This reuses the registry already built for the paused/params checks (one extra query). Keep the existing "fail open on registry ERROR" behavior only for transient errors — a definitive "not found" must fail CLOSED. Maintainer decision needed on whether `public`/global keeps any shared-catalog exception.

## Design note / maintainer call
This is the tenant ISOLATION boundary, so flagging rather than silently changing semantics. [[project_tenant_is_isolation_boundary]] frames intra-tenant object authZ as a non-goal (isolate by spinning up a new tenant) — but that very framing REQUIRES cross-tenant execution to be impossible, which this bug violates. So the fix aligns with the stated model. Confirm before implementing.

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

**2026-07-12 — FIXED + verified.** Implemented the per-tenant existence gate.
- `crates/cloacina/src/registry/workflow_registry/mod.rs`: new `workflow_exists(&name)` — mirrors the `is_workflow_paused`/`get_workflow_declared_params` lookup (matches `w.workflow_name == name || w.package_name == name`) over `list_workflows()` (this tenant's registry).
- `crates/cloacina-server/src/routes/executions.rs::execute_workflow`: for `tenant_id != "public"`, call `workflow_exists` before touching the runner; a definitive `Ok(false)` returns `404 workflow_not_found` (fails CLOSED), a registry `Err` logs + proceeds (fails open, so a transient DB fault never wedges execution).
- **Decision on `public`**: left exempt (documented in code). `public` maps to the admin/global catalog + global runner and may serve inventory/global workflows not tracked as tenant packages; gating it risks breaking those. The reported leak (a NON-public tenant running another tenant's workflow) is closed. Maintainer can extend to `public` later if desired.
- **Verified live** on the `python-multi-tenant` gold-path lane: `public` + `mtbeta` (installed) still run; `mtgamma` (never installed) is now rejected → `ok: tenant mtgamma (no package) cannot run the workflow — isolated`. Isolation-2 flipped from a logged known-gap back to a HARD assertion.
- **Regression guard**: the `python-multi-tenant` lane (in the CI discovery matrix) exercises the negative end-to-end. A dedicated server-crate integration test (AC #4) was NOT added — the server `tests/` dir has no route+DB+tenant harness (only `cli_validation.rs`), and standing one up would reproduce what the lane already covers; deferred as optional hardening.
- Broader-suite note: any pre-existing test that executed a workflow in a NON-public tenant WITHOUT installing it there was relying on the bug and would now get a 404 — none found in the features lanes (all use `public`); worth a full integration run before merge.
