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
let tenant = DefaultRunner::with_schema(db_url, "tenant_acme").await?;
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
UnifiedExecutor::with_schema(db_url, "tenant'; DROP TABLE --").await?;
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

### Using DatabaseAdmin for Tenant Provisioning

```rust
use cloacina::database::{Database, DatabaseAdmin, TenantConfig};

// Admin connection with privileges to create users/schemas
let admin_db = Database::new(
    "postgresql://admin:admin_pass@localhost/cloacina",
    "cloacina",
    10
);
let admin = DatabaseAdmin::new(admin_db);

// Create a tenant with auto-generated secure password
let tenant_creds = admin.create_tenant(TenantConfig {
    schema_name: "tenant_acme".to_string(),
    username: "acme_user".to_string(),
    password: "".to_string(), // Empty = auto-generate 32-char password
})?;

// Returns ready-to-use credentials
println!("Username: {}", tenant_creds.username);
println!("Password: {}", tenant_creds.password); // Secure 32-char password
println!("Schema: {}", tenant_creds.schema_name);
println!("Connection: {}", tenant_creds.connection_string);
```

### Password Security

- **Auto-Generation**: Empty password string triggers generation of 32-character secure password
- **Character Set**: 94 characters including uppercase, lowercase, digits, and symbols
- **Entropy**: ~202 bits of entropy for auto-generated passwords
- **PostgreSQL Hashing**: All passwords are hashed with SCRAM-SHA-256 by PostgreSQL
- **No Storage**: Cloacina never stores passwords - they're passed to PostgreSQL and returned to admin

### Complete Tenant Lifecycle

```rust
// 1. Create tenant with all database objects
let creds = admin.create_tenant(TenantConfig {
    schema_name: "tenant_xyz".to_string(),
    username: "xyz_user".to_string(),
    password: "custom_password".to_string(), // Or "" for auto-generation
})?;

// 2. Distribute credentials to tenant (via secure channel)
send_credentials_to_tenant(&creds);

// 3. Tenant application uses their specific credentials
let executor = UnifiedExecutor::with_schema(
    &creds.connection_string,
    &creds.schema_name
).await?;

// 4. Later: Remove tenant when needed
admin.remove_tenant("tenant_xyz", "xyz_user")?;
```

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

The same `UnifiedExecutor::with_schema()` API works for both approaches:

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

You can migrate from shared to per-tenant credentials progressively:

```rust
// Phase 1: Some tenants still use shared credentials
let legacy = DefaultRunner::with_schema(shared_url, "old_tenant").await?;

// Phase 2: New tenants get their own credentials
let new_creds = admin.create_tenant(TenantConfig { /* ... */ })?;
let new_tenant = DefaultRunner::with_schema(
    &new_creds.connection_string,
    "new_tenant"
).await?;

// Phase 3: Gradually migrate existing tenants
// Create new credentials, update connection strings, remove shared access
```

### Requirements and Limitations

- **PostgreSQL Only**: Not available for SQLite deployments
- **Admin Privileges**: Requires database user with `CREATEDB` and `CREATEROLE`
- **Connection Pools**: Each tenant gets their own connection pool
- **Not a Complete Solution**: Still requires application-level auth/authz

See the [per-tenant credentials example](https://github.com/your-repo/cloacina/tree/main/examples/per_tenant_credentials) for a complete working demonstration.

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
use cloacina::runner::DefaultRunner;

// Create tenant-specific runners
let tenant_a = DefaultRunner::with_schema(
    "postgresql://user:pass@localhost/cloacina",
    "tenant_a"
).await?;

let tenant_b = DefaultRunner::with_schema(
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
let executor = DefaultRunner::builder()
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
let executor = DefaultRunner::with_schema(db_url, "new_tenant").await?;
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

let executor = DefaultRunner::with_schema(&database_url, &tenant_id).await?;
```

### Service-Based Isolation

```rust
// Different services can use different schemas for isolation
let api_executor = DefaultRunner::with_schema(db_url, "api_service").await?;
let batch_executor = DefaultRunner::with_schema(db_url, "batch_processor").await?;
let analytics_executor = DefaultRunner::with_schema(db_url, "analytics").await?;
```

## SQLite File-Based Multi-Tenancy

For SQLite deployments, multi-tenancy is achieved through separate database files.

### Basic Usage

```rust
// Each tenant gets their own database file
let tenant_a = DefaultRunner::new("sqlite://./data/tenant_a.db").await?;
let tenant_b = DefaultRunner::new("sqlite://./data/tenant_b.db").await?;
let tenant_c = DefaultRunner::new("sqlite://./data/tenant_c.db").await?;
```

### Dynamic File Paths

```rust
let tenant_id = env::var("TENANT_ID")?;
let db_path = format!("sqlite://./data/{}.db", tenant_id);
let executor = DefaultRunner::new(&db_path).await?;
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
let executor = DefaultRunner::with_schema(db_url, "my_tenant").await?;
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
let legacy_executor = DefaultRunner::with_schema(db_url, "legacy_tenant").await?;

// New tenants use their own schemas
let new_tenant = DefaultRunner::with_schema(db_url, "new_customer").await?;
```

#### Option 2: Run Side-by-Side

```rust
// Existing single-tenant runner (uses public schema)
let legacy_executor = DefaultRunner::new(db_url).await?;

// New multi-tenant runners use schemas
let tenant_a = DefaultRunner::with_schema(db_url, "tenant_a").await?;
let tenant_b = DefaultRunner::with_schema(db_url, "tenant_b").await?;
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
async fn create_tenant_runner(
    db_url: &str,
    tenant_id: &str
) -> Result<DefaultRunner, AppError> {
    // Validate tenant ID comes from trusted source
    validate_tenant_id(tenant_id)?;

    // Create runner with monitoring
    let runner = DefaultRunner::with_schema(db_url, tenant_id)
        .await
        .map_err(|e| AppError::TenantSetup(tenant_id.to_string(), e))?;

    // Log tenant creation for audit trail
    audit_log!("Tenant runner created: {}", tenant_id);

    Ok(runner)
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
