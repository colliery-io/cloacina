---
id: accumulator-and-reactor-input
level: task
title: "Accumulator and reactor input interface derivation + API exposure"
short_code: "CLOACI-T-0758"
created_at: 2026-06-20T16:46:01.362198+00:00
updated_at: 2026-06-21T00:23:41.618139+00:00
parent: CLOACI-I-0128
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0128
---

# Accumulator and reactor input interface derivation + API exposure

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0128]]

## Objective **[REQUIRED]**

{Clear statement of what this task accomplishes}

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

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

### 2026-06-20 — Scoped + grounded (ready to build); paused for focused resume

Task D of [[CLOACI-I-0128]]. Builds on B's `get_input_interface` entrypoint
(which currently emits only `surface_kind:"workflow"` entries) + the
`schema_for` helper (T-0755) + the `InputInterfaceEntry`/`Descriptor` wire types.

Recon findings (anchors):
- Accumulator boundary types ARE known at macro-expansion time:
  `crates/cloacina-macros/src/computation_graph/accumulator_macros.rs` —
  `extract_return_type(output)` yields `output_type` (the boundary `Output`), and
  the source fn is `fn name(event: EventType) -> OutputType` so `EventType` (the
  injected event) is also available. So both schemas are derivable via
  `schema_for::<EventType>()` / `schema_for::<OutputType>()`.
- Inventory entries (`cloacina-workflow-plugin/src/inventory_entries.rs`):
  `ReactorEntry` / `ComputationGraphEntry` carry accumulator **names**
  (`accumulator_names`) + `reaction_mode` but NOT boundary **types**. D must add
  a schema-bearing field (e.g. `fn() -> String` slots, like
  `WorkflowDescriptorEntry::params`) populated by the accumulator/CG macro.

Plan:
1. Accumulator/CG macros (`accumulator_macros.rs` + `codegen.rs`) emit a
   `schema_for::<…>()`-built slots JSON into the relevant inventory entry
   (accumulator input = EventType; reactor per-source = upstream Output).
2. Extend `package!` `get_input_interface` to also walk reactor/accumulator
   inventory and push `surface_kind:"accumulator"`/`"reactor"` entries.
3. Host: the descriptor parse already handles all entries; route the
   accumulator/reactor entries to the CG-health detail responses
   (`routes/health_graphs.rs` + `cloacina-api-types` accumulator/reactor types).

Size: comparable to Task B (deep CG-macro + ABI-walk + API). Tractable; the
hard de-risking (FFI entrypoint, schema_for, wire types) is already done in A/B.
**Paused here** at a clean boundary — A/B/C (workflow vertical) shipped + verified
+ committed; D is a fresh focused vertical, resumable from this grounded spec.
Gates E (T-0759) typed accumulator/reactor injection validation.

### 2026-06-20 — BLOCKED: boundary types need `JsonSchema` (CG authoring change)

Deeper recon found a genuine wall, distinct from B's. The reactor/accumulator
**boundary types are captured at the `#[computation_graph]` macro** (node-fn
signatures, e.g. `compute(alpha: Option<&AlphaIn>) -> ReactorOutput` → source
`alpha` has type `AlphaIn`), NOT via `#[passthrough_accumulator]` (mixed-rust
doesn't use it; `accumulators=[alpha]` is name-only and packaged
`AccumulatorDeclarationEntry` is `{name, type-category, config}` — no Rust type).

The blocker: those boundary types (`AlphaIn`, `ReactorOutput`, …) derive only
`Serialize`/`Deserialize` — **not `schemars::JsonSchema`**. `schema_for::<T>()`
requires `T: JsonSchema` at compile time, with no stable way to do it
opportunistically. So deriving accumulator/reactor schemas requires **making
`JsonSchema` a required bound on every CG boundary type** — a breaking change to
the CG authoring contract + `#[derive(JsonSchema)]` across all CG examples/
fixtures. That is the in-flux fidius/CG authoring surface, and it's a
maintainer-level authoring decision (unlike B's clean optional method).

**Options:**
1. Require boundary types to derive `JsonSchema` (macro requires/auto-adds it;
   update all CG fixtures). Sizable; changes the CG authoring contract.
2. Defer D until the CG authoring settles (fidius shift); ship accumulator/
   reactor surfaces as **names-only** (no rich schema) for now, or not at all.
3. Best-effort schemas — not feasible: `schema_for` needs the compile-time bound;
   no stable specialization to skip types lacking it.

Gates **E (T-0759)** (typed accumulator/reactor injection validation). Does NOT
gate **T-0753** (untyped accumulator injection endpoint), **F** (workflow-params
Python), or **G** (workflow docs/tests) — the loop continues on those.
Blocked pending the maintainer's call on the boundary-`JsonSchema` requirement.

### 2026-06-20 — DECISION: opt-in (not mandatory), via autoref specialization

Maintainer asked "can we provide an interface without making it mandatory" —
**yes**. Resolved with a 4th option better than the three above: a stable-Rust
**autoref-specialization probe** (`SchemaProbe<T>` + `ProbeTyped`/`ProbeFallback`
in `cloacina-workflow::input_interface`). The `#[computation_graph]` macro emits
the probe per boundary type; it resolves to `schema_for::<T>()` when
`T: JsonSchema` and to a permissive `{}` otherwise. Authors opt a boundary type
into rich typing by adding `#[derive(JsonSchema)]`; others degrade gracefully —
**no required derive, no breakage**.

### 2026-06-20 — Part 1 DONE + VERIFIED (derivation + ABI); consumption remains

Committed (`feat: opt-in typed accumulator/reactor input interface … part 1`):
- `SchemaProbe`/`ProbeTyped`/`ProbeFallback` (autoref specialization) +
  core re-export. Unit-tested (typed→schema, untyped→`{}`, scalar→schema).
- CG macro captures each cache source's boundary type from the consuming node
  fn signature (`Option<&AlphaIn>` → `AlphaIn`) and emits InputSlots via the
  probe into a new `ComputationGraphEntry.input_interface` fn (per build-mode
  path; core re-export updated so the non-packaged arm resolves).
- `package!` `get_input_interface` now also emits `graph` + `reactor` surface
  entries (typed slots) and names-only `reactor` entries for CG-less reactors.
- `mixed-rust` fixture opts `AlphaIn`/`ReactorOutput` into `JsonSchema`.
- Verified: unit + integration green (314+98+6, 0 failed); new FFI test
  `mixed_fixture_exposes_typed_reactor_input_interface` asserts the reactor
  surface carries a typed `alpha` slot.

**Remaining (Part 2 — consumption, the runtime/registry bridge):**
1. Host: capture the `graph`/`reactor`/`accumulator` entries from
   `get_input_interface` (today `package_loader` keeps only `workflow` →
   `declared_params`) into package metadata / endpoint registration.
2. CG-health API: expose declared input schemas on
   `AccumulatorStatus`/`ReactorStatus` (join runtime status with declared
   surfaces).
3. **E (T-0759)**: validate reactor-fire / accumulator-inject payloads against
   the surface's declared slots — reuse `validate_declared_params` (T-0757) once
   the schema is reachable at the fire/inject handler.

This Part-2 bridge touches the live endpoint-registry / CG-health surface (the
in-flux runtime area); the hard, decision-bearing derivation is done.

### 2026-06-20 — Part 2 DONE + VERIFIED (D complete)

Committed (`feat: validate + expose accumulator/reactor input interfaces … part 2`):
- **Host capture**: `package_loader` now splits the `get_input_interface`
  entries — `workflow` → `declared_params`, everything else → a new
  `PackageMetadata.declared_surfaces` (`Vec<DeclaredSurface{kind,name,slots}>`),
  carried through the build-success merge.
- **Registry queries**: `WorkflowMetadata.declared_surfaces` +
  `find_surface_input_slots(kind, name)` / `find_accumulator_input_slot(name)`.
- **API exposure**: `GET /v1/health/{reactors,accumulators}/{name}/interface`
  return the declared `DeclaredSurface` (read-only discovery; no list-hot-path
  coupling; backs typed UI forms without touching the frozen frontend). OpenAPI
  synced.

Chose the package-metadata carry path over threading schemas into the live
endpoint registration (which lives deep in the in-flux runtime spin-up) — the
CG-health endpoints are global, so the admin/public registry is the correct
scope and the query is cheap for infrequent operator actions.

Verified: unit + integration green (314+100+6, 0 failed). **D complete.**
