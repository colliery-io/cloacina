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

### 2026-06-30 (resume) — WIRE DE-RISKED + final build plan locked
Read the whole subsystem cold: macro (`constructor_attr.rs` 1183L), loader (`constructor_loader.rs` 1633L), packaging (`packaging/constructor_provider.rs`), the existing `constructor_provider_package_wasm.rs` test, fs-grant fixture, and fidius internals.

**KEY DE-RISK (the fragile heart — name-in-configure wire):** fidius serializes config through a SHARED module on BOTH sides — host `fidius_core::wire::serialize` (host.rs:495 `load_wasm_configured_with_grants`) and guest `#crate::wire::deserialize` (fidius-macro impl_macro.rs:388 `fidius_configure`). `fidius_core` is already a cloacina dep. So the suite wire is flavor-matched, zero bincode-version risk:
- Host: `inner = fidius_core::wire::serialize(&ordered_config)` (the SAME bytes the guest already decodes as `__XConfig` today — proven by 33 green tests) → wrap `__ProviderConfigure { name: String, config: Vec<u8> }` → hand `&wrapper` to `load_wasm_configured_with_grants` (fidius wire-serializes the wrapper: `wire(name) ++ wire(len-prefixed inner)`).
- Guest shell `configure(c: __ProviderConfigure)`: fidius decodes the wrapper → look up member by `c.name` → `member.__constructor_make(&c.config)` → `wire::deserialize::<__XConfig>` → box as `dyn TaskObject`.
fidius `configure` (impl_macro.rs:386) stores ONE `OnceLock<impl_type>` per `#[plugin_impl]` per LOAD, and the loader does a fresh `load_wasm_configured` per constructor instance — so read_file + write_file = two independent configured wasm instances of the one component. Name-in-configure is the only host→guest channel at load; confirmed unavoidable + now safe.

**DESIGN DECISION — inventory-free explicit-list shell (deviates from the doc's "inventory" sketch):** `inventory` relies on life-before-main; fidius only uses it for MULTI-plugin and its viability inside a fidius wasm guest is unproven. The shell instead takes an EXPLICIT member list `task = [ReadFile, WriteFile]` and references each member's associated fns — deterministic, host==wasm identical, greppable, no linker-section magic. One extra author line; arguably a clearer suite declaration.

**FINAL SHAPE:**
- *Contract crate:* add 4 object-safe traits `TaskObject/TriggerObject/AccumulatorObject/ReactorObject` (`fn <m>(&self, String)->String`, Send+Sync, pure-serde, no fidius dep).
- *`#[constructor]` macro:* STOP emitting the per-struct `#[plugin_interface]`, `#[plugin_impl]`, and free `__constructor_manifest()`. Instead emit (mostly un-gated, pure serde): clean struct + set/get + `__XConfig` + `__XConfigured{cfg}` + `impl <Kind>Object for __XConfigured` (the old `__constructor_run`) + ASSOCIATED `impl X { pub fn __constructor_name()->&str; pub fn __constructor_manifest()->ConstructorManifest; #[cfg(wasm32)] pub fn __constructor_make(&[u8])->Box<dyn <Kind>Object> } ` (make uses `wire::deserialize`).
- *`constructor_provider!` shell (NEW macro in cloacina-macros):* `name/version[/component/contract/fidius_crate], task=[..], trigger=[..], accumulator=[..], reactor=[..]`. Per kind present emit (wasm32): the ONE `#[plugin_interface] <Kind>Constructor` + `__Provider<Kind>Configure{name,config:Vec<u8>}` + `__Provider<Kind>{inner:Option<Box<dyn <Kind>Object>>}` + `#[plugin_impl(<Kind>Constructor, config=__Provider<Kind>Configure)]` (dispatch to inner, error-json if None) + `configure` (if-chain over `<Member>::__constructor_name()` → `__constructor_make`). Un-gated: `pub fn __provider_manifest()->ProviderManifest{ constructors: vec![<all members>.__constructor_manifest()...] }`. (Multi-KIND-per-provider → multi fidius plugin = needs multi-plugin-in-wasm; fs demo is task-only/single-plugin → safe; mixed-kind a noted follow-on.)
- *Packaging:* `emit_manifest` bin → `__provider_manifest()` (emits `provider.json` = contract `ProviderManifest`). `package_constructor_provider` writes `provider.json`, drops packaging-local `ProviderManifest`/`ProviderConstructorEntry` for the contract type; one component; `[wasm].interface` from the (homogeneous) members.
- *Loader:* add `read_provider_manifest` (provider.json); `load_task_constructor` GAINS `constructor_name` → select member from ProviderManifest, bind config vs THAT member's `config_fields`, serialize `(constructor_name, inner)` wrapper. host descriptor decl unchanged (still exports TaskConstructor). `load_constructor_node` selects member by `constructor=`.

**BLAST RADIUS (all under `constructors-wasm`/wasm32 — DEFAULT cloacina build stays green throughout):** contract, macro(+shell), loader sig, packaging, cloacinactl noun, 6 example fixtures (each +1 `constructor_provider!` line + emit_manifest→`__provider_manifest`), ~6 heavyweight wasm tests (call-site updates), NEW fs read_file+write_file suite, docs. Multi-session. Default `cargo build`/unit tests unaffected since fixtures+wasm tests are feature/target-gated.

**BUILD ORDER (keep tree compiling): (1)** contract traits → **(2)** macro keystone (cloacina-macros compiles; fixtures break, OK — gated) → **(3)** loader provider.json + name-in-configure → **(4)** packaging → **(5)** fixtures incl. fs suite → **(6)** wasm tests → **(7)** cloacinactl → **(8)** docs → then T-0836/T-0832/python.

### 2026-06-30 (resume) — KEYSTONE LANDED (steps 1+2 green)
**(1) Contract:** added 4 object-safe member traits `TaskObject/TriggerObject/AccumulatorObject/ReactorObject` (pure serde, no fidius). `angreal check crate cloacina-constructor-contract` ✅.
**(2) Macro keystone:** rewrote all 3 `expand_*` in `constructor_attr.rs` — each `#[constructor]` now emits the clean struct + `__XConfig` + `__XConfigured` + `impl <Kind>Object for __XConfigured` (the old `__constructor_run`, un-gated/pure-serde) + ASSOCIATED `impl X { __constructor_name(); __constructor_manifest(); #[cfg(wasm32)] __constructor_make(&[u8])->Box<dyn <Kind>Object> via fidius wire::deserialize }`. DROPPED the per-struct `#[plugin_interface]`/`#[plugin_impl]`/`configure`/free `__constructor_manifest()`. NEW `constructor_provider.rs` shell macro (`constructor_provider!`): per-kind `#[plugin_interface]` + `__Provider<Kind>Configure{name,config:Vec<u8>}` + `__Provider<Kind>{inner:Option<Box<dyn _>>}` + `#[plugin_impl(config=__Provider<Kind>Configure)]` (dispatch to inner) + name-dispatched `configure` (if-chain `<Member>::__constructor_name()`→`__constructor_make`) + un-gated `__provider_manifest()->ProviderManifest`. Registered as `#[proc_macro] constructor_provider` in lib.rs. `angreal check crate cloacina-macros` ✅. fidius wire confirmed = plain `bincode::serialize/deserialize` default-opts, `fidius_core::wire` public → host/guest flavor-matched.

**NEXT — step 3 loader** (`constructor_loader.rs`): add `PROVIDER_MANIFEST_FILE="provider.json"` + `read_provider_manifest` + `read_member_manifest(dir, name)`; host wire wrapper `struct ProviderConfigure{name:String,config:Vec<u8>}` + `wrap = ProviderConfigure{name, config: fidius_core::wire::serialize(ordered)}`; thread `constructor_name` through `load_task_constructor`/`load_trigger_constructor`/`load_accumulator_constructor`/`load_reactor_constructor` (+ `load_constructor`, `load_task_constructor_from_package`, `load_constructor_node`, `load_reactor_constructor_node`) — select member, validate kind+iface on the member, pass `&wrapper` to `load_wasm_configured_with_grants`. Then step 4 packaging (provider.json via contract type), step 5 fixtures (+`constructor_provider!`, emit_manifest→`__provider_manifest`, fs read_file+write_file suite), step 6 wasm tests, step 7 cloacinactl, step 8 docs.

### 2026-06-30 (resume cont.) — STEPS 3+4 GREEN (loader + packaging migrated)
**(3) Loader** (`constructor_loader.rs`): added `PROVIDER_MANIFEST_FILE="provider.json"`, `read_provider_manifest`, `read_member_manifest(dir,name)` (fails closed naming members), host wire wrapper `struct ProviderConfigure{name:String,config:Vec<u8>}` + `wrap_member_config()` (= `fidius_core::wire::serialize(ordered)` tagged w/ name). Threaded `constructor_name` through `load_task_constructor`, `load_task_constructor_from_package`, `load_trigger_constructor`, `load_accumulator_constructor`, `load_reactor_constructor`, `load_constructor`, `load_constructor_node`, `load_reactor_constructor_node` — each selects the member, validates kind+iface on the member, passes `&wrapper` to `load_wasm_configured_with_grants`. Dropped the now-redundant post-load name checks. `cargo check -p cloacina --features constructors-wasm` ✅ (wasmtime links).
**(4) Packaging** (`packaging/constructor_provider.rs`): rewrote around `cloacina_constructor_contract::ProviderManifest`; dropped packaging-local `ProviderManifest`/`ProviderConstructorEntry` + the redundant `constructor.json` write. `emit_manifest` now parses a `ProviderManifest` (from `__provider_manifest()`), overrides `.component` with the actual built artifact, writes ONE `provider.json`; `package.toml` interface/version/kind from the (homogeneous) members. `cargo check -p cloacina --features constructor-packaging` ✅. cloacinactl noun unaffected (same `ProviderPackageResult`).

ALL external callers of the changed loader sigs are in the gated wasm tests (step 6). `read_constructor_manifest`/`CONSTRUCTOR_MANIFEST_FILE` left in loader (pub, unused now) — clean up with tests.

**NEXT — step 5 fixtures (the demo path):** each of the 6 `examples/constructor-contract/*` fixtures needs (a) its `emit_manifest` bin → `__provider_manifest()`, (b) a `constructor_provider!(name,version,<kind>=[Struct])` line. THEN convert `fs-grant-constructor` to a 2-member suite (read_file + write_file) — the session-goal centerpiece. Then step 6 wasm tests (pass `constructor_name`; the provider-package test asserts `constructors == ["read_file","write_file"]`; missing-manifest test deletes `provider.json` not `constructor.json`), step 7 cloacinactl doc text, step 8 docs. Then T-0836 (bundle) → T-0832 held → packaged **rust + python** demos + server-load test.

### 2026-06-30 (resume cont.) — STEP 5 (macro fixtures) DONE + COMMITTED c1cb2ef5
**fs suite (centerpiece):** `fs-grant-constructor` is now a real 2-member suite (`read_file` + `write_file`, provider name `cloacina-provider-fs`). Builds to a wasm32-wasip2 component; `emit_manifest` emits a correct `provider.json` = `List[Constructor]` with BOTH members, one component. **The keystone is validated end-to-end at build+manifest level.**
**5 macro fixtures** (task-macro/accumulator/reactor/twocfg/trigger-macro) gained `constructor_provider!` + `emit_manifest→__provider_manifest`; ALL FOUR KINDS build to wasm components in suite mode. Vendored examples contract crate got `ProviderManifest` + 4 object traits. Fixture fidius deps bumped 0.5.4→0.5.5. Single-member fixtures keep provider name = constructor name (minimizes wasm-test churn).
**Committed c1cb2ef5** on `feat/i0132-constructors` (28 files). Pre-commit fmt + Cargo-check-both-backends PASSED; license hook added a header to the vendored contract (no dup).

### REMAINING (next session) — to a green `constructors-wasm` lane + the demo
**(6a) DONE ✅ — the Rust half of the session goal is PROVEN.** `constructor_provider_package_wasm.rs` migrated to the fs suite + **5/5 green through wasmtime** (committed b7e39fd3): package+sign `cloacina-provider-fs` → `provider.json` lists both members, one component → load `read_file` AND `write_file` BY NAME from the SAME signed archive (name-in-configure) → with fs grants, read_file READS a file + write_file WRITES a file (members coexist + run independently) → fail-closed on unknown member (names the suite), wrong key, tampered package, missing `provider.json`. Grants built via `translate(GrantSpec::from_lists([],[],["ro:<dir>"|"rw:<dir>"],[]))`. Run: `cargo test -p cloacina --features constructors-wasm --test constructor_provider_package_wasm`. **The packaged provider-as-entity works end-to-end in Rust.**
**(6b) 7/9 wasm test files MIGRATED + GREEN** (commits 45951604, fc45a460): `constructor_provider_package_wasm` (5), `constructor_macro_wasm` (4), `constructor_trigger_macro_wasm` (3), `constructor_accumulator_wasm` (4), `constructor_reactor_wasm` (3), `constructor_reactor_scheduler_wasm` (3), `constructor_workflow_node_wasm` (2, needed NO change — packaging+`load_constructor_node` suite-aware end-to-end). Pattern: stage `provider.json` (from `__provider_manifest()`), parse `ProviderManifest`+select member, pass `constructor_name` to loaders.
**REMAINING 2 = the RAW spike fixtures** (`task-constructor-fixture`, `trigger-constructor-fixture`, hand-written fidius glue with `configure(cfg)` NOT name-in-configure → incompatible with the new loader): tests `constructor_loader_wasm` (raw task; redundant w/ `constructor_macro_wasm`) + `constructor_trigger_wasm` (raw trigger + UNIQUE `load_constructor` runtime-registration coverage). AWAITING HUMAN DECISION: (A) convert raw fixtures to `#[constructor]`+`constructor_provider!` (then ~dup of macro fixtures), (B) retire raw fixtures + `constructor_loader_wasm`, migrate `constructor_trigger_wasm`'s `load_constructor` coverage to a macro fixture, (C) hand-write name-in-configure glue into the raw fixtures (preserves the "no-macro" proof).
**(6c) RAW fixtures** `task-constructor-fixture` + `trigger-constructor-fixture` (NON-macro, hand-written fidius glue, single `configure(cfg)` NOT name-in-configure) — used by `constructor_loader_wasm.rs` (+ maybe `constructor_trigger_wasm.rs`). DECISION NEEDED: either hand-write a suite shell for them (name-in-configure + provider.json) OR repoint those tests at the macro fixtures and retire the raw ones. Lean: repoint tests to macro fixtures (raw path is the obsolete T-0821 spike; macro fixtures cover it canonically) — but flag to human.
**(7) cloacinactl** noun: only doc-comment text mentions `__constructor_manifest` (harmless); optional polish.
**(8) embedded `fs-grant-demo`** — update for the suite (it consumes via `constructor!`/`load_constructor_node`, which already takes `constructor=`; mainly needs the provider staged under the provider_search_path + grants for read_file vs write_file).
**(8b) docs** (A-0011 mandate): authoring guide — provider=suite, `from`=provider crate, `constructor`=member, `cloacina-provider-<name>` convention, worked fs example.
**THEN:** T-0836 (resolve provider Cargo dep → build wasm → bundle into `.cloacina`) → T-0832 held resolution → packaged **rust + python** demos + server-load test → lands.

**Heavy test runs are the user's to kick (angreal):** the `constructors-wasm` lane builds N wasm components + wasmtime. Give the command, don't run in-tool.

**State at break:** host fully migrated+committed (c1cb2ef5); fs suite + all macro fixtures build to wasm + emit provider.json. wasm TESTS + raw fixtures + demo NOT yet migrated (that lane red on the WIP branch; default build/unit/fmt lanes green). T-0834 done+verified+green (uncommitted on main). T-0832 plumbing (P1/P2) in+green (uncommitted). Decisions all captured: A-0010, A-0011 (both decided), S-0015. Tasks: T-0837 (this, active) → T-0836 → T-0832 (held) ; T-0835 deferred. **Nothing committed/PR'd yet — a large uncommitted pile on main.**
