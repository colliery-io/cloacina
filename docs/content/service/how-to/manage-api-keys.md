---
title: "Manage API Keys"
description: "Create, list, and revoke API keys for the Cloacina server with cloacinactl or the REST API."
weight: 25
---

# Manage API Keys

API keys authenticate every call to the `cloacina-server` HTTP API. This guide
covers creating, listing, and revoking them with `cloacinactl` (and the equivalent
REST endpoints). For the trust model behind keys, roles, and the bootstrap key, see
[Security Model]({{< ref "/service/explanation/security-model" >}}).

## Prerequisites

- `cloacinactl` installed and a reachable server.
- An **admin** key to authenticate with — on first startup the server writes a
  bootstrap admin key once to `~/.cloacina/bootstrap-key` (see
  [Deploy a Server]({{< ref "/service/tutorials/01-deploy-a-server" >}})).

Either pass `--server`/`--api-key` on each command or save a profile (see
[Use CLI Profiles]({{< ref "/service/how-to/use-cli-profiles" >}})).

## Create a key

```bash
cloacinactl key create ci-bot --role write \
  --server http://127.0.0.1:8080 --api-key "$ADMIN_KEY"
```

`--role` is one of `read`, `write`, or `admin` (default `read`). The response
prints the secret **once** — store it immediately; it cannot be retrieved again.

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
[HTTP API Reference]({{< ref "/reference/http-api" >}})):

| Action | Method + path |
|--------|---------------|
| Create (public tenant) | `POST /v1/auth/keys` (body: `{ "name": ..., "role": ... }`; platform-admin only, always scoped to `public`) |
| Create (named tenant) | `POST /v1/tenants/{tenant_id}/keys` (same body; tenant-admin or platform-admin) |
| List | `GET /v1/auth/keys` (platform-admin) or `GET /v1/tenants/{tenant_id}/keys` (tenant-admin) |
| Revoke | `DELETE /v1/auth/keys/{key_id}` or `DELETE /v1/tenants/{tenant_id}/keys/{key_id}` |

## See also

- [Security Model]({{< ref "/service/explanation/security-model" >}}) — roles, `is_admin`, bootstrap-key invariants.
- [Configure a Multi-Tenant Deployment]({{< ref "/service/how-to/configure-multi-tenant-deployment" >}}) — per-tenant keys and credentials.
