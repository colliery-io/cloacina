---
title: "Manage API Keys"
description: "Create, list, and revoke API keys for the Cloacina server with cloacinactl or the REST API."
sidebar:
  order: 25
---

API keys authenticate every call to the `cloacina-server` HTTP API. This guide
covers creating, listing, and revoking them with `cloacinactl` (and the equivalent
REST endpoints). For the trust model behind keys, roles, and the bootstrap key, see
[Security Model](../../../explanation/service/security-model/).

## Prerequisites

- `cloacinactl` installed and a reachable server.
- An **admin** key to authenticate with — on first startup the server writes a
  bootstrap admin key once to `~/.cloacina/bootstrap-key` (see
  [Deploy a Server](../../../tutorials/service/01-deploy-a-server/)).

Either pass `--server`/`--api-key` on each command or save a profile (see
[Use CLI Profiles](../use-cli-profiles/)).

## Create a key

```bash
cloacinactl key create ci-bot --role write \
  --server http://127.0.0.1:8080 --api-key "$ADMIN_KEY"
```

`--role` is one of `read`, `write`, `admin` or `agent` (the CLI's default is
`read`). The response prints the secret **once** — store it immediately; it
cannot be retrieved again.

| Role | What the key can do |
|------|---------------------|
| `read` | List and inspect its tenant. |
| `write` | `read`, plus upload packages, run workflows, operate triggers, fire and pause reactors, inject into accumulators. |
| `admin` | `write`, plus the keys, local accounts, secrets, audit record and fleet size of its tenant. |
| `agent` | Act as an execution agent, and nothing else. Give this role to the key of an agent you start yourself. |

The API needs the role in the request: `POST /v1/auth/keys` and
`POST /v1/tenants/{tenant_id}/keys` answer `400` when the body has no `role`.
The full list of permissions by role is in the
[Security Model](../../../explanation/service/security-model/#authorization-permissions-roles-and-the-route-table).

**Tenant scope:** `key create` calls the global `POST /v1/auth/keys`
endpoint, which is restricted to platform-admin (god-mode) keys and
always mints keys scoped to the built-in **`public`** tenant. To mint a
key for a **named** tenant, use the tenant-scoped endpoint (available
to that tenant's admins as well as the platform admin) — the plaintext
is in the `key` field of the response:

```bash
curl -s -X POST http://127.0.0.1:8080/v1/tenants/acme/keys \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{"name": "acme-bot", "role": "write"}' | jq -r '.key'
```

## Keys that expire

A key can have an expiry. Give `--expires-at` (or `"expires_at"` in the
body) an RFC 3339 time in the future:

```bash
cloacinactl key create ci-bot --role write --expires-at 2027-01-31T00:00:00Z \
  --server http://127.0.0.1:8080 --api-key "$ADMIN_KEY"

curl -s -X POST http://127.0.0.1:8080/v1/tenants/acme/keys \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{"name": "acme-bot", "role": "write", "expires_at": "2027-01-31T00:00:00Z"}'
```

The answer and the key list show `expires_at`. A key created with no
`expires_at` does not expire. A time in the past, or a value that is not an
RFC 3339 time, is refused with `400` `invalid_expires_at`. After the expiry
every route answers `401` with the code `key_expired`.

**Agents.** The Kubernetes actuator gives its agents keys that expire after
30 days and replaces them when 10 days are left; you do nothing. Docker
actuator keys do not expire. An agent you start yourself logs its key's
expiry when it registers (`the agent's API key expires`), and a warning when
less than 10 days are left. Before the expiry, create a new key of role
`agent`, restart the agent with it, and revoke the old key.

## List keys

```bash
cloacinactl key list --server http://127.0.0.1:8080 --api-key "$ADMIN_KEY"
```

Add `--output json` for scripting. Secrets are never returned — only key ids,
names, roles, and metadata.

## Revoke a key

```bash
cloacinactl key revoke <key-id> --server http://127.0.0.1:8080 --api-key "$ADMIN_KEY"
```

Revocation is immediate. The CLI prompts for confirmation; pass `--force` to skip
it in automation.

## REST equivalents

The CLI is a thin wrapper over these endpoints (full details in the
[HTTP API Reference](../../../reference/http-api/)):

| Action | Method + path |
|--------|---------------|
| Create (public tenant) | `POST /v1/auth/keys` (body: `{ "name": ..., "role": ..., "expires_at": ... }`, `expires_at` optional; platform-admin only, always scoped to `public`) |
| Create (named tenant) | `POST /v1/tenants/{tenant_id}/keys` (same body; tenant-admin or platform-admin) |
| List | `GET /v1/auth/keys` (platform-admin) or `GET /v1/tenants/{tenant_id}/keys` (tenant-admin) |
| Revoke | `DELETE /v1/auth/keys/{key_id}` or `DELETE /v1/tenants/{tenant_id}/keys/{key_id}` |

## See also

- [Security Model](../../../explanation/service/security-model/) — roles, `is_admin`, bootstrap-key invariants.
- [Configure a Multi-Tenant Deployment](../configure-multi-tenant-deployment/) — per-tenant keys and credentials.
