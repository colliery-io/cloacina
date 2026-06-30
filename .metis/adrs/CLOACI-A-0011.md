---
id: 001-providers-are-suites-one-crate
level: adr
title: "Providers are suites — one crate exposes N constructors via a ProviderManifest, selected by name in the configure payload (no fidius/loader churn)"
number: 1
short_code: "CLOACI-A-0011"
created_at: 2026-06-30T16:17:00.200321+00:00
updated_at: 2026-06-30T16:18:42.997503+00:00
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

# ADR-1: Providers are suites — one crate exposes N constructors via a ProviderManifest, selected by name in the configure payload (no fidius/loader churn)

*This template includes sections for various types of architectural decisions. Delete sections that don't apply to your specific use case.*

## Context **[REQUIRED]**

[[CLOACI-A-0010]] settled provider *distribution* (a provider is a Cargo dependency, built + bundled into the consumer `.cloacina`). This ADR settles the provider *contract*: **can one provider crate expose multiple constructors (a "suite"), and how — without churning fidius or the host loader?**

The consumer grammar already anticipates it: `constructor!(from = "<provider>", constructor = "<name>", …)` has a `constructor` selector, and the packaging already defines a `ProviderManifest { name, version, constructors: Vec<ProviderConstructorEntry> }` documented as "N-capable." But today the rest is single: one `#[constructor]` per crate → one `ConstructorManifest` → a `ProviderManifest` carrying exactly one member; the loader calls a **fixed per-kind interface descriptor** (`TaskConstructor_WASM_DESCRIPTOR`, etc.). A consumer who wants `cloacina-provider-fs` to offer `read_file` + `write_file` + `stat` cannot today — they must ship one crate per constructor.

Driver ([[CLOACI-I-0132]], human 2026-06-30): the **delivered unit is the workflow**, workflows are independent, and authoring should let a provider be a *library* of related constructors.

## Decision **[REQUIRED]**

**A provider is a suite: one provider crate may declare N constructors, compiled into one WASM component, indexed by a `ProviderManifest = List[Constructor]`. The member is selected by name carried in the `configure` payload — so fidius and the loader are essentially unchanged.**

- **Manifest shape.** The package's top-level manifest is the **`ProviderManifest`** (`name`, `version`, `constructors: Vec<Constructor>`). `ConstructorManifest` becomes the **element** type (the per-member descriptor: name, primitive_kind, interface, `config_fields`, …) — consolidated into the one `provider.json` list; the separate per-constructor `constructor.json` sidecars + the lightweight `ProviderConstructorEntry` pointer collapse into it.
- **Selection grammar.** `from = "<provider crate>"` selects the provider (per [[CLOACI-A-0010]], the exact Cargo package name); `constructor = "<name>"` selects the member within it.
- **The "free" mechanism — name-in-configure.** All constructors of a kind share the **same fidius interface** (identical method shapes: `configure`/`execute`/…), so the host keeps its **one fixed per-kind descriptor** and the interface hash is unchanged. The selected constructor **name is encoded into the opaque `configure(&[u8])` payload** alongside the bincode config; the guest's macro-generated `configure` decodes the name, instantiates *that* constructor, binds its config; `execute` runs the bound instance (fidius's persistent configured-store model). The host loader change is just: serialize `(name, config)` instead of `config`.
- **Where the (bounded) work is.** Only the `#[constructor]` macro: allow N per crate, aggregate them into one component with a name-dispatched `configure`, and emit the `ProviderManifest` list (`emit_manifest`). fidius, the host loader, the bundling, the runtime, and the packaged-FFI plumbing are unchanged.
- **Documentation is a first-class deliverable** — the "provider = suite; `from` = crate, `constructor` = member" authoring model is invisible unless written down.
- A **single-constructor provider is just a suite of one** — the status quo becomes a special case, not a separate path.

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

| Option | Pros | Cons | Risk Level | Implementation Cost |
|--------|------|------|------------|-------------------|
| **Suite via name-in-configure (CHOSEN)** | One crate = a library of constructors; matches the `constructor=` selector; NO fidius change; host loader barely changes (one extra name in the config bytes); shared per-kind interface hash keeps the host descriptor fixed; bundling/runtime untouched | Macro gains real (but contained) codegen; the `configure` wire gains a name field to version | Low | Medium (macro only) |
| Suite via distinct named interfaces per constructor | "Proper" per-constructor interfaces | The host is generic — it can't carry a static per-constructor descriptor; needs dynamic descriptor/hash construction → churns fidius + the loader | Medium | High |
| N components per package (one wasm per constructor) | Reuses the single-constructor component as-is | A `cdylib` crate emits ONE artifact — getting N wasms from one crate is mechanically awkward; more artifacts to bundle | Medium | Medium–High |
| One constructor per crate (status quo) | Simplest; nothing to build | Forces a crate per constructor; fights the `constructor=` selector + the "provider as library" intent | Low | None |

## Rationale **[REQUIRED]**

- **The grammar already promised it.** `constructor = "…"` only makes sense if a provider can hold more than one; the manifest was already declared N-capable. This decision finishes a half-built model rather than inventing one.
- **Cheap under the hood by construction.** Because every constructor of a kind has the *same interface shape*, the differentiator (which constructor + its config) is *data*, not *type* — so it belongs in the opaque `configure` payload, leaving fidius's type-keyed interface dispatch and the host's fixed descriptor untouched. The alternative (a distinct interface per constructor) pushes that differentiator into the *type* system, which the generic host can't follow without dynamic descriptor machinery.
- **Matches the delivery model.** The workflow is the delivered unit and bundles its own provider versions ([[CLOACI-A-0010]]); a provider being a versioned library of constructors fits that cleanly.

## Consequences **[REQUIRED]**

### Positive
- Providers can be libraries (`cloacina-provider-fs` → read/write/stat) — fewer crates, coherent grouping, matches the selector.
- No fidius change; the host loader change is a one-line "serialize the name too"; bundling/runtime/FFI unchanged.
- Shared per-kind interface ⇒ stable interface hash ⇒ the host keeps one fixed descriptor per kind.

### Negative
- The `#[constructor]` macro gains real codegen (aggregate a crate's constructors + name-dispatched `configure`).
- The `configure` wire format gains a constructor-name field — a contract/wire detail that must be versioned.
- Authoring is invisible without docs (mitigated by making docs a first-class deliverable).

### Neutral
- A single-constructor provider is a suite of one — the previous default is now a degenerate case.
- `ConstructorManifest` is demoted from "the manifest" to "the element of the `ProviderManifest` list" (a rename/relocation in the contract crate).

## Review Schedule **[CONDITIONAL: Temporary Decision]**

### Review Triggers
- A constructor kind ever needs genuinely *different* interface shapes per member (would break the shared-interface assumption that makes name-in-configure free).
- Non-Rust provider authoring (interacts with how the suite/component is produced).
