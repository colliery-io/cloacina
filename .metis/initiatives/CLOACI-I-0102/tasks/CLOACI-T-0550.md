---
id: t-d-reactor-only-rust-cdylib-end
level: task
title: "T-D: Primitive-only Rust cdylib end-to-end integration tests"
short_code: "CLOACI-T-0550"
created_at: 2026-04-30T04:09:50.469748+00:00
updated_at: 2026-04-30T04:09:50.469748+00:00
parent: CLOACI-I-0102
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0102
---

# T-D: Primitive-only Rust cdylib end-to-end integration tests

## Parent Initiative

[[CLOACI-I-0102]]

## Objective

End-to-end integration coverage for the primitive-only and cross-package authoring shapes that I-0102 was opened to enable. Four fixture crates, one integration test module that exercises them through the actual reconciler:

1. **Reactor-only Rust cdylib** — `#[reactor]` only, no CG, no workflow, no triggers.
2. **Trigger-only Rust cdylib** — `#[trigger]` only (cron + custom), no CG, no workflow, no reactors.
3. **Mixed package** — combines a reactor, a custom trigger, a CG bound to that reactor, and a workflow subscribing to the trigger. Exercises the full precedence-ordered loader pipeline within a single cdylib.
4. **Cross-package binding** — separate cdylib whose CG references the reactor declared in fixture (1) by string name. Validates fan-out across packages and the runtime contract validator.

This is the "primitive-only Rust just works" + "string-named cross-package refs just work" proof point.

## Acceptance Criteria

### Fixture crates

- [ ] `examples/fixtures/reactor-only-rust/` cdylib crate. Contains exactly one `#[reactor]` declaration + `cloacina::package!();`. No CG, no workflow, no trigger. Builds via the existing angreal pre-build harness.
- [ ] `examples/fixtures/trigger-only-rust/` cdylib crate. Contains one `#[trigger(cron = "...")]` and one `#[trigger(custom)]` declaration + `cloacina::package!();`. No CG, no workflow, no reactor. Builds.
- [ ] `examples/fixtures/mixed-rust/` cdylib crate. Contains one reactor, one custom trigger, one CG bound to the reactor (`trigger = reactor("...")`), one workflow subscribing to the trigger (`triggers = ["..."]`) + `cloacina::package!();`. Builds.
- [ ] `examples/fixtures/reactor-subscriber-rust/` cdylib crate. Contains a CG with `trigger = reactor("shared_rx")` referencing the reactor name from `reactor-only-rust`. No reactor declaration of its own. Builds.

### Integration test coverage

- [ ] Integration test under `crates/cloacina/tests/integration/packaging.rs` (or a new sibling file) loads each fixture through the actual reconciler and asserts:
  - **Reactor-only:** package loads; reactor handle is addressable in the endpoint registry; `scheduler.list_graphs()` empty.
  - **Trigger-only:** package loads; cron trigger is registered with `cron_scheduler`; custom trigger is registered with `runtime`. No reactor, no graph.
  - **Mixed:** package loads; loader runs the precedence-ordered pipeline (cron → custom → reactor → CG bind → workflow bind) without errors; one event into the trigger fires the workflow; one event into the reactor's accumulator fires the CG.
  - **Cross-package binding:** load `reactor-only-rust`, then `reactor-subscriber-rust` separately. Subscriber's CG binds to the existing reactor by name. Pushing one event into the shared accumulator fires the subscriber CG.
  - **Cross-package contract mismatch:** if the subscriber declares incompatible accumulator names, load fails with a clear error naming the offending package + the missing reactor.
  - **Lifecycle ordering:** unloading `reactor-only-rust` while the subscriber is bound is rejected (T-0544 M4 lifecycle guard); unload subscriber first, then reactor-only — both succeed.
- [ ] Test runs on both backends (sqlite + postgres) via `angreal test integration`.

### Test gates

- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres` green.

## Implementation Notes

### Technical Approach

1. **Fixtures.** Borrow the layout from `examples/features/computation-graphs/packaged-graph/` for each fixture, stripping out everything but the targeted primitive. Cargo.toml mirrors existing fixtures (cdylib crate-type, `feature = "packaged"` enabled). Each fixture's `package.toml` is minimal: name + version only — no `package_type`, no `[[triggers]]` (per T-C and T-E).
2. **Test harness.** Extend `crates/cloacina/tests/integration/packaging.rs` (or add a sibling file `packaging_primitive_only.rs`). Reuse existing reconciler scaffolding; add helpers for "load fixture by name" + "push event to accumulator" + "push trigger fire."
3. **Pre-build wiring.** angreal's `test integration` task already pre-builds fixture cdylibs (see `.angreal/test/integration.py`). Add the four new fixtures to the pre-build list.

### Key Files

- `examples/fixtures/reactor-only-rust/` — new crate.
- `examples/fixtures/trigger-only-rust/` — new crate.
- `examples/fixtures/mixed-rust/` — new crate.
- `examples/fixtures/reactor-subscriber-rust/` — new crate.
- `crates/cloacina/tests/integration/packaging.rs` (or sibling) — integration tests.
- `.angreal/test/integration.py` — pre-build registration.
- `examples/Cargo.toml` (workspace members) — register the new crates.

### Dependencies

- **T-0547 (T-A)** — provides `cloacina::package!()`, `get_reactor_metadata`, `get_trigger_metadata`, and the macro-layer string-name surface.
- **T-0548 (T-B)** — provides the precedence-ordered loader that all four fixtures route through.
- **T-0549 (T-C)** — strips per-macro emission. Fixtures are authored against the unified shell from the start.
- **T-0551 (T-E)** — manifest cleanup. Fixtures' `package.toml` should be authored without `package_type` / `[[triggers]]` from day one; if T-E lands first, this task can assume hard-error semantics for those keys.

### Risk Considerations

- **Cross-package load order.** The subscriber-binds-late case must work even if the subscriber package is loaded *before* the reactor-only package, per T-B's "fail current load, retry on next reconcile pass" semantics. Test should cover both orderings.
- **Trigger event-fire plumbing.** Custom-trigger fan-out into workflows is end-to-end fresh territory; the test must drive it through the actual `runtime.fire_trigger(...)` (or equivalent) path, not a mocked shortcut, otherwise the test passes but the integration is fictional.

## Status Updates

*To be added during implementation.*
