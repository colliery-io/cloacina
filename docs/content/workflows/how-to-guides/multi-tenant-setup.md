---
title: "Configure PostgreSQL Schema-Based Multi-Tenancy"
description: "Set up isolated, per-tenant workflow execution using PostgreSQL schemas"
weight: 40
---

# Configure PostgreSQL Schema-Based Multi-Tenancy

## Goal

After completing this guide you will have a working PostgreSQL multi-tenant setup where each tenant's workflows run in an isolated database schema, with optional per-tenant database credentials for defense-in-depth security.

## Prerequisites

- Cloacina added to your project (`cloacina = "0.1.0"` in `Cargo.toml`)
- A running PostgreSQL server
- For per-tenant credentials: an admin user with `CREATEDB` and `CREATEROLE` privileges

## 1. Create per-tenant executors with `DefaultRunner::with_schema`

Each call to `DefaultRunner::with_schema` provisions a PostgreSQL schema (if it does not already exist) and returns an executor scoped to that schema:

```rust
use cloacina::runner::DefaultRunner;

let database_url = "postgresql://user:pass@localhost/cloacina";

let tenant_a = DefaultRunner::with_schema(database_url, "tenant_a").await?;
let tenant_b = DefaultRunner::with_schema(database_url, "tenant_b").await?;
```

All workflow state for `tenant_a` is fully isolated from `tenant_b`.

## 2. Provision tenants with `DatabaseAdmin`

Use `DatabaseAdmin` to create a schema **and** a dedicated database user in one step. Passing an empty password triggers secure auto-generation:

```rust
use cloacina::database::{Database, DatabaseAdmin, TenantConfig};

let admin_db = Database::new(
    "postgresql://admin:admin_pass@localhost/cloacina",
    "cloacina",
    10,
);
let admin = DatabaseAdmin::new(admin_db);

let creds = admin.create_tenant(TenantConfig {
    schema_name: "tenant_secure".to_string(),
    username: "secure_user".to_string(),
    password: "".to_string(), // auto-generate
})?;

// creds exposes: username, password, schema_name, connection_string
```

Store the returned `TenantCredentials` in a secrets manager (e.g., HashiCorp Vault, AWS Secrets Manager) and pass the connection string when creating the executor:

```rust
let executor = DefaultRunner::with_schema(
    &creds.connection_string,
    &creds.schema_name,
).await?;
```

To tear down a tenant and its database user:

```rust
admin.remove_tenant(&creds.schema_name, &creds.username)?;
```

### Why per-tenant credentials?

- **Database-level isolation** -- each tenant can only access their own schema
- **Audit compliance** -- PostgreSQL logs attribute operations to the correct user
- **Independent credential rotation** -- rotate one tenant without affecting others

## 3. Verify tenant isolation

A minimal check that two tenants cannot see each other's executions:

```rust
let tenant_a = DefaultRunner::with_schema(database_url, "test_tenant_a").await?;
let tenant_b = DefaultRunner::with_schema(database_url, "test_tenant_b").await?;

let result_a = tenant_a.execute_async("test_workflow", Context::new()).await?;
let result_b = tenant_b.execute_async("test_workflow", Context::new()).await?;

assert_ne!(result_a.execution_id, result_b.execution_id);
```

For full integration-testing patterns, see [Testing Workflows]({{< ref "testing-workflows" >}}).

## Alternative: SQLite file-based tenancy

If you do not need PostgreSQL, you can isolate tenants with one SQLite file per tenant:

```rust
let executor = DefaultRunner::new("sqlite:///data/tenant_a.db").await?;
```

Each file is a self-contained database, so isolation is guaranteed by the file system. `DatabaseAdmin` and per-tenant credentials are not available for SQLite.

## Related guides

- [Multi-Tenant Recovery]({{< ref "multi-tenant-recovery" >}}) -- automatic recovery and migration in multi-tenant deployments
- [Testing Workflows]({{< ref "testing-workflows" >}}) -- integration testing patterns including tenant isolation tests
- [Performance Tuning]({{< ref "performance-tuning" >}}) -- connection pool sizing, concurrency limits, and monitoring
