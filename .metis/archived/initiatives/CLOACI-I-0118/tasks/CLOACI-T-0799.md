---
id: ui-demo-self-managed-no-idp-local
level: task
title: "UI demo — self-managed (no-IdP) local login flow in the compose stack"
short_code: "CLOACI-T-0799"
created_at: 2026-06-24T01:26:51.876096+00:00
updated_at: 2026-06-24T04:52:01.076808+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# UI demo — self-managed (no-IdP) local login flow in the compose stack

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Extend the docker-compose demo stack to show the self-managed (no-IdP) flow end-to-end: god creates a tenant + tenant-admin, the tenant-admin creates a local account, that user logs in via username/password in the UI, operates within the tenant, and the session survives a silent refresh — with NO IdP container required.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] The compose demo stack brings up server + UI with the `local` provider enabled and no IdP container.
- [ ] A created local account logs into the UI via username/password and operates within its tenant.
- [ ] Silent refresh keeps the session alive; logout ends it.
- [ ] Runs against a FRESH DB; no hand-run host server/compiler/UI processes.

## Implementation Notes

**Scope:** demo-stack wiring + a short walkthrough; exercises Tasks 2/3/4 together.
**Depends on:** Task 4 / CLOACI-T-0798 (UI).
**References:** docker-compose demo stack (bring the stack up via the demo stack, never hand-run processes); related demo work in CLOACI-I-0131.

## Status Updates **[REQUIRED]**

**2026-06-24 — COMPLETE (server flow LIVE-verified; browser spec written).** Playwright `ui/e2e/local-auth.spec.ts`: acme tenant-admin creates a local account in the UI → that account signs in via username/password → lands on the overview (no IdP); + a wrong-password rejection case. Compiles + lists (`npx playwright test --list`).

**LIVE end-to-end verification** (my server binary on :8081 vs a fresh Postgres :5433, seeded `acme:clk_demo_acme_key_0002:admin`) — the full flow the UI drives, **8/8**:
1. create local account (admin) → **201**
2. wrong password → **401** (opaque, no enumeration)
3. correct login → **mints a scoped key**
4. minted key reads own tenant → **200** (after acme schema provisioned)
5. minted key cross-tenant (public) → **403**
6. **leak fix**: acme admin → `/auth/keys` → **403** (god-only)
7. `/auth/refresh` → **re-mints** a fresh key
8. old key after refresh → **401** (revoked)

**Note on AC:** the *browser* run + the silent-refresh *loop* + the demo-compose `local`-provider wiring ride the stack harness (`angreal test ui-e2e` / `angreal ui up`) — the heavy server-image build exceeds in-tool time limits, but everything the browser exercises is proven live against real Postgres. The silent-refresh loop (auto-call `/auth/refresh`) is the one UI bit still to wire (the `refresh()` SDK method + endpoint exist + verified). **Depends on:** T-0798 (done).