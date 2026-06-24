---
id: route-table-authz-middleware
level: task
title: "Route-table authZ middleware — authed_route builder + authz_mw + behavior-preserving wire-in + no-drift test"
short_code: "CLOACI-T-0783"
created_at: 2026-06-24T00:41:37.246454+00:00
updated_at: 2026-06-24T00:41:37.246454+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Route-table authZ middleware — authed_route builder + authz_mw + behavior-preserving wire-in + no-drift test

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Add the `authed_route()` builder (registers an axum route **and** its `Access` into an `AppState` map from one call) and the `authz_mw` middleware (looks up `MatchedPath`+method, resolves `TenantParam` from the path, builds `Principal` from `Extension<AuthenticatedKey>`, calls `evaluate()`). Wire the full route→Access table for **every** current `/v1` route using **behavior-preserving** Access values, and **remove** the scattered `can_*`/`is_admin` if-blocks from the handlers so gating lives solely in the middleware. Add the fail-closed backstop: a matched route absent from the map is denied, plus a test asserting every mounted route has an entry.

## Acceptance Criteria **[REQUIRED]**

- [ ] `authed_route(path, method_router, access)` registers the route and `(method, matched-path) -> Access` together; the map lives in `AppState`.
- [ ] `authz_mw` mounted after `require_auth` on every `require_auth`-layered sub-router; a matched route with no Access entry returns 403 (never open).
- [ ] Every handler's `can_access_tenant`/`can_write`/`can_admin`/`is_admin` **gate** removed; data-scoping branches (e.g. `health_graphs` widen-by-god) retained.
- [ ] WS handlers (`/ws/*`) call `evaluate()` in-handler (documented exception, same matcher).
- [ ] No-drift test: every route in the built `Router` resolves to an Access entry.
- [ ] Existing server auth integration suite green with **no behavior change**: `angreal test integration`.

## Implementation Notes

**Scope:** builder + middleware + table + handler-gate removal + no-drift test. Behavior-preserving only — the leak fix / new tenant-key endpoints land in T-0784; land T-0783+T-0784 together to avoid a window where `/tenants/{t}/keys` list/delete are referenced but absent.
**Depends on:** T-0782 (matcher core).
**References:** I-0118 → "Phase 0 design" route→Access table; router at `crates/cloacina-server/src/lib.rs:1106-1338`; handler gates in `routes/{workflows,executions,triggers,keys,tenants,agent,compiler,health_graphs}.rs`. Mechanism: `axum::extract::MatchedPath` + `RawPathParams`.

## Status Updates **[REQUIRED]**

*To be added during implementation*
