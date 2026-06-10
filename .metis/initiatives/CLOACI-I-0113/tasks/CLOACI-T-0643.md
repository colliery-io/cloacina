---
id: openapi-spec-emission-utoipa
level: task
title: "OpenAPI spec emission — utoipa annotations, /openapi.json, emit-openapi, CORS, drift CI"
short_code: "CLOACI-T-0643"
created_at: 2026-06-10T01:30:11.563540+00:00
updated_at: 2026-06-10T01:30:11.563540+00:00
parent: CLOACI-I-0113
blocked_by: ["CLOACI-T-0642"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0113
---

# OpenAPI spec emission — utoipa annotations, /openapi.json, emit-openapi, CORS, drift CI

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0113]]

## Objective **[REQUIRED]**

Annotate the full REST surface with `utoipa` and publish the API contract: OpenAPI 3.1 emitted via an `emit-openapi` subcommand, checked in at `docs/reference/openapi.json`, served at runtime `/openapi.json` (REQ-001). Ship configurable CORS so browser consumers can exist at all (REQ-009). Add the `angreal docs spec-check` drift gate to CI (NFR-001). Route handlers and DTOs may churn freely to become cleanly spec-able — encouraged, not avoided.

## Acceptance Criteria **[REQUIRED]**

- [ ] `utoipa` + `utoipa-axum` wired; every route handler and DTO annotated; spec covers the full REST surface: auth, keys, tenants, workflows, executions, triggers, health_graphs (REQ-003)
- [ ] OpenAPI 3.1 doc emitted via `cloacina-server emit-openapi`, committed at `docs/reference/openapi.json`, served at runtime `/openapi.json` (REQ-001)
- [ ] API-key + tenant-header auth modeled as security schemes in the spec (REQ-005)
- [ ] Configurable CORS layer — allowed origins/methods/headers from server config, disabled by default (REQ-009)
- [ ] `angreal docs spec-check` diffs committed spec vs freshly emitted; wired into CI, fails on drift (NFR-001)

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
`utoipa` derives on the `cloacina-api-types` DTOs plus `#[utoipa::path]` annotations per handler; assemble via `utoipa-axum` router bindings. `emit-openapi` as a server subcommand writing the doc to stdout. CORS via `tower-http::cors::CorsLayer` driven from server config (origins/methods/headers lists, off by default). `angreal docs spec-check` runs emit-openapi and diffs against the committed `docs/reference/openapi.json`.

### Dependencies
Blocked by CLOACI-T-0642 (DTOs must live in `cloacina-api-types` so utoipa derives go on the shared types).

### Risk Considerations
utoipa has gaps around some axum extractor patterns — handlers may need reshaping (explicitly allowed). Annotation-vs-handler semantic drift is NOT caught by this task — the live contract suites (T-0645/0646/0647) are the drift detector, per the initiative's design-around of past SDK drift pain.

## Status Updates **[REQUIRED]**

*To be added during implementation*
