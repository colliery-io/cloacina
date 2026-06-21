---
id: injectable-input-descriptor
level: task
title: "Injectable input descriptor foundation — schemars + InputSlot type + JSON-Schema helpers"
short_code: "CLOACI-T-0755"
created_at: 2026-06-20T16:45:57.264407+00:00
updated_at: 2026-06-20T16:53:11.886941+00:00
parent: CLOACI-I-0128
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0128
---

# Injectable input descriptor foundation — schemars + InputSlot type + JSON-Schema helpers

Task A of [[CLOACI-I-0128]]. Spec: [[CLOACI-S-0013]]; decisions: [[CLOACI-A-0007]].

## Objective

Lay the shared foundation every other I-0128 task builds on: the API/wire type
for a declared input slot, and the runtime helper that turns a Rust type into a
JSON Schema. No surface wiring yet (that's tasks B–E).

## Deliverables

- **`InputSlot`** in `crates/cloacina-api-types/src/` — `{ name: String, schema:
  serde_json::Value, required: bool, default: Option<serde_json::Value> }`, with
  serde + `utoipa::ToSchema` behind the `openapi` feature; re-exported from the
  crate root. `schema` is a JSON Schema fragment (serde_json::Value).
- **`schema_for<T: schemars::JsonSchema>() -> serde_json::Value`** helper in
  `cloacina` (new small `input_interface` module) — generates a JSON Schema from
  a Rust type via `schemars` and returns it as `serde_json::Value`. The single
  place runtime schema generation lives; tasks B (workflow params) and D
  (accumulator/reactor) call it.
- `schemars` added as a workspace/`cloacina` dependency.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `InputSlot` exists in `cloacina-api-types`, serde round-trips, ToSchema
      under `openapi`, re-exported.
- [ ] `schema_for::<T>()` returns a valid JSON Schema `serde_json::Value` for a
      sample `#[derive(JsonSchema)]` type (unit-tested).
- [ ] `cargo check` clean on cloacina + cloacina-api-types; `angreal test unit`
      green.

## Open fork to resolve before Task B/D (carry path)

The ADR says JSON Schema via `schemars` AND compiler-side carry "not through the
FFI wire struct." For **rich** Rust types these pull against each other:
`schemars` derivation is a *runtime* operation (needs the compiled type), while a
pure source-parse (T-0752 style) can only map *scalar* declared types. Options
to settle in Task B/D:
- (a) compiler-side source-parse → JSON Schema for the v1 **scalar** param set
  (string/integer/number/boolean + default); rich types deferred;
- (b) a **new dedicated FFI descriptor entrypoint** returning the `schemars` JSON
  (does NOT touch the drift-prone `TaskMetadataEntry` bincode struct, so it
  respects the ADR's intent while getting full `schemars`);
- (c) build-time emission of the descriptor to a metadata file.
Lean: (b) for full fidelity without the drift-prone struct. **Confirm with
maintainer at Task B.**

## Status Updates

### 2026-06-20 — DONE + VERIFIED

- `cloacina-api-types`: `InputSlot { name, schema: serde_json::Value, required,
  default }` (+ `required`/`optional` ctors, `utoipa::ToSchema` under `openapi`),
  re-exported from the crate root.
- `cloacina`: added `schemars = "0.8"`; new `input_interface` module with
  `schema_for::<T: JsonSchema>() -> serde_json::Value` (re-exports `InputSlot`).
- Verified: `cargo check` clean on both; `angreal test unit` → **712 passed**
  incl. 3 new tests (scalar schema, struct properties, InputSlot round-trip).

**Carry-path fork still open for Task B (T-0756)** — see the "Open fork" section
above. schemars derivation is runtime; compiler-side source-parse handles only
scalars. Confirm the path (lean: a dedicated FFI descriptor entrypoint that does
NOT touch the drift-prone `TaskMetadataEntry`) before wiring workflow-param carry.
