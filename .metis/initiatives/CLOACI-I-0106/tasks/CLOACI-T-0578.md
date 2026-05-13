---
id: t-01-span-enrichment-tenant-id-key
level: task
title: "T-01: Span enrichment — tenant_id/key_id/role on request spans"
short_code: "CLOACI-T-0578"
created_at: 2026-05-13T19:38:40.896514+00:00
updated_at: 2026-05-13T19:38:40.896514+00:00
parent: CLOACI-I-0106
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0106
---

# T-01: Span enrichment — tenant_id/key_id/role on request spans

## Parent Initiative

[[CLOACI-I-0106]]

## Objective

Enrich `cloacina-server`'s request-handling tracing spans with `tenant_id`, `key_id`, and `role` so operator debugging gets per-tenant filtering for free. Today spans carry method/route/status; after auth middleware extracts `AuthenticatedKey`, these three identifiers should be attached to the current span. Every downstream `tracing::info!`, every audit emit, every OTLP trace span inherits them. Closes OPS-03 and OPS-12.

## Acceptance Criteria

- [ ] Request-handling spans declare `tenant_id`, `key_id`, `role` as `tracing::field::Empty` at entry.
- [ ] Auth middleware (or the post-extraction layer) records the three fields on the current span via `Span::current().record(...)` after extracting `AuthenticatedKey`.
- [ ] JSON log output renders all three fields per request line.
- [ ] OTLP trace export (when configured) carries them as span attributes.
- [ ] `audit::log_*` emit sites inside request handlers inherit the fields without re-passing them.
- [ ] Unit test: an axum service composed with the auth layer, invoked with a tenant-scoped key, asserts span fields are present at handler invocation.
- [ ] Unit test: an anonymous (no-auth) request leaves the fields empty (rendered as `<none>` or absent depending on subscriber).
- [ ] **Test harness updated as we go**: any existing test asserting span fields by name extended for the three new ones. Run `angreal lint` + `angreal test unit` + `angreal test integration` after each meaningful change (don't batch to the end of the task).

## Test Cases

- **TC-1 (anon):** GET on an unauthenticated route → span carries no tenant_id/key_id/role.
- **TC-2 (tenant key):** GET with a tenant-scoped key → span shows `tenant_id=<uuid>`, `key_id=<uuid>`, `role=read|write|admin`.
- **TC-3 (admin key):** GET with an admin key → span shows `role=admin` (and `tenant_id` either the admin's home tenant or the `<admin>` sentinel — implementer's call, document the choice).
- **TC-4 (inherited audit emit):** `audit::log_package_load_success` invoked inside a handler with tenant context — the emitted event carries the same `tenant_id` as the parent span.

## Implementation Notes

### Technical Approach

- Span declaration: extend the request span macro (or wherever axum's `TraceLayer` constructs the entry span) to pre-declare `tenant_id`, `key_id`, `role` as `tracing::field::Empty`.
- Recording: in `crates/cloacina-server/src/middleware/auth.rs` (verify path during implementation), after `AuthenticatedKey` is extracted, call `tracing::Span::current().record("tenant_id", &display(key.tenant_id))` etc.
- For `role`, map the existing role enum (admin/write/read) to a `&'static str`.

### Dependencies

- None. Stands alone; T-0579/T-0580 will use the populated spans for their own assertions.

### Risk Considerations

- **OTLP cardinality.** `tenant_id` is bounded by tenant count (typically < hundreds). `key_id` is high-cardinality. If the operator's OTLP sink has per-attribute cardinality caps, document this in T-0577's audit doc when we update it.
- **Span field pre-declaration is order-sensitive.** Recording a field that wasn't pre-declared as `Empty` is silently a no-op. Test asserts the fields are actually visible at handler invocation, not just "the recording call was made."
- **Daemon path:** the daemon uses the same `cloacina` library; the auth middleware is server-only. No daemon impact, but verify no shared span-construction code that needs daemon updates.

## Status Updates

*To be added during implementation.*
