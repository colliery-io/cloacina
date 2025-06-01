---
title: "DatabaseAdmin API Reference"
description: "API documentation for per-tenant database credential management"
weight: 20
---

# DatabaseAdmin API Reference

The `DatabaseAdmin` module provides utilities for creating and managing per-tenant database users and schemas in PostgreSQL multi-tenant deployments.

## Overview

```rust
use cloacina::database::{Database, DatabaseAdmin, TenantConfig, TenantCredentials};
```

## Types

### DatabaseAdmin

Administrative interface for tenant provisioning operations.

```rust
pub struct DatabaseAdmin {
    // Private fields
}

impl DatabaseAdmin {
    /// Create a new database administrator
    pub fn new(database: Database) -> Self;

    /// Create a complete tenant setup (schema + user + permissions + migrations)
    pub fn create_tenant(&self, tenant_config: TenantConfig) -> Result<TenantCredentials, AdminError>;

    /// Remove a tenant (user + schema)
    pub fn remove_tenant(&self, schema_name: &str, username: &str) -> Result<(), AdminError>;
}
```

### TenantConfig

Configuration for creating a new tenant.

```rust
pub struct TenantConfig {
    /// Schema name for the tenant (e.g., "tenant_acme")
    pub schema_name: String,

    /// Username for the tenant's database user (e.g., "acme_user")
    pub username: String,

    /// Password for the tenant user - empty string triggers auto-generation
    pub password: String,
}
```

### TenantCredentials

Credentials returned after tenant creation.

```rust
pub struct TenantCredentials {
    /// Username of the created tenant user
    pub username: String,

    /// Password (either provided or auto-generated)
    pub password: String,

    /// Schema name for the tenant
    pub schema_name: String,

    /// Ready-to-use connection string for the tenant
    pub connection_string: String,
}
```

### AdminError

Errors that can occur during database administration operations.

```rust
#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),

    #[error("Connection pool error: {0}")]
    Pool(#[from] diesel::r2d2::PoolError),

    #[error("SQL execution error: {message}")]
    SqlExecution { message: String },

    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },
}
```

## Examples

### Basic Tenant Creation

```rust
use cloacina::database::{Database, DatabaseAdmin, TenantConfig};

// Create admin connection
let admin_db = Database::new(
    "postgresql://admin:admin_pass@localhost/cloacina",
    "cloacina",
    10
);
let admin = DatabaseAdmin::new(admin_db);

// Create tenant with auto-generated password
let creds = admin.create_tenant(TenantConfig {
    schema_name: "tenant_acme".to_string(),
    username: "acme_user".to_string(),
    password: "".to_string(), // Empty = auto-generate
})?;

println!("Created tenant:");
println!("  Username: {}", creds.username);
println!("  Password: {}", creds.password);
println!("  Connection: {}", creds.connection_string);
```

### Custom Password

```rust
// Create tenant with custom password
let creds = admin.create_tenant(TenantConfig {
    schema_name: "tenant_xyz".to_string(),
    username: "xyz_user".to_string(),
    password: "custom_secure_password".to_string(),
})?;
```

### Remove Tenant

```rust
// Remove tenant and all associated data
admin.remove_tenant("tenant_xyz", "xyz_user")?;
```

## Password Security

### Auto-Generated Passwords

When `password` is an empty string, a cryptographically secure password is generated:

- **Length**: 32 characters
- **Character Set**: 94 characters (uppercase, lowercase, digits, symbols)
- **Entropy**: ~202 bits
- **Generator**: Uses `rand::thread_rng()` for cryptographic randomness

### Password Handling

1. **No Storage**: Cloacina never stores passwords
2. **PostgreSQL Hashing**: All passwords are hashed with SCRAM-SHA-256 by PostgreSQL
3. **Immediate Return**: Credentials are returned to the admin for secure distribution
4. **One-Time View**: The plaintext password is only available at creation time

## Database Operations

### What `create_tenant` Does

The method performs these operations in a single transaction:

1. **Create Schema**
   ```sql
   CREATE SCHEMA IF NOT EXISTS tenant_xyz
   ```

2. **Create User**
   ```sql
   CREATE USER xyz_user WITH PASSWORD '...'
   ```

3. **Grant Permissions**
   ```sql
   GRANT USAGE ON SCHEMA tenant_xyz TO xyz_user;
   GRANT CREATE ON SCHEMA tenant_xyz TO xyz_user;
   GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA tenant_xyz TO xyz_user;
   GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA tenant_xyz TO xyz_user;
   ALTER DEFAULT PRIVILEGES IN SCHEMA tenant_xyz GRANT ALL ON TABLES TO xyz_user;
   ALTER DEFAULT PRIVILEGES IN SCHEMA tenant_xyz GRANT ALL ON SEQUENCES TO xyz_user;
   ```

4. **Run Migrations**
   - Sets search_path to the tenant schema
   - Executes all Cloacina migrations in that schema

### What `remove_tenant` Does

The method performs these operations in a single transaction:

1. **Revoke Permissions**
2. **Drop User**: `DROP USER IF EXISTS xyz_user`
3. **Drop Schema**: `DROP SCHEMA IF EXISTS tenant_xyz CASCADE`

## Requirements

### PostgreSQL Requirements

- **Version**: PostgreSQL 10+ (for SCRAM-SHA-256 password hashing)
- **Admin Privileges**: The admin user needs:
  - `CREATEDB` privilege
  - `CREATEROLE` privilege
  - Permission to create schemas
  - Permission to grant privileges

### Feature Requirements

- Only available when using the `postgres` feature
- Not available for SQLite deployments

## Error Handling

### Common Errors

1. **User Already Exists**
   ```
   AdminError::SqlExecution { message: "Failed to create user 'xyz_user': ..." }
   ```

2. **Invalid Schema Name**
   ```
   AdminError::InvalidConfig { message: "Schema name cannot be empty" }
   ```

3. **Insufficient Privileges**
   ```
   AdminError::SqlExecution { message: "Failed to create schema: permission denied" }
   ```

## Integration with UnifiedExecutor

The credentials returned by `DatabaseAdmin` are designed to work seamlessly with `UnifiedExecutor`:

```rust
// Create tenant
let creds = admin.create_tenant(config)?;

// Use credentials with UnifiedExecutor
let executor = UnifiedExecutor::with_schema(
    &creds.connection_string,
    &creds.schema_name
).await?;

// Execute workflows with full isolation
executor.execute(workflow).await?;
```

## Best Practices

1. **Secure Credential Storage**: Store returned credentials in a secrets management system
2. **Audit Logging**: Log tenant creation/deletion for compliance
3. **Connection String Parsing**: Consider parsing the admin connection to build tenant connection strings
4. **Error Recovery**: Wrap operations in proper error handling for production use
5. **Resource Limits**: Monitor total connection count across all tenants

## See Also

- [Multi-Tenancy Architecture]({{< ref "/explanation/multi-tenancy" >}})
- [Multi-Tenant Setup Guide]({{< ref "/how-to-guides/multi-tenant-setup" >}})
- [Per-Tenant Credentials Example](https://github.com/your-repo/cloacina/tree/main/examples/per_tenant_credentials)
