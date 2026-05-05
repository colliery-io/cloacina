---
title: "Configure a Multi-Tenant Deployment"
description: "Set up cloacina-server with per-tenant schema isolation, tenant-scoped API keys, and the operational caveats you need to know."
weight: 26
---

# How to Configure a Multi-Tenant Deployment

This guide walks through provisioning a multi-tenant `cloacina-server`
deployment: bootstrap key handling, tenant creation, scoped API keys,
and the known isolation caveats you need to design around.

> **Prerequisites:**
> - `cloacina-server` running with a PostgreSQL backend. SQLite is
>   single-tenant only; multi-tenancy requires Postgres schemas.
> - PostgreSQL 14+ accessible from the server.
> - An admin API key (the bootstrap key from first startup, or any
>   `is_admin=true` key).

## Mental Model

For the architectural design (per-schema isolation, the
`TenantDatabaseCache`, the role/scope model, the rationale behind
each choice), see [Multi-Tenancy Architecture]({{< ref "/platform/explanation/multi-tenancy" >}}).
This guide focuses on the operational recipe.

## Step 1: Capture the Bootstrap Key

On first startup, `cloacina-server` writes the bootstrap admin key to
`~/.cloacina/bootstrap-key` with mode `0600`. **This is the only time
the plaintext is surfaced.**

```bash
# Start the server (first time)
cloacinactl server start \
    --database-url 'postgres://cloacina:secret@localhost/cloacina' \
    --bind 127.0.0.1:8080

# In another terminal, capture the key
ADMIN_KEY=$(cat ~/.cloacina/bootstrap-key)
chmod 600 ~/.cloacina/bootstrap-key   # already 0600, but verify
```

Alternatively, supply your own bootstrap key via
`--bootstrap-key clk_yourkey...` or the `CLOACINA_BOOTSTRAP_KEY`
environment variable on first startup. On subsequent starts the
bootstrap path is skipped if any keys exist.

> **Once captured, treat the key like a root password.** Store it in
> your secret manager. There is no way to retrieve it again.

## Step 2: Create Tenants

Each tenant gets a Postgres schema, a database user, permissions, and
fresh migrations.

```bash
cloacinactl tenant create acme \
    --server http://127.0.0.1:8080 \
    --api-key "$ADMIN_KEY"

cloacinactl tenant create globex \
    --server http://127.0.0.1:8080 \
    --api-key "$ADMIN_KEY"

cloacinactl tenant list \
    --server http://127.0.0.1:8080 \
    --api-key "$ADMIN_KEY"
# acme
# globex
```

The `tenant create` HTTP response includes the schema name and
username but **not the password** (per SEC-08 / T-0557 Bug 2 fix).
The password is set during provisioning; if you need it (e.g., for
direct DB tooling), capture it via the database admin layer at
provisioning time, not via this endpoint.

## Step 3: Create Tenant-Scoped API Keys

Tenant-scoped keys (recommended for application clients) can only
access their assigned tenant. Only `is_admin` keys can create them.

```bash
# Create a write-role key for acme's CI/CD
cloacinactl key create acme-ci \
    --role write \
    --tenant acme \
    --server http://127.0.0.1:8080 \
    --api-key "$ADMIN_KEY"
# clk_xxx... (shown exactly once — capture it now)

# Create a read-role key for acme's monitoring dashboards
cloacinactl key create acme-monitor \
    --role read \
    --tenant acme \
    --server http://127.0.0.1:8080 \
    --api-key "$ADMIN_KEY"
```

Roles:
- `read` — list/inspect workflows, executions, triggers; no writes.
- `write` — execute workflows, upload packages, manage tenant
  resources.
- `admin` — tenant-admin: can create/revoke/list keys within the
  tenant. **Distinct from `is_admin`** which is god-mode.

## Step 4: Configure Client Profiles

Each tenant client gets its own profile in `~/.cloacina/config.toml`:

```toml
default_profile = "acme-prod"

[profiles.acme-prod]
server = "https://cloacina.example.com"
api_key = "env:ACME_PROD_KEY"

[profiles.globex-prod]
server = "https://cloacina.example.com"
api_key = "file:/etc/cloacina/globex-prod.key"

[profiles.admin]
server = "https://cloacina.example.com"
api_key = "env:CLOACINA_ADMIN_KEY"
```

Clients then run with the appropriate profile:

```bash
cloacinactl --profile globex-prod workflow run nightly-etl

# Or override per-command:
cloacinactl --server https://cloacina.example.com \
            --api-key env:ONE_OFF_KEY \
            --tenant acme \
            workflow list
```

Profile resolution precedence: explicit `--server` / `--api-key`
flags > named profile > `default_profile`. See [CLI Reference]({{< ref "/platform/reference/cli" >}}#profile-resolution).

## Step 5: Per-Tenant Package Deployments

Packages are scoped to the tenant they're uploaded under:

```bash
# Acme's workflows go into Acme's schema
cloacinactl --profile acme-prod \
    package upload acme-etl-1.2.0.cloacina

# Globex's workflows are completely separate
cloacinactl --profile globex-prod \
    package upload globex-billing-3.0.0.cloacina
```

Tenant-scoped keys can only `package upload` to their own tenant.
The reconciler runs per-tenant, so package loads/unloads are
isolated.

## Operational Caveats You MUST Know

Build your deployment around these. The full enumeration with
implementation details lives in [HTTP API Reference → Operational
Caveats]({{< ref "/platform/reference/http-api" >}}#operational-caveats);
the deployment-relevant summary follows.

### 1. Workflow execution scheduling is NOT tenant-scoped

`POST /v1/tenants/{id}/workflows/{name}/execute` runs through a
single global `DefaultRunner`. Executions land in the runner's
schema (typically `public`), not the tenant's. In multi-tenant
deployments this is a real isolation gap.

**Mitigations:**
- Run a separate `cloacina-server` per tenant if compliance
  requires strict isolation. Each server gets its own database
  (or schema for the runner's home) and its own runner.
- For low-isolation use cases (internal multi-tenancy, dev/stage),
  document the gap and proceed.

### 2. `TenantDatabaseCache` never evicts

Deleting a tenant via `DELETE /v1/tenants/{name}` drops the schema
but leaves the cached connection pool in memory. Subsequent
requests to the deleted tenant fail with stale-pool errors.

**Mitigation:** restart `cloacina-server` after any `tenant
delete`. There is no in-process workaround as of v0.5.

### 3. Trigger list is global

`GET /v1/tenants/{id}/triggers` returns the global schedule list
filtered client-side by name; it is not schema-aware. Tenant-scoped
keys can read all schedule *names* (but not manipulate other
tenants' schedules).

**Mitigation:** treat schedule names as non-sensitive. If names
themselves leak business intent, segregate by deployment.

### 4. `/metrics` is unauthenticated

Reverse-proxy `/metrics` if your deployment requires access
control. Sample Caddyfile:

```text
cloacina.example.com {
    @metrics path /metrics
    @internal remote_ip 10.0.0.0/8 192.168.0.0/16
    handle @metrics {
        reverse_proxy @internal localhost:8080
    }

    reverse_proxy /v1/* localhost:8080
    reverse_proxy /health localhost:8080
    reverse_proxy /ready localhost:8080
}
```

### 5. Bootstrap key is single-capture

If you lose the bootstrap key and have no other admin key,
recovery requires direct database access (delete the row in
`api_keys` to trigger a re-bootstrap on next startup). Plan for at
least two admin keys: one for routine ops, one stored offline as a
cold backup.

## Verification

After provisioning, smoke-test isolation:

```bash
# Tenant key cannot access another tenant
cloacinactl --profile acme-prod --tenant globex workflow list
# → 403 Forbidden

# Tenant key cannot create tenants
cloacinactl --profile acme-prod tenant create new-tenant
# → 403 Forbidden

# Admin key can do both
cloacinactl --profile admin tenant list
cloacinactl --profile admin --tenant globex workflow list
# both succeed
```

## Related

- [Multi-Tenancy Architecture]({{< ref "/platform/explanation/multi-tenancy" >}}) — schema isolation design.
- [HTTP API Reference]({{< ref "/platform/reference/http-api" >}}) — the tenant + key endpoints, full operational caveats list.
- [Production Deployment]({{< ref "/platform/how-to-guides/production-deployment" >}}) — TLS termination, reverse proxy.
- [Multi-Tenant Setup]({{< ref "/workflows/how-to-guides/multi-tenant-setup" >}}) — embedded-mode multi-tenancy via `DefaultRunner::with_schema`.
