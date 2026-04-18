---
id: t7-tenant-key-trigger-verbs
level: task
title: "T7: tenant + key + trigger verbs"
short_code: "CLOACI-T-0516"
parent: CLOACI-I-0098
blocked_by: [CLOACI-T-0513]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0098
---

# T7: tenant + key + trigger verbs

## Parent Initiative

CLOACI-I-0098 — cloacinactl CLI redesign

## Objective

Implement the admin and observation nouns: `tenant` (admin-only), `key` (admin + tenant), `trigger` (read-only).

## Acceptance Criteria

- [ ] `tenant create <NAME> [--description <TEXT>]` — POST `/v1/tenants`. Rejected (exit 4) for tenant-scoped keys.
- [ ] `tenant list` — GET `/v1/tenants`. Admin-only.
- [ ] `tenant delete <NAME> [--force]` — DELETE `/v1/tenants/{name}`. Confirmation without `--force`.
- [ ] `key create [--role admin|write|read] [--ttl <DUR>] [--description <TEXT>]` — POST `/v1/keys`. Tenant-scoped unless caller uses an admin key and `--role admin` is explicit. Secret printed exactly once; clear one-time warning in human output.
- [ ] `key list` — GET `/v1/keys`. Admin sees all; tenant key sees own-tenant keys. Columns: ID, ROLE, TENANT, TTL, CREATED.
- [ ] `key revoke <ID>` — DELETE `/v1/keys/{id}`. Confirmation without `--force`.
- [ ] `trigger list` — GET `/v1/triggers`. Columns: NAME, WORKFLOW, SCHEDULE, LAST_FIRE, ENABLED.
- [ ] `trigger inspect <NAME>` — GET `/v1/triggers/{name}`. Full definition, recent fire history.
- [ ] Tenant-resolution rule enforced: admin key → `--tenant` required for tenant-scoped ops; tenant key → implicit.
- [ ] Integration tests for admin (multi-tenant) and tenant-scoped (single-tenant) happy paths.

## Implementation Notes

### `key create` secret surfacing

Server returns the key value once. Print with a one-time-only warning:

```
created key: 7e9a-...-f3
ID:          abc123
role:        write
tenant:      acme
NOTE: this is the only time the secret will be shown. Save it now.
```

In `-o json`, the secret is in `secret` unredacted for piping.

### Admin vs tenant-scoped behavior

T4's `whoami()` cache determines the path. Helper: `require_admin_key()` on `CliClient` — returns `Err(CliError::Auth(...))` with exit 4 if the key isn't admin.

### Destructive ops

Same `confirm(...)` helper from T5 for `tenant delete` and `key revoke`.

## Status Updates

*To be added during implementation*
