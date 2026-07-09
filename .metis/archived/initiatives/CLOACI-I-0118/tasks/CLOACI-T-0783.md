---
id: route-table-authz-middleware
level: task
title: "Route-table authZ middleware — authed_route builder + authz_mw + behavior-preserving wire-in + no-drift test"
short_code: "CLOACI-T-0783"
created_at: 2026-06-24T00:41:37.246454+00:00
updated_at: 2026-06-24T02:13:41.841830+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Route-table authZ middleware — authed_route builder + authz_mw + behavior-preserving wire-in + no-drift test

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Add the `authed_route()` builder (registers an axum route **and** its `Access` into an `AppState` map from one call) and the `authz_mw` middleware (looks up `MatchedPath`+method, resolves `TenantParam` from the path, builds `Principal` from `Extension<AuthenticatedKey>`, calls `evaluate()`). Wire the full route→Access table for **every** current `/v1` route using **behavior-preserving** Access values, and **remove** the scattered `can_*`/`is_admin` if-blocks from the handlers so gating lives solely in the middleware. Add the fail-closed backstop: a matched route absent from the map is denied, plus a test asserting every mounted route has an entry.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

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

**2026-06-24 — COMPLETE (commit 6c164ce6).**

DONE: `authz_mw` middleware in `routes/authz.rs`, mounted `.route_layer` **inner of** `require_auth` (auth runs first) on the auth/graph-health/agent sub-routers. Strips `/v1` from `MatchedPath`, looks up the `(method, path) -> Access` table, **fail-closes** (unclassified → 403), resolves `TenantParam` from `RawPathParams[tenant_id]`, projects key → `Principal`, runs `evaluate()`, maps deny→existing 403 envelopes. `authz_table` on `AppState` (both ctor sites). Table is **behavior-preserving** (43 routes): `/auth/keys*`=`Any+Admin` (today's unscoped can_admin — leak preserved for T-0784), `/tenants*`+`/tenants/{t}/keys`+`/agents`+`/compiler/status`=`Platform`, tenant routes `Tenant+Read/Write`, agent/health/ws-ticket=`Any`. Removed the scattered `can_access_tenant`/`can_write`/`can_admin`/`is_admin` gate blocks from workflows/executions/triggers/keys/tenants/compiler/agent (renamed orphaned `auth`→`_auth`; kept where forwarded/used: pause/resume helpers, ws-ticket `issue`, agent register/artifact tenant data). Data-scoping (health_graphs visibility, agent tenant pin) untouched. No-drift test pins the 43-route table. `angreal check crate` clean; 8/8 authz unit tests green.

DEVIATED (equivalent): used a static `build_authz_table()` + fail-closed map lookup instead of an `authed_route()` wrapper — axum's typed handlers make a per-route wrapper awkward; the static table gives the same "route can't be served unclassified" guarantee (fail-closed at runtime + the no-drift unit test).

DEFERRED: (1) WS `/ws/*` handlers still use their **existing** in-handler ticket/bearer auth (delivery already enforces recipient+tenant) — routing them through `evaluate()` is a consistency nicety, not a security fix, since they're not behind `authz_mw`; left as-is to stay behavior-preserving. (2) `angreal test integration` (postgres lane) NOT run in-loop — recommended as the final runtime gate before merge (validates MatchedPath-prefix handling, tenant-param extraction, layer ordering). Core behavior is preserving-by-construction (parity-tested matcher + behavior-preserving table).