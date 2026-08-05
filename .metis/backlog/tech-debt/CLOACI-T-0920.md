---
id: provider-contract-hygiene-converge
level: task
title: "Provider contract hygiene — converge the two contract crates, surface the native trust cliff at the call site"
short_code: "CLOACI-T-0920"
created_at: 2026-08-02T16:33:50.270635+00:00
updated_at: 2026-08-02T16:33:50.270635+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Provider contract hygiene — converge the two contract crates, surface the native trust cliff at the call site

## Objective

Two hygiene items on the provider/constructor surface from the deep dive. Neither is a defect in the shipped providers; both are drift traps in the contract layer that the wave process depends on.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (nice to have)

## Findings

1. TWO CONTRACT CRATES. The promoted crates/cloacina-constructor-contract coexists with the original spike copy under examples/constructor-contract, and the seed providers still path-dep the EXAMPLES copy. The kwarg-config machinery makes this dangerous: fidius binds config as positional, width-sensitive bincode reconstructed from ConfigField declaration order + Rust type names in a generated JSON manifest — the exact kind of contract where a silently-diverged crate copy breaks consumers invisibly (wave doctrine already says: renaming a config key is breaking even if no Rust signature moved). Fix: seeds depend on the promoted crate; delete or tombstone the spike copy; add a guard that no in-repo crate path-deps the examples copy.
2. NATIVE TRUST CLIFF IS INVISIBLE AT THE CONSUMPTION SITE. runtime is an emission target, so a provider can change wasm→native between versions — and a consumer's grants = {...} silently flips from enforced to decorative (load_native_member takes no grants at all, by design per I-0139(e)). Load-time log + CLI banner exist, but nothing at the constructor!(...) call site or at version-bump time warns the AUTHOR. Fix options: a manifest-recorded runtime pin in the consumer (constructor!(..., runtime = "wasm") that fails the load if the resolved provider is native), and/or a wave-guard rule flagging runtime changes as major-version-only; at minimum a compile/load-time warning when grants are present but unenforced.

## Acceptance Criteria

- [ ] Single contract crate; seeds on the promoted one; guard against path-depping the spike copy
- [ ] grants-present-but-unenforced is impossible to hit silently: either a consumer-side runtime pin, a wave rule, or a hard load-time warning (decided + implemented)
- [ ] Wave docs (providers/PROVIDERS.md) record the chosen rule

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (providers-extensibility report; DEEPDIVE.md register medium tier). Verified against main @ 5216e632.

- 2026-08-04 (item 1a — CRATE DIVERGENCE DIFF): **NOT materially diverged. Safe to converge.** Diffed
  `crates/cloacina-constructor-contract` (promoted, `cloacina-constructor-contract` v0.10, published)
  against `examples/constructor-contract/constructor-contract` (spike, `constructor-contract` v0.0.0,
  `publish = false`, own empty `[workspace]`). `src/lib.rs` 670 vs 553 lines; raw diff is 773 lines but
  almost entirely doc-comment/test churn. Structural comparison of every `pub struct`/`pub enum`:
  - **All 14 shared wire/manifest types are byte-identical** (modulo comments): `ConstructorError`,
    `PrimitiveKind`, `ConfigField`, `ConstructorManifest`, `ProviderRuntime`, `ProviderManifest`,
    `TaskInvocation`, `TaskOutcome`, `TriggerInvocation`, `PollOutcome`, `AccumulatorInvocation`,
    `AccumulatorOutcome`, `ReactorInvocation`, `ReactorOutcome`. **Critically: `ConfigField` field
    order + types are identical**, so the positional/width-sensitive fidius bincode binding is
    unaffected by the convergence.
  - PROMOTED-ONLY (superset): `METHOD_{EXECUTE,POLL,INGEST,EVALUATE,SOURCE}`, the five
    `*_CONSTRUCTOR_INTERFACE_VERSION` consts, `STREAM_ACCUMULATOR_INTERFACE`,
    `ProviderRuntime::grants_enforced()`. All host-side-only (consumed by
    `crates/cloacina/src/registry/loader/constructor_loader.rs`); no guest crate references them.
  - SPIKE-ONLY: a vendored `InputSlot` (promoted re-exports the canonical
    `cloacina_api_types::InputSlot`) and unused convenience ctors
    `AccumulatorOutcome::{emit,buffered,err}` / `ReactorOutcome::{fire,hold,err}`. Grepped: **zero
    in-repo callers** of those ctors, so removing them breaks nothing.
  - `InputSlot` delta is additive-and-compatible: canonical adds `#[serde(default)] pub encrypted: bool`
    (I-0133/T-0859) on top of the spike's `{name, schema, required, default}`. `params: Vec<InputSlot>`
    only ever crosses as JSON in `constructor.json` (never bincode), and the extra field defaults, so
    spike-emitted manifests deserialize unchanged.
  ⇒ Promoted is a strict functional superset. Proceeding with 1b/1c/1d.
  Path-deps on the spike (9): `examples/constructor-contract/{cloacina-provider-extract,
  cloacina-provider-quorum,cloacina-provider-sensor,accumulator-constructor-fixture,
  reactor-constructor-fixture,task-constructor-macro-fixture,task-constructor-twocfg-fixture,
  trigger-constructor-macro-fixture,native-task-provider-fixture}`. (Seed set confirmed = extract,
  quorum, sensor; `fs` already promoted to `providers/cloacina-provider-fs` on a crates.io version dep.)

- 2026-08-04 (item 1b/1c/1d — CONVERGED, DONE): all 9 spike path-deps retargeted to
  `cloacina-constructor-contract = { path = "../../../crates/cloacina-constructor-contract" }` (mirrors
  the existing `cloacina-macros` path-dep-into-the-workspace pattern these excluded crates already use);
  Rust refs renamed `constructor_contract::` → `cloacina_constructor_contract::` (incl. the
  `contract = ...` macro arg, kept explicit for parity with `providers/cloacina-provider-*`).
  **`examples/constructor-contract/constructor-contract` DELETED** — deletion is the structural guard
  (any future `path = "../constructor-contract"` now fails at manifest-load).
  Sweep: no live references remained anywhere — `.angreal/`, `scripts/`, `.github/`, certify harnesses,
  `emit_manifest` bins, and `providers/PROVIDERS.md` never named the spike; docs already document only
  `cloacina-constructor-contract`. Fixed two stale "this example's vendored contract crate" doc comments
  (task-/trigger-constructor-macro-fixture) and the stale provenance comment in the promoted Cargo.toml.
  **Builds (all green, rustc 1.96.1, wasm32-wasip2 already installed — no `rustup target add` needed):**
  `cargo check --all-targets` (host) for all 9; `cargo check --target wasm32-wasip2 --lib` for the 8
  wasm ones (extract/quorum/sensor + accumulator/reactor/task-macro/task-twocfg/trigger-macro fixtures);
  native-task-provider-fixture host-only (it is a cdylib host provider).

- 2026-08-04 (item 2 — DESIGN, before implementing):
  **2c WIRE VERDICT: FFI wire DOES force out-of-band.** The packaged declaration struct is
  `cloacina_workflow_plugin::ConstructorPackageMetadata` (`crates/cloacina-workflow-plugin/src/types.rs`),
  emitted by the `package!()` shell (`lib.rs` ~830) and read host-side via
  `handle.call_method::<(), Vec<ConstructorPackageMetadata>>(METHOD_GET_CONSTRUCTOR_METADATA /* 10 */)`
  in `crates/cloacina/src/computation_graph/packaging_bridge.rs:152`. That call is **bincode**
  (non-self-describing — the struct's own doc comment already notes it, which is why `config` values are
  pre-stringified). Appending a `runtime` field therefore shifts the layout and mis-decodes every
  already-built cdylib. ⇒ **the pin does NOT go on the FFI struct.** Carried host-side instead:
  a new `load_constructor_node_pinned(..)` / `load_reactor_constructor_node_pinned(..)` taking
  `Option<ProviderRuntime>`, with the existing unpinned fns delegating with `None`. This keeps
  `crates/cloacina/src/registry/reconciler/loading.rs` (the packaged/server call site) **untouched**.
  Packaged workflows therefore resolve UNPINNED — and are still covered, because rule 2b keys off
  `grants`, which ALREADY cross the wire.
  **Resolution site:** `load_constructor_node` / `load_reactor_constructor_node` in
  `crates/cloacina/src/registry/loader/constructor_loader.rs` — both the embedded (`#[workflow]` macro
  `OnceLock` load block) and the packaged/server (reconciler step 5b → `load_constructor_node`) paths
  funnel through them, so one check covers both.
  **Finding worth recording:** today a `constructor!`/`#[reactor]` consumer can NEVER resolve a native
  provider at all — both fns call `PluginHost::find_wasm_package`, which fidius 0.5.6 gates on
  `PackageRuntime::Wasm` (`fidius-host-0.5.6/src/host.rs:398-424`). A wasm→native provider flip today
  yields an opaque `PluginNotFound` ("locate provider package ... "), not a silent grant downgrade. The
  silent-downgrade surface is the DIRECT loaders (`load_task_constructor` etc., native fast-path at
  `constructor_loader.rs:483`) and the native stream-accumulator path. So the new check does double duty:
  it makes the trust cliff explicit AND replaces the opaque not-found with a diagnostic that names the
  runtime. Checks are ordered BEFORE `find_wasm_package` for exactly that reason.
  **Grammar:** `runtime = "wasm" | "native"` added to `constructor!(...)` (`ConstructorNodeDecl`,
  `crates/cloacina-macros/src/workflow_attr.rs`) and to `#[reactor(...)]`
  (`crates/cloacina-macros/src/reactor_attr.rs`), which shares `from`/`constructor`/`grants` verbatim.

- 2026-08-04 (item 2 — IMPLEMENTED, DONE):
  **Grammar (2a).** `parse_runtime_pin` in `workflow_attr.rs` (shared by both surfaces; a value other
  than `"wasm"`/`"native"` is a COMPILE error — a typo must never degrade to "unpinned").
  `constructor!` gained `runtime`; `#[reactor]` gained `runtime` (only valid alongside a `constructor`
  ref, mirroring the existing `config`/`grants` rules). `cloaca.constructor(..., runtime=...)` added for
  Python parity (`crates/cloacina-python/src/constructor.rs`), validated the same way.
  **Enforcement (2a/2b).** `enforce_runtime_pin` in `constructor_loader.rs`, called from
  `load_constructor_node_pinned` / `load_reactor_constructor_node_pinned` before any load work.
  Rule 1: pin ≠ resolved ⇒ error naming BOTH runtimes (exact in both directions). Rule 2: grants
  non-empty + resolved NATIVE + pin ≠ `Native` ⇒ error, verbatim:
  `grants are not enforced on native providers; either pin runtime = "wasm", acknowledge with
  runtime = "native", or remove grants`. Acknowledged path loads and emits a `tracing::warn!`.
  Grants-absent native is untouched.
  **Bonus fix.** `enforce_runtime_pin` returns the native package dir, and both consumer fns now use it
  instead of the wasm-only `find_wasm_package` — so the acknowledged-native escape hatch is REAL
  (a `constructor!`/`#[reactor]` node can now resolve a native provider at all, which it could not
  before; previously it died on an opaque `PluginNotFound`).
  **Wire (2c).** Confirmed out-of-band: `ConstructorPackageMetadata` untouched, `loading.rs` untouched.
  `ReactorConstructorRef` (`cloacina-computation-graph`) DID gain `runtime: Option<String>` — safe, it
  is a plain in-process struct with no `Serialize` and the packaged submission always carries `None`.
  It stays a `String` (not `ProviderRuntime`) so the CG leaf crate gains no contract-crate dep; the
  scheduler re-parses it fail-closed via the exported `parse_runtime_pin`.
  **Wave guard (2d) — the cheap seam EXISTED, so it was taken (not just documented).**
  `scripts/provider_wave.py`: new `provider_runtime()` (reads `[package.metadata.cloacina] runtime`,
  default `wasm`) + `compat_rows()` (tolerant row parser); `runtime` is now a recorded COMPAT.toml
  field; `check_runtime_flip()` in `pr-check` FAILS a runtime flip without a MAJOR bump and notes an
  allowed one. `providers/COMPAT.toml` regenerated (via the script, wave 2026.08.2 preserved) to
  backfill `runtime`: fs=wasm, kafka=native. Verified by driving the helpers directly.
  **Docs.** `providers/PROVIDERS.md` conventions gained the MAJOR/breaking rule with both enforcement
  sides; `docs/.../grants.md` gained "You cannot reach unenforced grants by accident";
  `consume-a-provider.md` (Rust + Python) documents the pin.
  **TESTS — `crates/cloacina/tests/constructor_runtime_pin_wasm.rs` (NEW, 7/7 green).** Stages BOTH a
  native provider (native-task-provider-fixture, provider.json patched) and a wasm one
  (task-constructor-macro-fixture, packaged) into one search path. Covers: pinned-wasm+native→error;
  pinned-native+wasm→error; grants+native+no-ack→error (exact message + all three remedies);
  grants+native+ack→loads; native+no-grants→unchanged; grants+wasm pinned/unpinned/via the unpinned
  entry point→unchanged; and a `#[workflow]` + `constructor!(runtime = "wasm")` executed end-to-end
  through `DefaultRunner` proving the macro lowering threads the pin.
  **Regression sweep, all green:** 12 constructor/provider suites — constructor_{runtime_pin,
  workflow_node,reactor_scheduler,macro,trigger_macro,trigger,accumulator,reactor,seed_library}_wasm,
  constructor_provider_native{,_package}, provider_bundle, packaged_constructor_e2e (44 tests);
  `cargo test -p cloacina-macros` 44 passed.
  **Validation:** `cargo fmt --all --check` clean; `cargo check` green for `-p cloacina`
  (`--no-default-features --features postgres,sqlite`; default; and
  `macros,sqlite,constructors-wasm,constructor-packaging --tests`), `-p cloacina-macros`,
  `-p cloacina-computation-graph`, `-p cloacinactl`, `-p cloacina-server`, `-p cloacina-agent`,
  `-p cloacina-python`, and the `fs-grant-demo` consumer example.
  **Residual:** the direct primitive loaders (`load_task_constructor` / `load_trigger_constructor` /
  `load_accumulator_constructor` / `load_reactor_constructor`) still take `&ResolvedGrants` with no pin
  — they are the low-level API used by tests and by the consumer fns above, and `ResolvedGrants` has
  already lost the "was anything asked for" signal by that point. Every AUTHORED consumption surface
  (`constructor!`, `#[reactor]`, `cloaca.constructor`) funnels through the guarded fns, so the cliff is
  closed at the call site; a direct caller of the low-level API is opting out deliberately.
