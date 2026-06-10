---
id: openapi-spec-emission-utoipa
level: task
title: "OpenAPI spec emission — utoipa annotations, /openapi.json, emit-openapi, CORS, drift CI"
short_code: "CLOACI-T-0643"
created_at: 2026-06-10T01:30:11.563540+00:00
updated_at: 2026-06-10T02:31:08.521537+00:00
parent: CLOACI-I-0113
blocked_by: [CLOACI-T-0642]
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0113
---

# OpenAPI spec emission — utoipa annotations, /openapi.json, emit-openapi, CORS, drift CI

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0113]]

## Objective **[REQUIRED]**

Annotate the full REST surface with `utoipa` and publish the API contract: OpenAPI 3.1 emitted via an `emit-openapi` subcommand, checked in at `docs/reference/openapi.json`, served at runtime `/openapi.json` (REQ-001). Ship configurable CORS so browser consumers can exist at all (REQ-009). Add the `angreal docs spec-check` drift gate to CI (NFR-001). Route handlers and DTOs may churn freely to become cleanly spec-able — encouraged, not avoided.

## Acceptance Criteria **[REQUIRED]**

- [x] `utoipa` wired (plain `#[utoipa::path]` + central `ApiDoc` — `utoipa-axum` deliberately skipped, see status update); every route handler and DTO annotated; spec covers the full REST surface: auth, keys, tenants, workflows, executions, triggers, health_graphs (REQ-003)
- [x] OpenAPI 3.1 doc emitted via `cloacina-server emit-openapi`, committed at `docs/static/openapi.json` (Hugo static root — repo has no flat `docs/reference/`), served at runtime `/openapi.json` (REQ-001)
- [x] Bearer API-key auth modeled as the `api_key` security scheme (REQ-005 — note: tenant scope rides the key + path; there is no tenant *header* in the actual server, and the scheme description documents this)
- [x] Configurable CORS layer — `--cors-allowed-origins/-methods/-headers` flags + `CLOACINA_CORS_*` env, disabled unless origins set (REQ-009)
- [x] `angreal docs spec-check` diffs committed spec vs freshly emitted; new `spec-check` job in `ci.yml` gated on an `api` paths filter, fails on drift (NFR-001)

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
`utoipa` derives on the `cloacina-api-types` DTOs plus `#[utoipa::path]` annotations per handler; assemble via `utoipa-axum` router bindings. `emit-openapi` as a server subcommand writing the doc to stdout. CORS via `tower-http::cors::CorsLayer` driven from server config (origins/methods/headers lists, off by default). `angreal docs spec-check` runs emit-openapi and diffs against the committed `docs/reference/openapi.json`.

### Dependencies
Blocked by CLOACI-T-0642 (DTOs must live in `cloacina-api-types` so utoipa derives go on the shared types).

### Risk Considerations
utoipa has gaps around some axum extractor patterns — handlers may need reshaping (explicitly allowed). Annotation-vs-handler semantic drift is NOT caught by this task — the live contract suites (T-0645/0646/0647) are the drift detector, per the initiative's design-around of past SDK drift pain.

## Status Updates **[REQUIRED]**

**2026-06-09** — Implemented on `i0113-server-sdks`:
- **api-types:** `openapi` cargo feature gates `utoipa = "5"` derives — every DTO gets `ToSchema` (query structs also `IntoParams`) via `cfg_attr`, so SDK consumers stay serde-only.
- **Server annotations:** all 21 REST handlers carry `#[utoipa::path]` (keys 5, tenants 3, workflows 4, triggers 2, executions 4, graph-health 3) + public `/health`,`/ready`. Agent-fleet routes deliberately excluded (internal protocol); WS endpoints deferred to T-0644's protocol doc.
- **Decision — no `utoipa-axum`:** converting `build_router` to `OpenApiRouter` would churn ~50 test call sites and the carefully-ordered `route_layer` middleware (see the agent-routes auth comment in lib.rs). Plain annotations + central `ApiDoc` in `src/openapi.rs` gets the same spec; the forgot-a-route risk is covered by T-0648's coverage rule (every spec endpoint exercised per SDK) and a unit test asserting the doc builds.
- **Emission:** `cloacina-server emit-openapi` (no DB needed — `database_url` became `Option`, validated at serve time); runtime route `/openapi.json` serves the identical document from the same `openapi_json()` fn. Spec: OpenAPI **3.1.0**, 20 paths, 35 schemas, `info.version` = workspace version (lockstep, unit-tested).
- **Committed spec location:** `docs/static/openapi.json` — the docs tree is a Hugo site with no flat `docs/reference/`; static root means the published docs site serves the spec directly.
- **CORS (REQ-009):** `CorsConfig` in lib.rs — origins/methods/headers from `--cors-allowed-*` flags / `CLOACINA_CORS_*` env, `*` supported, invalid values fail fast at boot, layer applied only in `run()` (the ~50 `build_router` test call sites untouched). Defaults when enabled: GET/POST/DELETE/OPTIONS + authorization/content-type.
- **Drift gate:** `angreal docs spec-check` (in `.angreal/task_docs.py`) emits fresh + unified-diffs against committed; verified in-sync locally. New `spec-check` CI job in `ci.yml` behind a new `api` paths filter (`crates/cloacina-server/**`, `crates/cloacina-api-types/**`, the spec itself, `Cargo.*`); actionlint clean. (Noticed in passing: the existing `rust` filter doesn't include `cloacina-server` — the new `api` filter covers the spec-relevant gap.)
- Compile-checked green: api-types (with `openapi` feature), server; spec-check task passes.
