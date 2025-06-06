# Per-Tenant Credentials Example

This example demonstrates how to use Cloacina's `DatabaseAdmin` to create database-level isolation for multi-tenant applications using PostgreSQL.

## What This Example Shows

1. **Admin Tenant Creation**: How an administrator creates isolated tenant users with dedicated database credentials
2. **Password Handling**: Both admin-provided and auto-generated secure passwords
3. **Schema Isolation**: Each tenant gets their own PostgreSQL schema with proper permissions
4. **API Compatibility**: The same `DefaultRunner::with_schema()` API works with both shared and per-tenant credentials

## Security Benefits

- **Database-level isolation**: Each tenant uses their own PostgreSQL user
- **Principle of least privilege**: Tenant users can only access their own schema
- **Audit trail**: PostgreSQL logs show which tenant user performed operations
- **Credential rotation**: Independent password rotation per tenant
- **Defense in depth**: Database-level access control in addition to application-level

## Prerequisites

1. **PostgreSQL Running**: Docker Compose or local PostgreSQL instance
2. **Admin Privileges**: The admin user needs `CREATEDB` and `CREATEROLE` privileges

## Running the Example

```bash
# Start PostgreSQL (if using Docker)
docker run --name postgres-cloacina \
  -e POSTGRES_USER=cloacina \
  -e POSTGRES_PASSWORD=cloacina \
  -e POSTGRES_DB=cloacina \
  -p 5432:5432 \
  -d postgres:13

# Run the example
cargo run --bin per_tenant_credentials

# Or with custom admin credentials
ADMIN_DATABASE_URL="postgresql://admin_user:admin_pass@localhost/cloacina" \
cargo run --bin per_tenant_credentials
```

## Expected Output

The example will:

1. **Create tenants** with both admin-provided and auto-generated passwords
2. **Show credential details** (with passwords masked in logs)
3. **Demonstrate API usage** patterns for per-tenant credentials
4. **Explain security benefits** of database-level isolation

## Password Security

- **Auto-generated passwords**: 32 characters with 94-character charset (~202 bits entropy)
- **PostgreSQL hashing**: All passwords hashed with SCRAM-SHA-256
- **No storage**: Cloacina never stores plaintext passwords
- **Secure distribution**: Admin receives credentials for secure distribution to tenants

## Real-World Usage

In production, the workflow would be:

```rust
// 1. Admin creates tenant
let admin = DatabaseAdmin::new(admin_database);
let creds = admin.create_tenant(TenantConfig {
    schema_name: "tenant_acme".to_string(),
    username: "acme_user".to_string(),
    password: "".to_string(), // Auto-generate secure password
})?;

// 2. Admin securely distributes credentials to tenant
// (via secrets management, secure communication, etc.)

// 3. Tenant application uses dedicated credentials
let executor = DefaultRunner::with_schema(
    &creds.connection_string,
    &creds.schema_name
).await?;
```

## Migration Path

Per-tenant credentials are fully backwards compatible:

```rust
// Existing code (shared credentials)
let executor = DefaultRunner::with_schema(
    "postgresql://shared_user:shared_pw@host/db",
    "tenant_a"
).await?;

// Enhanced security (per-tenant credentials)
let executor = DefaultRunner::with_schema(
    "postgresql://tenant_a_user:tenant_a_pw@host/db",
    "tenant_a"
).await?;
```

Same API, same functionality, enhanced security!

## Troubleshooting

**Permission errors**: Ensure the admin user has `CREATEDB` and `CREATEROLE` privileges:
```sql
ALTER USER admin_user CREATEDB CREATEROLE;
```

**Connection errors**: Verify PostgreSQL is running and accessible with the provided credentials.

**Schema errors**: The example handles permission failures gracefully and explains what might be wrong.
