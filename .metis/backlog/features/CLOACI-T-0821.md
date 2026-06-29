---
id: wasm-operator-spike-rust-wasm
level: task
title: "WASM-constructor spike: Rust -> WASM-component + configured-load (de-risk)"
short_code: "CLOACI-T-0821"
created_at: 2026-06-28T23:57:39.265598+00:00
updated_at: 2026-06-29T00:58:26.876437+00:00
parent: CLOACI-I-0132
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# WASM-constructor spike: Rust -> WASM-component + configured-load (de-risk)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

De-risk the linchpin: confirm a cloacina-macro-authored operator can compile to a **WASM component** and be loaded **configured** + invoked.

**AC:** a minimal Rust operator (one primitive, e.g. a task) compiles to a WASM component; cloacina loads it via fidius `load_wasm_configured(component, &config)` with bound config; invoking it returns the expected result end-to-end. Document the toolchain (wasm32 target / cargo-component / wit), Python's path (componentize-py vs `load_python_configured`), and any blockers. **Gates Phase B.** Blocked by CLOACI-T-0820.

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

### 2026-06-28 — SPIKE PROVEN END-TO-END (de-risked; gates Phase B / contract in T-0822)

A fidius-macro-authored SYNCHRONOUS operator compiles to a wasm32-wasip2 component
and is loaded **configured** (`load_wasm_configured`) + invoked, all inside cloacina.
Both host tests PASS. Spike is self-contained + workspace-excluded under
`examples/wasm-operator-spike/` (removable). NOT committed (per review gate).

**Layout**
- `examples/wasm-operator-spike/operator-fixture/` — author crate, `crate-type=["cdylib"]`,
  deps `fidius-guest=0.5.4`, `fidius-macro=0.5.4`, `wit-bindgen=0.44`, `serde`. Standalone
  (empty `[workspace]`). Built to `target/wasm32-wasip2/release/wasm_operator_fixture.wasm`.
- `examples/wasm-operator-spike/host-spike/` — host test, dep
  `fidius-host={version="0.5.4",features=["wasm"]}` + `fidius-macro`/`fidius-core`. Test
  `tests/operator_wasm_spike.rs` builds the fixture, stages it, loads + invokes.

**Working recipe (verified)**
1. Author: `#[fidius_macro::plugin_interface(version=1, buffer=PluginAllocated, crate="fidius_guest")]`
   on a `trait MinimalOperator: Send+Sync { fn apply(&self, input:String)->String; }`;
   `#[fidius_macro::plugin_impl(MinimalOperator, crate="fidius_guest", config=Config)] impl ...`;
   plus `impl Configured { fn configure(cfg: Config) -> Self {..} }`. `Config` is
   `Serialize+Deserialize`.
2. Build: `rustup target add wasm32-wasip2` (already present here); from the fixture dir
   `cargo build --target wasm32-wasip2 --release`. ~27s cold.
3. Validate: `wasm-tools validate --features component-model <wasm>` -> VALID COMPONENT (58,370 bytes).
   `wasm-tools component wit` shows exports:
   `apply: func(input: string) -> string`, `fidius-interface-hash: func() -> u64`,
   `fidius-configure: func(config: list<u8>)`, plus the world export
   `fidius:minimal-operator/minimal-operator@0.1.0`. Imports: only WASI
   (`wasi:io/error|streams`, `wasi:cli/environment|exit|stderr`) — deny-by-default, no
   egress unless declared.
4. Host load+invoke: host **re-declares the same interface** with `crate="fidius_core"` so the
   macro emits the matching descriptor `MinimalOperator_WASM_DESCRIPTOR` inside companion module
   `__fidius_MinimalOperator`. Then
   `host.load_wasm_configured("wasm-operator-pkg", &__fidius_MinimalOperator::MinimalOperator_WASM_DESCRIPTOR, &Config{op:"prefix"})`,
   `handle.call_method(0, &("test".to_string(),))` -> `"prefix: test"`. Two instances with
   `op:"prefix"` vs `op:"suffix"` coexist with independent persistent stores. Both tests green.

**Descriptor symbol**: `<Trait>_WASM_DESCRIPTOR` in module `__fidius_<Trait>` (kebab interface
export `fidius:<kebab>/<kebab>@0.1.0`). The host links against `descriptor.interface_export`;
the package.toml `interface` field is metadata only. `interface_version` must match (1).
Integrity is enforced via the `fidius-interface-hash` export vs `descriptor.interface_hash`.

**Sync constraint (CONFIRMED — central to T-0822 contract)**: WASM guests have no async
runtime, so operator methods MUST be synchronous primitives-in/out. cloacina's primitives
(`Task::execute`, `Trigger::poll`, accumulator, reactor) are async today; the right bridge is
an **async host-side wrapper that awaits a spawn_blocking-style call into the sync WASM
method** (the wasmtime call is a blocking host call). The existing async `CloacinaPlugin`
interface is NOT WASM-compatible and must not be the WASM operator trait — define a separate
minimal sync operator trait for the WASM contract.

**Config binding**: the `configure(cfg)` hook + macro-emitted `fidius-configure` export bind
config ONCE at load (`load_wasm_configured` serializes config and calls the export onto a
persistent store). Per-call methods then dispatch on the configured instance — config crosses
the sandbox exactly once. (Internally a OnceLock-style persistent store per instance.)

**Wire**: bincode; single-arg methods require a 1-tuple `(T,)` — `&("test".to_string(),)`.

**WASI**: deny-by-default. Component imports the standard WASI set even when unused; fidius
provides a zero-grant WasiCtx. Outbound HTTP egress only if the package declares the `http`
capability AND the embedder supplies an EgressPolicy.

**wasm-feature weight (relevant to gating the operator-execution crate)**: turning on
`fidius-host` `wasm` pulls wasmtime 45 + cranelift + wasmtime-wasi(+http) — ~55 wasmtime/cranelift
crates. Cold compile of the host test was **3m04s**; the host-spike `target/` reached **~1.0 GB**.
Strong signal: gate WASM behind a feature on the operator-execution crate so cdylib/Python-only
or non-operator builds stay thin. The guest/author side is light (wit-bindgen + fidius-guest,
~27s, 58 KB artifact).

**Sharp edges**: (1) benign `unexpected cfg "host"` warning from the macro under
check-cfg — cosmetic. (2) wit-bindgen pinned 0.44; component-model must be explicitly enabled in
`wasm-tools validate`. (3) author crate must be a standalone `[workspace]` so `cargo build`
doesn't attach to the cloacina workspace (and stays workspace-excluded via `examples/*`).

**Blockers**: none for the Rust path — fully de-risked. Python operator path
(componentize-py vs `load_python_configured`) was NOT exercised in this spike; carry into
T-0822 if a Python-authored WASM operator is in scope.

**Commands + results**
- `cargo build --target wasm32-wasip2 --release` (fixture) -> Finished, 26.75s.
- `wasm-tools validate --features component-model wasm_operator_fixture.wasm` -> VALID COMPONENT.
- `cargo test --release` (host-spike) -> `test configured_operator_loads_and_invokes ... ok`,
  `test n_differently_configured_instances_coexist ... ok`; `2 passed; 0 failed`.