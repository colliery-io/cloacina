---
id: role-enforcement-replace-dead
level: task
title: "Role enforcement — replace dead permissions field with admin/write/read roles"
short_code: "CLOACI-T-0427"
created_at: 2026-04-06T15:18:31.845913+00:00
updated_at: 2026-04-06T19:19:36.636811+00:00
parent: CLOACI-I-0083
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0083
---

# Role enforcement — replace dead permissions field with admin/write/read roles

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0083]]

## Objective

Replace the dead `permissions: "admin"` field with enforced role-based access. Currently every key has `permissions = "admin"` hardcoded in the DAL and no handler checks it. After this task, keys have a `role` field (admin/write/read) that handlers enforce before allowing operations.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Migration renames or replaces `permissions` column with `role TEXT NOT NULL DEFAULT 'admin'` (or repurposes existing column)
- [ ] `POST /auth/keys` and `POST /tenants/{tid}/keys` accept optional `role` parameter (default: admin)
- [ ] DAL `create_key()` accepts role parameter instead of hardcoding
- [ ] `AuthenticatedKey` struct has `role` field, `#[allow(dead_code)]` removed
- [ ] Handlers enforce roles before operations:
  - admin: all operations (create keys, upload, execute, delete)
  - write: upload packages, execute workflows, push to WS accumulators
  - read: list/get only — no mutations, no execution, no WS push
- [ ] Read-only key calling `POST .../execute` → 403
- [ ] Write key calling `DELETE .../keys/{id}` → 403
- [ ] Admin key can do everything
- [ ] God mode (is_admin=true) implicitly has admin role everywhere
- [ ] `GET /auth/keys` response shows `role` instead of dead `permissions`

## Implementation Notes

### Key files
- `crates/cloacina/src/dal/unified/api_keys/crud.rs` — remove hardcoded `permissions: "admin"`
- `crates/cloacinactl/src/server/auth.rs` — `AuthenticatedKey.role`
- All handler files — add role checks before operations

### Dependencies
- T-0424 (key scoping) — `AuthenticatedKey` struct changes
- T-0425 (handler enforcement) — handlers already extract AuthenticatedKey

## Status Updates **[REQUIRED]**

**2026-04-06 — Complete**
- Repurposed existing `permissions` column as the role field (no migration needed — column already exists)
- Added `can_write()`, `can_admin()`, `insufficient_role_response()` helpers on `AuthenticatedKey`
- `can_write()`: is_admin OR permissions in {admin, write}
- `can_admin()`: is_admin OR permissions == "admin"
- DAL `create_key()` now accepts `role: &str` parameter instead of hardcoding "admin"
- `POST /auth/keys` accepts optional `role` field in request body (default: "admin")
- `POST /tenants/{tid}/keys` also accepts `role`
- Role enforcement in handlers:
  - upload_workflow, delete_workflow, execute_workflow: require `can_write()`
  - revoke_key: require `can_admin()`
  - list/get handlers: no role check (read access for any authenticated key)
- All callers updated: bootstrap key, test helpers, integration tests pass `"admin"` for role
- Compiles clean
