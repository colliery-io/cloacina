# Per-Tenant Database Credentials Implementation Plan

## Overview

Add support for per-tenant database users/passwords to provide database-level isolation on top of the existing schema isolation. This would allow each tenant to have their own PostgreSQL user with permissions scoped only to their schema.

## Goals

1. **Database-level isolation**: Each tenant uses dedicated PostgreSQL user
2. **Zero API changes**: Existing code continues to work unchanged
3. **No naming conventions**: Users can structure credentials however they want
4. **Explicit behavior**: Schema is always explicitly specified
5. **Clean separation**: Cloacina provides the mechanism, consumers handle provisioning

## Architecture Overview

```rust
// Current approach (shared credentials)
let executor = UnifiedExecutor::with_schema(
    "postgresql://shared_user:shared_pw@host/db",
    "tenant_acme"
).await?;

// New approach (per-tenant credentials - same API!)
let executor = UnifiedExecutor::with_schema(
    "postgresql://tenant_specific_user:tenant_pw@host/db",
    "tenant_acme"
).await?;
```

**Key insight**: The same `with_schema` API works for both shared and per-tenant credentials. Cloacina doesn't need to know or care about the credential structure - it just uses whatever connection string is provided and sets the schema accordingly.

## Implementation Plan

### Phase 1: Database Administration Module

#### 1.1 Create Database Admin Module
- **File**: `cloacina/src/database/admin.rs` (new)
- **Purpose**: Administrative functions for tenant management

```rust
use diesel::prelude::*;
use crate::database::connection::Database;

pub struct DatabaseAdmin {
    database: Database,
}

impl DatabaseAdmin {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    /// Create a complete tenant setup (schema + user + permissions)
    pub fn create_tenant(&self, tenant_config: TenantConfig) -> Result<(), AdminError> {
        let mut conn = self.database.pool().get()?;

        // Execute all tenant setup SQL in a transaction
        conn.transaction(|conn| {
            // 1. Create schema
            self.create_schema(conn, &tenant_config.schema_name)?;

            // 2. Create user
            self.create_user(conn, &tenant_config.username, &tenant_config.password)?;

            // 3. Grant permissions
            self.grant_schema_permissions(conn, &tenant_config.schema_name, &tenant_config.username)?;

            // 4. Run migrations in the schema
            self.run_migrations_in_schema(conn, &tenant_config.schema_name)?;

            Ok(())
        })
    }

    /// Remove a tenant (user + schema)
    pub fn remove_tenant(&self, schema_name: &str, username: &str) -> Result<(), AdminError> {
        let mut conn = self.database.pool().get()?;

        conn.transaction(|conn| {
            // 1. Revoke permissions
            self.revoke_schema_permissions(conn, schema_name, username)?;

            // 2. Drop user
            self.drop_user(conn, username)?;

            // 3. Drop schema (with CASCADE)
            self.drop_schema(conn, schema_name)?;

            Ok(())
        })
    }
}

pub struct TenantConfig {
    pub schema_name: String,
    pub username: String,
    pub password: String,
}
```

#### 1.2 Password Handling and Secure Generation
```rust
use rand::Rng;

/// Generate a cryptographically secure password
fn generate_secure_password(length: usize) -> String {
    let charset: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                          abcdefghijklmnopqrstuvwxyz\
                          0123456789\
                          !@#$%^&*()_+-=[]{}|;:,.<>?";
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..charset.len());
            charset[idx] as char
        })
        .collect()
}

impl DatabaseAdmin {
    /// Create a complete tenant setup with password handling
    pub fn create_tenant(&self, tenant_config: TenantConfig) -> Result<TenantCredentials, AdminError> {
        // Password logic: use provided password or generate secure one
        let final_password = if tenant_config.password.is_empty() {
            generate_secure_password(32)  // Auto-generate if none provided
        } else {
            tenant_config.password.clone()  // Use admin-provided password
        };

        let mut conn = self.database.pool().get()?;

        // Execute all tenant setup SQL in a transaction
        conn.transaction(|conn| {
            // 1. Create schema
            self.create_schema(conn, &tenant_config.schema_name)?;

            // 2. Create user with determined password
            self.create_user(conn, &tenant_config.username, &final_password)?;

            // 3. Grant permissions
            self.grant_schema_permissions(conn, &tenant_config.schema_name, &tenant_config.username)?;

            // 4. Run migrations in the schema
            self.run_migrations_in_schema(conn, &tenant_config.schema_name)?;

            Ok(())
        })?;

        // Return credentials for admin to share with tenant
        Ok(TenantCredentials {
            username: tenant_config.username,
            password: final_password,  // Either provided or generated
            schema_name: tenant_config.schema_name,
            connection_string: self.build_connection_string(&tenant_config.username, &final_password),
        })
    }

    fn build_connection_string(&self, username: &str, password: &str) -> String {
        // Note: This would need actual host/db values from admin connection
        format!("postgresql://{}:{}@host:5432/database", username, password)
    }
}

pub struct TenantConfig {
    pub schema_name: String,
    pub username: String,
    pub password: String,  // Empty string triggers auto-generation
}

pub struct TenantCredentials {
    pub username: String,
    pub password: String,      // Always returned - either provided or generated
    pub schema_name: String,
    pub connection_string: String,  // Ready-to-use connection string
}
```

#### 1.3 Core Administrative SQL Operations
```rust
impl DatabaseAdmin {
    fn create_schema(&self, conn: &mut PgConnection, schema_name: &str) -> Result<(), AdminError> {
        let sql = format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name);
        diesel::sql_query(&sql).execute(conn)?;
        Ok(())
    }

    fn create_user(&self, conn: &mut PgConnection, username: &str, password: &str) -> Result<(), AdminError> {
        // PostgreSQL handles password hashing automatically (SCRAM-SHA-256)
        let sql = format!("CREATE USER {} WITH PASSWORD '{}'", username, password);
        diesel::sql_query(&sql).execute(conn)?;
        Ok(())
    }

    fn grant_schema_permissions(&self, conn: &mut PgConnection, schema_name: &str, username: &str) -> Result<(), AdminError> {
        let sqls = vec![
            format!("GRANT USAGE ON SCHEMA {} TO {}", schema_name, username),
            format!("GRANT CREATE ON SCHEMA {} TO {}", schema_name, username),
            format!("GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA {} TO {}", schema_name, username),
            format!("GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA {} TO {}", schema_name, username),
            format!("ALTER DEFAULT PRIVILEGES IN SCHEMA {} GRANT ALL ON TABLES TO {}", schema_name, username),
            format!("ALTER DEFAULT PRIVILEGES IN SCHEMA {} GRANT ALL ON SEQUENCES TO {}", schema_name, username),
        ];

        for sql in sqls {
            diesel::sql_query(&sql).execute(conn)?;
        }
        Ok(())
    }

    fn run_migrations_in_schema(&self, conn: &mut PgConnection, schema_name: &str) -> Result<(), AdminError> {
        // Set search_path to the schema
        let set_path_sql = format!("SET search_path TO {}, public", schema_name);
        diesel::sql_query(&set_path_sql).execute(conn)?;

        // Run migrations
        crate::database::run_migrations(conn)?;

        Ok(())
    }
}
```

### Phase 2: Simple UnifiedExecutor Usage

#### 2.1 Keep UnifiedExecutor Simple
- **File**: `cloacina/src/executor/unified_executor.rs`
- **Purpose**: No detection logic - just use whatever credentials are provided

```rust
impl UnifiedExecutorBuilder {
    pub async fn build(self) -> Result<UnifiedExecutor, PipelineError> {
        // ... existing code ...

        // Simple approach: just use provided credentials and schema
        if let Some(ref schema) = self.schema {
            // Try to set up schema - will fail with clear error if permissions lacking
            database
                .setup_schema(schema)
                .map_err(|e| PipelineError::Configuration {
                    message: format!(
                        "Failed to set up schema '{}': {}.
                         If using tenant credentials, ensure schema and permissions
                         were created using DatabaseAdmin::create_tenant()",
                        schema, e
                    ),
                })?;
        }

        // ... rest of existing code unchanged ...
    }
}
```

### Phase 3: Example and Documentation

#### 3.1 Complete Usage Examples
- **File**: `examples/per_tenant_credentials/`
- **Purpose**: Show the full admin → tenant workflow with different password scenarios

```rust
use cloacina::database::{Database, DatabaseAdmin, TenantConfig};
use cloacina::executor::UnifiedExecutor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Admin sets up database connection
    let admin_db = Database::new("postgresql://admin_user:admin_pw@host/db", "cloacina", 10);
    let admin = DatabaseAdmin::new(admin_db);

    // Scenario A: Admin provides password
    let tenant_a_creds = admin.create_tenant(TenantConfig {
        schema_name: "tenant_acme".to_string(),
        username: "acme_user".to_string(),
        password: "admin_chosen_password".to_string(),
    })?;

    println!("Tenant A created with admin-provided password");
    println!("Username: {}", tenant_a_creds.username);
    println!("Password: {}", tenant_a_creds.password); // "admin_chosen_password"

    // Scenario B: Auto-generated secure password
    let tenant_b_creds = admin.create_tenant(TenantConfig {
        schema_name: "tenant_xyz".to_string(),
        username: "xyz_user".to_string(),
        password: "".to_string(),  // Empty = auto-generate
    })?;

    println!("Tenant B created with auto-generated password");
    println!("Username: {}", tenant_b_creds.username);
    println!("Password: {}", tenant_b_creds.password); // Generated 32-char secure password
    println!("Connection string: {}", tenant_b_creds.connection_string);

    // Step 2: Applications use tenant credentials
    let tenant_a_executor = UnifiedExecutor::with_schema(
        &tenant_a_creds.connection_string,
        &tenant_a_creds.schema_name
    ).await?;

    let tenant_b_executor = UnifiedExecutor::with_schema(
        &tenant_b_creds.connection_string,
        &tenant_b_creds.schema_name
    ).await?;

    // Step 3: Normal operations work with full isolation
    // Each executor operates in complete isolation from the other
    println!("Both tenants ready for normal operations!");

    Ok(())
}
```

#### 3.2 Password Security Documentation

**Cryptographic Security:**
- **PostgreSQL handles all password hashing** using SCRAM-SHA-256 (PostgreSQL 10+) or MD5 (older versions)
- **Cloacina never stores passwords** - they are passed directly to PostgreSQL and immediately forgotten
- **Auto-generated passwords** use cryptographically secure random generation with 94-character charset
- **32-character default length** provides ~202 bits of entropy

**Password Flow:**
1. Admin calls `create_tenant()` with either provided password or empty string
2. If empty, Cloacina generates secure 32-character password using `rand::thread_rng()`
3. Password (provided or generated) is passed to PostgreSQL via `CREATE USER`
4. PostgreSQL hashes and stores password securely
5. Cloacina returns credentials to admin for sharing with tenant
6. Cloacina forgets the plaintext password immediately

**Security Responsibilities:**
- **Cloacina**: Secure password generation, immediate PostgreSQL handoff
- **PostgreSQL**: Password hashing, storage, authentication
- **Admin/Consumer**: Secure credential distribution and storage (secrets management, etc.)

### Phase 4: Testing and Examples

#### 4.1 Integration Tests
- **File**: `cloacina/tests/integration/executor/per_tenant_credentials.rs`
- **Coverage**:
  - Verify existing `with_schema` works with per-tenant credentials
  - Permission validation and error handling
  - Migration execution with tenant-specific users
  - Isolation verification between tenant users

#### 4.2 Example Application
- **File**: `examples/per_tenant_credentials/`
- **Demonstrates**:
  - Setting up tenant-specific PostgreSQL users
  - Using `with_schema` with different credentials
  - Operational patterns and best practices

### Phase 5: Documentation

#### 5.1 Enhanced Documentation
- Update multi-tenancy docs to show per-tenant credential option
- Add security considerations and operational guidance
- Document PostgreSQL user setup procedures

#### 5.2 Best Practices Guide
- Recommended PostgreSQL permission patterns
- Credential management strategies
- Monitoring and troubleshooting

## Implementation Reality Check

### What Currently Does NOT Work
After examining the code, per-tenant credentials will fail because:

```rust
// This FAILS today with tenant-specific credentials:
let isolated_executor = UnifiedExecutor::with_schema(
    "postgresql://tenant_a_user:tenant_a_pw@host/db",
    "tenant_a"
).await?;
// Fails because tenant_a_user likely can't CREATE SCHEMA or run migrations
```

### Current Implementation Problems
1. **Schema Creation**: `setup_schema()` runs `CREATE SCHEMA IF NOT EXISTS`
2. **Migration Execution**: Runs all migrations as the connected user
3. **No Permission Checks**: Assumes user has admin privileges
4. **No User Grants**: No setup of tenant user permissions

### What We Need to Implement
1. **Permission Detection**: Check if user can create schemas/tables
2. **Operational Modes**: Admin setup vs runtime operations
3. **Migration Strategy**: Handle limited-privilege tenant users
4. **Error Handling**: Clear messages for permission failures

## Backwards Compatibility

### Perfect Backwards Compatibility
- All existing `UnifiedExecutor::with_schema()` calls continue to work unchanged
- Current shared credential deployments remain fully functional
- Zero breaking changes to public APIs
- Same method, just different connection strings

### Migration Path (Zero API Changes!)
```rust
// Phase 1: Existing shared credentials (no changes)
let executor = UnifiedExecutor::with_schema(
    "postgresql://shared_user:shared_pw@host/db",
    "tenant_a"
).await?;

// Phase 2: Mixed mode (some tenants migrated)
let shared_executor = UnifiedExecutor::with_schema(
    "postgresql://shared_user:shared_pw@host/db",
    "tenant_a"
).await?;
let isolated_executor = UnifiedExecutor::with_schema(
    "postgresql://tenant_a_user:tenant_a_pw@host/db",
    "tenant_a"
).await?;

// Phase 3: Full per-tenant credentials (same method!)
let executor = UnifiedExecutor::with_schema(
    "postgresql://tenant_a_user:tenant_a_pw@host/db",
    "tenant_a"
).await?;
```

## Operational Considerations

### Admin User Requirements
- `CREATEDB`, `CREATEROLE` privileges for tenant provisioning
- Superuser not required (good for managed PostgreSQL services)
- Can use separate admin connection for provisioning vs runtime

### Tenant User Permissions
```sql
-- Minimal required permissions
GRANT USAGE ON SCHEMA tenant_xyz TO tenant_xyz_user;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA tenant_xyz TO tenant_xyz_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA tenant_xyz TO tenant_xyz_user;

-- For migrations
GRANT CREATE ON SCHEMA tenant_xyz TO tenant_xyz_user;

-- Default search_path
ALTER USER tenant_xyz_user SET search_path TO tenant_xyz, public;
```

### Connection Pool Scaling
- Each tenant gets dedicated connection pool with their credentials
- Monitor total connection count across all tenants
- Consider connection limit implications for large tenant counts

## Security Benefits

1. **Defense in Depth**: Database-level access control in addition to application-level
2. **Audit Trail**: PostgreSQL logs show which tenant user performed operations
3. **Credential Rotation**: Independent password rotation per tenant
4. **Compliance**: Meet regulations requiring database-level user separation

## Error Handling

### Permission Errors
```rust
// Clear error messages for common issues
PipelineError::DatabaseAccess {
    message: "Tenant user 'tenant_xyz_user' lacks CREATE permission on schema 'tenant_xyz'.
              Ensure user was created with proper migration privileges."
}
```

### Schema Detection Failures
```rust
PipelineError::Configuration {
    message: "Unable to auto-detect schema for user 'tenant_xyz_user'.
              User may not have a default search_path configured."
}
```

## Testing Strategy

### Unit Tests
- Schema detection logic
- Permission validation
- SQL generation functions

### Integration Tests
- End-to-end tenant provisioning
- Schema isolation verification
- Migration execution with tenant users
- Error handling scenarios

### Performance Tests
- Connection pool behavior with multiple tenant users
- Schema detection query performance
- Migration execution timing

## Simplified Rollout Plan

### Phase 1: Database Admin Module (3-4 days)
- Create `cloacina/src/database/admin.rs`
- Implement `DatabaseAdmin` with `create_tenant` and `remove_tenant`
- Add proper SQL for user creation, schema setup, and permissions
- Include migration execution within schema context

### Phase 2: Error Message Improvement (1-2 days)
- Update `UnifiedExecutor` error messages to mention `DatabaseAdmin`
- No logic changes - just better guidance when setup fails
- Maintain full backwards compatibility

### Phase 3: Example and Testing (2-3 days)
- Create complete example showing admin → tenant workflow
- Integration tests with actual PostgreSQL tenant users
- Verify isolation and functionality end-to-end

### Phase 4: Documentation (1-2 days)
- Update multi-tenancy docs to show per-tenant credential option
- Document the `DatabaseAdmin` usage pattern
- Add operational guidance

## Success Criteria

1. **Functionality**: Existing `with_schema` API works with per-tenant credentials
2. **Zero Breaking Changes**: All existing code continues to work unchanged
3. **Security**: Database-level isolation verified through testing
4. **Usability**: Clear documentation and examples
5. **Simplicity**: No complex auto-detection or naming conventions

## Key Advantages of This Approach

1. **Minimal Implementation**: No API changes, just documentation and testing
2. **Maximum Flexibility**: Users structure credentials however they want
3. **Progressive Adoption**: Can migrate tenants one at a time
4. **Operational Simplicity**: Same deployment patterns, just different connection strings

## Next Steps

1. **Validate the approach**: Test with actual tenant PostgreSQL user
2. **Create integration tests**: Verify isolation and functionality
3. **Build example**: Show complete tenant provisioning workflow
4. **Document patterns**: Best practices and operational guidance

**Bottom line**: This feature requires significant implementation work to handle permission detection and conditional schema setup, but maintains the same clean API.
