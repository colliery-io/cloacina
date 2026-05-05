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

## Architecture in 30 Seconds

Each tenant lives in its own **PostgreSQL schema** with its own
database user and migrations. The server holds a global
`TenantDatabaseCache` of per-tenant connection pools (2 connections
per tenant). API keys are scoped: either global (`tenant_id = null`,
admin only) or tenant-scoped (`tenant_id = <name>`, can only access
that tenant). The `is_admin` ("god-mode") flag bypasses tenant
scoping entirely.

See [Multi-Tenancy Architecture]({{< ref "/platform/explanation/multi-tenancy" >}}) for the detailed design.

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

These caveats are surfaced from the implementation. Build your
deployment around them.

### 1. Workflow Execution Scheduling Is NOT Tenant-Scoped

The `DefaultRunner` that backs
`POST /v1/tenants/{id}/workflows/{name}/execute` is a **single global
instance**. Executions land in the **runner's schema** (typically
`public`), not the tenant's schema.

In single-tenant deployments this is transparent. In multi-tenant
deployments it's a real isolation gap: tenant A's executions
co-mingle with tenant B's at the execution-state layer. The DAL
context, registry, and packages are tenant-scoped; the
*scheduling/execution state* is not.

**Mitigations until per-tenant runner support ships:**
- Run a separate `cloacina-server` instance per tenant if isolation
  matters for compliance. Each server gets its own database (or its
  own schema as the runner's home) and its own runner.
- For development/staging with low isolation requirements, accept
  the gap and document it for ops.

### 2. `TenantDatabaseCache` Never Evicts

The server lazily creates a per-tenant connection pool the first
time a request hits a tenant's routes. The pool is then cached for
the server's lifetime. Deleting the tenant via
`DELETE /v1/tenants/{name}` drops the schema but **leaves the cached
pool**. Subsequent requests to the deleted tenant fail with stale-
pool errors.

**Mitigation:** restart `cloacina-server` after any
`tenant delete`. This is the only way to reclaim the pool.

### 3. Trigger List Is Global

`GET /v1/tenants/{id}/triggers` returns the global schedule list,
filtered client-side by name. It is not a true per-tenant audit.
Tenant-scoped key holders can see all schedule names, just not
manipulate other tenants' schedules.

**Mitigation:** treat schedule names as non-sensitive. If schedule
names themselves leak business intent, segregate by deployment
rather than tenant.

### 4. Public `/metrics` Endpoint

`/metrics` exposes Prometheus output unauthenticated. A reverse
proxy must enforce access control if your deployment requires it.

**Recommended Caddyfile snippet:**

```text
cloacina.example.com {
    @metrics path /metrics
    reverse_proxy @metrics localhost:8080 {
        # Enforce internal-only access:
        @internal remote_ip 10.0.0.0/8 192.168.0.0/16
        rewrite @internal /metrics
    }

    reverse_proxy /v1/* localhost:8080
    reverse_proxy /health localhost:8080
    reverse_proxy /ready localhost:8080
}
```

### 5. Bootstrap Key Is Single-Capture

If you lose the bootstrap key and have no other admin key, you
**cannot** recover admin access without resetting the
`api_keys` table directly (effectively a manual re-bootstrap). Plan
for at least two admin keys: one for emergencies, one for routine
ops.

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
