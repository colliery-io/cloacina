---
title: "Multi-Tenancy Architecture"
description: "Understanding how Cloacina implements multi-tenancy, its isolation guarantees, and security implications"
weight: 50
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

## How It Works

### PostgreSQL Schema Implementation

When you create a tenant-specific executor:

```rust
let tenant = UnifiedExecutor::with_schema(db_url, "tenant_acme").await?;
```

Cloacina performs these operations:

1. **Schema Creation**: `CREATE SCHEMA IF NOT EXISTS tenant_acme`
2. **Connection Pool Setup**: Each connection automatically runs `SET search_path TO tenant_acme, public`
3. **Migration Execution**: All tables are created within the tenant schema
4. **Isolated Operations**: All queries operate within the schema namespace

The connection pool ensures every database operation is scoped:

```rust
// From Cloacina's connection.rs
impl CustomizeConnection<PgConnection, R2D2Error> for SchemaCustomizer {
    fn on_acquire(&self, conn: &mut PgConnection) -> Result<(), R2D2Error> {
        if let Some(ref schema) = self.schema {
            // Every connection is automatically scoped to the tenant
            let sql = format!("SET search_path TO {}, public", schema);
            diesel::sql_query(&sql).execute(conn)?;
        }
        Ok(())
    }
}
```

### SQLite File Implementation

```rust
let tenant = UnifiedExecutor::new("sqlite://./tenant_acme.db").await?;
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
    let executor = UnifiedExecutor::with_schema(&db_url, &tenant_id).await?;

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
UnifiedExecutor::with_schema(db_url, "tenant'; DROP TABLE --").await?;
// Error: Schema name must contain only alphanumeric characters and underscores
```

**Accidental Cross-Access Prevention**: Impossible to accidentally query another tenant
```sql
-- This fails because tenant_xyz schema is not in search_path
SELECT * FROM tenant_xyz.contexts; -- Error: schema "tenant_xyz" does not exist
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

## PostgreSQL Schema-Based Multi-Tenancy

PostgreSQL schema-based multi-tenancy provides the strongest isolation guarantees by leveraging PostgreSQL's native schema support.

### Key Benefits

- **Zero collision risk** - Impossible for tenants to access each other's data
- **No query changes** - All existing DAL code works unchanged
- **Native PostgreSQL feature** - Battle-tested and performant
- **Performance** - No overhead from filtering every query
- **Clean separation** - Each tenant can even have different schema versions

### Basic Usage

```rust
use cloacina::executor::unified_executor::UnifiedExecutor;

// Create tenant-specific executors
let tenant_a = UnifiedExecutor::with_schema(
    "postgresql://user:pass@localhost/cloacina",
    "tenant_a"
).await?;

let tenant_b = UnifiedExecutor::with_schema(
    "postgresql://user:pass@localhost/cloacina",
    "tenant_b"
).await?;

// Each executor operates in complete isolation
let result_a = tenant_a.execute("my_workflow", context_a).await?;
let result_b = tenant_b.execute("my_workflow", context_b).await?;
```

### Builder Pattern

For more complex configurations, use the builder pattern:

```rust
let executor = UnifiedExecutor::builder()
    .database_url("postgresql://user:pass@localhost/cloacina")
    .schema("production_tenant_123")
    .max_concurrent_tasks(8)
    .task_timeout(Duration::from_secs(600))
    .build()
    .await?;
```

### Schema Management

Schemas are automatically created and migrated on first use:

```rust
// First time accessing a schema
let executor = UnifiedExecutor::with_schema(db_url, "new_tenant").await?;
// This will:
// 1. Create the 'new_tenant' schema if it doesn't exist
// 2. Run all migrations in that schema
// 3. Set up connection pool with correct search_path
```

### Environment-Based Configuration

```rust
use std::env;

let tenant_id = env::var("TENANT_ID")?;
let database_url = env::var("DATABASE_URL")?;

let executor = UnifiedExecutor::with_schema(&database_url, &tenant_id).await?;
```

### Service-Based Isolation

```rust
// Different services can use different schemas for isolation
let api_executor = UnifiedExecutor::with_schema(db_url, "api_service").await?;
let batch_executor = UnifiedExecutor::with_schema(db_url, "batch_processor").await?;
let analytics_executor = UnifiedExecutor::with_schema(db_url, "analytics").await?;
```

## SQLite File-Based Multi-Tenancy

For SQLite deployments, multi-tenancy is achieved through separate database files.

### Basic Usage

```rust
// Each tenant gets their own database file
let tenant_a = UnifiedExecutor::new("sqlite://./data/tenant_a.db").await?;
let tenant_b = UnifiedExecutor::new("sqlite://./data/tenant_b.db").await?;
let tenant_c = UnifiedExecutor::new("sqlite://./data/tenant_c.db").await?;
```

### Dynamic File Paths

```rust
let tenant_id = env::var("TENANT_ID")?;
let db_path = format!("sqlite://./data/{}.db", tenant_id);
let executor = UnifiedExecutor::new(&db_path).await?;
```

## Schema Naming Rules

When using PostgreSQL schemas, names must follow these rules:

- **Alphanumeric characters only**: a-z, A-Z, 0-9
- **Underscores allowed**: `_`
- **No special characters**: hyphens, spaces, symbols not allowed

### Valid Examples
- ✅ `tenant_123`
- ✅ `acme_corp`
- ✅ `production_api`
- ✅ `customer_abc123`

### Invalid Examples
- ❌ `tenant-123` (hyphens not allowed)
- ❌ `tenant 123` (spaces not allowed)
- ❌ `tenant@123` (special characters not allowed)
- ❌ `tenant.123` (dots not allowed)

## Migration Strategies

### For New Deployments

Simply start using schemas from the beginning:

```rust
let executor = UnifiedExecutor::with_schema(db_url, "my_tenant").await?;
```

### For Existing Single-Tenant Deployments

#### Option 1: Move Existing Data to a Schema

```sql
BEGIN;
-- Create new schema for existing data
CREATE SCHEMA legacy_tenant;

-- Move all tables to the schema
ALTER TABLE pipeline_executions SET SCHEMA legacy_tenant;
ALTER TABLE task_executions SET SCHEMA legacy_tenant;
ALTER TABLE contexts SET SCHEMA legacy_tenant;
-- ... repeat for all tables

COMMIT;
```

Then update your application:

```rust
// Existing data now in 'legacy_tenant' schema
let legacy_executor = UnifiedExecutor::with_schema(db_url, "legacy_tenant").await?;

// New tenants use their own schemas
let new_tenant = UnifiedExecutor::with_schema(db_url, "new_customer").await?;
```

#### Option 2: Run Side-by-Side

```rust
// Existing single-tenant executor (uses public schema)
let legacy_executor = UnifiedExecutor::new(db_url).await?;

// New multi-tenant executors use schemas
let tenant_a = UnifiedExecutor::with_schema(db_url, "tenant_a").await?;
let tenant_b = UnifiedExecutor::with_schema(db_url, "tenant_b").await?;
```

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

### Production Deployment

```rust
// Production setup with proper error handling
async fn create_tenant_executor(
    db_url: &str,
    tenant_id: &str
) -> Result<UnifiedExecutor, AppError> {
    // Validate tenant ID comes from trusted source
    validate_tenant_id(tenant_id)?;

    // Create executor with monitoring
    let executor = UnifiedExecutor::with_schema(db_url, tenant_id)
        .await
        .map_err(|e| AppError::TenantSetup(tenant_id.to_string(), e))?;

    // Log tenant creation for audit trail
    audit_log!("Tenant executor created: {}", tenant_id);

    Ok(executor)
}
```

### Monitoring and Observability

Track tenant-specific metrics:
- Schema sizes and growth rates
- Query performance per tenant
- Connection pool usage
- Migration status

### Backup and Recovery

```bash
# Backup specific tenant
pg_dump -h host -d cloacina --schema=tenant_acme -f tenant_acme.sql

# Restore specific tenant
psql -h host -d cloacina -f tenant_acme.sql
```

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
