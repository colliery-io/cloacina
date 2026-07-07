---
title: "Multi-Tenancy Architecture"
description: "Understanding how Cloacina implements multi-tenancy, its isolation guarantees, and security implications"
weight: 50
aliases:
  - "/platform/explanation/multi-tenancy/"

---

# Multi-Tenancy Architecture

Cloacina implements multi-tenancy through database-level isolation, providing strong data separation between tenants while maintaining shared infrastructure. This document explains how it works, what it guarantees, and important security considerations.

## Implementation Overview

Multi-tenancy in Cloacina is **not** a security feature - it's a data organization feature that provides strong isolation against accidental cross-tenant access but requires proper authentication and authorization at the application layer.

### What It IS

- **Data isolation mechanism** using PostgreSQL schemas or SQLite files
- **Protection against accidental cross-tenant queries**
- **Operational isolation** for workflows and recovery
- **Foundation for building multi-tenant applications**

### What It IS NOT

- **Authentication/authorization system**
- **User access control**
- **Security boundary against malicious code**
- **Complete multi-tenant solution**

## Embedded vs server mode

Multi-tenancy lives at two different layers depending on how Cloacina is deployed; the differences are non-trivial and worth getting straight.

- **Embedded library (`DefaultRunner::with_schema`)** — the host application is the trust boundary. Schema scoping is enforced at the connection pool layer; the application chooses which schema to address per-runner. Cloacina provides the isolation primitives; the host owns enforcement.
- **Server (`cloacina-server`)** — the server itself enforces multi-tenancy. Every authenticated request runs through a tenant-access check, executes against a per-tenant cached `DefaultRunner`, and uses fail-closed `SET search_path` enforcement at connection acquisition. The server is the trust boundary.

The rest of this document discusses both. Where the two diverge, sections are tagged accordingly.

### Server-mode guarantees (the current state)

Earlier server releases had known "isolation gaps"; current releases have closed them. The current state in server mode:

- **Fail-closed `SET search_path`.** Per-tenant connection acquisition sets `search_path` strictly to the tenant's schema; a failed `SET search_path` is a hard error, **not** a silent fall-through to `public`. Closes the cross-tenant data-leak risk that existed in earlier releases.
- **Per-tenant `DefaultRunner` instances.** Each tenant has its own runner — its own scheduler loop, executor pool, and per-tenant DB connection pool — cached in `TenantRunnerCache` (LRU, default 256 entries, controlled by `--tenant-runner-cache-size`). Workflow execution lands in the tenant's schema, not in a shared global runner.
- **Per-tenant trigger / graph / accumulator filtering.** `GET /v1/tenants/{id}/triggers` (and the graph/accumulator health endpoints) route through a tenant-scoped `Database`; the underlying SQL hits the tenant's schedules table, not a shared global table.
- **4-step teardown orchestration on `DELETE /v1/tenants/{name}`:**
  1. Revoke every still-active API key for the tenant (close the auth surface).
  2. Evict the tenant's `DefaultRunner` from `TenantRunnerCache`, awaiting a bounded graceful drain (`--tenant-deletion-drain-timeout-s`, default 30s). Past the timeout the runner is **hard-evicted** — tasks that ignore cooperative cancellation error on their next DB write once step 4 lands.
  3. Evict the tenant's `Database` from `TenantDatabaseCache` (releases the connection pool).
  4. Drop the schema + user via `DatabaseAdmin::remove_tenant`.

  Each step emits a structured audit event with duration. Per-step failures bail out; earlier steps stay committed and are idempotent, so a retry resumes from the failure point.

The "restart `cloacina-server` to reclaim the cache after a tenant delete" workaround documented in earlier releases is **gone** — both `TenantRunnerCache` and `TenantDatabaseCache` are evicted as part of the teardown.

For the operational mechanics (how to actually decommission a tenant), see [Decommission a Tenant]({{< ref "/service/how-to/decommission-a-tenant" >}}). For the trust-model implications of multi-tenancy and what it does *not* protect against (CPU side-channels, privileged-key compromise, Postgres-level RLS), see [Security Model]({{< ref "security-model" >}}).

### Per-tenant isolation layers (0.9.0)

Earlier releases isolated tenant **data** (schema, runner, connection pool).
Version 0.9.0 carries that isolation through the rest of the tenant lifecycle —
build, execution, and operational visibility — so a tenant's packages build, run,
and report entirely within the tenant's own realm.

- **Build isolation (per-tenant compiler).** A `cloacina-compiler` can be scoped
  to a single tenant via `tenant_schema`: when set, it claims and builds only
  that tenant's packages, against that tenant's Postgres schema. A tenant's build
  queue is the tenant's own, not a shared global pipeline.

- **Execution namespacing.** Each per-tenant `DefaultRunner` stamps its
  `tenant_id` onto the tasks it registers, so they are namespaced
  `tenant::package::workflow::task` rather than the previously hardcoded
  `public::package::workflow::task`. The namespace is load-bearing: it is what
  routes a tenant's tasks to **the tenant's own agents** and makes the agent
  fetch the cdylib from **the tenant's schema**. Before this, every tenant's
  tasks were namespaced `public::…` regardless of tenant, so they neither matched
  the tenant's agents nor could fetch their (tenant-schema) cdylib — tenant runs
  hung in `Running`. The wiring is `DefaultRunnerConfig.tenant_id →
  ReconcilerConfig.default_tenant_id` (`"public"` for the admin/global runner; a
  per-tenant runner sets it to its tenant).

- **Operational metrics are tenant-scoped.** The ops-metrics stream is gathered
  per **view**: `None` is the admin/global view (sees everything), and a tenant
  view sees only items it owns — its own build queue, reconciler, fleet, and
  graph health. It is no longer a single global admin snapshot; each tenant sees
  its own operational state, never another tenant's. (Public/null-tenant items —
  `tenant_id = None` — are surfaced under the `"public"` view.)

### Fleet resource limits & per-tenant namespace

The agent self-management control plane adds a **per-tenant agent-capacity
limit**: a tenant scales its execution-agent fleet only within an *effective
limit* (a platform default, optionally overridden per tenant by an admin), and a
tenant cannot raise its own ceiling. This is a fleet-sizing bound on the number
of `cloacina-agent` workers a tenant runs — not a CPU/memory quota (the
shared-infrastructure caveats below still apply). When the Kubernetes fleet
actuator is enabled, each tenant's agents run in the tenant's **own namespace**
(`cloacina-tenant-<t>`), extending tenant isolation to the agent workloads
themselves. See
[Execution-Agent Fleet]({{< ref "execution-agent-fleet" >}}#capacity-limits--autoscaling).

## How It Works

### PostgreSQL Schema Implementation

When you create a tenant-specific executor in embedded mode:

```rust
let tenant = DefaultRunner::with_schema(db_url, "tenant_acme").await?;
```

Cloacina performs these operations:

1. **Schema Creation**: `CREATE SCHEMA IF NOT EXISTS tenant_acme`
2. **Connection Pool Setup**: Each connection automatically runs `SET search_path TO tenant_acme, public` (the *strict* form also rejects fall-through to `public` if the tenant schema is unreachable).
3. **Migration Execution**: All tables are created within the tenant schema.
4. **Isolated Operations**: All queries operate within the schema namespace.

The connection pool ensures every database operation is scoped:

```rust
// From Cloacina's connection.rs (fail-closed form)
impl CustomizeConnection<PgConnection, R2D2Error> for SchemaCustomizer {
    fn on_acquire(&self, conn: &mut PgConnection) -> Result<(), R2D2Error> {
        if let Some(ref schema) = self.schema {
            // Every connection is automatically scoped to the tenant.
            // The strict variant (set_strict_search_path) errors hard if
            // the SET fails, rather than silently falling through to public.
            let sql = format!("SET search_path TO {}, public", schema);
            diesel::sql_query(&sql).execute(conn)?;
        }
        Ok(())
    }
}
```

### SQLite File Implementation

```rust
let tenant = DefaultRunner::new("sqlite://./tenant_acme.db").await?;
```

Each tenant gets a completely separate SQLite database file, providing physical isolation.

## Isolation Guarantees

### Strong Guarantees (What Cloacina Provides)

1. **Data Isolation**
   - Tenant data cannot accidentally access other tenant data
   - SQL queries are automatically scoped to tenant schema
   - No possibility of cross-tenant data leakage through normal operations

2. **Operational Isolation**
   - Migration failures affect only one tenant
   - Recovery operations are scoped per tenant
   - Workflow execution is isolated

3. **Schema Validation**
   - Tenant names are validated to prevent SQL injection
   - Only alphanumeric characters and underscores allowed

### Weak Guarantees (Shared Infrastructure)

1. **Resource Isolation**
   - CPU, memory, and I/O are shared between tenants
   - No built-in resource quotas or limits
   - One tenant can impact others through resource exhaustion

2. **Database-Level Operations**
   - Shared PostgreSQL instance and connection pool
   - Shared transaction logs and buffer cache
   - Database-wide locks can affect all tenants

## Security Implications

### YOU Must Handle

**Authentication**: Who is making the request?
```rust
// Your application code
let user = authenticate_token(&request.auth_token)?;
```

**Authorization**: Which tenant(s) can they access?
```rust
// Your application code
let allowed_tenants = get_user_tenants(&user)?;
if !allowed_tenants.contains(&requested_tenant) {
    return Err("Access denied");
}
```

**API-Level Security**: Ensuring requests are properly scoped
```rust
// Your application code
async fn handle_request(auth: AuthToken, tenant_id: String, req: Request) {
    // 1. Authenticate user
    let user = authenticate(auth)?;

    // 2. Authorize tenant access
    authorize_tenant_access(&user, &tenant_id)?;

    // 3. Create scoped executor
    let executor = DefaultRunner::with_schema(&db_url, &tenant_id).await?;

    // 4. Process request in isolated context
    executor.handle_request(req).await
}
```

### Cloacina Provides

**Data Scoping**: Automatic query scoping to prevent accidents
```rust
// This query only sees tenant_acme.contexts
let contexts = executor.get_dal().list_contexts().await?;
```

**Schema Validation**: Protection against basic injection
```rust
// This will fail validation
DefaultRunner::with_schema(db_url, "tenant'; DROP TABLE --").await?;
// Error: Schema name must contain only alphanumeric characters and underscores
```

**Accidental Cross-Access Prevention**: Impossible to accidentally query another tenant
```sql
-- This fails because tenant_xyz schema is not in search_path
SELECT * FROM tenant_xyz.contexts; -- Error: schema "tenant_xyz" does not exist
```

**Per-Tenant Database Credentials** (PostgreSQL only): Enhanced security with database-level user isolation
```rust
// Using DatabaseAdmin to create isolated tenant users
use cloacina::database::{DatabaseAdmin, TenantConfig};

let admin = DatabaseAdmin::new(admin_database);
let creds = admin.create_tenant(TenantConfig {
    schema_name: "tenant_acme".to_string(),
    username: "acme_user".to_string(),
    password: "".to_string(), // Auto-generates secure 32-char password
})?;

// Each tenant uses their own database credentials
let executor = DefaultRunner::with_schema(
    &creds.connection_string,  // postgresql://acme_user:***@host/db
    &creds.schema_name
).await?;
```

## Trust Model

Cloacina's multi-tenancy assumes:

1. **Trusted Code**: Application code is not malicious
2. **Proper Auth**: Application handles authentication/authorization
3. **Validated Input**: Schema names come from trusted sources
4. **Shared Database**: All tenants use the same database credentials

It does NOT protect against:

1. **Malicious SQL**: Intentional cross-tenant queries
2. **Privilege Escalation**: Code that bypasses application auth
3. **Resource Attacks**: One tenant consuming all resources
4. **Side-Channel Attacks**: Timing attacks or cache analysis

## Enhanced Security: Per-Tenant Database Credentials

While the default multi-tenancy implementation uses shared database credentials with schema isolation, Cloacina also supports **per-tenant database credentials** for enhanced security in PostgreSQL deployments.

### Benefits of Per-Tenant Credentials

1. **Database-Level Access Control**: Each tenant has their own PostgreSQL user
2. **Audit Trail**: PostgreSQL logs show exactly which tenant performed operations
3. **Defense in Depth**: Database permissions as an additional security layer
4. **Credential Rotation**: Independent password rotation per tenant
5. **Compliance**: Meet regulations requiring database-level user separation

Provisioning a tenant with its own credentials is done through `DatabaseAdmin::create_tenant`,
which creates the schema, the database user, the grants, and runs migrations in a
single transaction. The step-by-step recipe is in
[Configure PostgreSQL Schema-Based Multi-Tenancy]({{< ref "/service/how-to/multi-tenant-setup" >}}#2-provision-tenants-with-databaseadmin);
the API surface is in the [DatabaseAdmin API Reference]({{< ref "/reference/database-admin" >}}).

### Password Security

- **Auto-Generation**: Empty password string triggers generation of 32-character secure password
- **Character Set**: 94 characters including uppercase, lowercase, digits, and symbols
- **Entropy**: ~202 bits of entropy for auto-generated passwords
- **PostgreSQL Hashing**: All passwords are hashed with SCRAM-SHA-256 by PostgreSQL
- **No Storage**: Cloacina never stores passwords - they're passed to PostgreSQL and returned to admin

### What DatabaseAdmin Does

The `create_tenant` method performs these operations in a transaction:

1. **Creates PostgreSQL Schema**: `CREATE SCHEMA IF NOT EXISTS tenant_xyz`
2. **Creates Database User**: `CREATE USER xyz_user WITH PASSWORD '...'`
3. **Grants Permissions**:
   - `GRANT USAGE ON SCHEMA tenant_xyz TO xyz_user`
   - `GRANT CREATE ON SCHEMA tenant_xyz TO xyz_user`
   - `GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA tenant_xyz TO xyz_user`
   - Sets default privileges for future tables
4. **Runs Migrations**: Executes all migrations in the tenant schema

### Zero API Changes

The same `DefaultRunner::with_schema()` API works for both approaches:

```rust
// Shared credentials (original approach)
let executor = DefaultRunner::with_schema(
    "postgresql://shared_user:shared_pw@host/db",
    "tenant_acme"
).await?;

// Per-tenant credentials (enhanced security)
let executor = DefaultRunner::with_schema(
    "postgresql://acme_user:tenant_pw@host/db",
    "tenant_acme"
).await?;
```

### Migration Path

Because the API surface is identical, migration from shared to per-tenant
credentials can be progressive: existing tenants keep using the shared
connection string while new tenants are provisioned with their own credentials
via `DatabaseAdmin`, and existing tenants are cut over one at a time (mint new
credentials, update the connection string, revoke shared access). No code changes
are required beyond the connection string each runner is given.

### Requirements and Limitations

- **PostgreSQL Only**: Not available for SQLite deployments
- **Admin Privileges**: Requires database user with `CREATEDB` and `CREATEROLE`
- **Connection Pools**: Each tenant gets their own connection pool
- **Not a Complete Solution**: Still requires application-level auth/authz

See the [per-tenant credentials example](https://github.com/colliery-io/cloacina/tree/main/examples/per_tenant_credentials) for a complete working demonstration.

## PostgreSQL Schema-Based Multi-Tenancy

PostgreSQL schema-based multi-tenancy provides the strongest isolation guarantees by leveraging PostgreSQL's native schema support.

### Key Benefits

- **Zero collision risk** - Impossible for tenants to access each other's data
- **No query changes** - All existing DAL code works unchanged
- **Native PostgreSQL feature** - Battle-tested and performant
- **Performance** - No overhead from filtering every query
- **Clean separation** - Each tenant can even have different schema versions

### Automatic schema management

Each tenant runner is created with `DefaultRunner::with_schema(db_url, schema)`.
On first use, the schema is created if it does not exist, all migrations are run
inside it, and the connection pool is configured with the correct `search_path`
so that every subsequent query is scoped to that tenant. Because scoping happens
at the connection layer, existing DAL code needs no query changes — the same
runner API serves single-tenant and multi-tenant deployments alike. Schemas can
also be used to isolate distinct *services* (an `api_service` schema separate
from a `batch_processor` schema), not only tenants.

For the concrete provisioning steps — including the builder pattern for custom
runner configuration and driving the schema from an environment variable — see
[Configure PostgreSQL Schema-Based Multi-Tenancy]({{< ref "/service/how-to/multi-tenant-setup" >}}).

## SQLite File-Based Multi-Tenancy

For SQLite deployments, isolation is achieved through separate database files:
each tenant gets its own file (`DefaultRunner::new("sqlite://./data/<tenant>.db")`),
so isolation is guaranteed by the file system. `DatabaseAdmin` and per-tenant
credentials are not available for SQLite. The setup steps are in the
[multi-tenant setup guide]({{< ref "/service/how-to/multi-tenant-setup" >}}#alternative-sqlite-file-based-tenancy).

## Schema Naming Rules

Schema names are validated before use in SQL to prevent injection: 1–63
characters, starting with a letter or underscore, alphanumeric-and-underscore
only, and not a reserved PostgreSQL name. The full rule table lives in the
[Configuration Reference]({{< ref "/reference/configuration" >}}#schema-naming-rules).

## Migrating from a single-tenant deployment

Moving an existing single-tenant deployment to schema-based tenancy — either by
relocating the existing tables into a named schema or by running schema-based
tenants side-by-side with the legacy `public`-schema runner — is a procedure, not
a concept. See [Configure PostgreSQL Schema-Based Multi-Tenancy]({{< ref "/service/how-to/multi-tenant-setup" >}}#migrate-an-existing-single-tenant-deployment).

## Performance Considerations

### PostgreSQL Schema Benefits

- **No query overhead** - Each tenant operates in their own namespace
- **Index isolation** - Each schema has its own indexes
- **Connection pooling** - Shared connection pool with per-connection schema setting
- **Parallel execution** - Multiple tenants can execute simultaneously

### SQLite File Benefits

- **Complete isolation** - Separate processes, separate files
- **Simple backup** - Each tenant database is a single file
- **Easy cleanup** - Delete the file to remove a tenant
- **No connection conflicts** - Each file has its own connection pool

## Practical Considerations

### Monitoring and Observability

Because each tenant occupies its own schema, the metrics worth watching are
tenant-scoped rather than global:

- Schema sizes and growth rates
- Query performance per tenant
- Connection pool usage
- Migration status

### Provisioning, backup, and decommissioning

The operational procedures live in the how-to guides:

- **Provisioning** — [Configure PostgreSQL Schema-Based Multi-Tenancy]({{< ref "/service/how-to/multi-tenant-setup" >}}).
- **Backup and restore** — each tenant is an isolated schema, so it can be backed
  up and restored independently with `pg_dump --schema`; see
  [Back Up and Restore]({{< ref "/service/how-to/backup-and-restore" >}}).
- **Decommissioning** — [Decommission a Tenant]({{< ref "/service/how-to/decommission-a-tenant" >}}).

## Summary

Cloacina's multi-tenancy provides **strong data isolation** but is **not a complete security solution**.

### Think of it as:
- ✅ **Strong foundation** for building multi-tenant applications
- ✅ **Protection against accidents** (cross-tenant data mixing)
- ✅ **Operational isolation** (migrations, recovery, execution)
- ❌ **NOT authentication/authorization** (you must implement this)
- ❌ **NOT a security boundary** (assumes trusted code)

### Key takeaway:
Cloacina handles the complex database-level isolation so you can focus on application-level security, authentication, and business logic. Use it as a building block, not a complete solution.
