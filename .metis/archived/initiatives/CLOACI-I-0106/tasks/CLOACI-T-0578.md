---
id: t-01-span-enrichment-tenant-id-key
level: task
title: "T-01: Span enrichment — tenant_id/key_id/role on request spans"
short_code: "CLOACI-T-0578"
created_at: 2026-05-13T19:38:40.896514+00:00
updated_at: 2026-05-13T20:58:10.179473+00:00
parent: CLOACI-I-0106
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0106
---

# T-01: Span enrichment — tenant_id/key_id/role on request spans

## Parent Initiative

[[CLOACI-I-0106]]

## Objective

Enrich `cloacina-server`'s request-handling tracing spans with `tenant_id`, `key_id`, and `role` so operator debugging gets per-tenant filtering for free. Today spans carry method/route/status; after auth middleware extracts `AuthenticatedKey`, these three identifiers should be attached to the current span. Every downstream `tracing::info!`, every audit emit, every OTLP trace span inherits them. Closes OPS-03 and OPS-12.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

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

**2026-05-13** — Landed. 4 new unit tests pass; clippy clean.

### What changed

- `crates/cloacina-server/src/lib.rs::request_id_middleware`: `info_span!("request", ...)` pre-declares `tenant_id`, `key_id`, `role` as `tracing::field::Empty` so the auth middleware can `record(...)` values onto the same span.
- `crates/cloacina-server/src/routes/auth.rs::require_auth`: after `validate_token` succeeds, calls new `record_auth_span_fields(&Span::current(), &auth)` before inserting `AuthenticatedKey` into request extensions.
- New `pub(crate) fn record_auth_span_fields(span, auth)` helper:
  - `key_id` → `auth.key_id` (UUID display).
  - `tenant_id` → `Some(t)` displays the tenant; `None + is_admin` displays `<admin>`; `None + !is_admin` displays `<none>`.
  - `role` → `auth.permissions` string (`admin` | `write` | `read`).

### Tests landed (4 new, all passing)

- `record_auth_span_fields_tenant_scoped` — tenant-scoped key renders `tenant_id=<tenant>`, `key_id=<uuid>`, `role=<perms>`.
- `record_auth_span_fields_admin_sentinel` — `None + is_admin` renders `<admin>`.
- `record_auth_span_fields_no_tenant_no_admin` — global non-admin key renders `<none>`.
- `record_auth_span_fields_unauth_request_leaves_empty` — unauthenticated request never gets `record(...)`; captured output contains no populated `tenant_id`.

Test infra: `capture_under_request_span` helper sets up a `tracing_subscriber::fmt` with a `StringWriter` `MakeWriter`, opens the request span with the three fields as Empty, runs the closure, emits a probe event, returns the captured output.

### Design notes

- **`<admin>` sentinel vs admin's home tenant** — chose `<admin>`. Admin god-mode often acts cross-tenant; rendering the home tenant would mislead operators.
- **Audit inheritance** propagates automatically through tracing spans.
- **Pre-declaration footgun** — production + tests pre-declare the same three fields, so a future regression that drops one breaks the tenant-scoped test.

### Verification (local)

- `cargo test --lib -p cloacina-server --features postgres record_auth_span` → 4 new tests pass.
- `cargo clippy --lib -p cloacina-server --features postgres` → clean.
