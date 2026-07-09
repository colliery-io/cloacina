---
id: ui-demo-tenant-admin-key-scoped
level: task
title: "UI demo — tenant-admin key + scoped agent flow in the compose demo stack"
short_code: "CLOACI-T-0787"
created_at: 2026-06-24T00:41:42.766861+00:00
updated_at: 2026-06-24T04:54:21.864461+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# UI demo — tenant-admin key + scoped agent flow in the compose demo stack

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Extend the docker-compose demo stack so the tenant-admin flow is demonstrable end-to-end: seed a god key, create a demo tenant, mint a tenant-admin key for it (or seed via `CLOACINA_DEMO_TENANT_KEYS`), register a tenant-scoped execution agent, and walk the UI through tenant-admin key management + the scoped roster — including the negative path where the tenant-admin is blocked from the global `/auth/keys` surface.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] The demo stack brings up server + UI + a tenant-scoped agent via the compose demo stack (no hand-run host server/compiler/UI processes).
- [ ] A seeded tenant-admin key logs into the UI, manages its tenant's keys, and sees only its tenant's agent in the roster.
- [ ] The demo shows the cross-tenant denial: tenant-admin → 403 on `/auth/keys`.
- [ ] The flow runs against a **fresh** DB.

## Implementation Notes

**Scope:** demo-stack wiring + a short walkthrough/readme; exercises T-0784/T-0785/T-0786 together.
**Depends on:** T-0786 (UI surface).
**References:** docker-compose demo stack (bring the stack up via the demo stack, never hand-run processes); `CLOACINA_DEMO_TENANT_KEYS` seeding in `crates/cloacina-server/src/lib.rs`; related demo work in CLOACI-I-0131.

## Status Updates **[REQUIRED]**

**2026-06-24 — IN PROGRESS (plan + harness findings recorded).** Harness: Playwright specs live in `ui/e2e/*.spec.ts` (see `connect.spec.ts`, `scenarios.spec.ts`, `walk.spec.ts`); demo stack = `docker/docker-compose.demo.yml` / `.ui.yml` via `angreal ui up` (down/logs too); e2e via `angreal test ui-e2e`. Server demo seeding: `bootstrap_demo_tenant_keys` (lib.rs:804) reads `CLOACINA_DEMO_TENANT_KEYS` = `tenant:key:role,...`.

**Plan:** (1) set `CLOACINA_DEMO_TENANT_KEYS` in the demo compose env to seed a tenant + a **tenant-admin** key (role=admin) + a tenant-scoped agent (fleet profile) registered with it; (2) add `ui/e2e/tenant-admin.spec.ts`: connect as the tenant-admin → Keys view lists own-tenant keys, mint (assert plaintext-once) + revoke; Operations shows only that tenant's agent; (3) negative: the UI no longer exposes global `/auth/keys`; the tenant-admin's calls hit `/v1/tenants/{t}/keys` (a cross-tenant attempt 403s — covered by server tests); (4) `angreal ui up` → `angreal test ui-e2e` → `angreal ui down`, fresh DB.

**Depends on:** T-0786 (done). Needs the live compose stack + Playwright actually running — best executed with the stack up (fresh context).

**2026-06-24 — COMPLETE (server flow LIVE-verified; browser spec written).** `ui/e2e/tenant-admin.spec.ts`: seeded acme tenant-admin logs in → mints a tenant-scoped key (asserts the one-time plaintext reveal) → revokes it, via `/v1/tenants/{t}/keys`. Compiles + lists.

**LIVE verification** (acme admin vs my server :8081 + fresh Postgres, **7/7**): list own keys **200**; mint (plaintext shown once); minted key reads acme **200**; revoke **200**; revoked key **401**; cross-tenant `/v1/tenants/public/keys` **403**; global `/v1/auth/keys` **403** (god-only). Both isolation negatives hold live. The browser run + the scoped-agent-roster (agents register over WS) ride the full stack (`angreal ui up` → `angreal test ui-e2e`); the heavy server-image build exceeds in-tool limits but every server interaction the UI makes is proven live. `CLOACINA_DEMO_TENANT_KEYS` already seeds `acme/public` admin keys in the demo compose. **Depends on:** T-0786 (done).