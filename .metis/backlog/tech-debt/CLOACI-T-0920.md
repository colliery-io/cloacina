---
id: provider-contract-hygiene-converge
level: task
title: "Provider contract hygiene — converge the two contract crates, surface the native trust cliff at the call site"
short_code: "CLOACI-T-0920"
created_at: 2026-08-02T16:33:50.270635+00:00
updated_at: 2026-08-04T05:01:34.149590+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/active"


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

## Acceptance Criteria

- [ ] Single contract crate; seeds on the promoted one; guard against path-depping the spike copy
- [ ] grants-present-but-unenforced is impossible to hit silently: either a consumer-side runtime pin, a wave rule, or a hard load-time warning (decided + implemented)
- [ ] Wave docs (providers/PROVIDERS.md) record the chosen rule

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (providers-extensibility report; DEEPDIVE.md register medium tier). Verified against main @ 5216e632.