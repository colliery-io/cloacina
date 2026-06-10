---
id: cloacina-server-sdks-openapi-spec
level: initiative
title: "Cloacina server SDKs — OpenAPI spec, Rust/Python/TS clients for service consumers"
short_code: "CLOACI-I-0113"
created_at: 2026-05-22T13:41:37.227777+00:00
updated_at: 2026-06-10T01:39:11.495286+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: cloacina-server-sdks-openapi-spec
---

# Cloacina server SDKs — OpenAPI spec, Rust/Python/TS clients for service consumers Initiative

## Context **[REQUIRED]**

Cloacina today ships two consumption modes: (1) embedded — link `cloacina` (Rust) or `cloaca` (Python via PyO3) into a host process; (2) operate the server and talk to it through `cloacinactl` on the terminal. There is no library-shaped way for an external platform/service to integrate `cloacina-server` as a composed dependency.

Concretely:
- `cloacina-server` exposes a REST surface (axum handlers under `crates/cloacina-server/src/routes/{auth,executions,health_graphs,keys,tenants,triggers,workflows,ws}.rs`) plus a WebSocket protocol for computation-graph telemetry. No machine-readable spec is emitted (`utoipa`/OpenAPI absent from `Cargo.toml`).
- The only existing client is `crates/cloacinactl/src/shared/client.rs` (`CliClient` + `ClientContext`). It is CLI-shaped, crate-private, and not published.
- No Python or TypeScript HTTP client exists. `cloaca` / `cloacina-python` are the embedded runtime, not a service client.

This blocks platform-style adoption where a team wants to run cloacina-server as a managed orchestration service and call into it from their own backend, scripts, or UI.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- A single source-of-truth API contract published from `cloacina-server` (OpenAPI 3.1 for REST; documented message protocol for WS computation-graph streams).
- A reusable `cloacina-client` Rust crate extracted from `cloacinactl/src/shared/`, covering auth, tenant scoping, pagination, retries, error model, and WS subscription handling.
- A Python SDK (`cloacina-client` on PyPI, importable name TBD — must not collide with embedded `cloaca`) targeting Python 3.10+.
- A TypeScript SDK (`@cloacina/client` on npm) usable from Node 20+ and modern browsers (where CORS permits).
- Contract tests: each SDK exercised against a live `cloacina-server` in CI via existing `angreal test e2e` infrastructure.
- Versioning policy: SDK versions move in **explicit lockstep** with `cloacina` release versions. SDK `vX.Y.Z` is built against and only supported on server `vX.Y.Z`.
- **Live-server SDK verification**: each SDK has a dedicated test suite that exercises the *generated* code against a running `cloacina-server`. This is the primary gate against utoipa-annotation drift from the actual handler implementations — past pain point we are explicitly designing around.

**Non-Goals:**
- Holding server routes/DTOs fixed. Route handlers and DTOs may churn freely to become cleanly spec-able under utoipa; this is *encouraged*, not avoided.
- Replacing `cloacinactl`. The CLI keeps shipping; internally it migrates to depend on `cloacina-client`.
- Embedded-runtime changes. `cloacina` (Rust lib) and `cloaca` (Python embedded) are untouched.
- Admin UI / dashboard. SDKs only. (A UI is the planned follow-on initiative consuming `@cloacina/client`; it informs sequencing here but its scope lives elsewhere.)
- Backwards-compatibility guarantees prior to first tagged SDK release.

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

### System Requirements

- REQ-001: `cloacina-server` emits an OpenAPI 3.1 document at a stable endpoint (e.g., `/openapi.json`) and as a build artifact checked into the repo for diffing.
- REQ-002: WS computation-graph protocol is documented (message types, lifecycle, reconnection semantics) in a versioned spec file alongside the OpenAPI doc.
- REQ-003: SDKs cover full REST surface: auth, keys, tenants, workflows, executions, triggers, health_graphs.
- REQ-004: SDKs expose a typed client for the WS computation-graph stream with reconnection and backpressure handling.
- REQ-005: Each SDK supports the existing auth model (API key + tenant header) consistent with `cloacinactl`.
- REQ-006: Each SDK ships with quickstart + reference docs integrated into the existing Diataxis site.
- REQ-007: Each SDK ships a **live-server contract test suite** that runs the generated client against a real `cloacina-server` instance (not mocks, not the spec alone). Suite must cover every documented endpoint and at least one WS subscription lifecycle. Failures here mean the utoipa annotation has drifted from the handler — the test is *the* drift detector, not the spec-vs-spec diff.
- REQ-008: SDK releases are version-locked to server releases. SDK `vX.Y.Z` is built, tested, and published against server `vX.Y.Z`; no independent SDK MINOR/PATCH cadence.
- REQ-009: `cloacina-server` ships configurable CORS (allowed origins, methods, headers) so the TS SDK is actually usable from browsers — disabled by default, explicit opt-in via server config. This is Phase 1 server-side work; a browser UI cannot exist without it.
- REQ-010: API key + tenant header is the accepted v1 auth story for browser consumers (first-party admin UI). This is a deliberate decision, not an accident — browser-grade auth (sessions/OIDC) is out of scope for this initiative and owned by the future UI initiative. TS SDK docs must state this explicitly.
- NFR-001: Generated code regenerates deterministically; CI fails if checked-in spec or generated client drifts from a fresh generation against the live server.
- NFR-002: SDK calls add < 5ms overhead vs raw `reqwest`/`httpx`/`fetch` for a single round-trip on localhost.
- NFR-003: No SDK leaks server-internal types (diesel models, internal enums); all DTOs live in a server-public schema module.

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

### Overview

Three layers:

1. **Spec layer (server-side).** Annotate axum handlers and DTOs with `utoipa`; produce `openapi.json` at build time and at a runtime endpoint. WS protocol gets a hand-written `docs/reference/ws-protocol.md` with a JSON-schema for message envelopes.
2. **Rust client crate (`cloacina-client`).** Hand-written ergonomic layer over `reqwest` + `tokio-tungstenite`. DTOs imported from a new `cloacina-api-types` crate that both the server and client depend on (single source of truth, no codegen for Rust). `cloacinactl` migrates to depend on `cloacina-client`.
3. **Python + TypeScript SDKs.** Generated from `openapi.json` using **language-native generators** (`openapi-python-client` for Python; `openapi-typescript-codegen` or `openapi-typescript` for TS — final pick in phase 1 after generating against the real spec). No JVM-based `openapi-generator`. Thin hand-written ergonomics shim (auth, tenant, pagination iterators, WS wrapper) on top. Generated code lives in a `generated/` subtree; the shim is hand-maintained.
4. **Live-server contract layer (cross-cutting).** Each SDK has a test suite that boots `cloacina-server` (via existing compose) and exercises the generated client end-to-end. This is the trust boundary between utoipa annotations and actual handler behavior — spec-vs-handler drift surfaces here as test failures, not as silent broken SDKs.

### Component Diagram (textual)

```
cloacina-server  ──┐                                ┌── cloacinactl (Rust bin)
                   │                                │
                   ├── cloacina-api-types (Rust) ───┤
                   │                                │
                   ├── openapi.json  ───────────────┼── cloacina-client (Rust crate)
                   │       │                        │
                   │       ├────► cloacina-client (Python, PyPI)
                   │       └────► @cloacina/client  (TypeScript, npm)
                   │
                   └── ws-protocol.md (+ JSON schema) ──► all three SDKs
```

## Detailed Design **[REQUIRED]**

### Phase 1 — API contract

- Add `utoipa` + `utoipa-axum` to `cloacina-server`. Annotate each handler + request/response DTO.
- Extract DTOs into a new `crates/cloacina-api-types` crate (no diesel, no engine deps) so the server and Rust client share types without circular crates.
- Emit `openapi.json` via `cargo run --bin cloacina-server -- emit-openapi > docs/reference/openapi.json` and serve at `/openapi.json` at runtime.
- Add `angreal docs spec-check` to diff committed spec vs build-time spec; CI fails on drift.
- Document the WS computation-graph protocol in `docs/reference/ws-protocol.md` with JSON-schema for each message variant.

### Phase 2 — Rust client crate

- New crate `crates/cloacina-client` depending on `cloacina-api-types` + `reqwest` + `tokio-tungstenite`.
- Migrate `cloacinactl/src/shared/{client,client_ctx,error}.rs` into `cloacina-client`; `cloacinactl` becomes a consumer.
- Public API mirrors REST nouns: `client.workflows()`, `client.executions()`, `client.tenants()`, etc. Each verb returns typed DTOs from `cloacina-api-types`.
- WS support via `client.computation_graph().subscribe(...)` returning a typed `Stream`.
- Auth helpers: `ClientBuilder::api_key(...)`, `tenant(...)`, `profile_from_cloacinactl_config()` for parity.

### Phase 3 — Python SDK

- Repo path `clients/python/`. Package name `cloacina-client` on PyPI; import name `cloacina_client` (avoids collision with `cloaca`).
- Generate from `openapi.json` using `openapi-python-client` (httpx-based). Pin generator version.
- Hand-written ergonomics module: `cloacina_client.Client(server, api_key, tenant=...)`, sync + async variants, pagination iterators, WS wrapper using `websockets`.
- Build via `uv` / `hatchling`; publish via existing release workflow.

### Phase 4 — TypeScript SDK

- Repo path `clients/typescript/`. Package `@cloacina/client` on npm.
- Generate types + fetch client from `openapi.json`. Hand-written ergonomics: auth header injection, tenant scoping, pagination async iterator, WS wrapper using native `WebSocket` (browser) and `ws` (node).
- Dual ESM/CJS build via `tsup` or similar.

### Phase 5 — Live-server contract tests, docs, release

- New angreal task `angreal test sdk-contract` (or per-language `sdk-contract-python` / `sdk-contract-ts` / `sdk-contract-rust`): boots `cloacina-server` via existing compose, then runs each SDK's contract suite end-to-end. This is the explicit drift gate against utoipa-annotation regressions.
- The Rust client gets the same live-server treatment despite sharing `cloacina-api-types` with the server — DTO sharing prevents schema drift, but does not prove handler semantics match the documented contract.
- Coverage rule: every endpoint in the spec is exercised at least once per SDK; every documented WS message variant is round-tripped at least once.
- Diataxis docs: tutorial (build a small consumer in each language), how-to (auth, pagination, WS subscribe), reference (auto-generated from spec).
- Version policy: SDK version == server release version, lockstep. Initial release tagged at whatever the next `cloacina` release is.

### Resolved decisions

- **Repo layout:** monorepo. Client crates/packages live under this repo so spec/server/client drift surfaces in a single PR and CI run. Python SDK under `clients/python/`, TS under `clients/typescript/`, Rust client as `crates/cloacina-client`.
- **Generator choice:** language-native (`openapi-python-client` for Python; `openapi-typescript-codegen` or `openapi-typescript` for TS). Avoids JVM dep. Final pick happens in phase 1 after generating against the real spec. **TS pick (T-0645): `openapi-typescript@7.13.0` + `openapi-fetch` runtime.** Both candidates were generated against the real spec; `openapi-typescript-codegen` is unmaintained (its README points to a successor project) while `openapi-typescript` v7 is actively maintained, handles OpenAPI 3.1 natively, and emits a single deterministic types file with a ~6 kB fetch-based runtime that works in browsers and node without per-service class codegen.
- **API versioning:** explicit lockstep with `cloacina` release version. No independent SDK MINOR/PATCH cadence.
- **Server DTO refactor appetite:** broad. Route handlers and DTOs may churn freely to become cleanly spec-able.

### Still-open (resolve during discovery, not blocking decomposition)

- Whether to expose the generated client directly or wrap it in a hand-written ergonomics layer (likely wrap, but defer until we see generator output).
- WS protocol versioning strategy — embed `protocol_version` in the connect handshake.
- License/attribution headers for generated code subtrees.

## Alternatives Considered **[REQUIRED]**

- **Hand-roll all three SDKs without OpenAPI.** Rejected: triples drift surface; every route change requires three coordinated PRs; no machine-readable contract for third parties.
- **Publish only an OpenAPI spec, no first-party SDKs.** Rejected: leaves WS protocol undocumented and pushes onboarding cost onto every consumer; first-party SDKs are the adoption signal for "use cloacina as a service."
- **gRPC instead of REST+WS.** Rejected for this initiative: would require reworking the existing server transport. Worth a separate ADR if/when streaming demands outgrow WS.
- **Reuse `cloaca` as the Python client.** Rejected: `cloaca` is the embedded engine via PyO3; conflating embedded and service consumption is the exact confusion this initiative removes (see [[project_reactor_vs_computation_graph]] for prior naming-discipline incident).
- **Codegen Rust client from OpenAPI too.** Rejected: Rust types live in the same workspace as the server, so a shared `cloacina-api-types` crate is simpler and avoids generator quirks.

## Implementation Plan **[REQUIRED]**

Phased, each phase a candidate task batch on decomposition:

1. **Spec foundation** — utoipa wiring, `cloacina-api-types` extract, `openapi.json` emission, WS protocol doc, drift CI. Exit: spec committed, CI gate green.
2. **Rust client extract** — new `cloacina-client` crate, `cloacinactl` migrated to consume it, WS support landed. Exit: `cloacinactl` builds on top of `cloacina-client`, no behavior change.
3. **Python SDK** — generator pinned, ergonomics shim, sync+async, WS, packaged, live-server contract suite green. Exit: `pip install cloacina-client` works against a local server *and* the SDK contract suite passes in CI.
4. **TypeScript SDK** — generator pinned, ergonomics shim, ESM/CJS, WS, packaged, live-server contract suite green. Exit: `npm install @cloacina/client` works in node + browser smoke *and* the SDK contract suite passes in CI.
5. **Cross-SDK release** — full `angreal test sdk-contract` matrix green against current `cloacina-server`, Diataxis tutorial/how-to/reference per language, version-lockstep release tooling, tagged release matching the next server version.

Sequencing note: **Phase 1 → Phase 4 is the spine.** The follow-on UI initiative consumes the TS SDK, so the TS SDK starts as soon as the spec lands and is not queued behind the Rust extract or Python SDK. Phases 2, 3, and 4 all run in parallel once phase 1 lands. Phase 2 sequences before phase 5 so `cloacinactl` exercises the same client surface third parties see — the UI plays the identical forcing-function role for the TS SDK. Each of phases 2–4 owns its own live-server contract suite — the phase doesn't exit until its suite is green; the cross-SDK matrix in phase 5 is an aggregation, not the first time anything is tested.
