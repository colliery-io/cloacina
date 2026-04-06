---
id: authorization-model-tenant
level: initiative
title: "Authorization Model — Tenant Isolation, Key Scoping, and CG Policy Wiring"
short_code: "CLOACI-I-0083"
created_at: 2026-04-06T11:27:55.819508+00:00
updated_at: 2026-04-06T16:06:49.942650+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: authorization-model-tenant
---

# Authorization Model — Package Security and Tenant-Scoped Access

## Context

The upload-to-access pipeline has no security thread running through it. When a package (workflow or computation graph) is uploaded via `POST /tenants/{tenant_id}/workflows`, nothing connects it to that tenant. The package lands in a global namespace, the `tenant_id` in the URL is decorative, and no authorization policies are established for accessing what the package provides.

This was discovered while building the computation graph soak test (T-0404). CG packages load correctly — the reconciler compiles them, the scheduler registers accumulators and reactors in the EndpointRegistry — but no `AccumulatorAuthPolicy` or `ReactorAuthPolicy` is ever set. The WebSocket handlers correctly enforce deny-by-default, so all CG endpoints are permanently 403. There's no way for any key to push events to any accumulator.

### What Works

- **Authentication**: Bearer-token PAKs with SHA-256 hashing, LRU cache (256/30s), soft revocation. The `require_auth` middleware correctly gates all protected routes. `AuthenticatedKey { key_id, name, permissions }` is inserted into request extensions.
- **Tenant provisioning**: `POST /tenants` creates real Postgres schemas with isolated users, grants, and per-schema migrations. Tenants are schema-level isolation.
- **CG WebSocket enforcement**: `ws.rs` validates tokens, then checks per-endpoint policies via `check_accumulator_auth` / `check_reactor_auth`, then per-command policies via `check_reactor_op_auth`. This is the only real authz in the system and it works correctly — just has no policies to check against.

### What's Broken

**1. Packages have no tenant ownership**
- `workflow_packages` table has no `tenant_id` column
- Upload handler extracts `tenant_id` from URL but never stores it with the package
- Reconciler has a `default_tenant_id` field (defaults to `"public"`) used only as a namespace prefix
- All packages are effectively global

**2. Handlers never use AuthenticatedKey**
- `require_auth` middleware inserts `AuthenticatedKey` into request extensions
- No REST handler extracts it — `key_id`, `name`, `permissions` are all `#[allow(dead_code)]`
- Every handler treats the auth gate as binary: valid key → proceed, no key → 401
- Any valid key can upload to, execute in, or read from any tenant

**3. API server doesn't use tenant schemas**
- Tenants are provisioned as separate Postgres schemas with dedicated users
- But the API server uses a single global database connection for all queries
- `tenant_id` in URL paths is never used to switch schema or filter data
- Handler calls like `registry.list_workflows()`, `runner.execute_async()`, `dal.schedule().list()` all hit the default schema

**4. CG auth policies never populated**
- `ReactiveScheduler.load_graph()` registers accumulators and reactors in the EndpointRegistry
- Never calls `set_accumulator_policy()` or `set_reactor_policy()`
- No API endpoint exists to manage policies after the fact
- Result: all WS connections get 403 — the auth infrastructure is correct but starved of data

**5. `permissions` field is dead code**
- `api_keys.permissions` hardcoded to `"admin"` in DAL (`crud.rs:77`)
- Never read by any handler
- Planned RBAC (`auth_tokens` table with `role`, `tenant_id`) exists as commented-out schema but has no migrations, no DAL, no feature flag

**6. Bootstrap key has no special privilege**
- Created via same `create_key()` path as any other key
- Distinguished only by name `"bootstrap-admin"` — no flag, no elevated access
- Cannot do anything that any other key cannot

### Relationship to Other Work

- **I-0079** (Soak Tests) — T-0404 blocked; cannot inject events via WebSocket without CG policy wiring
- **I-0051** (Hardening) — auth is a hardening concern, but this is standalone since it's the foundation for all access control

## Deployment Modes

The system supports two modes, determined by whether tenants are created:

### Single-tenant (default, zero-config)

The easiest startup for a single-tenant deployment. Everything is globally scoped:

1. Server starts → bootstrap key created (god mode — the operator's key)
2. Upload packages to `/tenants/public/workflows` → stored in default schema
3. Execute workflows → works with bootstrap key
4. CG packages load → auth policies set to **allow any authenticated key**
5. WS connect with bootstrap key → 101, works immediately

No tenant configuration required. The system behaves like a single-user application. Any valid key can access anything. This is today's behavior, minus the CG policy gap.

### Multi-tenant (opt-in)

When tenants are created and tenant-scoped keys issued, isolation kicks in:

1. Bootstrap key (god mode) calls `POST /tenants` → acme schema created
2. Bootstrap key creates key scoped to acme → `POST /tenants/acme/keys`
3. Acme's key uploads to `/tenants/acme/workflows` → package bound to acme's schema
4. CG packages load → auth policies set to **allow acme's keys only**
5. Acme's key connects WS → 101. Other tenant's key → 403.
6. Bootstrap key retains god mode — can access any tenant, any endpoint

The transition is seamless: single-tenant deployments never need to think about tenants. Multi-tenant deployments add isolation incrementally.

## Goals & Non-Goals

**Goals:**
- Thread tenant ownership through the package lifecycle: upload → store → load → access
- Connect handlers to the tenant schema system that already exists
- Wire CG auth policies when packages load so WebSocket endpoints become usable
- Make `AuthenticatedKey` actually used by handlers for authorization decisions
- Establish bootstrap key as god mode — the operator key that can do anything
- Preserve single-tenant default: everything works globally when no tenants are configured

**Non-Goals:**
- Redesigning tenant schema isolation — it already works at the DB level
- OAuth2/OIDC — PAK-based auth is sufficient for MVP
- Fine-grained per-workflow permissions — tenant-level scoping is the target
- Rate limiting, audit logging — separate concerns
- Implementing the commented-out `auth_tokens` RBAC system as-designed — may inform the design but isn't the plan

## Detailed Design

### Phase 1 — CG Policy Wiring (unblock the system)

Make CG WebSocket endpoints usable immediately. This is the minimal fix:

- When `ReactiveScheduler.load_graph()` loads a package, call `set_accumulator_policy()` and `set_reactor_policy()` for each endpoint
- In single-tenant mode (no tenant scoping configured): policy = **allow any authenticated key**
- Bootstrap key (god mode) always included in policies
- This makes the current single-tenant flow work end-to-end and unblocks T-0404

### Phase 2 — Package Tenant Ownership

Thread `tenant_id` through the package storage and loading pipeline:

- Add `tenant_id` column to `workflow_packages` table (nullable — NULL = global/public)
- Upload handler stores the `tenant_id` from the URL path with the package
- Reconciler passes tenant context when loading packages
- When listing/getting workflows, filter by tenant from URL path
- Packages with `tenant_id = NULL` (or `"public"`) remain globally accessible (single-tenant default)

### Phase 3 — Key Scoping and Bootstrap Privilege

- Add `tenant_id` to `api_keys` table (nullable — NULL = global, not tenant-scoped)
- Add `is_admin` flag to `api_keys` table — bootstrap key is god mode via this flag, not via NULL tenant_id
- `POST /auth/keys` (no tenant context) creates global keys (single-tenant mode)
- `POST /tenants/{tid}/keys` creates keys scoped to that tenant (multi-tenant mode)
- Global keys (tenant_id=NULL) can access global packages — they are unscoped, not privileged
- Admin keys (is_admin=true) can access any tenant, create tenants, create keys for any tenant — this is god mode
- Bootstrap key is the first admin key, created at startup

### Phase 4 — Handler Tenant Enforcement

Connect handlers to the existing tenant schema system:

- Handlers extract `AuthenticatedKey` from extensions
- For tenant-scoped keys: verify `key.tenant_id` matches `tenant_id` from URL path
- For global keys (tenant_id=NULL): can access global/public resources only
- For admin keys (is_admin=true): pass all tenant checks — god mode
- Handlers use `tenant_id` from URL to scope database queries to the correct schema

### Phase 5 — Tenant-Scoped CG Policies

Upgrade Phase 1's "allow any authenticated key" to tenant-scoped:

- When a package has a `tenant_id`, CG policies restrict access to keys belonging to that tenant + god mode keys
- When a package has no `tenant_id` (global/public), policies allow any authenticated key (single-tenant behavior preserved)
- Admin keys (is_admin=true) always included in all policies — god mode

### Phase 6 — Role Enforcement

Replace the dead `permissions: "admin"` with enforced roles:

- admin: full access within tenant (manage keys, upload, execute, delete)
- write: upload packages, execute workflows
- read: list/get only
- God mode (bootstrap) implicitly has admin everywhere

### Phase 7 — Testing

- Single-tenant flow: bootstrap key can do everything, CG WebSocket works
- Multi-tenant flow: tenant keys isolated, cross-tenant access denied
- CG WebSocket: tenant A's keys → tenant A's accumulators only
- God mode: bootstrap key accesses everything regardless of tenant
- Package lifecycle: upload → compile → load → access with correct scoping
- Update server-soak test to exercise both modes

## Alternatives Considered

**Alt 1: Just fix CG policy wiring (Phase 1 only)**
Would unblock the soak test and make single-tenant work. Acceptable as an interim but doesn't address multi-tenant gaps. Phases are ordered so Phase 1 delivers immediate value.

**Alt 2: Implement the commented-out `auth_tokens` RBAC**
The schema exists but may be over-designed for current needs. Better to build what's needed now and evolve.

## Implementation Plan

1. **Phase 1** — CG policy wiring (allow any authenticated key) — **unblocks T-0404 and single-tenant CG flow**
2. **Phase 2** — Package tenant ownership (schema + upload + reconciler)
3. **Phase 3** — Key scoping and bootstrap god mode
4. **Phase 4** — Handler tenant enforcement
5. **Phase 5** — Tenant-scoped CG policies (upgrade Phase 1)
6. **Phase 6** — Role enforcement
7. **Phase 7** — Testing
