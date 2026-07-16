---
id: first-party-kafka-event-source
level: initiative
title: "First-party Kafka event-source provider — the flagship provider authoring story"
short_code: "CLOACI-I-0139"
created_at: 2026-07-15T02:15:31.122534+00:00
updated_at: 2026-07-15T12:14:45.111644+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: first-party-kafka-event-source
---

# First-party Kafka event-source provider — the flagship provider authoring story Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context **[REQUIRED]**

> **Phase: discovery — design resolved (2026-07-15).** All forks + sub-questions decided with the maintainer: NATIVE provider variant (not WASM); `kind = accumulator` (stream type, not a new kind); native = trusted (grants advisory); signing/per-arch reuse existing machinery. Ready to decompose pending go-ahead. Original scope fork (A/B/C) and sub-questions (a)–(e) retained below as the decision record.

**Maintainer goal (2026-07-14):** author Cloacina's FIRST first-party **provider** — a Kafka event-source — that users consume in their packaged workflows. The deliverable is NOT "kafka works"; it's a **repeatable provider story**: an author builds a provider → packages/signs it → publishes it (Cargo-native) → a user's packaged workflow consumes it via `constructor!(from=, constructor=, grants=)`. Kafka is the flagship + proving vehicle. As a consequence, kafka's migration OUT of core also achieves the original [[CLOACI-T-0898]] framing (core `cloacina` drops the `kafka` feature + `rdkafka`).

**Grounded state (read-only research, 2026-07-14).** The constructor/provider mechanism is SHIPPED and complete for **request/response WASM primitives**:
- `#[constructor(kind = task|trigger|accumulator|reactor)]` + `constructor_provider!(...)` suites (`cloacina-macros/src/constructor_attr.rs`, `constructor_provider.rs`); contract in `cloacina-constructor-contract`; `cloacinactl constructor package` builds to `wasm32-wasip2` + emits `provider.json` (`ProviderManifest`); Ed25519 signing; Cargo-native distribution (S-0015, T-0836 done) — a provider is an ordinary Cargo dep, bundled hermetically into the consumer `.cloacina`.
- Consumption: `constructor!(from = "crate@ver", constructor = "member", grants = { fs = ["ro:/..."] })` with fail-closed tenant capability grants (S-0014, A-0009). Proven end-to-end by `examples/constructor-contract/fs-grant-demo`.
- Existing providers (all in `examples/constructor-contract/`, not yet promoted — [[CLOACI-T-0871]]): `cloacina-provider-fs` (task, 2-member suite), `-extract` (accumulator transform), `-quorum` (reactor), `-sensor` (trigger).

**THE CRUX — why Kafka is the *hardest* possible first provider:** a Kafka SOURCE is a long-running consumer LOOP that PUSHES events into the host. It fits NEITHER shipped path:
1. **Providers are WASM-only** (`wasm32-wasip2`), and **`rdkafka` is native C (librdkafka)** — it will NOT compile to wasm. There is no in-process/native provider load path and no `configure_in_process` (grep-confirmed). So Kafka cannot be a WASM provider.
2. **The `kind = accumulator` constructor is per-event `ingest(event) -> boundary` — a STATELESS transform, NOT a loop.** The host owns the event loop; the constructor transforms one already-delivered event. No constructor `kind` today owns a long-running push loop.
3. **Kafka's ONLY working path today is native + host-side and NOT a provider:** the `EventSource` trait (`accumulator.rs:149`, `async fn run(events_tx, shutdown_rx)`) + `KafkaStreamBackend` (`stream_backend.rs`, `#[cfg(feature="kafka")]`) + `KafkaEventSource` (`packaging_bridge.rs:419`), wired by `StreamBackendAccumulatorFactory`, tied to accumulator spawn/shutdown lifecycle, activated by `[[metadata.accumulators]] accumulator_type = "stream"` in a package's `package.toml`. And `rdkafka` is a **default-ON feature of core `cloacina`** (`Cargo.toml:20`), re-exported by server/cli/python.

**So the gap is architectural, not cosmetic:** making Kafka a *consumable provider* requires a NEW path that neither the WASM-provider mechanism nor the native-host-accumulator path provides — a way for a **provider to supply a long-running, load/unload-scoped background source loop**. The closest working analogue to model is the native `EventSource` / `AccumulatorFactory::spawn` (returns a `JoinHandle`, takes `shutdown_rx`) lifecycle.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- A **first-party Kafka event-source provider** an author builds → packages/signs → publishes, and a user's PACKAGED workflow CONSUMES to stream Kafka topics into a computation graph — end-to-end on the demo stack.
- The provider authoring + consumption story is **documented + proven** (the [[CLOACI-T-0891]] `cg-feature-tour` kafka surface re-enabled against the provider; a lifecycle/capability story for a background source).
- Core `cloacina` / `cloacina-server` build with **NO `kafka` feature and NO `rdkafka` dep** — kafka lives only in the provider (subsumes [[CLOACI-T-0898]]).
- The result generalizes: the "source provider" shape is reusable for other long-running sources (webhooks, queues, pollers), not a kafka special-case.

**Non-Goals:**
- Rewriting the request/response WASM provider mechanism (task/trigger/accumulator-transform/reactor) — that ships and stays as-is.
- Making rdkafka run in WASM (it can't — native C).
- Provider *marketplace* / registry hosting (distribution is Cargo-native today; hosting is out of scope).
- Broadly promoting all `examples/constructor-contract` providers to release homes ([[CLOACI-T-0871]] — related but separate).

## Open design fork — MAINTAINER SIGN-OFF NEEDED (blocks design/decomposition)

The whole initiative shape hinges on ONE decision: **how does a provider supply Kafka's long-running native source loop?** rdkafka can't be WASM, and no in-process/native provider path exists. Options:

- **(A) Build a native (in-process cdylib) "source provider" path.** New provider load path that dlopens a native cdylib (like packaged *workflows* already do via `cloacina-workflow-plugin`, but for providers) + a new **"source" primitive kind** that owns an `EventSource`-style `run(events_tx, shutdown_rx)` loop, tied to reactor/package load↔unload lifecycle. Kafka provider = native cdylib shipping rdkafka. Most work; but it's the only path that makes kafka a *real distributable provider* and generalizes to any native source. rdkafka leaves core.
- **(B) "Provider" wraps config only; rdkafka stays a host capability.** The provider ships the source *wiring/config/schema* and a capability declaration; the host keeps a built-in kafka runtime behind a tenant grant. Lighter, but rdkafka stays in the host (doesn't fully satisfy "kafka out of core"), and it's a weaker notion of "provider" (no code shipped).
- **(C) Reconsider whether kafka is a "provider" at all.** Treat event-sources as a distinct first-class *host connector* plugin surface (not the constructor/provider mechanism), and pick a request/response provider (promote `fs`) as the flagship "author your first provider" story instead. Honest about the mechanism's current shape; defers the native-source architecture.

### RESOLVED (maintainer, 2026-07-15): NATIVE provider variant — "why does it need to be WASM? it just needs to be a provider package/type."

Correct, and de-risking. The provider mechanism wired WASM FIRST; the WASM-only-ness is incidental. The native path already exists in the runtime:
- **fidius is already 0.5.5**, exposing `PluginHandle::configure_in_process<C>` (`fidius-host/src/handle.rs:94`) — a native, in-process configured-instance path — with passing e2e tests for configured cdylibs (`fidius-host/tests/configured_cdylib_e2e.rs`) AND configured streaming (`configured_python_stream_e2e.rs`, `load_python_configured`).
- **Packaged WORKFLOWS already load native cdylibs this exact way** (`fidius_host::loader::load_library` + `PluginHandle::from_loaded`, `registry/loader/package_loader.rs:266,282`). Providers reuse the same native loading + fidius `configure_in_process`.

So the direction is **(A) native provider variant**, but correctly scoped as WIRING an existing, tested capability — NOT inventing architecture. Chosen over (B)/(C): it ships real provider code (kafka's rdkafka travels in the package), fully removes kafka from core, and generalizes to any native source. WASM providers stay for portable/sandboxed request-response primitives; native providers serve native-dep + streaming sources.

**Trust model (design note, for sign-off):** WASM providers are sandboxed (wasmtime); NATIVE providers run in-process with host trust — like packaged-workflow cdylibs already do. Tenant `grants` still gate consumption. The initiative must state this explicitly (native provider = trusted code, same trust surface as a packaged Rust workflow).

## Detailed Design (post-resolution sketch — to be filled in the design phase)

**Anchor: the operator authoring surface already exists and is target-agnostic.** `#[constructor(kind = task|trigger|accumulator|reactor, name, version)]` + `#[config]` fields (`cloacina-macros/src/constructor_attr.rs`) IS the "author a configurable operator" model — the author writes ONLY the struct (config bound once per instance at load) + one body fn (`execute`/`ingest`/`poll`/`evaluate`); the macro generates the config struct, the `configure` hook, and `__constructor_manifest()`. Verified in `cloacina-provider-fs` (a 2-member task suite: author writes `#[config] path` + `execute`, consumer supplies `grants`). **The ONLY WASM-specific part is the generated guest glue, which is literally `#[cfg(target_arch = "wasm32")]`** — the struct + manifest build on the host regardless. So "native provider" does NOT change the author surface; it emits **native fidius `#[plugin_impl]` glue** (loaded via `configure_in_process`) alongside the wasm glue. This is the whole reframe: operators are already authored the same way; runtime (wasm vs native) is an emission target, not an authoring concern.

The concrete pieces (all wiring existing primitives, roughly ordered):
1. **Native provider load path.** Add a native variant to `registry/loader/constructor_loader.rs` alongside `load_wasm_configured*`: load the provider cdylib via `load_library` + `PluginHandle::configure_in_process` (config bincode-encoded in declaration order, same as WASM). Provider manifest gains a `runtime = "native"` (vs `"wasm"`) discriminator; loader dispatches.
2. **Stream accumulator supplied by a provider (kind = accumulator).** Extend the accumulator-constructor path so an accumulator constructor can produce a `stream`-type accumulator (loop-owning), not only the per-event `ingest` transform. On native, drive it via fidius `call_streaming` → `ChunkStream`; a loader shim drains it and pushes into the host accumulator boundary channel — the provider replaces today's host-side `KafkaEventSource`/`StreamBackendAccumulatorFactory`. Load↔unload scoped to the consuming accumulator (drop the stream → producer tears down, matching `EventSource` shutdown).
3. **Authoring:** `#[constructor(kind = accumulator)]` (stream variant) + `constructor_provider!` emit the NATIVE (cdylib, non-wasm) provider; `cloacinactl constructor package` grows a native-build path (crate-type cdylib, native host target — drop `--target wasm32-wasip2`) parallel to the WASM path.
4. **Kafka provider crate** (`cloacina-provider-kafka`): a `kind = accumulator` (stream) provider shipping rdkafka + the `KafkaStreamBackend` logic, configured by broker/topic/group. `grants` advisory (native = trusted).
5. **Core cleanup ([[CLOACI-T-0898]]):** delete `KafkaEventSource`/`StreamBackendAccumulatorFactory`'s kafka branch from `packaging_bridge.rs`, drop `kafka`/`rdkafka` from core `cloacina` + server/cli/python; the stream accumulator's `accumulator_type="stream"` metadata path resolves to the kafka *provider* instead.
6. **Consumption + proof:** a packaged CG workflow `constructor!(from = "cloacina-provider-kafka@…", constructor = "kafka_source", grants = { net = […] })` streaming a topic into a graph on the demo stack; re-enable the [[CLOACI-T-0891]] `cg-feature-tour` kafka surface as the regression lane.

### Distribution / publish home (PREREQUISITE — activates [[CLOACI-T-0871]] + [[CLOACI-T-0872]])

A first-party provider is a PUBLISHED Cargo crate (`constructor!(from = "crate@ver")` resolves via `cargo metadata` against crates.io). Today's provider examples are `publish = false` demos under `examples/constructor-contract/` with PATH deps — not shippable. I-0139 makes kafka the first REAL published provider, so it must resolve:
- **The home ([[CLOACI-T-0871]]):** a top-level `providers/` directory — standalone crates, own versions, excluded from the core workspace. `cloacina-provider-kafka` is its first real resident (T-0871 notes only `cloacina-provider-fs` has a real-provider name; kafka joins it).
- **The release path ([[CLOACI-T-0872]]):** core crates publish via `unified_release.yml` (`cargo publish -p …`); providers need an equivalent version/publish flow. TWO wrinkles: (1) a published provider must depend on `cloacina-constructor-contract` + `cloacina-macros` as **crates.io version deps** (not the examples' path deps) → those crates must be published (same lean-version-dep-resolved-locally pattern as the packaged-workflow examples); (2) native providers publish the SOURCE crate to crates.io, but the per-arch cdylib ARTIFACT is built on ingest by the compiler (sub-question (c)) — publish ≠ ship-the-binary.

So the decomposition below gains a "provider publish home + release path" task (folds T-0871/T-0872) as a prerequisite for shipping the kafka provider.

### Sub-question resolutions (grounded 2026-07-15; ★ = needs maintainer sign-off)

**(a) `runtime = native` loader/manifest shape — RESOLVED (mechanical).** The loaded-handle abstraction is already `fidius_host::PluginHandle`, and `configure_in_process` returns the same type — so everything below the `Arc<PluginHandle>` wrappers (`Wasm*Constructor`, `call_method`, async↔sync bridges) is backend-agnostic and reused unchanged. Design: add a `runtime` discriminator to `ProviderManifest` (whole suite = one runtime) and emit `runtime = "native"` in `render_package_toml` (`packaging/constructor_provider.rs:268`, today hardcodes `"wasm"`). Each of the 4 `load_*_constructor` fns (`constructor_loader.rs:332/619/909/1113`) branches at its `load_wasm_configured_with_grants` call site (:382/664/953/1157): native → `load_library`+`from_loaded` / `configure_in_process`. The `component: String` field is WASM-specific; native needs a per-arch library reference → ties to (c).

**(b) RESOLVED (maintainer, 2026-07-15): it's `kind = accumulator` — NOT a new kind, NOT overloaded.** *"We're building an accumulator FACTORY — not a generic long-running event source. So it's an accumulator and not overloaded."* The provider member is an accumulator constructor (`kind = accumulator`); Kafka is simply the **`stream` accumulator type** (the existing accumulator-type dimension — `passthrough/state/batch/polling/stream`, wired in [[CLOACI-T-0896]]'s `accumulator_factory_for`), so the "loop" is a property of the stream accumulator, not a new taxonomy node. NO 5th `PrimitiveKind`, NO `mode` flag — the exhaustive-match churn evaporates. The technical work: the accumulator-constructor path must be able to produce a **stream accumulator** (loop-owning), not only the current per-event `ingest` transform. Native mechanism: the stream accumulator's backend is driven via fidius `PluginHandle::call_streaming` → `ChunkStream` (`fidius-host/src/stream.rs`, configured-cdylib streaming e2e-proven in `configured_cdylib_stream_e2e.rs`); a loader shim drains the `ChunkStream` (`.next().await`, host-pull) and pushes each item into the host accumulator boundary channel — i.e. the provider SUPPLIES the stream backend that today's host-side `KafkaEventSource`/`StreamBackendAccumulatorFactory` provides. Backpressure/cancel are structural in fidius (drop the stream → tears down the producer), matching the `EventSource` shutdown lifecycle.

**(c) native per-arch build — RESOLVED, reuses workflow machinery + exposes one existing gap.** Packaged workflows are already multi-arch (T-0780): `run_per_target` (`compiler/src/loopp.rs:211`) does NATIVE builds per-container (no cross-compile) into `package_artifacts.target_triple` (`dal/unified/workflow_packages.rs`). A native provider slots into the same table + scan-and-fill loop; the build just drops the `--target wasm32-wasip2` the WASM path uses (`constructor_provider.rs:300`) and builds natively. **Pre-existing gap surfaced:** the agent-side "pick MY arch's artifact at load" (`content_hash_for_target` DAL primitive) is NOT yet wired into the reconciler load path (`reconciler/loading.rs:271` reads the primary `compiled_data`). This gap must be closed for native providers — and it benefits packaged workflows too. (Own task.)

**(d) signing — RESOLVED (zero changes).** Signing is over the whole fidius package directory digest (`fidius_core::package::package_digest` → Ed25519 → `package.sig`, `constructor_provider.rs:402`), agnostic to whether the component is `.wasm` or a native `.so`. `verify_package` on load is identical. A native cdylib package gets signing/verification parity for free.

**(e) ★ trust model — the real security decision.** Native providers run **in-process with full host trust, isolated only by the tenant schema/runner boundary** — identical to packaged Rust workflow cdylibs (`from_loaded`/`configure_in_process` take NO `WasiCtx`/`EgressPolicy`, `fidius-host/handle.rs:74,94`; the build sandbox was excised, tenant = the boundary). The WASM `grants = { fs/http/tcp/env/secrets }` capability sandbox (`grants.rs`, default-closed, `EgressPolicy::authorize`) is **inert for native** — a native kafka provider opens its broker socket directly regardless of a `net` grant. So for native providers, `grants` degrade from *enforced sandbox* to *advisory/audit declaration*, and the trust boundary collapses to "the tenant chose to install this signed native cdylib" — the same trust they already extend to a packaged Rust workflow. **RESOLVED (maintainer, 2026-07-15): accept native = trusted; grants are advisory for native.** Two-tier model: WASM providers stay capability-sandboxed (enforced `grants`); NATIVE providers are trusted code — `grants` degrade to advisory/audit metadata (a native kafka provider opens its broker socket regardless), same trust surface a tenant already extends to a packaged Rust workflow cdylib, bounded only by the tenant schema/runner. The `runtime` discriminator (sub-question (a)) must make this asymmetry explicit at author + install time. No new native sandbox (the build/process sandbox was deliberately excised; tenant = the boundary). Design note: consider whether to surface the trust tier in `constructor!`/CLI output so a consumer sees "this provider is native (trusted, unsandboxed)" vs "wasm (sandboxed)" — a documentation/UX task, not an enforcement one.

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

{Delete if not a requirements-focused initiative}

### User Requirements
- **User Characteristics**: {Technical background, experience level, etc.}
- **System Functionality**: {What users expect the system to do}
- **User Interfaces**: {How users will interact with the system}

### System Requirements
- **Functional Requirements**: {What the system should do - use unique identifiers}
  - REQ-001: {Functional requirement 1}
  - REQ-002: {Functional requirement 2}
- **Non-Functional Requirements**: {How the system should behave}
  - NFR-001: {Performance requirement}
  - NFR-002: {Security requirement}

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

{Delete if not user-facing}

### Use Case 1: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

### Use Case 2: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

{Delete if not technically complex}

### Overview
{High-level architectural approach}

### Component Diagrams
{Describe or link to component diagrams}

### Class Diagrams
{Describe or link to class diagrams - for OOP systems}

### Sequence Diagrams
{Describe or link to sequence diagrams - for interaction flows}

### Deployment Diagrams
{Describe or link to deployment diagrams - for infrastructure}

## Detailed Design **[REQUIRED]**

See **Detailed Design (post-resolution sketch)** above — the 6 concrete pieces (native provider load path, provider-supplied stream accumulator, native emission, kafka provider crate, core cleanup, consumption+proof), the distribution/publish-home prerequisite, and the sub-question resolutions (a)–(e). The decomposition below maps those pieces 1:1 to tasks.

## UI/UX Design **[CONDITIONAL: Frontend Initiative]**

{Delete if no UI components}

### User Interface Mockups
{Describe or link to UI mockups}

### User Flows
{Describe key user interaction flows}

### Design System Integration
{How this fits with existing design patterns}

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

{Delete if covered by separate testing initiative}

### Unit Testing
- **Strategy**: {Approach to unit testing}
- **Coverage Target**: {Expected coverage percentage}
- **Tools**: {Testing frameworks and tools}

### Integration Testing
- **Strategy**: {Approach to integration testing}
- **Test Environment**: {Where integration tests run}
- **Data Management**: {Test data strategy}

### System Testing
- **Strategy**: {End-to-end testing approach}
- **User Acceptance**: {How UAT will be conducted}
- **Performance Testing**: {Load and stress testing}

### Test Selection
{Criteria for determining what to test}

### Bug Tracking
{How defects will be managed and prioritized}

## Alternatives Considered **[REQUIRED]**

The scope fork above (see *Open design fork*) is the decision record:
- **(A) Native (in-process cdylib) provider — CHOSEN.** Reframed by the maintainer ("why does it need to be WASM?") and de-risked once we found fidius 0.5.5 already ships `configure_in_process` + `call_streaming`, both e2e-tested, and packaged workflows already load native cdylibs the same way. Ships real provider code, removes kafka from core, generalizes to any native source.
- **(B) Provider wraps config only; rdkafka stays a host capability — REJECTED.** Lighter, but rdkafka stays in core (fails "kafka out of core") and it's a weaker notion of "provider" (no code shipped).
- **(C) Event-sources as a distinct host-connector surface, promote `fs` as the flagship instead — REJECTED.** Honest about the mechanism's shape but defers the native-source architecture and doesn't deliver the kafka goal.

Sub-decisions: `kind = accumulator` with the `stream` accumulator type (NOT a new `PrimitiveKind` — avoids exhaustive-match churn); native = trusted with advisory grants (NOT a new native sandbox — the build sandbox was deliberately excised, tenant = the boundary).

## Implementation Plan **[REQUIRED]**

**Decomposition (2026-07-15).** Eight tasks — six new (T-0902..T-0907) plus two folded-in pre-existing tickets. Roughly ordered by dependency; T-0905 is independent and can land first.

| Task | Piece | Depends on |
|------|-------|-----------|
| [[CLOACI-T-0902]] | Native provider LOAD path — `runtime` discriminator on `ProviderManifest`, 4 `load_*_constructor` sites branch to `load_library`+`configure_in_process`; native gets no enforced grants | — |
| [[CLOACI-T-0903]] | Native provider EMISSION — macros emit native `#[plugin_impl]` glue; `cloacinactl constructor package --native` (cdylib, drop `wasm32-wasip2`) | T-0902 |
| [[CLOACI-T-0904]] | Provider-supplied STREAM accumulator — `kind=accumulator` produces a loop-owning stream accumulator via fidius `call_streaming`→`ChunkStream`→boundary shim | T-0902/0903, builds on T-0896 |
| [[CLOACI-T-0905]] | Per-arch native artifact SELECTION at load — wire `content_hash_for_target` into `reconciler/loading.rs:271` (independent; also fixes packaged multi-arch workflows) | — |
| [[CLOACI-T-0906]] | `cloacina-provider-kafka` — flagship native stream-accumulator provider shipping rdkafka + `KafkaStreamBackend` | T-0902/0903/0904 + T-0871/0872 |
| [[CLOACI-T-0907]] | Consumption PROOF + docs — packaged CG consumes the kafka provider on the demo stack, re-enable cg-feature-tour kafka lane, "author your first provider" + trust-tier docs | T-0906 + T-0898 |

**Folded-in pre-existing tickets (kept as their own tickets, sequenced under this initiative):**
- **[[CLOACI-T-0871]]** (provider publish home — `providers/` dir) + **[[CLOACI-T-0872]]** (provider release/publish flow): PREREQUISITE for T-0906 — a published kafka provider needs a real home + a crates.io release path (contract + macros published as version deps). Not re-parented (reassign_parent unavailable on this server); tracked here as the distribution prerequisite.
- **[[CLOACI-T-0898]]** (drop `kafka`/`rdkafka` from core): SUBSUMED — its "kafka out of core" goal is achieved by T-0906 (kafka moves INTO the provider) + T-0907 (the `stream` accumulator metadata path resolves to the provider). Completed as part of T-0906/0907, not separately.

**Sequence:** T-0905 can start immediately (independent). T-0902 → T-0903 → T-0904 form the native-provider mechanism spine. T-0871/T-0872 (publish home) run in parallel with the spine. T-0906 needs the spine + publish home; T-0907 caps it with the consumption proof + docs and closes T-0898.