# First-Party Providers

Constructor providers: crates a workflow author imports as ordinary Cargo
dependencies and instantiates with bound config via
`constructor!(from = "<name>@<version>", ...)`. The **workflow package** is what
gets compiled; the compiler resolves provider crates from crates.io in
production and bundles native cdylibs during workflow compilation.

Providers version **independently** of cloacina core (ADR A-0010) and are
deliberately outside the core version lockstep. They release in **waves**
(batched ceremony, independent versions — see CLOACI-T-0872): each wave
re-certifies every provider against released core and publishes the changed
ones. This table is the tested-set record until the wave workflow regenerates
it mechanically.

## Roster

| Provider | Version | Kind | Members | Certified core | Status |
|---|---|---|---|---|---|
| `cloacina-provider-kafka` | 0.1.0 | native stream accumulator | `kafka_source` | 0.10 (in-repo build) | pre-publication (`publish = false` until first wave) |
| `cloacina-provider-fs` | 0.1.0 | WASM task suite | `read_file`, `write_file` | 0.10 (in-repo build) | pre-publication (`publish = false` until first wave) |

## Seed providers (in `examples/constructor-contract/`, not yet promoted)

`cloacina-provider-extract` (accumulator: field projection),
`cloacina-provider-quorum` (reactor: N-of-M firing), and
`cloacina-provider-sensor` (trigger: file presence) are the T-0825 seed set —
each the canonical generic for its kind, but stateless-only and minimally
configured. They graduate to `providers/` when there is real demand for them at
version-dep quality: promotion means ship-form deps (published
`cloacina-constructor-contract` + `cloacina-macros` version pins), a CHANGELOG,
a consumer fixture, and joining the wave roster (CLOACI-T-0871 records the
audit that drew this line).

They already bind the **promoted** `crates/cloacina-constructor-contract` (by
path, since they are not yet published); the T-0822 spike copy that used to live
at `examples/constructor-contract/constructor-contract` was deleted in
CLOACI-T-0920 so a second, silently-divergent contract crate cannot exist. That
matters because `#[config]` binding is positional, width-sensitive bincode
reconstructed from `ConfigField` declaration order + Rust type names — a forked
contract breaks consumers invisibly.

## Conventions (the wave contract, T-0872)

- Standalone crate, own `[workspace]`, own semver, excluded from the core
  workspace (`providers/*` in the root `exclude`).
- Core deps pinned to a released minor (`cloacina-constructor-contract = "0.10"`,
  `cloacina-macros = "0.10"`) — never path deps in ship form.
- Any PR touching `providers/<name>/src` must bump the provider version and add
  a CHANGELOG entry (config-schema changes are breaking even when no Rust
  signature moves). Enforced by the **Provider Guard** workflow
  (`scripts/provider_wave.py pr-check`).
- **Changing a provider's `runtime` is a MAJOR / breaking change** (CLOACI-T-0920).
  `runtime` is an emission target — the same crate can build to a sandboxed
  `wasm32-wasip2` component or a native host cdylib (`[package.metadata.cloacina]
  runtime = "native"`) — but it is **part of the provider's public contract**,
  because capability `grants` are **ENFORCED** on wasm (fidius `WasiCtx` +
  `EgressPolicy`) and only **ADVISORY** on native (a native provider runs
  unsandboxed in-process with full host trust). A `wasm → native` flip therefore
  silently converts every consumer's `grants = { .. }` from a security control
  into decoration. So: flip the runtime only in a MAJOR version, say so at the top
  of the CHANGELOG entry, and expect consumers to re-pin.
  Enforced on **both** sides:
  - **Producer** — `runtime` is recorded per row in `providers/COMPAT.toml`, and
    `scripts/provider_wave.py pr-check` fails a runtime flip that is not
    accompanied by a MAJOR bump.
  - **Consumer** — `constructor!(.., runtime = "wasm" | "native")` and
    `#[reactor(.., runtime = "..")]` pin the expected tier; the load fails if the
    resolved provider disagrees. Independently, declaring `grants` against a
    provider that resolves NATIVE is a hard load error unless the author
    acknowledges it with `runtime = "native"` — so unenforced grants can never be
    reached silently.
- Per-provider `CHANGELOG.md`; the **`certify/` harness** is the certification
  instrument: a bin crate whose cloacina deps resolve from CRATES.IO (never
  path deps, never patched) and which runs the provider E2E. Exit 0 EARNS the
  compat claim. `certify/` is `exclude`d from the published crate.

## The wave machinery

- **`providers/COMPAT.toml`** — the tested set, machine-generated
  (`scripts/provider_wave.py compat`). A row means that provider version
  passed its certify harness against crates.io core in the named wave, **at the
  recorded `runtime`**. Never hand-edit rows.
- **`angreal providers check`** — classify vs crates.io + run candidate guards.
- **`angreal providers wave`** — pre-flight a wave and print the tag commands
  (tag push stays a human step, same convention as core's release).
- **Tag `providers-vYYYY.MM[.N]`** (or workflow dispatch) →
  `.github/workflows/provider_release.yml`: classify → guard → **certify
  everyone** (unchanged providers re-earn their claim — silent rot is the
  enemy) → publish candidates → compat PR + wave release notes. Waves are
  per-provider atomic.
