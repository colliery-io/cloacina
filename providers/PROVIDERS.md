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

## Conventions (the wave contract, T-0872)

- Standalone crate, own `[workspace]`, own semver, excluded from the core
  workspace (`providers/*` in the root `exclude`).
- Core deps pinned to a released minor (`cloacina-constructor-contract = "0.10"`,
  `cloacina-macros = "0.10"`) — never path deps in ship form.
- Any PR touching `providers/<name>/src` must bump the provider version and add
  a CHANGELOG entry (config-schema changes are breaking even when no Rust
  signature moves).
- Per-provider `CHANGELOG.md`; consumer fixture is the certification instrument.
