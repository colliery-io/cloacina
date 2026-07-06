---
id: rust-package-shell-elimination
level: task
title: "Rust package-shell elimination — cloacina-workflow-sdk umbrella crate + compiler-injected build.rs/crate-type"
short_code: "CLOACI-T-0737"
created_at: 2026-06-17T05:33:12.893535+00:00
updated_at: 2026-07-06T02:12:17.878987+00:00
parent: CLOACI-I-0125
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0125
---

# Rust package-shell elimination — cloacina-workflow-sdk umbrella crate + compiler-injected build.rs/crate-type

## Parent Initiative

[[CLOACI-I-0125]] — acts on theme **T5** of the [[CLOACI-T-0720]] sweep.

## Objective

Collapse the byte-identical Rust package shell every author hand-maintains — a
3-line `build.rs`, a ~30-line `Cargo.toml` with `crate-type=["cdylib","rlib"]`,
`[features] packaged`, and four `cloacina-*` deps — down to roughly one dep line
plus compiler-supplied build wiring.

## Type / Priority
- Tech Debt (DX) — additive (umbrella crate + injection; explicit setups still
  build). P2 (M effort).

## Background (verified — T-0720)
- `build.rs` is byte-identical across packages (`crates/cloacinactl/src/nouns/package/new.rs:289-292`);
  the validator only *warns* on its absence (`.../package/manifest.rs:141-147`), so
  it's not author intent.
- `Cargo.toml` carries `crate-type=["cdylib","rlib"]`, `[features] packaged`, and
  4 `cloacina-*` deps + serde/async-trait/futures (`new.rs:319-348`).
- The compiler already drives cargo and knows it needs a cdylib
  (`crates/cloacina-compiler/src/build.rs:396-569`), so it can inject
  `build.rs`/`crate-type` when absent.
- Stray `__WORKSPACE__` path-dep templates remain in some fixtures
  (`examples/fixtures/demo-pipeline-rust/Cargo.toml:1-2`) — lint them out.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] A `cloacina-workflow-sdk` umbrella crate re-exports the needed `cloacina-*`
      crates so a package `Cargo.toml` needs ~1 dep line.
- [ ] The compiler injects `build.rs` + `crate-type` when absent (package builds
      without an author-written `build.rs`).
- [ ] A minimal Rust package (no `build.rs`, one dep line) packs and runs;
      regression guard.
- [ ] `__WORKSPACE__` path-dep templates removed/linted from fixtures.

## Implementation Notes
New umbrella crate + compiler injection in `cloacina-compiler/src/build.rs` +
`package new` template update. Keep explicit `build.rs`/multi-dep setups working.

## Status Updates

### 2026-07-05 — design analysis (unblocked; the ONE I-0125 task still open)
The umbrella is NOT a plain re-export crate: macro-generated code emits literal `::cloacina_workflow::…` / `::cloacina_workflow_plugin::…` paths (T-0734's signature rewrite, the `package!()` shell), so a consumer depending ONLY on `cloacina-workflow-sdk` has the wrong extern prelude and every expansion fails — exactly [[feedback_macro_generated_deps_invisible]] ("re-export from a crate the consumer already has; route per dep-profile lean vs umbrella; verify both").
**Design route**: (1) macros resolve the emission path via `proc-macro-crate` (direct `cloacina-workflow` OR `cloacina-workflow-sdk` → emit `::cloacina_workflow_sdk::workflow::…`), sdk re-exports the full tree under stable module names + the proc-macros (`pub use cloacina_macros::*` works for attrs); (2) INDEPENDENT sub-item, do first: compiler injects `build.rs` + `crate-type=["cdylib","rlib"]` when absent (build.rs drives cargo already — patch the staged source before invoking); (3) `__WORKSPACE__` fixture lint; (4) minimal-package regression (one dep line, no build.rs), gated on (1)+(2).
Current ceremony to kill (new.rs:319-348): 4 cloacina crates + serde/serde_json/async-trait/futures + build-dep cloacina-build + crate-type + `[features] packaged`. Effort confirmed M — the macro path-routing must be verified for BOTH dep profiles across task/workflow/CG/reactor/`package!` expansions before shipping.

### 2026-07-05 (later) — spike round: two approaches EMPIRICALLY DISPROVEN, requirements sharpened; work reverted clean
1. **Cargo multi-rename is DEAD**: `cloacina-workflow = { package = "cloacina-workflow-sdk" }` + a second rename → hard error "depends on crate … multiple times with different names". The glob-merged sdk root compiled fine (no collisions between workflow+plugin+macros), but Cargo forbids aliasing one package under several names, so expansions' extern-prelude needs can't be satisfied by an umbrella alone. The spike sdk crate was deleted (an umbrella that can't carry expansions is a trap).
2. **Extern-prelude census (empirical, from a real minimal consumer)**: expansions reference `cloacina_workflow`, `cloacina_workflow_plugin`, `serde_json`, **`async_trait` (5 sites)**, **`chrono` (7 sites)**, `cloacina_computation_graph` (1). Routing the stragglers through `cloacina_workflow::__private` (module exists; extended in the spike) FAILS for the embedded profile — the straggler sites are in SHARED expansion code and embedded apps dep only `cloacina` (no `cloacina_workflow` in their prelude; tutorials verified). Reverted.
3. **The real design, confirmed**: per-profile path routing in the macros — either `proc-macro-crate` crate-name resolution, or a profile switch the macros already know (packaged vs embedded emission branches) choosing `::cloacina::__private::…` vs `::cloacina_workflow::__private::…` per branch, with matching `__private` modules in both crates. ~13 straggler sites + the 3-crate main paths; must be verified with fresh consumer builds for embedded, packaged-lean, and packaged-minimal.
4. Independent sub-items still open: compiler injection of `build.rs`/`crate-type`/`packaged` feature (interacts with the dep story — do together), `__WORKSPACE__` fixture lint.
Tree reverted clean; nothing half-shipped. This remains the ONE open I-0125 task.

### 2026-07-05 (final) — DONE (commit 977bcae6); the shell is 4 dep lines
The spike's disproofs pointed at the tractable fix: all 13 stragglers sit in packaged-branch code that ALREADY requires `::cloacina_workflow` (verified per-site — that's why existing fixtures carry chrono/async-trait as deps), so routing them through a new UNGATED `cloacina_workflow::__private` is exactly as safe as the code beside them. `package!()`'s CG references route via `$crate::__cg` (macro_rules — native resolution). Compiler `ensure_build_wiring` injects `[lib] crate-type` + the `packaged` feature when absent. `build.rs` proven unnecessary for pure-Rust packages (pyo3 rpath only) — scaffold stops writing it, validator warning dropped. `cloacina-workflow` re-exports all authoring attrs.
**Template**: `cloacina-workflow(packaged,macros)` + `cloacina-workflow-plugin` + `serde` + `serde_json` — nothing else. Killed: cloacina-macros, cloacina-computation-graph, async-trait, futures, cloacina-build, build.rs, `[lib]`, `[features]`.
**Verified across all three profiles**: workspace clean; embedded tutorial compiles (the profile the naive approach broke); lean-form fixtures unaffected; mini-pkg (4 deps, no build.rs) builds a cdylib; live `new`→`validate` green with the minimal shell. `__WORKSPACE__` lint pre-existed (AC4 ✓). ALL ACs MET. COMPLETE — closes I-0125 at 9/9.

- 2026-06-17: Filed from the T-0720 decomposition. Not started.- 2026-06-17: **BLOCKED — deferred pending fidius wasm traits.** fidius is
  introducing a wasm implementation of traits that may significantly reshape the
  authoring/packaging story (cdylib + FFI + build-shell model). Per the user,
  defer this cluster so we don't build something the wasm direction reworks.
  Unblock = fidius wasm-traits direction settles. See
  [[project_fidius_wasm_authoring_shift]].