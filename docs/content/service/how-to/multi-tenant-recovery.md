---
title: "Multi-Tenant Recovery"
description: "Guide for handling recovery in multi-tenant Cloacina deployments"
weight: 30
---

# Multi-Tenant Recovery

This guide covers how recovery works in multi-tenant Cloacina deployments, how to handle common failure scenarios, and how to monitor tenant health.

## Automatic Recovery

Cloacina has recovery enabled by default. Each tenant's recovery operates independently, so a failure in one tenant does not affect others.

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

## Recovery Scenarios

### Runner Crash During Tenant Execution

When a runner crashes mid-execution, tasks that were in progress become orphaned. On restart, the recovery service detects these and re-queues them.

**Step 1**: Restart the runner for the affected tenant:

```rust
let runner = DefaultRunner::with_schema(db_url, "tenant_acme").await?;
```

**Step 2**: The runner automatically scans for orphaned tasks on startup. Tasks that were in a `Running` state with no active executor are identified as orphaned and rescheduled. No additional code is needed.

**Step 3**: Verify recovery by checking execution status:

```rust
let stats = runner
    .get_cron_execution_stats(chrono::Utc::now() - chrono::Duration::try_minutes(30).unwrap())
    .await?;

info!("Recovered executions: {}", stats.lost_executions);
info!("Successful executions: {}", stats.successful_executions);
```

### Runner Crash Affecting Multiple Tenants

If a process hosts runners for multiple tenants and crashes, each tenant must be recovered independently:

```rust
let tenants = vec!["tenant_acme", "tenant_globex", "tenant_initech"];

for tenant_id in &tenants {
    info!("Recovering tenant: {}", tenant_id);

    // Each call triggers independent recovery for that tenant's schema
    let runner = DefaultRunner::with_schema(db_url, tenant_id).await?;

    // Optionally check for recovered work
    let stats = runner
        .get_cron_execution_stats(chrono::Utc::now() - chrono::Duration::try_hours(1).unwrap())
        .await?;

    info!(
        "Tenant {}: {} total, {} successful, {} lost",
        tenant_id, stats.total_executions, stats.successful_executions, stats.lost_executions
    );
}
```

### Database Failover (PostgreSQL)

When a PostgreSQL primary fails over to a replica, existing connection pools become invalid. To recover:

**Step 1**: Shut down all tenant runners gracefully (if possible):

```rust
for (tenant_id, runner) in tenant_runners.drain() {
    if let Err(e) = runner.shutdown().await {
        warn!("Failed to shut down runner for {}: {}", tenant_id, e);
    }
}
```

**Step 2**: Re-create runners pointing to the new primary:

```rust
let new_db_url = "postgresql://cloacina:cloacina@new-primary:5432/cloacina";

for tenant_id in &tenants {
    let runner = DefaultRunner::with_schema(new_db_url, tenant_id).await?;
    tenant_runners.insert(tenant_id.to_string(), runner);
}
```

The new runners will pick up any orphaned tasks from the database and resume execution.

## PostgreSQL vs SQLite Recovery Differences

### PostgreSQL (Schema-Based)

With PostgreSQL, all tenants share a single database but operate in separate schemas. Recovery characteristics:

- **Shared infrastructure**: A single database failover affects all tenants simultaneously
- **Schema scoping**: Recovery queries are automatically scoped to the tenant's schema via `SET search_path`
- **Connection pool reset**: After a failover, all connection pools must be re-established
- **Concurrent recovery**: Multiple tenant runners can recover in parallel since schemas are independent

```rust
// PostgreSQL: all tenants share the same database URL, differ by schema
let runner = DefaultRunner::with_schema(
    "postgresql://cloacina:cloacina@localhost:5432/cloacina",
    "tenant_acme"
).await?;
```

### SQLite (File-Based)

With SQLite, each tenant has a completely separate database file. Recovery characteristics:

- **Physical isolation**: A corrupt database file affects only one tenant
- **Independent recovery**: Each tenant's file can be backed up and restored independently
- **No failover complexity**: No connection pool invalidation since each file is self-contained
- **WAL mode recommended**: Use WAL journal mode for better crash recovery

```rust
// SQLite: each tenant gets its own database file
let runner = DefaultRunner::new(
    "sqlite://./data/tenant_acme.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL"
).await?;
```

For SQLite tenants, if a database file becomes corrupt, you can restore from a backup without affecting other tenants:

```bash
# Stop the runner for the affected tenant
# Replace the corrupt file with a backup
cp backups/tenant_acme.db data/tenant_acme.db

# Restart the runner -- recovery will handle any orphaned tasks
```

## Monitoring Tenant Health

### Periodic Health Checks

Run periodic checks across all tenants to detect problems early:

```rust
async fn check_tenant_health(
    db_url: &str,
    tenant_ids: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    for tenant_id in tenant_ids {
        let runner = DefaultRunner::with_schema(db_url, tenant_id).await?;

        let stats = runner
            .get_cron_execution_stats(
                chrono::Utc::now() - chrono::Duration::try_minutes(10).unwrap()
            )
            .await?;

        if stats.lost_executions > 0 {
            warn!(
                "Tenant {} has {} lost executions in the last 10 minutes",
                tenant_id, stats.lost_executions
            );
        }

        if stats.success_rate < 95.0 {
            warn!(
                "Tenant {} success rate is {:.1}% (below 95% threshold)",
                tenant_id, stats.success_rate
            );
        }

        runner.shutdown().await?;
    }

    Ok(())
}
```

### Logging and Alerting

Use structured logging to make tenant-specific issues easy to filter:

```rust
use tracing::info_span;

for tenant_id in &tenants {
    let span = info_span!("tenant", id = tenant_id);
    let _guard = span.enter();

    let runner = DefaultRunner::with_schema(db_url, tenant_id).await?;
    // All log output within this scope includes tenant.id
}
```

## Migration from Single-Tenant to Multi-Tenant

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

Key points:

- Existing data remains in the `public` schema
- Each new tenant gets their own schema
- No data migration required -- schemas are independent
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

See the [multi-tenant example](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials/workflows/service/06-multi-tenancy) for a working demonstration of these concepts.
