---
id: wire-websocket-ticket-consumption
level: task
title: "Wire WebSocket ticket consumption and enforce tenant DAL isolation"
short_code: "CLOACI-T-0473"
created_at: 2026-04-11T13:31:09.354460+00:00
updated_at: 2026-04-11T15:52:23.109886+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Wire WebSocket ticket consumption and enforce tenant DAL isolation

## Objective

Wire existing but disconnected security implementations into the runtime path: WebSocket ticket consumption, tenant schema isolation at the DAL layer, and ticket store bounds.

## Review Finding References

SEC-001, SEC-006, SEC-008 (from architecture review `review/10-recommendations.md` REC-001)

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P0 - Critical (blocks users/revenue)

### Technical Debt Impact
- **Current Problems**: WebSocket handlers accept raw API keys in URL query params (logged by proxies, leaked in browser history). Tenant-scoped endpoints query the public schema regardless of tenant ID. WsTicketStore is unbounded with no eviction.
- **Benefits of Fixing**: Multi-tenant deployments become actually isolated. WebSocket credential exposure eliminated. Memory exhaustion vector closed.
- **Risk Assessment**: Without this, a compromised write-scoped API key can access all tenants' data. The security promise of schema isolation is not realized.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `extract_ws_token` in `ws.rs` calls `state.ws_tickets.consume(&token)` for query-param tokens; raw API keys rejected in query params
- [ ] Bearer tokens in `Authorization` header still accepted for non-browser clients
- [ ] `WsTicketStore` bounded (LruCache or similar, cap ~1024) with expired ticket eviction
- [ ] Tenant-scoped HTTP handlers set `search_path` to tenant schema before DAL calls (or use per-tenant DAL instances)
- [ ] A tenant-scoped key for tenant_a cannot see tenant_b's data
- [ ] Integration test: issue ticket via `POST /auth/ws-ticket`, connect WebSocket using ticket, verify connection succeeds
- [ ] Integration test: attempt WebSocket with raw API key in query param, verify rejection

## Implementation Notes

### Technical Approach

**WebSocket tickets (SEC-001, SEC-008):**
- In `extract_ws_token`, replace `validate_token(&state, &token)` with `state.ws_tickets.consume(&token)` when token source is query param
- Keep `Authorization: Bearer` header path using `validate_token` for programmatic clients
- Replace `HashMap<String, WsTicket>` with bounded LRU; add lazy eviction of expired entries in `issue()`

**Tenant DAL isolation (SEC-006):**
- `Database::try_new_with_schema()` already supports schema-scoped connections
- Option A: Per-request `SET search_path TO <tenant_schema>` before handler DAL calls
- Option B: Per-tenant `DefaultRunner` instances (heavier, but cleaner isolation)
- The tenant schema is derivable from the URL path param `tenant_id`; `auth.can_access_tenant()` already validates access

### Key Files
- `crates/cloacinactl/src/server/ws.rs` — WebSocket token extraction
- `crates/cloacinactl/src/server/auth.rs` — WsTicketStore, KeyCache
- `crates/cloacinactl/src/server/executions.rs` — tenant-scoped handlers
- `crates/cloacinactl/src/server/workflows.rs` — tenant-scoped handlers
- `crates/cloacinactl/src/commands/serve.rs` — single DefaultRunner creation

### Dependencies
None. All required code exists; this is integration work.

## Status Updates

**2026-04-11**: WebSocket ticket + bounded store complete, compiles clean. Tenant DAL isolation remaining.
- `extract_ws_token` now returns `WsTokenSource::Header` or `WsTokenSource::QueryTicket`
- New `authenticate_ws()` routes Header→`validate_token`, QueryTicket→`ws_tickets.consume()`
- `WsTicketStore` bounded to 1024 entries with lazy expiry eviction on `issue()`
- Both `accumulator_ws` and `reactor_ws` handlers updated
- Files changed: `server/ws.rs`, `server/auth.rs`
- SEC-006 tenant DAL isolation split out to CLOACI-T-0485 (needs design decision on approach)
