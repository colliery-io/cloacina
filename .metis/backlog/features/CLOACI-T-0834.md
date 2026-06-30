---
id: constructor-capability-layer
level: task
title: "Constructor capability layer: grants to fidius EgressPolicy + WasiCtx, default-closed + load-time lint"
short_code: "CLOACI-T-0834"
created_at: 2026-06-29T16:17:54.400032+00:00
updated_at: 2026-06-29T18:57:47.668374+00:00
parent: CLOACI-I-0132
blocked_by: [CLOACI-S-0014]
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Constructor capability layer: grants to fidius EgressPolicy + WasiCtx, default-closed + load-time lint

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Implement the constructor capability & egress layer specified in [[CLOACI-S-0014]] (decision [[CLOACI-A-0009]]): tenant-authored, default-closed capability grants at construction time, enforced via fidius.

**Scope:**
- **Grant grammar + syntax** — a `grants = { http=[host:port|url], tcp=[host:port], fs=[paths], env=[keys] }` form, **identical** across `constructor!`, `#[constructor]`, `#[reactor]` (and structured so the Python/cloaca path mirrors it). Globs allowed.
- **Carry grants** from the consumer site through the declaration/manifest to the loader.
- **Translate + enforce** — cloacina builds a fidius `EgressPolicy` (`authorize` from http, `authorize_tcp` from tcp) + a scoped WASI `WasiCtx` (preopens from fs, env from env) and supplies it at `load_wasm_configured`. Default-closed: no grant -> deny.
- **Load-time capability lint (v1)** — inspect the component's imports (`wasi:http` / `wasi:sockets` / fs / env) vs the grants; warn (or fail) on an imported capability with no matching grant. Decided as v1 so we don't retrofit enforcement into the loader later.

**AC:** a constructor granted `http`/`tcp`/`fs`/`env` reaches exactly the scoped targets and nothing else (fail-closed); a constructor importing a capability with no grant is denied at runtime + flagged at load; the `grants` syntax is identical on all Rust surfaces. Gates [[CLOACI-T-0825]] (seed library). First step: confirm fidius-host's `WasiCtx` fs/env surface.

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

### 2026-06-29 — fidius surface verification (first step, DONE)
Verified fidius-host 0.5.4 (`crates/fidius-host/src/executor/wasm.rs`, `host.rs`):
- **fs**: `fs:ro:<path>` / `fs:rw:<path>` → `WasiCtxBuilder::preopened_dir`. Bare `fs`/`filesystem` rejected (FIDIUS-A-0008).
- **env**: `env:VAR_NAME` → `b.env(name, host_value)` — **passes through the HOST's env var by name** (skipped silently if unset). Bare `env` rejected (FIDIUS-T-0142). So v1 `env` grant = host-passthrough, not literal injection (matches S-0014 "env-sourced; literal/secrets deferred").
- **http**: cap intent `http` + host `EgressPolicy::authorize` (per-request). Two-key.
- **tcp**: cap intent `tcp` + `EgressPolicy::authorize_tcp(&SocketAddr)` (per host:port, default-deny). `tcp`/`udp` mutually exclusive with coarse `network`/`sockets`.
- Egress is injectable on the configured path via `PluginHost::builder().egress(policy)` (flows into `load_wasm_configured`). **No fidius change needed for egress.**

**Gap found:** fidius derives the capability list from the *signed package manifest* (`wasm_meta.capabilities` in `load_wasm_impl`); there is **no capability-override argument** on `load_wasm_configured`. But ADR A-0009 puts fs/env *specifics* (path, env key) in the tenant's hands at the consumer site, and those specifics live inside the cap string with no separate policy layer. → the tenant's translated caps must be injected **at load**, which fidius can't do today on the configured path. The executor itself (`WasmComponentExecutor::from_component_bytes_with_egress`) already takes `capabilities: Vec<String>` as a parameter; only `PluginHost` hardcodes it from the manifest.

**Decision needed (recorded, pending human):** how to inject tenant caps —
- (A) add `PluginHost::load_wasm_configured_with_grants(name, desc, cfg, caps, egress)` in fidius (clean; fidius 0.5.4→0.5.5 bump across both repos);
- (B) cloacina builds `WasmComponentExecutor` directly + wraps the handle (no fidius change; duplicates signature-verify / cwasm-resolution / interface-hash internals).

**Effective-caps model (chosen design):** the load-time caps = the *translated tenant grants* (default-closed: no grant → empty caps → deny-all WasiCtx). The manifest's `[wasm].capabilities` (author intent) feeds the **load-time lint** only (REQ-1.3.1), not the WasiCtx wiring.

### 2026-06-29 — decision: Option A (add fidius API), develop against local patch
cloacina pins fidius from **crates.io** (`="0.5.4"`, no `[patch]`), so Option A requires publishing **fidius 0.5.5**. Human chose A anyway (cleanest long-term).
**Sequencing:**
1. Implement `PluginHost::load_wasm_configured_with_grants(name, desc, cfg, caps, egress)` in `../fidius` (refactor `load_wasm_impl` to take `caps_override: Option<Vec<String>>`); bump fidius→0.5.5; add a test.
2. cloacina dev against a temporary `[patch.crates-io] fidius-* = { path = "../fidius/..." }` so the layer is fully built+tested locally.
3. **Human publishes fidius 0.5.5 to crates.io** (release action — blocks the cloacina layer PR merge).
4. cloacina pins `=0.5.5`, removes the patch, merges the capability-layer PR.

**Translation design (security core):**
- `grants.http=[host[:port] | url-glob]` → caps gets intent `"http"` + a `GrantEgressPolicy::authorize` matching request URI host/path vs patterns. `*` = any.
- `grants.tcp=[host:port]` → caps gets `"tcp"` + `authorize_tcp(&SocketAddr)`. NOTE fidius hands the policy a *resolved* `SocketAddr` (IP:port), not the hostname → v1 enforces **port (+ literal IP)**; a DNS-name host is resolved once at policy-build and matched by (ip,port). Document the limitation.
- `grants.fs=[ro:<path> | rw:<path>]` → caps `fs:ro:<path>`/`fs:rw:<path>`.
- `grants.env=[KEY]` → caps `env:KEY` (host passthrough).
- Empty/no grants → empty caps + no egress policy → deny-all (fail-closed).

### 2026-06-29 — STEP 1 DONE: fidius enabler + cloacina path-pin (no patch)
**fidius (`../fidius`):**
- Added `PluginHost::load_wasm_configured_with_grants(name, desc, &cfg, caps: Vec<String>, egress)` in `crates/fidius-host/src/host.rs`; refactored `load_wasm_impl` to take `caps_override: Option<Vec<String>>` (None → manifest caps; Some → REPLACES them). 3 existing callers pass `None`.
- New fixture `tests/wasm-fixtures/macro-configured-fs` (configured plugin + real `std::fs` read, appends bound `cfg.suffix`).
- New test `crates/fidius-host/tests/wasm_grants_e2e.rs` — 3 cases PASS: (a) load-time `fs:ro:` grant overrides empty manifest + config bound; (b) empty grant denies I/O even when manifest grants fs (override replaces, default-closed); (c) bare `env` rejected at load. `cargo fmt -p fidius-host` clean.
- **Pending (human):** bump fidius→0.5.5 + `cargo publish`. Commit not yet made in the fidius repo.

**cloacina:** per "no patch" — path-pinned all fidius deps to `../../../fidius/crates/*` (relative path deps, NOT `[patch.crates-io]`) in cloacina, cloacinactl, cloacina-compiler, cloacina-python, cloacina-workflow-plugin. Each marked `# TEMPORARY (CLOACI-T-0834) … re-pin to =0.5.5`. Lock re-resolved to a single source per fidius crate. `cargo check -p cloacina --features constructors-wasm` PASSES against local fidius.

### NEXT (STEP 2) — the cloacina capability layer
1. **Grant grammar** parsed identically in `constructor!` (workflow_attr.rs), `#[constructor]` (constructor_attr.rs), `#[reactor]`: `grants = { http=[..], tcp=[..], fs=[..], env=[..] }`. Lower to `Vec<(kind, Vec<String>)>` carried on the constructor node alongside the existing name-keyed `config`.
2. **Carry** grants from consumer site → constructor node lowering (`load_constructor_node`) → translate.
3. **Translate** (`grants → caps Vec<String> + Option<Arc<dyn EgressPolicy>>`): new module e.g. `registry/loader/grants.rs`. fs→`fs:ro:`/`fs:rw:`, env→`env:KEY`, http→`http`+policy, tcp→`tcp`+policy. `GrantEgressPolicy` (http: URI host/path match; tcp: resolve host→(ip,port) once, match `&SocketAddr`).
4. **Wire** every `load_wasm_configured(...)` site in constructor_loader.rs → `load_wasm_configured_with_grants(..., caps, egress)`.
5. **Load-time lint (v1)**: compare package manifest `[wasm].capabilities` (author intent) vs tenant grants; warn (tracing) on an imported capability with no matching grant. (True wasm-import introspection = later hardening.)
6. **Tests**: cloacina e2e — granted fs/env/http/tcp reaches exactly the scoped target, nothing else (fail-closed); identical `grants` syntax on all 3 Rust surfaces.

### 2026-06-29 — STEP 2a DONE: translation engine (`grants.rs`)
New `crates/cloacina/src/registry/loader/grants.rs` (mod added in loader/mod.rs, gated on `constructors-wasm`). Public API:
- `GrantSpec { http, tcp, fs, env: Vec<String> }` + `from_lists(..)` (the shape the macro lowers to) + `is_empty()`.
- `translate(&GrantSpec) -> Result<ResolvedGrants{capabilities: Vec<String>, egress: Option<Arc<dyn EgressPolicy>>}, GrantError>` — fail-closed: empty spec → empty caps + None egress (deny-all). fs→`fs:ro:`/`fs:rw:`, env→`env:NAME` (rejects `K=v` literal + empty), http→`http` marker + policy, tcp→`tcp` marker + policy.
- `GrantEgressPolicy` impl `EgressPolicy`: http matches URI host(glob)/port(exact)/path(glob) via a small `glob_match`; tcp matches `*` (any) / `*:PORT` (port) / resolved `(ip,port)` (host resolved once at load, literal IP exact).
- `lint_unmet_intents(manifest_caps, &GrantSpec) -> Vec<String>` (REQ-1.3.1 advisory).
- Uses `fidius_host::http_types` (re-exported `http`) — no new cloacina dep.
**14 unit tests PASS** (`cargo test -p cloacina --features constructors-wasm registry::loader::grants`). fmt clean.

### 2026-06-29 — STEP 2b IN PROGRESS: loader threading + macro grammar + lint
**Loader (`constructor_loader.rs`):** threaded grants through all 6 low-level loaders (`load_task_constructor`, `load_task_constructor_from_package`, `load_trigger_constructor`, `load_accumulator_constructor`, `load_reactor_constructor`, `load_constructor`) — each gained `grants: &ResolvedGrants` and now calls `host.load_wasm_configured_with_grants(.., caps, egress)`. The two consumer entries (`load_constructor_node`, `load_reactor_constructor_node`) gained `grants: GrantSpec`, `translate()` it (fail-closed → LoaderError), and call `lint_constructor_grants()` (reads pkg `[wasm].capabilities` via `fidius_core::package::load_manifest_untyped` → `lint_unmet_intents` → `tracing::warn!`). cloacina lib compiles clean.
**Grants carrier:** `ResolvedGrants::deny_all()`/`Default` added; `GrantSpec::from_pairs(Vec<(String,Vec<String>)>)` (the raw `(kind,patterns)` shape both macros lower to). `ReactorConstructorRef` (cloacina-computation-graph) gained `grants: Vec<(String,Vec<String>)>` (carried opaquely like `config`); scheduler builds `GrantSpec::from_pairs(cref.grants)`.
**Macro grammar (consumer surfaces, identical):** shared `parse_grants_block()` in workflow_attr.rs parses `grants = { http=[..], tcp=[..], fs=[..], env=[..] }` (unknown kind = compile error). Wired into `constructor!` (workflow_attr) → emits `GrantSpec::from_pairs(..)` into `load_constructor_node`; and `#[reactor]` (reactor_attr) → emits raw pairs into `ReactorConstructorRef.grants` (+ validation: grants only valid with a `constructor` ref). **Confirmed:** the `#[constructor]` AUTHOR macro takes NO grants — grants are a consumer concern (ADR A-0009). cloacina-macros compiles clean.
**Test call sites:** 30 call sites across 9 test files updated to the deny-all default (`ResolvedGrants::deny_all()` / `GrantSpec::default()` / `grants: vec![]`).
**STEP 2b VERIFIED GREEN:** all 9 `constructor_*_wasm` binaries pass — **33 tests, 0 failures** (accumulator 4, loader 3, macro 4, provider_package 4, reactor_scheduler 2, reactor 4, trigger_macro 3, trigger 6, workflow_node 3). The deny-all default did NOT regress the existing pure-compute fixtures. `cargo fmt --all` clean. Total layer coverage: 33 constructor e2e + 14 grants unit + 3 fidius grants e2e.

### 2026-06-30 — STEP 2c DONE: runnable example (the AC-closing artifact)
Built (per human: "the e2e test should be an example") two crates under `examples/constructor-contract/`:
- **`fs-grant-constructor`** (author) — a `#[constructor(kind=task, name="read_file")]` with one `#[config] path`; `execute` does `std::fs::read_to_string(self.path)`. Identical code regardless of consumer.
- **`fs-grant-demo`** (runnable `cargo run`) — packages the author crate into a WASM provider, stages it, runs the SAME constructor in two `#[workflow]`s: `granted` (`grants = { fs = ["ro:/tmp/cloacina-fs-grant-demo"] }`) and `ungranted` (no grants). Exits 1 LOUDLY if the ungranted run ever reads the secret.

**Verified `cargo run` (exit 0), with my own eyes:**
- granted → read `"the launch codes are 0000"` THROUGH the sandbox via the grant.
- ungranted → fidius denied: *"failed to find a pre-opened file descriptor through which …secret.txt could be opened"* → node Failed, secret never reached the downstream task (`contents=""`). Default-closed proven end-to-end.

Notes: `ConstructorError::msg(..)` is the available error ctor (no `::execution`). Demo deps mirror filtered-reactor (macro-generated `async_trait`/`chrono`/etc — see `feedback_macro_generated_deps_invisible`). Denial surfaces as `Ok(result)` w/ `status=Failed` (not `Err`) — demo checks both + treats only "Completed AND secret leaked" as a breach. Observed: cloacina ran the downstream task even though its dependency failed ("1 failed, 1 completed, 0 skipped") — no leak, but a scheduling quirk worth a separate look (dependents of a failed task arguably should skip). target/ gitignored; co-located under constructor-contract (NOT auto-registered in `angreal demos` — optional follow-up).

### 2026-06-30 — MERGE GATE CLEARED: re-pinned to crates.io fidius 0.5.5
fidius 0.5.5 published (lockstep — all 4 crates: fidius/core/host/macro). Re-pinned all fidius deps from the local path to crates.io `"0.5.5"` (dropped `path` + TEMPORARY comments) across cloacina, cloacinactl, cloacina-compiler, cloacina-python, cloacina-workflow-plugin. Lockfile now shows all four at 0.5.5 `source = registry`. Verified:
- `cargo build -p cloacina --features constructors-wasm` against crates.io → clean.
- Full constructor suite re-run against published 0.5.5 → **33 tests, 0 failures**.
- Default build still wasmtime-free (`cargo tree -p cloacina -i wasmtime` → none).
**T-0834 implementation COMPLETE + green on the published dep. Ready to commit/PR.**

### REMAINING (post-implementation):
- **Commit + PR** the cloacina-side work (branch off main; squash-merge; `cargo fmt --all`).
- Refresh the example crates' Cargo.lock against published fidius (re-run `cargo run` in fs-grant-demo).
- **Python/cloaca grants surface** — mirror the grammar (T-0831 territory, separate task).
- Optional: file the scheduling quirk (dependents of a FAILED task ran: "1 failed, 1 completed, 0 skipped" — no leak, but arguably should skip); wire the demo into `angreal demos`.
- **Python/cloaca grants surface** — mirror the grammar (T-0831 territory, separate task).
- Optional: wire the demo into `angreal demos`; http/tcp/env example variants (enforcement already covered by unit + fidius e2e).
