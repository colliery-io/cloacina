---
title: "Multi-Tenant Recovery"
description: "Guide for handling recovery in multi-tenant Cloacina deployments"
weight: 30
---

# Multi-Tenant Recovery

This guide covers how recovery works in multi-tenant Cloacina deployments and basic migration considerations.

## Recovery in Multi-Tenant Systems

### Automatic Recovery

Cloacina has recovery enabled by default. Each tenant's recovery operates independently:

```rust
// First runner creates schema and runs migrations
let runner1 = DefaultRunner::with_schema(db_url, "tenant_acme").await?;
// ... work gets interrupted ...
runner1.shutdown().await?;

// Second runner automatically recovers any interrupted work
let runner2 = DefaultRunner::with_schema(db_url, "tenant_acme").await?;
// - Schema already exists (not recreated)
// - Migrations already applied (not re-run)
// - Orphaned tasks automatically detected and recovered
// - Failed tasks retried based on retry configuration
```

Key points:
- **Always on**: Recovery is enabled by default for all executors
- **Per-tenant isolation**: Each tenant's recovery is independent
- **Automatic**: No manual intervention needed
- **Stateful**: Schemas and data persist across restarts

## Migration from Single-Tenant to Multi-Tenant

### Basic Migration Approach

When migrating an existing single-tenant deployment to multi-tenant:

```rust
// Existing single-tenant application uses public schema
let legacy_runner = DefaultRunner::new(db_url).await?;

// New tenant uses isolated schema
let tenant_runner = DefaultRunner::with_schema(db_url, "tenant_001").await?;

// Both can run side-by-side during migration
// Existing data remains in public schema
// New tenant data is isolated in tenant_001 schema
```

### Key Points

- Existing data remains in the `public` schema
- Each new tenant gets their own schema
- No data migration required - schemas are independent
- Applications can be migrated gradually
- Both single-tenant and multi-tenant can coexist

## Common Patterns

### Development vs Production

```rust
// Development: Quick setup for testing
let dev_tenant = DefaultRunner::with_schema(
    "postgresql://localhost/dev_db",
    "test_tenant"
).await?;

// Production: Full configuration
let prod_tenant = DefaultRunner::builder()
    .database_url(&production_url)
    .schema(&tenant_id)
    .enable_recovery(true)
    .max_concurrent_tasks(10)
    .build()
    .await?;
```

### Schema Naming

- Use only alphanumeric characters and underscores
- Examples: `tenant_001`, `acme_corp`, `customer_xyz`
- Avoid special characters, spaces, or hyphens


See the [multi-tenant example](https://github.com/your-repo/cloacina/tree/main/examples/multi_tenant) for a working demonstration of these concepts.
