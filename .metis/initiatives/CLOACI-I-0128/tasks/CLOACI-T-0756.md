---
id: workflow-params-workflow-params
level: task
title: "Workflow params — workflow(params) authoring + compiler-side carry + API exposure"
short_code: "CLOACI-T-0756"
created_at: 2026-06-20T16:45:58.531737+00:00
updated_at: 2026-06-20T16:45:58.531737+00:00
parent: CLOACI-I-0128
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0128
---

# Workflow params — workflow(params) authoring + compiler-side carry + API exposure

Task B of [[CLOACI-I-0128]]. Spec [[CLOACI-S-0013]]; decisions [[CLOACI-A-0007]].
Depends on [[CLOACI-T-0755]] (done: `InputSlot` + `schema_for`).

## Objective

Let a workflow author declare typed params, and surface them as `InputSlot`s on
the workflow API — via the **dedicated FFI descriptor entrypoint** carry path
(ADR-0007 decision #4), so it works for rich types and shares one mechanism with
the accumulator/reactor surfaces (Task D).

## Technical approach (grounded anchors)

1. **Authoring (macro)** — extend `UnifiedWorkflowAttributes`
   (`crates/cloacina-macros/src/workflow_attr.rs:50`) to parse
   `#[workflow(params( name: Type [= default], … ))]` (dup/unknown rejection).
2. **Descriptor entrypoint (FFI)** — the macro emits a **new dedicated plugin
   method** (e.g. `get_input_interface() -> InputInterfaceDescriptor`) alongside
   the existing `get_task_metadata()` (`workflow_attr.rs:797`). The descriptor is
   a **new type** (its own (de)serialization) in `cloacina-workflow-plugin` —
   NOT a field on `PackageTasksMetadata` (the drift-prone struct). Schema per
   param via `schema_for::<T>()` (T-0755).
3. **Host extraction** — call the new entrypoint where the host reads plugin
   metadata (`crates/cloacina/src/registry/loader/package_loader.rs`
   `extract_metadata`); tolerate older packages that lack the method (Option/None).
4. **Compiler capture** — at build success, capture the descriptor into
   `workflow_packages.metadata` JSON alongside the existing merge
   (`registry/workflow_registry/database.rs` `extract_and_merge_build_metadata`),
   under a new `declared_params` field on `PackageMetadata` (`#[serde(default)]`).
5. **API exposure** — `declared_params: Vec<InputSlot>` on `WorkflowMetadata`
   (`registry/types.rs`) + `WorkflowDetail` (`cloacina-api-types/workflows.rs`),
   populated at the list/inspect build sites; OpenAPI regen.

## Acceptance Criteria

- [ ] `#[workflow(params(...))]` parses typed params with optional defaults;
      no-params workflows are unchanged.
- [ ] A dedicated FFI descriptor entrypoint returns per-param `InputSlot`s
      (JSON-Schema via `schema_for`), independent of `PackageTasksMetadata`.
- [ ] Declared params land in `workflow_packages.metadata` at build success and
      surface on `WorkflowDetail.declared_params`; undeclared → empty list.
- [ ] Back-compat: packages without the entrypoint deserialize/resolve to empty.
- [ ] `angreal test unit` + `angreal test integration` green; OpenAPI in sync.

## Notes / risks
- This is the macro + plugin-FFI + host + compiler + API vertical — the largest
  single I-0128 task. Build incrementally (macro parse → entrypoint → host →
  compiler → API), `cargo check` after each layer.
- Coordinates with I-0116 (it reuses this `declared_params` descriptor for its
  `WorkflowInstance` partials).

## Status Updates

*To be added during implementation*
