---
title: "Database Admin"
description: "Python API reference for multi-tenant database administration"
weight: 50
reviewer: "automation"
review_date: "2025-06-08"
---

# Database Admin API

The Database Admin API provides Python bindings for multi-tenant database administration in PostgreSQL deployments. These classes are only available when using the `cloaca_postgres` backend.

## DatabaseAdmin

The main class for administrative operations on multi-tenant PostgreSQL databases.

### Constructor

```python
DatabaseAdmin(database_url: str)
```

**Parameters:**
- `database_url` (str): PostgreSQL connection string with administrative privileges

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

**Example:**
```python
config = cloaca.TenantConfig(
    schema_name="tenant_acme",
    username="acme_user",
    password=""  # Auto-generate secure password
)

credentials = admin.create_tenant(config)
print(f"Tenant created with schema: {credentials.schema_name}")
print(f"Connection string: {credentials.connection_string}")
```

## TenantConfig

Configuration object for creating new tenants.

### Constructor

```python
TenantConfig(schema_name: str, username: str, password: str)
```

**Parameters:**
- `schema_name` (str): Name of the PostgreSQL schema for this tenant
- `username` (str): Database username for this tenant
- `password` (str): Password for the user, or empty string to auto-generate

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
    password=""  # Will generate secure 32-character password
)
```

### Attributes

- `schema_name` (str): The schema name for the tenant
- `username` (str): The database username for the tenant
- `password` (str): The password (may be auto-generated)

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

```python
try:
    credentials = admin.create_tenant(config)
except Exception as e:
    print(f"Failed to create tenant: {e}")
    # Handle tenant creation failure
    # Common causes:
    # - Admin user lacks necessary privileges
    # - Schema or username already exists
    # - Database connection issues
```

## See Also

- [Multi-Tenancy Guide]({{< ref "/explanation/multi-tenancy" >}})
- [Multi-Tenant Setup]({{< ref "/how-to-guides/multi-tenant-setup" >}})
- [Rust Database Admin API]({{< ref "/reference/database-admin" >}})
- [Tutorial: Multi-Tenancy]({{< ref "/python-bindings/tutorials/06-multi-tenancy" >}})
