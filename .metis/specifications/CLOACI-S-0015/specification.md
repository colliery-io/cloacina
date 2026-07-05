---
id: constructor-provider-distribution
level: specification
title: "Constructor provider distribution mechanics — Cargo-native resolve, build-to-wasm, bundle into the consumer package"
short_code: "CLOACI-S-0015"
created_at: 2026-06-30T15:37:35.808677+00:00
updated_at: 2026-07-05T16:30:02.536545+00:00
parent: CLOACI-I-0132
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/published"


exit_criteria_met: false
initiative_id: NULL
---

# Constructor provider distribution mechanics — Cargo-native resolve, build-to-wasm, bundle into the consumer package

*This template provides structured sections for system-level design. Delete sections that don't apply to your specification.*

## Overview **[REQUIRED]**

The mechanics implementing [[CLOACI-A-0010]]: a constructor **provider is a normal Cargo dependency**, resolved + built + bundled at the consumer's build time so the `.cloacina` is hermetic. This spec covers how the build tooling (`cloacinactl` / the compiler) discovers which deps are providers, builds each to a WASM component, bundles them into the package, and how the `from = "name@version"` consumer reference reconciles with the Cargo dependency (including the `path`/`git` escape hatches). The runtime half (carrying the constructor *declaration* through the package + server-side resolution of the BUNDLED provider) is [[CLOACI-T-0832]]; this spec is the **distribution/build** half.

## System Context **[CONDITIONAL: System-Level Spec]**

### Actors
- **Provider author**: publishes a provider crate (independently versioned) to crates.io — or exposes it via a `path`/`git` dep. Authors with `#[constructor]`; no distribution work beyond `cargo publish`.
- **Workflow author (consumer)**: declares the provider as an ordinary Cargo dependency in `Cargo.toml` and references it from `constructor!(from = "name@version", …)` / `#[reactor]`.
- **Build tooling (`cloacinactl` / compiler)**: resolves the provider dep through Cargo, builds it to a wasm component, bundles it into the `.cloacina`.
- **Server / loader**: loads the hermetic package; resolves the constructor node against the BUNDLED provider (no provider dir, no network).

### External Systems
- **Cargo / crates.io**: the dependency resolution + distribution substrate (incl. `path` + `git` sources).
- **fidius packaging**: builds the provider crate to a wasm component + the signed provider manifest.

### Boundaries
In scope: provider-dep discovery, build-to-wasm, bundle format in the `.cloacina`, `from`↔Cargo-dep reconciliation, path/git handling. Out of scope: the runtime constructor-node declaration + server resolution ([[CLOACI-T-0832]]); non-Rust provider authoring (deferred); a pre-built binary provider registry (rejected in [[CLOACI-A-0010]]).

## Requirements **[REQUIRED]**

### Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.1.1 | A provider is consumed by declaring it as an ordinary Cargo dependency (crates.io / `path` / `git`) in the consumer's `Cargo.toml`; no cloacina-specific registry or sourcing. | [[CLOACI-A-0010]] — ride Cargo. |
| REQ-1.1.2 | `constructor!(from = "name@version")` (and `#[reactor]`) resolves to the declared Cargo dependency `name`; the build matches reference→dep and errors clearly if the dep is absent/mismatched. | The `from` grammar binds to a real, resolvable crate. |
| REQ-1.2.1 | The build tooling discovers the set of provider deps the package actually uses, builds each to a wasm component (wasm32-wasip2 + fidius provider packaging), and **bundles** the built provider(s) into the `.cloacina`. | Hermetic packages; build-time, not runtime. |
| REQ-1.2.2 | The produced `.cloacina` is self-contained: the server resolves every constructor node against a bundled provider with no provider directory and no network. | Dumb, offline-capable server ([[CLOACI-A-0010]]). |
| REQ-1.3.1 | The `path` and `git` Cargo dependency forms are supported for providers, identically to how Cargo resolves them. | Escape hatches the human required. |
| REQ-1.4.1 | **Core / community-maintained providers follow the crate-name convention `cloacina-provider-<provider-name>`** (e.g. `cloacina-provider-fs-read`). This is a naming convention for the official/community set, not a hard requirement on third-party providers. | Predictable discovery + namespacing for the maintained ecosystem; the `cloacina-provider-` prefix is also a recognizable signal that a dep is a provider. |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-1.1.1 | A provider crate's version is **independent** of cloacina's version (its own semver). | Ecosystem evolves on its own cadence. |
| NFR-1.1.2 | Providers are built from source against the consumer's current fidius — no pre-built provider binaries ship, so providers can't ABI-drift. | [[CLOACI-A-0010]] rationale; shrinks the [[CLOACI-T-0835]] drift surface. |
| NFR-1.1.3 | The provider build relies on **Cargo's own build cache** — providers are built **in-place in a persistent `target/`** (not copied to a fresh temp dir per build), so an unchanged provider doesn't recompile. No dedicated cloacina provider cache (it would only save the cheap pack/sign step and risks stale-artifact drift). | The expensive part (wasm compile) is already cached by Cargo; a redundant cache is complexity + a correctness footgun. |
| NFR-1.1.4 | **Independently-delivered workflows targeting DIFFERENT versions of the same provider coexist** on one server: each workflow bundles its own provider version, kept apart by the per-package provider search path (workflow A → `cloacina-provider-fs@0.1.0`, B → `@0.2.0`, both loaded). The version-keyed `providers/<crate>-<version>/` bundle dir also lets one workflow disambiguate two majors via `from = "name@version"`. | The delivered unit is the workflow and workflows are independent (human 2026-06-30); a shared server provider dir couldn't do this — bundling can. A payoff of [[CLOACI-A-0010]]. |

## Architecture Framing **[CONDITIONAL: System-Level Spec]**

### Decision Area: provider distribution layer
- **Context**: how providers are published, versioned, sourced, resolved.
- **Constraints**: ride Cargo (crates.io + path/git); build-time resolve+build+bundle; hermetic package; toolchain-free server.
- **Required Capabilities**: dep discovery, build-to-wasm, bundle, `from`↔dep reconciliation.
- **ADR**: [[CLOACI-A-0010]].

## Open Items

**Resolved by [[CLOACI-A-0010]]:** provider = Cargo dependency; crates.io + path/git; independently versioned; build-time resolve + build + bundle; hermetic package; server needs no provider dir/network.

**Resolved (2026-06-30, design review with human) — S-0015 is DECIDED:**
- **`from` grammar → the exact Cargo package name.** `from` is the dependency/package name **as declared in the consumer's `Cargo.toml`**, verbatim (e.g. `from = "cloacina-provider-fs-read"`). No short alias, no de-sugar magic — one rule that works identically for core (convention) and third-party (arbitrary-name) providers. `@version` is **optional**; when present it must be satisfiable by the version Cargo actually resolved for that dep (else a build error), since Cargo already pins the real version.
- **Provider-dep discovery → the `from` references ARE the list.** The build collects every `from` across the package's `constructor!`/`#[reactor]` declarations, maps each to the matching `Cargo.toml` dependency, and builds+bundles **only those** (no dead providers, no separate declaration to keep in sync). Each referenced dep is validated as a real provider via its `__constructor_manifest()` export. The `cloacina-provider-` prefix (REQ-1.4.1) is a convention/UX aid, NOT the discovery/enforcement mechanism.
- **Bundle format → nested fidius provider packages.** Each built provider is bundled as an existing fidius provider package under a `providers/<crate>-<version>/` subtree inside the `.cloacina`. No new format — a provider is just a nested provider package; the workflow manifest records the `from`→bundled-dir map the loader reads.
- **Server-side resolution → per-package provider search path.** On load, the server points a **per-package** provider search path at the unpacked `providers/` dir (scoped to that package; NOT a global `CLOACINA_PROVIDER_PATH`). This is [[CLOACI-T-0832]]'s held `step_load_constructor_nodes`.
- **Build orchestration → `cargo metadata` + the existing packaging flow.** `cargo metadata` locates each referenced provider crate in the consumer's resolved dep graph (covering crates.io/path/git uniformly); run the existing `package_constructor_provider` flow (cargo build → wasm32-wasip2 → pack) on each; bundle the result.
- **Caching → rely on Cargo's build cache (NFR-1.1.3).** No dedicated cloacina provider cache: build providers in-place in a persistent `target/` so Cargo's fingerprinting skips unchanged recompiles (the only expensive step). The cloacina-only steps (manifest emit / pack / sign) are sub-second and not worth caching; a redundant cache would also be a stale-artifact footgun.
- **Provider manifest = the A-0011 suite shape.** The bundled provider's `provider.json` is the [[CLOACI-A-0011]] `ProviderManifest` (`List[Constructor]`); the loader selects the member by the consumer's `constructor = "<name>"`. So [[CLOACI-A-0011]] (the suite contract + name-in-configure mechanism) is a **prerequisite** of the build/bundle work ([[CLOACI-T-0836]]).

**Still deferred:** non-Rust provider authoring (would reopen the pre-built-artifact path, see [[CLOACI-A-0010]]).

## Decision Log **[CONDITIONAL: Has ADRs]**

| ADR | Title | Status | Summary |
|-----|-------|--------|---------|
| CLOACI-A-0010 | Constructor provider distribution rides Cargo's dependency model | decided | Provider = a Cargo dep (crates.io + path/git), independently versioned, build-time resolve+build+bundle into a hermetic `.cloacina`. |

## Constraints **[CONDITIONAL: Has Constraints]**

{Delete if no hard constraints exist}

### Technical Constraints
- {Constraint 1}
- {Constraint 2}

### Organizational Constraints
- {Constraint 1}

### Regulatory Constraints
- {Constraint 1}

## Changelog **[REQUIRED after publication]**

{Track significant changes after initial publication. Delete this section until the specification is published.}

| Date | Change | Rationale |
|------|--------|-----------|
| {YYYY-MM-DD} | {What changed} | {Why it changed} |
