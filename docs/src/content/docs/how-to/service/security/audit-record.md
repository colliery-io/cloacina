---
title: "Read the Audit Record"
description: "Who did what, when, in which tenant: the audit_events table, its actions and fields, and how to read it"
sidebar:
  order: 30
  label: "Audit Record"
---

The server keeps a durable record of control-plane actions: who did what, when,
in which tenant. Each action writes one row to the `audit_events` table after
it succeeds. The structured `tracing` audit events (package signing, tenant
teardown steps) are still emitted; the table is next to them, not in place of
them.

## Read it

With `cloacinactl` (the profile's key must be a tenant admin):

```bash
# The tenant of the profile, newest first
cloacinactl audit list

# Filters
cloacinactl audit list --action package.upload
cloacinactl audit list --actor robin-laptop --since 2026-11-01T00:00:00Z
cloacinactl audit list --limit 100 --cursor 4211

# Every tenant and platform actions (platform admin key)
cloacinactl audit list --all-tenants
```

Over HTTP:

```bash
curl -H "Authorization: Bearer $KEY" \
  "https://cloacina.example/v1/tenants/acme/audit?action=workflow.run&limit=20"
```

See [Audit](../../../../reference/http-api/#audit) in the HTTP API reference for
the query fields and the response.

| Who | Route | Sees |
|---|---|---|
| Tenant admin | `GET /v1/tenants/{tenant_id}/audit` | The rows of its own tenant |
| Platform admin | `GET /v1/audit` | Every row; `?tenant=` narrows to one tenant |

A `read` or `write` key cannot read the audit record.

## Fields

| Field | Meaning |
|---|---|
| `id` | Row id. Larger is newer. Also the paging cursor. |
| `created_at` | When the row was written (RFC 3339, UTC). |
| `tenant_id` | The tenant the action applied to. `null` for a platform action with no tenant (a global key revoke, a denied platform route). |
| `actor_key_id` | UUID of the API key that made the request; `server` on a row the server writes itself (a refused secret). |
| `actor_key_name` | Name of that key; `server` on a row the server writes itself. |
| `actor_identity` | Who the key was minted for at login: `oidc:<issuer>:<sub>` or `local:<account_id>`. `null` for a manually created key. |
| `action` | One of the fixed strings below. |
| `target_kind` | What the action applied to: `package`, `workflow`, `trigger`, `instance`, `key`, `tenant`, `secret`, `trusted_key`, `fleet`, `limit`, `pool`, `account`. |
| `target` | Name or id of the target. `null` on a `denied` row. |
| `outcome` | `ok`, or `denied` when authorization refused the request. |
| `detail` | A small JSON object with facts of the action. |

The record never holds a secret value, an API key, a password, a request body
or an execution context. A secret action records the secret's name only.

## Actions

| Action | Route | `target` | `detail` |
|---|---|---|---|
| `package.upload` | `POST /v1/tenants/{t}/workflows` | package id | `package_name`, `version`, `content_hash` (SHA-256), `size_bytes` |
| `package.delete` | `DELETE /v1/tenants/{t}/workflows/{name}/{version}` | package name | `version` |
| `package.activate` | `POST /v1/tenants/{t}/workflows/{name}/versions/{version}/activate` | package name | `version`, `package_id` |
| `workflow.pause` | `POST /v1/tenants/{t}/workflows/{name}/pause` | name as given | `package_id` |
| `workflow.resume` | `POST /v1/tenants/{t}/workflows/{name}/resume` | name as given | `package_id` |
| `workflow.run` | `POST /v1/tenants/{t}/workflows/{name}/execute` | workflow name | `execution_id` |
| `trigger.pause` | `POST /v1/tenants/{t}/triggers/{name}/pause` | name as given | `schedule_id` |
| `trigger.resume` | `POST /v1/tenants/{t}/triggers/{name}/resume` | name as given | `schedule_id` |
| `trigger.fire` | `POST /v1/tenants/{t}/triggers/{name}/fire` | trigger name | `execution_ids` |
| `instance.create` | `POST /v1/tenants/{t}/workflows/{name}/instances` | instance name | `workflow`, `schedule_id` |
| `instance.delete` | `DELETE /v1/tenants/{t}/workflows/{name}/instances/{instance}` | instance name | `workflow` |
| `backfill.create` | `POST /v1/tenants/{t}/workflows/{name}/backfills` | backfill id | `workflow`, `start`, `end`, `intervals` |
| `backfill.cancel` | `POST /v1/tenants/{t}/workflows/{name}/backfills/{id}/cancel` | backfill id | `workflow`, `was_enabled`, `remaining` |
| `backfill.retry` | `POST /v1/tenants/{t}/workflows/{name}/backfills/{id}/retry` | id of the backfill that is retried | `workflow`, `new_backfill_id`, `intervals` |
| `key.create` | `POST /v1/auth/keys`, `POST /v1/tenants/{t}/keys` | key id | `name`, `role` |
| `key.revoke` | `DELETE /v1/auth/keys/{id}`, `DELETE /v1/tenants/{t}/keys/{id}` | key id | — |
| `tenant.create` | `POST /v1/tenants` | tenant name | — |
| `tenant.delete` | `DELETE /v1/tenants/{name}` | tenant name | `revoked_keys` |
| `tenant.removed` | written by the delete itself, in the transaction that drops the schema | tenant name | — |
| `secret.create` | `POST /v1/tenants/{t}/secrets` | secret name | — |
| `secret.rotate` | `PUT /v1/tenants/{t}/secrets/{name}` | secret name | — |
| `secret.delete` | `DELETE /v1/tenants/{t}/secrets/{name}` | secret name | — |
| `trusted_key.add` | `POST /v1/tenants/{t}/trusted-keys` | trusted key id | `fingerprint`, `name` |
| `trusted_key.revoke` | `DELETE /v1/tenants/{t}/trusted-keys/{key_id}` | trusted key id | — |
| `fleet.provision` | `POST /v1/tenants/{t}/fleet/provision` | tenant name | `desired_count` |
| `fleet.deprovision` | `POST /v1/tenants/{t}/fleet/deprovision` | tenant name | `desired_count` |
| `limit.set` | `POST /v1/tenants/{t}/limits` | tenant name | `max_agents` |
| `limit.clear` | `DELETE /v1/tenants/{t}/limits` | tenant name | — |
| `pool.set` | `PUT /v1/tenants/{t}/pools/{pool}` | pool name | `settings` (the names of the settings given, not their values) |
| `pool.delete` | `DELETE /v1/tenants/{t}/pools/{pool}` | pool name | — |
| `account.create` | `POST /v1/tenants/{t}/accounts` | account id | `username`, `role` |
| `account.disable` | `DELETE /v1/tenants/{t}/accounts/{id}` | account id | — |
| `account.password_reset` | `POST /v1/tenants/{t}/accounts/{id}/password` | account id | — |

A `trigger.fire` row is written when at least one workflow started. A
`package.delete` row is written for each delete that returns `200`, and that
route returns `200` also when no such package existed. An upload of content
that is already registered returns the existing package id and writes a new
`package.upload` row.

### Refused secrets

A task reads only the secrets its package declares. When the server refuses a
secret to a task, it writes a row with no route behind it:

| Action | `actor_key_id`, `actor_key_name` | `target_kind`, `target` | `outcome` | `detail` |
|---|---|---|---|---|
| `secret.refused` | `server` | `secret`, the secret name | `denied` | `package`, `version`, `task`, `workflow_execution_id`, `task_execution_id`, `path` (`fleet`: not sent to an agent; `in_process`: refused in the server process), `reason` |

The row names the secret, never its value. A name a task asks for on an agent,
and that the work packet does not carry, is refused by the agent; that refusal
is in the task's error and the agent's log, not in the audit record.

### Packages refused after a revoke

With `--require-signatures`, a package whose only valid signature was by a
trusted key that is then revoked is unloaded on every replica, and that version
is refused (see [Package signing](../package-signing/#manage-a-tenants-trusted-keys-on-a-server)).
One row records it, written by the replica that refused the version:

| Action | `actor_key_id`, `actor_key_name` | `target_kind`, `target` | `outcome` | `detail` |
|---|---|---|---|---|
| `package.signer_revoked` | `server` | `package`, the package id | `denied` | `package`, `version`, `fingerprint` (of the revoked key), `code` (`revoked_signer` or `untrusted_signer`), `cancelled_runs` |

### Denied requests

When authorization refuses a request on one of the routes above (wrong tenant,
role too low, platform admin required), the server writes a row with the same
`action`, `outcome` `denied`, no `target`, and a `detail` with the `reason`
(`tenant_access_denied`, `insufficient_role`, `platform_admin_required`) and
the URL `path`. `tenant_id` is the tenant named in the path, so a tenant admin
sees attempts on its tenant.

Not recorded: a denied read, a request with no valid key (`401`), and a request
that passed authorization and then failed (`404`, `409`, `500`).

## Who uploaded a package

`GET /v1/tenants/{t}/workflows` and `GET /v1/tenants/{t}/workflows/{name}`
return `uploaded_by`: the `actor_identity` of the first `package.upload` row of
that package, or the key name when the key has no identity. It is read from the
audit record, not stored on the package. It is `null` for a package uploaded
before the audit record existed.

## Where the rows live

`audit_events` is in the admin schema (`public`) and has a tenant column, like
`api_keys`. It is not in each tenant's schema, for three reasons:

- Deleting a tenant drops its schema. The tenant's audit rows, and the row of
  the delete itself, must outlive it.
- Platform actions (a tenant create, a global key) have no tenant schema.
- The platform admin reads one table, not one per tenant.

So the rows of a deleted tenant stay readable by the platform admin with
`GET /v1/audit?tenant=<name>`.

A tenant is its name, so a tenant made again with a removed name would find
the old rows under its name. The delete writes one row `tenant.removed`
(`tenant_id` the name, actor `tenant-removal`) in the transaction that drops the
schema, and `GET /v1/tenants/{t}/audit` and `uploaded_by` read only the rows
after the newest `tenant.removed` of the tenant. The `tenant.delete` row of the
route has no tenant (a platform row), so it is in `GET /v1/audit` only.
(CLOACINA-T-1095)

## Failed writes

An audit row is written after the action. If the write fails, the request still
succeeds: the server logs a warning and counts it in
`cloacina_audit_write_errors_total`. Alert on any increase of that counter:
each one is an action with no audit row.

## Retention

Audit rows are not deleted. The
[run history retention job](../../production-deployment/#run-history-retention)
does not know about `audit_events` yet: `--run-history-retention-days` has no
effect on it, and rows are kept until that job learns about them. Plan disk for
it: one row per action above, a few hundred bytes each.
