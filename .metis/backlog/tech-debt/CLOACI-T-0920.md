---
id: provider-contract-hygiene-converge
level: task
title: "Provider contract hygiene — converge the two contract crates, surface the native trust cliff at the call site"
short_code: "CLOACI-T-0920"
created_at: 2026-08-02T16:33:50.270635+00:00
updated_at: 2026-08-06T03:52:12.406251+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


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

## Acceptance Criteria

- [x] Single contract crate: all 9 path-deps retargeted to crates/cloacina-constructor-contract, spike copy DELETED (deletion is the guard — a future path-dep to it fails at build); all 9 crates build host + wasm32-wasip2
- [x] grants-present-but-unenforced is impossible to hit silently: runtime = "wasm"|"native" pin on constructor!/#[reactor]/cloaca.constructor, enforced before any load work at both consumer resolution sites; grants + native + no acknowledgment = load error naming the three ways out; invalid literal = compile error
- [x] PROVIDERS.md records the rule (runtime change = MAJOR/breaking) AND provider_wave.py enforces it — runtime recorded in COMPAT.toml, pr-check fails a flip without a major bump

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (providers-extensibility report; DEEPDIVE.md register medium tier). Verified against main @ 5216e632.
- 2026-08-06: DONE — merged to main in PR #236 (squash). CONVERGENCE WAS CHECKED BEFORE ACTING, which is the point: a structural diff of every pub type showed all 14 shared wire/manifest types byte-identical — crucially ConfigField's field order and types, so the positional width-sensitive fidius bincode binding was never actually at risk; the 773-line raw diff was doc-comment churn. Spike-only items (a vendored InputSlot, six unused ctors) had zero in-repo callers. WIRE COMPAT FORCED OUT-OF-BAND CARRIAGE: ConstructorPackageMetadata crosses a bincode wire via FFI method 10, so appending a pin field would mis-decode every already-built cdylib — the pin rides new *_pinned entry points instead, leaving the metadata struct and reconciler/loading.rs untouched (same shape T-0918 used for input_strategy; this is now the house pattern for FFI-crossing additions). CORRECTION TO THE FILED PREMISE: a constructor! consumer could never actually resolve a native provider at all — both loaders gate on find_wasm_package — so the real symptom was an opaque PluginNotFound, NOT a silent grant downgrade. enforce_runtime_pin now returns the native package dir so the acknowledged-native escape hatch genuinely works instead of failing a second time. Tests: new constructor_runtime_pin_wasm.rs (7/7) staging a native and a wasm provider in one search path — all four required cases plus the inverse pin, the grants-free native no-op, and an end-to-end #[workflow] through DefaultRunner proving the macro lowering threads the pin; regression sweep green across 12 constructor/provider suites (44 tests) + cloacina-macros (44). RESIDUAL (open): low-level primitive loaders still take &ResolvedGrants with no pin — by that point the "was anything asked for" signal is gone; every AUTHORED consumption surface funnels through the guarded fns, so a direct caller of the low-level API is opting out deliberately.
