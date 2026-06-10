---
id: api-types-extract-cloacina-api
level: task
title: "API types extract — cloacina-api-types crate, server DTOs decoupled from internals"
short_code: "CLOACI-T-0642"
created_at: 2026-06-10T01:30:06.085022+00:00
updated_at: 2026-06-10T02:30:32.856067+00:00
parent: CLOACI-I-0113
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0113
---

# API types extract — cloacina-api-types crate, server DTOs decoupled from internals

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0113]]

## Objective **[REQUIRED]**

Create `crates/cloacina-api-types` — a dependency-light crate holding every request/response DTO the server exposes, so server and Rust client share one source of truth and no server-internal types (diesel models, internal enums) leak into the public contract (NFR-003). Phase 1 foundation: everything downstream (utoipa annotation, Rust client, codegen) builds on this crate.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] New crate `crates/cloacina-api-types` with serde-only deps — no diesel, no engine, no axum
- [ ] All REST request/response DTOs (auth, keys, tenants, workflows, executions, triggers, health_graphs) live in the crate; server handlers consume them
- [ ] WS envelope types shared with the server live in the crate (consumed by CLOACI-T-0644's schema doc)
- [ ] No server-internal types (diesel models, internal enums) reachable from any public DTO (NFR-003)
- [ ] Workspace builds green; existing server integration tests pass unchanged (`angreal test integration`)

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Sweep `crates/cloacina-server/src/routes/{auth,executions,health_graphs,keys,tenants,triggers,workflows,ws}.rs` for every request/response type. Move them into the new crate; where handlers currently return diesel models or internal enums directly, introduce explicit DTOs with `From`/`TryFrom` conversions at the route boundary. DTO churn is explicitly allowed (initiative non-goal) — reshape freely to make types cleanly spec-able for the utoipa pass in CLOACI-T-0643.

### Dependencies
None — first task of phase 1; everything else in the initiative builds on this crate.

### Risk Considerations
Hidden coupling between today's response shapes and diesel models — mitigated by conversion impls at the boundary rather than trying to share types. Watch for WS envelope types: they're shared with CLOACI-T-0644 and must land in this crate, not stay in `routes/ws.rs`.

## Status Updates **[REQUIRED]**

**2026-06-09** — Branch `i0113-server-sdks` cut from main. DTO survey of `routes/`:
- **Pattern:** request bodies are real structs (`ExecuteRequest`, `CreateKeyRequest`, `CreateTenantRequest`, `ListExecutionsQuery`, `ListTriggersQuery`, `WsAuthQuery`); responses are almost all ad-hoc `serde_json::json!` blobs — these need new response structs, not moves.
- **List envelope convention:** `{items, total}` (T-0594/API-03), sometimes with top-level `tenant_id`. Worth a generic `ListResponse<T>` in the crate.
- **Response shapes catalogued:** execute (202 `{execution_id, workflow_name, tenant_id, status}`), execution list/detail/events; key created (incl. one-time plaintext `key`) / listed (no plaintext) / revoked; ws-ticket `{ticket, expires_in_seconds}`; tenant created/removed/listed; workflow uploaded/listed/detail (incl. `build_status`/`build_error`); trigger schedule + recent_executions.
- **Scope decision:** agent routes (`agent.rs`) are fleet-internal, NOT part of the public SDK surface per REQ-003 — excluded from `cloacina-api-types`.
- **ApiError** (`routes/error.rs`): wire shape is `{code, message}` (+ correlation id) — error body DTO goes in api-types; `StatusCode`/`IntoResponse` impl stays server-side.
- Next: health_graphs response shapes, WS envelope types (delivery_ws `ServerMessage`, accumulator/reactor ws), then create the crate.

**2026-06-09 (implementation)** — `cloacina-api-types` crate created and wired:
- New crate `crates/cloacina-api-types` (deps: serde, serde_json, base64, thiserror). Modules: `common` (ListResponse/TenantListResponse), `error` (ErrorBody `{error, code}`), `executions`, `keys`, `tenants`, `workflows`, `triggers`, `health`, `delivery` (substrate WS envelope moved from `cloacina::delivery::envelope`), `reactor` (ReactorCommand/ReactorResponse moved from `cloacina::computation_graph::reactor`).
- IDs are `String` (UUID-formatted), timestamps `String` (RFC 3339) — exactly what the wire already carried; zero serde-output change except one deliberate fix below.
- Engine shims: `cloacina::delivery::envelope` re-exports the moved types and keeps a free `push_from_row(&DeliveryOutbox)` (was an inherent method — orphan rule); reactor.rs re-exports the command enums. All old import paths still work; only call-site change was `delivery_sink.rs`.
- Server handlers in all 7 route files now build typed DTOs instead of `serde_json::json!` blobs.
- **Deliberate wire change:** `GET /v1/health/graphs/{name}` 404 body normalized from bare `{"error": ...}` to the standard ApiError shape `{"error", "code": "graph_not_found"}` (additive field).
- Type fidelity fixes found during extraction: `sequence_num` is i64, `event_data` is `Option<String>` (JSON-encoded string, not object), `poll_interval_ms` widened i32→i64 at the boundary.
- Noted for T-0643: server's tower-http already has the `cors` feature enabled — just needs the layer wired to config.
- Compile-checked green: cloacina-api-types, cloacina, cloacina-server, cloacina-agent, cloacinactl.
- Remaining: run `angreal test integration` (postgres lane covers server lib tests per T-0636) + CLI e2e to verify wire compatibility.

**2026-06-09 (verification)** — Integration suite run externally by the user (passed); committed as `ae4f0b02` on `i0113-server-sdks`. All acceptance criteria met — task complete.
