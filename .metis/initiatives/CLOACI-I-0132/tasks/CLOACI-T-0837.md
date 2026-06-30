---
id: provider-as-suite-contract
level: task
title: "Provider-as-suite contract: ProviderManifest = List[Constructor], N constructors per crate via name-in-configure (A-0011) + authoring docs"
short_code: "CLOACI-T-0837"
created_at: 2026-06-30T16:19:42.294491+00:00
updated_at: 2026-06-30T16:23:54.436840+00:00
parent: CLOACI-I-0132
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0132
---

# Provider-as-suite contract: ProviderManifest = List[Constructor], N constructors per crate via name-in-configure (A-0011) + authoring docs

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0132]]

## Objective **[REQUIRED]**

Implement the provider-as-suite contract decided in [[CLOACI-A-0011]]: one provider crate may declare **N constructors**, compiled into one WASM component, indexed by a **`ProviderManifest = List[Constructor]`**, with the member selected by **name carried in the `configure` payload** (so fidius + the host loader are essentially unchanged). Plus the authoring docs that make the model usable.

**Prerequisite for [[CLOACI-T-0836]]** (the bundle's `provider.json` IS this `ProviderManifest`) and for the held [[CLOACI-T-0832]] resolution.

**Scope:**
- **Contract crate** (`cloacina-constructor-contract`): make `ProviderManifest { name, version, constructors: Vec<Constructor> }` the primary type; consolidate today's `ConstructorManifest` into the embedded `Constructor` element (name, primitive_kind, interface, `config_fields`, deps, description); collapse the per-constructor `constructor.json` sidecars + the lightweight `ProviderConstructorEntry` into the one `provider.json` list.
- **`#[constructor]` macro**: allow N `#[constructor]` per crate; aggregate them into one component with a **name-dispatched `configure`** (decode `(constructor_name, config)` from the opaque `configure(&[u8])` bytes → instantiate the named constructor → bind its config); emit a crate-level `__provider_manifest()` (the list) for `emit_manifest`.
- **Packaging** (`package_constructor_provider` + `emit_manifest`): emit the consolidated `ProviderManifest`; one component, one `provider.json`.
- **Loader** (`constructor_loader`): read the `ProviderManifest`, select the member by the consumer's `constructor = "<name>"`, and serialize `(name, config)` into the `configure` payload (the only host-side change). Keep the fixed per-kind descriptor + interface hash.
- **`configure` wire format**: define + version the `(name, config)` encoding.
- **DOCS (first-class, A-0011 mandate)**: the authoring model — "a provider is a suite; author N `#[constructor]`s in one crate; `from` = the provider crate, `constructor` = the member"; the `cloacina-provider-<name>` convention; a worked multi-constructor example.

**AC:** a single provider crate exposing ≥2 constructors builds to one component + one `ProviderManifest`; a consumer wires two different members (and multiple instances of one) that coexist and run; the loader selects by name; no fidius change; authoring docs published.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P1 - High (important for user experience)

### Business Justification
- **User Value**: providers become libraries (`cloacina-provider-fs` → read/write/stat), not one crate per constructor — the model the `constructor=` selector always implied.
- **Effort Estimate**: M (macro codegen + contract consolidation + a one-line loader change + docs).

## Implementation Notes

### Dependencies
- Decided by [[CLOACI-A-0011]] (suite + name-in-configure) and [[CLOACI-A-0010]] (`from` = exact crate name).
- **Blocks** [[CLOACI-T-0836]] (build/bundle uses this `ProviderManifest`) and the held resolution in [[CLOACI-T-0832]].

### Risk Considerations
- The `configure` wire change is a provider-ABI detail — version it; existing single-constructor providers must still load (a suite of one = the same shape with a single member + the name in configure).

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

### 2026-06-30 — STARTED: contract `ProviderManifest` (the foundation)
Session goal (human): **demonstrate providers as a packaged entity in BOTH Rust and Python** (T-0835 deferred). Chain: T-0837 (this) → T-0836 (build/bundle) → T-0832 held step → packaged examples (rust + python) + server test.

**Done — contract crate:** added canonical `ProviderManifest { name, version, component, constructors: Vec<ConstructorManifest> }` (the `provider.json` / List[Constructor]) + `constructor(name)` lookup + to_json/from_json. Additive (Constructor ≡ the existing `ConstructorManifest` element), so nothing breaks yet; gives macro/packaging/loader a stable target. `cargo build -p cloacina-constructor-contract` clean.

**NEXT (in order):**
1. **macro `constructor_attr.rs` (the crux):** N `#[constructor]`/crate → one component w/ **name-dispatched `configure`** (decode `(name, config)` → instantiate the named member → bind); emit crate-level `__provider_manifest()` → `ProviderManifest`.
2. **packaging (`constructor_provider.rs` + `emit_manifest`):** emit the consolidated `provider.json`; replace packaging-local ProviderManifest/ProviderConstructorEntry with the contract type; one component.
3. **loader (`constructor_loader.rs`):** read `ProviderManifest`, select member by `constructor=` name, serialize `(name, config)` into `load_wasm_configured_with_grants`'s config arg. Keep the fixed per-kind descriptor.
4. **docs** (A-0011) + multi-constructor example.
Then T-0836 (bundle) + T-0832 held step + packaged rust/python examples.

### 2026-06-30 — MACRO DESIGN (execute cold — this is the keystone)
**Collision today:** `#[constructor]` is per-struct → each emits a top-level `__constructor_manifest()` (name-collides if 2 in a crate) + its own `#[plugin_impl(TaskConstructor)]` (a component can't impl the same fidius interface twice). So N-in-one-crate doesn't compile today. `constructor_attr.rs` is 1183 lines: `expand_task` (249), `expand_trigger` (601), `expand_event_kind` (851 — accumulator/reactor); each emits `__constructor_manifest()` (455/743/967) + a `#[plugin_impl(<Trait>, config=…)]` on `__<Struct>Configured` (497/773/1041) with a `configure(cfg)` hook.

**Architecture — mirror `cloacina::package!()`: per-item inventory + a crate-level shell.**
1. **Each `#[constructor]` struct** keeps its configured type + `configure(cfg)`/execute logic BUT stops emitting the top-level `__constructor_manifest()` and the fidius `#[plugin_impl]`. Instead it `inventory::submit!`s a per-kind registration: `{ name: &str, manifest: fn()->ConstructorManifest, make: fn(&[u8]) -> Box<dyn <Kind>Object> }`. `make` bincode-decodes the per-constructor config tuple → builds the configured instance → boxes it.
2. **New crate-level shell macro** (e.g. `constructor_provider!()`, exported from cloacina-macros lib.rs) generates, per KIND present in the crate:
   - ONE `#[plugin_impl(TaskConstructor, …)]` on a `__ProviderTask` whose persistent state is `Option<Box<dyn TaskObject>>`. `configure(cfg: &[u8])`: decode `(name: String, config_bytes)` → look up the named entry in the kind's inventory → `make(config_bytes)` → store the Box. `execute(json)`: dispatch to the stored Box.
   - `__provider_manifest() -> ProviderManifest { name, version, component, constructors: [reg.manifest() …] }` by walking the inventory.
3. **Object-safe per-kind traits** (new): `TaskObject { fn execute(&self, json: String) -> String }`, `TriggerObject { poll }`, `AccumulatorObject { ingest }`, `ReactorObject { evaluate }` — object-safe wrappers over the existing sync methods so the shell can hold `Box<dyn _>`; each constructor's `make` boxes its configured instance.
4. **`configure` wire format (versioned provider-ABI):** host serializes `(name, ordered_config)` as bincode `(String, <existing OrderedConfig tuple>)`; guest's `configure` decodes the `String` first, hands the remainder to the named `make`. Loader change = serialize the name alongside config into `load_wasm_configured_with_grants`'s config arg (its only host-side change).

**Why fidius-free:** the component still exports the SAME per-kind interface (one TaskConstructor) — same descriptor, same interface hash. The name-dispatch lives entirely in the guest's `configure` (opaque bytes) + an inventory lookup. Host/loader only add the name to the config bytes.

**Single-constructor providers = a suite of one** — same shell, one inventory entry, name in configure. Existing single-constructor fixtures + the embedded `fs-grant-constructor` example MUST keep working (regenerate manifests via the new path).

**Build order next session:** (a) object-safe per-kind traits + per-constructor `inventory::submit!` in `expand_*` (drop the per-struct plugin_impl/`__constructor_manifest`); (b) the `constructor_provider!()` shell — TASK kind first, then trigger/accumulator/reactor; (c) `emit_manifest` bin → call `__provider_manifest()`; packaging read path → `provider.json` = `ProviderManifest`; (d) loader: select member by `constructor=` name + name-in-configure; (e) convert `fs-grant-constructor` to a 2-constructor suite (read_file + write_file) and verify embedded (mirror constructor_workflow_node_wasm.rs); (f) THEN T-0836 (bundle) → T-0832 held resolution → packaged **rust + python** examples + server-load test → lands.

**State at break:** contract `ProviderManifest` added + compiles (cloacina-constructor-contract). Nothing else changed for T-0837 yet. T-0834 done+verified+green (uncommitted on main). T-0832 plumbing (P1/P2) in+green (uncommitted). Decisions all captured: A-0010, A-0011 (both decided), S-0015. Tasks: T-0837 (this, active) → T-0836 → T-0832 (held) ; T-0835 deferred. **Nothing committed/PR'd yet — a large uncommitted pile on main.**
