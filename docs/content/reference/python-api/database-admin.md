---
title: "Database Admin"
description: "Python API reference for multi-tenant database administration"
weight: 50
reviewer: "automation"
review_date: "2025-06-08"
aliases:
  - "/python/api-reference/database-admin/"

---

# Database Admin API

The Database Admin API provides Python bindings for multi-tenant database administration in PostgreSQL deployments.

These classes (`DatabaseAdmin`, `TenantConfig`, `TenantCredentials`) are
**wheel-only** and gated behind the wheel's `postgres` Cargo feature
(`crates/cloacina-python/src/lib.rs`). The published PyPI wheel enables both
backends, so `pip install cloaca` includes them; they are absent from a
SQLite-only custom build and from the authoring surface inside packaged
workflows. At runtime the constructor additionally rejects non-PostgreSQL
URLs.

## DatabaseAdmin

The main class for administrative operations on multi-tenant PostgreSQL databases.

### Constructor

```python
DatabaseAdmin(database_url: str)
```

**Parameters:**
- `database_url` (str): PostgreSQL connection string with administrative privileges. The URL must start with `postgres://` or `postgresql://` and include a database name in the path; anything else raises `RuntimeError`.

**Example:**
```python
import cloaca

admin = cloaca.DatabaseAdmin("postgresql://admin:password@localhost:5432/mydb")
```

### Methods

#### create_tenant

```python
create_tenant(config: TenantConfig) -> TenantCredentials
```

Creates a new tenant with dedicated schema and database user.

**Parameters:**
- `config` (TenantConfig): Configuration for the new tenant

**Returns:**
- `TenantCredentials`: Credentials and connection information for the new tenant

**Raises:** `RuntimeError` on failure (insufficient privileges, schema or
username already exists, connection issues, invalid schema/username).

**Example:**
```python
config = cloaca.TenantConfig(
    schema_name="tenant_acme",
    username="acme_user",
    # password omitted — auto-generate a secure password
)

credentials = admin.create_tenant(config)
print(f"Tenant created with schema: {credentials.schema_name}")
print(f"Connection string: {credentials.connection_string}")
```

#### remove_tenant

```python
remove_tenant(schema_name: str, username: str) -> None
```

Drops the tenant's schema and database user.

**Parameters:**
- `schema_name` (str): Schema of the tenant to remove
- `username` (str): The tenant's database username

**Raises:** `RuntimeError` on failure.

**Example:**
```python
admin.remove_tenant("tenant_acme", "acme_user")
```

## TenantConfig

Configuration object for creating new tenants.

### Constructor

```python
TenantConfig(schema_name: str, username: str, password: str | None = None)
```

**Parameters:**
- `schema_name` (str): Name of the PostgreSQL schema for this tenant
- `username` (str): Database username for this tenant
- `password` (str, optional): Password for the user. Omitted, `None`, or empty string means auto-generate a secure 32-character password at `create_tenant` time

**Example:**
```python
# With admin-provided password
config = cloaca.TenantConfig(
    schema_name="tenant_acme",
    username="acme_user",
    password="secure_password123"
)

# With auto-generated password
config = cloaca.TenantConfig(
    schema_name="tenant_acme",
    username="acme_user",
)
```

### Attributes (read-only)

- `schema_name` (str): The schema name for the tenant
- `username` (str): The database username for the tenant
- `password` (str): The password as configured (empty string when auto-generation was requested; the generated password is returned on `TenantCredentials`)

## TenantCredentials

Returned credentials and connection information for a newly created tenant.

### Attributes

- `username` (str): Database username for the tenant
- `password` (str): Database password for the tenant
- `schema_name` (str): PostgreSQL schema name for the tenant
- `connection_string` (str): Complete PostgreSQL connection string for the tenant

**Example:**
```python
credentials = admin.create_tenant(config)

# Access individual components
print(f"Username: {credentials.username}")
print(f"Password: {credentials.password}")
print(f"Schema: {credentials.schema_name}")

# Use connection string directly
runner = cloaca.DefaultRunner(credentials.connection_string)
```

## Usage Patterns

### Basic Tenant Provisioning

```python
import cloaca

# Set up admin connection
admin = cloaca.DatabaseAdmin("postgresql://admin:admin@localhost:5432/myapp")

# Create tenant with auto-generated password
config = cloaca.TenantConfig(
    schema_name="tenant_customer123",
    username="customer123_user",
    password=""  # Auto-generate
)

credentials = admin.create_tenant(config)

# Store credentials securely for the customer
# In production, you would save these to your user management system
```

### SaaS Application Integration

```python
class TenantManager:
    def __init__(self, admin_db_url: str):
        self.admin = cloaca.DatabaseAdmin(admin_db_url)
        self.tenant_runners = {}

    def onboard_customer(self, customer_id: str) -> dict:
        """Provision new customer tenant"""
        config = cloaca.TenantConfig(
            schema_name=f"tenant_{customer_id}",
            username=f"{customer_id}_user",
            password=""  # Auto-generate secure password
        )

        credentials = self.admin.create_tenant(config)

        # Create dedicated runner for this tenant
        runner = cloaca.DefaultRunner(credentials.connection_string)
        self.tenant_runners[customer_id] = runner

        return {
            "tenant_id": customer_id,
            "schema": credentials.schema_name,
            "username": credentials.username,
            "connection_ready": True
        }

    def get_tenant_runner(self, customer_id: str):
        """Get workflow runner for specific tenant"""
        return self.tenant_runners.get(customer_id)
```

## Security Considerations

### Password Generation

When `password` is empty or not provided, the system generates a secure 32-character password using:
- Uppercase letters (A-Z)
- Lowercase letters (a-z)
- Numbers (0-9)

Special characters are excluded to avoid URL encoding issues in connection strings.

### Permissions

Created tenant users have:
- Full access to their dedicated schema
- No access to other tenants' schemas
- No access to administrative functions
- No access to the public schema (by design)

### Connection Strings

Generated connection strings use unencoded passwords. The underlying database driver handles any necessary encoding automatically.

## Error Handling

All admin failures surface as `RuntimeError`
(see [Exceptions]({{< ref "/reference/python-api/exceptions/" >}})):

```python
try:
    credentials = admin.create_tenant(config)
except RuntimeError as e:
    print(f"Failed to create tenant: {e}")
    # Common causes:
    # - Admin user lacks necessary privileges
    # - Schema or username already exists
    # - Database connection issues
```

## See Also

- [Multi-Tenancy Guide]({{< ref "/service/explanation/multi-tenancy" >}})
- [Multi-Tenant Setup]({{< ref "/service/how-to/multi-tenant-setup" >}})
- [Rust Database Admin API]({{< ref "/reference/database-admin" >}})
- [Tutorial: Multi-Tenancy]({{< ref "/embed/tutorials/06-multi-tenancy" >}})
