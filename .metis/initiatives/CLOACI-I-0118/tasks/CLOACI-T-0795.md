---
id: local-accounts-credential-store
level: task
title: "Local accounts — credential store + argon2 hashing + DAL (local_accounts table)"
short_code: "CLOACI-T-0795"
created_at: 2026-06-24T01:26:36.339896+00:00
updated_at: 2026-06-24T01:26:36.339896+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Local accounts — credential store + argon2 hashing + DAL (local_accounts table)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Add the minimal local-credential entity that backs self-managed login. Create a `local_accounts` table (`id, username, password_hash, tenant_id, role, status (active|disabled), created_at`) via an ADDITIVE migration (no DROP+CREATE). Add argon2id password hashing (add the `argon2` crate) with verification helpers. Add a DAL for create / get-by-username (active only, for auth) / list-by-tenant / disable / set-password. Password hashes are NEVER logged.

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

*To be added during implementation*
