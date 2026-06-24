---
title: "Configure Local Accounts"
description: "Set up self-managed username/password login (no identity provider) — provision accounts, sign in to mint a key, and manage the session lifecycle."
weight: 26
---

# Configure Local Accounts

Local accounts are the **self-managed, no-IdP** path to a bearer key: a
tenant-admin provisions username/password accounts, and a user signs in to
**mint a short-TTL key** scoped to that tenant. Use this when you want
authenticated UI/API access without running an OIDC provider. For the trust
model behind keys and authorization, see
[Security Model]({{< ref "/service/explanation/security-model" >}}); for OIDC
instead, see [Configure OIDC Single Sign-On]({{< ref "/service/how-to/configure-oidc-sso" >}}).

Local accounts are a **server-only** feature (Postgres-backed). They have no
meaning in embedded or daemon mode.

## Prerequisites

- A running `cloacina-server` (see [Deploying the API Server]({{< ref "/service/how-to/deploying-the-api-server" >}})).
- A **tenant-admin** key for the tenant you're provisioning accounts in — either
  an `is_admin` key, or a key created with `role=admin` for that tenant. See
  [Manage API Keys]({{< ref "/service/how-to/manage-api-keys" >}}).
- The tenant must already exist.

The examples use `acme` as the tenant and `$ADMIN` as the tenant-admin bearer
key; the server is at `http://localhost:8080`.

## 1. Provision an account

A tenant-admin creates accounts under the tenant's account surface. `role` is the
account's tenant role (`read` / `write` / `admin`) and is baked into every key the
account mints.

```bash
curl -X POST http://localhost:8080/v1/tenants/acme/accounts \
  -H "Authorization: Bearer $ADMIN" \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "password": "<initial-password>", "role": "write"}'
```

Accounts are unique per `(tenant, username)` — the same `username` may exist
independently in different tenants (this is how one person belongs to several
tenants; see [Multi-tenant individuals](#multi-tenant-individuals)). Passwords
are hashed with **argon2id**; the plaintext is never stored or logged.

## 2. Sign in to mint a key

Login is **public** (the caller has no key yet). It validates the credentials and
returns a freshly minted, tenant-scoped bearer key:

```bash
curl -X POST http://localhost:8080/v1/auth/local/login \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "password": "<password>", "tenant": "acme"}'
# => { "key": "clk_…", "tenant_id": "acme", "role": "write", "expires_at": "…" }
```

Use the returned `key` as the `Authorization: Bearer` token for subsequent calls.
A wrong username or password returns `401` with an opaque error (no account
enumeration).

In the **web UI**, the connect screen's *Username & password* tab does exactly
this: it calls `/auth/local/login`, stores the minted key, and lands the user
authenticated.

## 3. Manage accounts

All account-management routes are tenant-scoped and require a tenant-admin key.

```bash
# List the tenant's accounts
curl -H "Authorization: Bearer $ADMIN" \
  http://localhost:8080/v1/tenants/acme/accounts

# Disable an account (its existing keys keep working until they expire/are revoked)
curl -X DELETE -H "Authorization: Bearer $ADMIN" \
  http://localhost:8080/v1/tenants/acme/accounts/<account_id>

# Reset an account's password
curl -X POST http://localhost:8080/v1/tenants/acme/accounts/<account_id>/password \
  -H "Authorization: Bearer $ADMIN" -H "Content-Type: application/json" \
  -d '{"password": "<new-password>"}'
```

A disabled account can no longer log in; `refresh` (below) also fails for it, so
its sessions die at their next refresh. The UI exposes the same operations under
**Accounts** (visible to tenant-admins).

## 4. Keep a session alive (refresh) and end it (logout)

Minted keys carry a short TTL. Re-mint before it lapses rather than storing a
long-lived credential:

```bash
# Re-check the account is still active, mint a fresh key, revoke the old one
curl -X POST -H "Authorization: Bearer $KEY" http://localhost:8080/v1/auth/refresh

# Revoke the current key + clear the session
curl -X POST -H "Authorization: Bearer $KEY" http://localhost:8080/v1/auth/logout
```

The web UI runs the refresh loop automatically (silently, before the TTL lapses),
so a signed-in operator's session survives; a pasted long-lived API key is left
untouched.

## Multi-tenant individuals

The tenant is the isolation boundary and a key is always scoped to one tenant. A
person who works in several tenants has a **separate account in each** and holds
one key per tenant; the UI's tenant switcher flips between them. For example,
`alice` can be an `admin` in `acme` and `read` in `public` as two independent
accounts — each key is hard-isolated to its tenant (the other tenant's data is a
`403`). See the [Security Model]({{< ref "/service/explanation/security-model" >}}#multi-tenant-individuals)
for why the subject stays single-tenant.

## See also

- [Configure OIDC Single Sign-On]({{< ref "/service/how-to/configure-oidc-sso" >}}) — the IdP-backed alternative.
- [Manage API Keys]({{< ref "/service/how-to/manage-api-keys" >}}) — directly-created, long-lived keys.
- [Security Model]({{< ref "/service/explanation/security-model" >}}) — the ABAC authorization layer and session lifecycle.
