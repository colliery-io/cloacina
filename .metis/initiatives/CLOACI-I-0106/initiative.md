---
id: complete-multi-tenant-abstraction
level: initiative
title: "Complete multi-tenant abstraction across runner, reconciler, observability, and lifecycle"
short_code: "CLOACI-I-0106"
created_at: 2026-05-06T11:05:34.186707+00:00
updated_at: 2026-05-14T06:57:42.788345+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: complete-multi-tenant-abstraction
---

# Complete multi-tenant abstraction across runner, reconciler, observability, and lifecycle Initiative

## Context

Cloacina pitches multi-tenancy in three places: the system overview, the README, and the `--tenant` CLI flag. The implementation completed the storage layer (`TenantDatabaseCache`, schema search_path, per-tenant pools) but stops there. The runner, reconciler, request span, audit endpoints, health endpoints, and tenant-deletion lifecycle are all still admin-schema-bound.

The May 2026 cross-cutting review identified this as the project's **largest single architectural debt** — twelve findings across four lenses converge on one root cause. Symptoms include:

- Workflows execute in the admin schema regardless of which tenant submitted them (COR-01 + EVO-04).
- `triggers.rs::list_triggers` and `get_trigger` query the admin DB, leaking schedules across tenants (SEC-02).
- `/v1/health/accumulators`, `/v1/health/graphs` return cross-tenant data without filtering (SEC-05).
- `SET search_path` failure during connection acquisition silently routes the next query to `public` instead of failing (COR-01).
- `remove_tenant` does not stop reactors, cancel running executions, or evict the cached `Database` (SEC-14, SEC-17).
- Request spans don't carry `tenant_id`, `key_id`, or `role` — operator debugging has no per-tenant filter (OPS-03, OPS-12).
- `execution_events` lacks `request_id`/`runner_id`/`tenant_id` columns (OPS-16).

Per CLOACI-A-0005, multi-tenancy is a **server-only** concern. The daemon is high-trust and single-tenant by design; this initiative does not extend tenancy into the daemon. Within the server, the single abstraction must accommodate both single-org and multi-org deployment topologies without code forks.

## Goals & Non-Goals

**Goals:**
- Workflow execution writes to the correct tenant schema, every time, on every code path.
- Triggers, graph health, and accumulator health endpoints filter by the caller's authorized tenant set.
- `SET search_path` failure becomes a hard, fail-closed error rather than a silent route to `public`.
- `remove_tenant` orchestrates the full teardown: reactors stopped, executions cancelled, cache evicted, keys revoked, schema dropped — in that order.
- Request spans carry `tenant_id`, `key_id`, `role`. JSON logs and traces inherit them.
- `execution_events` has `request_id`, `runner_id`, `tenant_id` columns. Backfill where reasonable.
- The same server-side abstraction supports single-org and multi-org deployment topologies without code forks. Daemon is out of scope (single-tenant by design per CLOACI-A-0005).

**Non-Goals:**
- Cross-tenant analytics or admin overview UI.
- Tenant-scoped Prometheus labels (separate operability concern; tracked elsewhere if needed).
- Reworking authorization model beyond what's required for tenant scoping; broad role-checking work is REC-05.
- Removing the `is_admin` god-mode override.

## Source Findings (May 2026 review)

- **COR-01 (Critical)** — Multi-tenant `SET search_path` failures silently route queries to `public`.
- **EVO-04 (Critical-adjusted)** — Multi-tenant runner gap; runner stays admin-schema-bound.
- **SEC-02 (Major)** — Triggers route uses admin DB.
- **SEC-03 (Major)** — `execute_workflow` runs in admin schema.
- **SEC-05 (Major)** — Health endpoints leak cross-tenant names.
- **SEC-14 (Minor)** — Tenant delete leaves stale state.
- **SEC-17 (Minor)** — Unbounded tenant cache.
- **OPS-03 (Major)** — Request span lacks tenant/key/role.
- **OPS-12 (Major)** — Engine spans not tenant-aware.
- **OPS-16 (Minor)** — `execution_events` lacks request/runner correlation.
- **API-04 (Major)** — Auth model is dual-axis but undocumented (cross-references REC-05).
- **EVO-15 (Observation)** — Server tenant cache is a half-implementation.

## Locked decisions (2026-05-13)

- **Runner strategy: per-tenant `DefaultRunner` cache.** A new `TenantRunnerCache` mirrors the existing `TenantDatabaseCache` pattern. Each cached tenant gets a fully-constructed `DefaultRunner` with its own scheduler loop, heartbeat, and executor pool. The five inventory-seeded registries (`Task`/`Workflow`/`Trigger`/`Reactor`/`Graph`) are shared by `Arc` across all per-tenant runners (already supported by the post-T-0506 inventory→Runtime seeding path; no aliasing concern). Database-override on `WorkflowExecutor::execute_async` was rejected — too much per-call plumbing, easy to miss a path and silently fall back to admin schema (the original COR-01 footgun).
- **Tenant runner cache: LRU with configurable cap, default 256.** Operator override via `--tenant-runner-cache-size` flag. Evicts least-recently-active tenant on insertion past the cap. Eviction triggers a graceful runner shutdown (drain in-flight, stop reactors, close DB pool). Recreation on next request for the evicted tenant pays a runner-startup cost (~tens of ms).
- **No `--multi-tenant` mode flag.** Single-org deployments are "multi-tenant with one tenant." Same code path everywhere — no auto/single/multi branching. Abstraction overhead at N=1 is negligible (one cached runner) and not worth two code paths.
- **`remove_tenant` teardown order:** stop reactors → cancel executions → revoke keys → evict DB+runner cache → drop schema. Top-down: stop accepting new work first, then cancel in-flight, then close auth, then tear down caches, then DB. Each step is observable and idempotent; a retry is safe.
- **Inventory sharing** (decided, not user-facing): post-T-0506, all five registries (`Task`/`Workflow`/`Trigger`/`Reactor`/`Graph`) are inventory-seeded and held behind `Arc` on `Runtime`. Multiple per-tenant `Runtime` instances reading the same inventory is the supported path; T-0507 already exercises register/unregister on each per-package basis.
- **`execution_events` migration** (decided, mechanical): standard `ALTER TABLE ADD COLUMN ... NULL` on both Postgres and SQLite. Backfill leaves pre-migration rows NULL; new rows populate `request_id` / `runner_id` / `tenant_id` from day one. Per project convention (feedback_sqlite_migration_recreate), no DROP+CREATE on SQLite.

## Initial Sketch

Plan as one Metis initiative with phases (subject to design):

1. **Phase 1 — span enrichment** (OPS-03, OPS-12). Smallest, ships standalone value. ~30-line patch.
2. **Phase 2 — triggers + graph health route fixes** (SEC-02, SEC-05). Bounded.
3. **Phase 3 — per-tenant runner cache** (EVO-04, COR-01 partial). Largest single change.
4. **Phase 4 — tenant deletion teardown + cache eviction** (SEC-14, SEC-17).
5. **Phase 5 — COR-01 fail-closed + search_path defense-in-depth.**
6. **Phase 6 — `execution_events` correlation columns + migration** (OPS-16).

Each phase ships independently. Phases 1–2 are quick wins; Phase 3 is the main lift.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- `POST /v1/tenants/foo/workflows/bar/execute` writes execution rows into the `foo` schema, not the admin schema.
- A regression test calls `list_triggers` for tenant A and asserts that schedules created in tenant B's schema do not appear.
- `GET /v1/health/graphs` returns only graphs accessible to the caller's tenant; admin keys see all.
- Deleting a tenant via `DELETE /v1/tenants/foo` revokes all keys with `tenant_id = "foo"`, stops the tenant's reactors, cancels running executions, evicts the cached `Database`, then drops the schema.
- `SET search_path` failure surfaces as a connection-pool error to the caller; `current_schemas()` defense-in-depth check guards against silent fallback.
- Request spans carry `tenant_id`, `key_id`, `role` fields propagated to JSON logs and OTLP traces.
- `execution_events` rows carry `request_id`, `runner_id`, `tenant_id`.
- Single-org and multi-org server deployments use the same code path with deployment-config differences only.

## References

- ADR: CLOACI-A-0005 (deployment-mode trust model — defines multi-tenant abstraction as server-only)
- `review/02-correctness.md` — COR-01
- `review/03-evolvability.md` — EVO-04, EVO-15
- `review/05-api-design.md` — API-04
- `review/06-operability.md` — OPS-03, OPS-12, OPS-16
- `review/07-security.md` — SEC-02, SEC-03, SEC-05, SEC-14, SEC-17
- `review/08-cross-cutting.md` — multi-tenant cluster
- `review/10-recommendations.md` — REC-03, REC-28
- Prior initiative: CLOACI-I-0083 (Authorization Model — Tenant Isolation, completed)
- Prior task: CLOACI-T-0485 (Tenant schema isolation at DAL layer, completed)
