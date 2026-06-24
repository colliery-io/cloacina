---
id: local-accounts-credential-store
level: task
title: "Local accounts — credential store + argon2 hashing + DAL (local_accounts table)"
short_code: "CLOACI-T-0795"
created_at: 2026-06-24T01:26:36.339896+00:00
updated_at: 2026-06-24T03:32:52.825872+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Local accounts — credential store + argon2 hashing + DAL (local_accounts table)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Add the minimal local-credential entity that backs self-managed login. Create a `local_accounts` table (`id, username, password_hash, tenant_id, role, status (active|disabled), created_at`) via an ADDITIVE migration (no DROP+CREATE). Add argon2id password hashing (add the `argon2` crate) with verification helpers. Add a DAL for create / get-by-username (active only, for auth) / list-by-tenant / disable / set-password. Password hashes are NEVER logged.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] `local_accounts` table created via an additive migration (no DROP+CREATE).
- [ ] argon2id hash + verify helpers; password hashes never logged.
- [ ] DAL: create, get_by_username (active), list_for_tenant, disable, set_password.
- [ ] Unit tests for hash round-trip and DAL operations.

## Implementation Notes

**Scope:** storage + hashing + DAL only. No endpoints, no login flow.
**Depends on:** none (foundational for the strand).
**References:** CLOACI-I-0118 → "Local accounts strand"; additive-migration rule (no DROP+CREATE).

## Status Updates **[REQUIRED]**

**2026-06-24 — COMPLETE.** Postgres-only. Added `argon2 = 0.5`. Migration `034_create_local_accounts` (CREATE TABLE `local_accounts { id, username, password_hash, tenant_id, role, status, created_at }`, unique `(tenant_id, username)`; additive). schema.rs table!. New `dal/unified/local_accounts/` via `dal.local_accounts()` (`LocalAccountDAL`): `hash_password`/`verify_password` (argon2id, random salt, PHC; plaintext never stored); `create`, `authenticate` (→ `LoginOutcome::{Ok, Denied}`; same opaque Denied for unknown/wrong/disabled — no enumeration), `list_for_tenant`, `set_status` (disable), `set_password` (admin reset, OQ-12). The account row IS the tenant/role mapping → local login bypasses the OIDC allowlist. `angreal check` clean; **4/4 hashing unit tests** (roundtrip / wrong-pw / malformed / distinct-salts). DB CRUD + auth under the postgres lane. **Unblocks T-0796** (local login → `ResolvedPrincipal`) and thereby **T-0794** (local refresh = re-check `status==active`). OQ-13 brute-force throttle lands with the T-0796 login endpoint.