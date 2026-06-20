---
id: workflow-params-workflow-params
level: task
title: "Workflow params — workflow(params) authoring + compiler-side carry + API exposure"
short_code: "CLOACI-T-0756"
created_at: 2026-06-20T16:45:58.531737+00:00
updated_at: 2026-06-20T17:43:08.373753+00:00
parent: CLOACI-I-0128
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


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

## Acceptance Criteria

## Acceptance Criteria

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

### 2026-06-20 — BLOCKED on a carry-path realization (needs maintainer call)

Recon: the chosen carry path ("dedicated FFI descriptor entrypoint", ADR-0007 #4)
means adding a method to the **`CloacinaPlugin` fidius plugin-interface trait**
(`crates/cloacina-workflow-plugin/src/lib.rs:721`), which dispatches by
**positional method index** (`METHOD_* = 0..8`) under a load-time `INTERFACE_HASH`.
So it's a **plugin-ABI extension**: new method index 9 + `#[optional(since = 3)]`
+ interface-hash change, landed in the trait, the `package!` macro impl, and host
dispatch.

Supported pattern (how `get_reactor_metadata`/`get_trigger_metadata` were added at
v2) — **but it is the in-flux fidius packaging/FFI surface that
[[project_fidius_wasm_authoring_shift]] says to defer** until the fidius wasm
authoring shift settles. That deferral is why T-0752 went compiler-side and why we
said "no FFI" originally. The carry-path question didn't surface that "dedicated
FFI entrypoint" == extending this ABI.

Gates B (this), D (T-0758), E (T-0759). Options:
1. **Proceed** with the `#[optional(since=3)]` ABI extension (accept possible
   rework when fidius shifts).
2. **v1 without FFI**: scalar workflow params via compiler-side source-parse
   (covers T-0747's typed-execute case); defer rich-type + accumulator/reactor
   schemas (which need FFI) until fidius settles.
3. **Pause** FFI-dependent I-0128 tasks until the fidius shift lands.

Loop paused for the decision rather than grinding an ABI change against the
deferral guidance.

### 2026-06-20 — DECISION: proceed with the plugin-ABI extension (turnkey spec)

Maintainer chose option 1 — extend the `CloacinaPlugin` ABI. Fully de-risked
design below; execute as ONE atomic change (the `WorkflowDescriptorEntry` field
addition breaks every `#[workflow]` site until macro + host + compiler + API are
all updated — no compilable half-step).

**Wire types** (`crates/cloacina-workflow-plugin/src/types.rs`, `#[derive(Debug,
Clone, Serialize, Deserialize)]` like `PackageTasksMetadata`):
```
InputInterfaceDescriptor { entries: Vec<InputInterfaceEntry> }
InputInterfaceEntry { surface_kind: String, surface_name: String, slots_json: String }
```
`slots_json` = a JSON array of `cloacina_api_types::InputSlot` (kept as String so
the fidius bincode wire stays simple). Re-export both from plugin `lib.rs`.

**Inventory entry** (`inventory_entries.rs`): `WorkflowDescriptorEntry` gains
`pub params: fn() -> String` — mirrors the existing `triggers: fn() -> Vec<String>`
field. Runtime fn (set by the macro) returns the params' `slots_json`.

**Trait + ABI** (`lib.rs`): bump `#[fidius::plugin_interface(version = 2 -> 3)]`;
add `#[optional(since = 3)] fn get_input_interface(&self) ->
Result<InputInterfaceDescriptor, PluginError>;`; add
`METHOD_GET_INPUT_INTERFACE: usize = 9`. In `package!` macro impl, add the method
body: walk `inventory::iter::<WorkflowDescriptorEntry>`, call `(d.params)()` →
push `InputInterfaceEntry { surface_kind:"workflow", surface_name: workflow_name,
slots_json }`. (Task D later pushes accumulator/reactor entries here too.)

**Macro** (`cloacina-macros/src/workflow_attr.rs`): extend
`UnifiedWorkflowAttributes` (:50) to parse `params( name: Type [= default], … )`
(dup/unknown rejection). Emit, into the `WorkflowDescriptorEntry` inventory
submit, a `params` fn that builds the JSON at runtime:
`|| serde_json::json!([{ "name":"order_id", "schema":
cloacina::input_interface::schema_for::<String>(), "required": true }, …])
.to_string()`. No-params workflow → `|| "[]".to_string()`.

**Host** (`registry/loader/package_loader.rs`): after `get_task_metadata`, call
`handle.call_method::<(), InputInterfaceDescriptor>(METHOD_GET_INPUT_INTERFACE,
&())`; treat `NotImplemented` as empty (mirror the triggerless-graph optional
pattern at :554). Parse each entry's `slots_json` → `Vec<InputSlot>`; attach the
workflow entry to the extracted `PackageMetadata.declared_params`.

**Compiler/metadata** (`registry/.../database.rs` `extract_and_merge_build_metadata`):
carry `declared_params` into `workflow_packages.metadata` (new
`PackageMetadata.declared_params: Vec<InputSlot>`, `#[serde(default)]`).

**API**: `declared_params: Vec<InputSlot>` on `WorkflowMetadata`
(`registry/types.rs`, populate at every build site) + `WorkflowDetail`
(`cloacina-api-types/workflows.rs`); OpenAPI regen.

**Update all `WorkflowDescriptorEntry` construction sites** for the new `params`
field (the macro's embedded + packaged emission; any test fixtures).

Verify: `angreal check crate` per crate, then `angreal test unit` +
`angreal test integration` (the only lane that build+loads packaged workflows —
required to validate the new FFI method end-to-end).
