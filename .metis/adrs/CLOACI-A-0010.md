---
id: 001-constructor-provider-distribution
level: adr
title: "Constructor provider distribution rides Cargo's dependency model (crates.io + path/git), independently versioned, build-time-resolved and bundled"
number: 1
short_code: "CLOACI-A-0010"
created_at: 2026-06-30T15:36:02.292257+00:00
updated_at: 2026-06-30T15:37:15.183580+00:00
decision_date:
decision_maker:
parent:
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-1: Constructor provider distribution rides Cargo's dependency model (crates.io + path/git), independently versioned, build-time-resolved and bundled

*This template includes sections for various types of architectural decisions. Delete sections that don't apply to your specific use case.*

## Context **[REQUIRED]**

Constructors ([[CLOACI-I-0132]]) run as WASM **provider packages**: a consumer references one via `constructor!(from = "name@version", constructor = "…")` (and the `#[reactor]` consumer form). The open question this ADR settles is the **distribution layer** for providers — how they are published, versioned, sourced, and resolved.

The interim mechanism is a server-side provider directory (`CLOACINA_PROVIDER_PATH` / a `providers/` dir): the loader resolves `from` against a filesystem path of pre-packed `.cloacina` provider archives. Surfaced while wiring packaged-workflow constructor support ([[CLOACI-T-0832]]), this invents a bespoke registry + operational surface, risks runtime drift (the deployed package and the server's provider dir can disagree), and requires operating a provider distribution channel cloacina doesn't otherwise need.

## Decision **[REQUIRED]**

**A constructor provider is a normal Rust crate dependency. Provider distribution rides Cargo's existing dependency model; cloacina invents no provider registry.**

- A provider is an **independently published, independently versioned crate** (its own semver, decoupled from cloacina's version). An author publishes it once (to crates.io); a consumer just **declares it as a dependency and "goes" — it compiles**.
- Sourcing rides **Cargo's existing dependency sources**: **crates.io** (the primary path) **plus the escape hatches Cargo already supports — `path` (filesystem) and `git` deps**. cloacina adds no sourcing of its own; whatever Cargo can resolve, a provider can come from.
- The consumer **declares the provider as an ordinary Cargo dependency** in its `Cargo.toml` (the standard `name = "ver"` / `{ path = … }` / `{ git = … }` forms). `constructor!(from = "name@version", …)` **references it by name + version**; the build matches the `from` reference to the declared Cargo dependency.
- **Resolution + build + bundle happen at the consumer's BUILD time.** `cloacinactl` / the compiler resolves each referenced provider through Cargo, builds it to a WASM component (wasm32-wasip2 + the fidius provider packaging), and **bundles the built provider into the consumer's `.cloacina`**. The resulting package is **self-contained / hermetic**.
- The **server loads a hermetic package** — it never resolves a provider directory, never fetches from crates.io at load, and needs no Rust toolchain or network. The server-side provider directory survives only as a **dev/test convenience**, not the distribution layer.

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

| Option | Pros | Cons | Risk Level | Implementation Cost |
|--------|------|------|------------|-------------------|
| **Provider = Cargo dependency, build-time resolve + bundle (CHOSEN)** | Reuses Cargo's whole stack (semver, yank, signing, discovery) + path/git escape hatches; independently versioned; hermetic packages; dumb server; providers rebuilt from source against the consumer's fidius ⇒ no provider ABI drift | Heavier build side (cloacinactl/compiler must build + bundle); `.cloacina` grows by the provider wasm | Low | Medium |
| Bespoke server-side provider registry / `CLOACINA_PROVIDER_PATH` dir | Simple to stand up for a demo | Invents a registry + ops surface; runtime drift (package vs server dir); a distribution channel to operate; non-hermetic | Medium | Medium |
| Server fetches provider from crates.io at load | Central | Puts a Rust toolchain + network + build on the server; non-hermetic; slow/fragile loads | High | High |
| Custom binary-artifact registry for pre-built `.cloacina` providers | Pre-built (no consumer build) | Rebuilds crates.io badly; binary artifacts drift vs the host fidius ABI; large ops cost | High | High |

## Rationale **[REQUIRED]**

- **"Declare a dependency and go."** Authoring a provider should feel like authoring a crate, and using one should feel like adding a dependency. Riding Cargo delivers exactly that with zero new concepts for the user.
- **Reuse over reinvention.** Versioning, semver resolution, yanking, signing, discovery, and the **path/git escape hatches** all already exist in Cargo. A bespoke provider registry would reimplement them worse and add an operational channel to run.
- **Hermetic beats ambient.** Bundling the built provider into the `.cloacina` means a deployed package carries its providers — it can't fail to find them or silently bind a different version than it was built against.
- **Source distribution kills provider ABI drift.** Because providers are *source* crates compiled at the consumer's build time, they are always built against the consumer's current fidius — so a provider can't go stale the way a pre-built packaged *workflow* can ([[CLOACI-T-0835]]). The distribution choice shrinks the drift surface.
- **Independent versioning** lets the provider ecosystem evolve on its own cadence, decoupled from cloacina releases.

## Consequences **[REQUIRED]**

### Positive
- Authoring/using providers reduces to authoring/using crates; no new distribution concepts.
- Inherits crates.io **and** `path`/`git` sourcing for free; nothing bespoke to operate.
- Packages are hermetic (provider travels inside the `.cloacina`); the server stays toolchain-free and offline-capable.
- Providers are rebuilt from source against the consumer's fidius ⇒ no provider ABI-drift class.

### Negative
- The build side (`cloacinactl`/compiler) gets heavier: it must resolve provider deps through Cargo, build each to wasm, and pack them in.
- `.cloacina` artifacts grow by the size of their bundled provider component(s).
- Build-time provider compilation lengthens the consumer build (cacheable, but real).

### Neutral
- The interim `CLOACINA_PROVIDER_PATH` / `providers/` dir is demoted to a dev/test convenience, not the distribution layer.
- The `from = "name@version"` grammar must be reconcilable with how the provider is declared in `Cargo.toml` (incl. path/git deps where the "version" comes from the resolved crate). Mechanics → the companion spec.

## Review Schedule **[CONDITIONAL: Temporary Decision]**

### Review Triggers
- A concrete need appears for **pre-built** provider artifacts (e.g. providers authored in a non-Rust language that can't be `cargo build`-ed at consumer time) — would reopen the binary-artifact-registry option.
- Build-time provider compilation becomes a material pain point (build latency / cache misses) at scale.

### Scheduled Review
- **Review Criteria**: whether Cargo-native, build-time-bundled distribution remains sufficient as the provider ecosystem and non-Rust authoring grow.
