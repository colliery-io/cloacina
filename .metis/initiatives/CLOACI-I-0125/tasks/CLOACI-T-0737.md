---
id: rust-package-shell-elimination
level: task
title: "Rust package-shell elimination — cloacina-workflow-sdk umbrella crate + compiler-injected build.rs/crate-type"
short_code: "CLOACI-T-0737"
created_at: 2026-06-17T05:33:12.893535+00:00
updated_at: 2026-07-06T00:54:43.150413+00:00
parent: CLOACI-I-0125
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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
- 2026-06-17: Filed from the T-0720 decomposition. Not started.- 2026-06-17: **BLOCKED — deferred pending fidius wasm traits.** fidius is
  introducing a wasm implementation of traits that may significantly reshape the
  authoring/packaging story (cdylib + FFI + build-shell model). Per the user,
  defer this cluster so we don't build something the wasm direction reworks.
  Unblock = fidius wasm-traits direction settles. See
  [[project_fidius_wasm_authoring_shift]].