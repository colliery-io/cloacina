# Tutorial 06: Multi-Tenancy

This example demonstrates how to deploy isolated workflows for multiple tenants using PostgreSQL schema-based multi-tenancy and the Database Admin API.

## Features Demonstrated

- Schema-based tenant isolation
- Basic multi-tenant setup with DefaultRunner
- Advanced tenant provisioning with Database Admin API
- Tenant-specific workflow execution
- Complete data isolation between tenants

## Prerequisites

- PostgreSQL database running
- Admin privileges for tenant provisioning (advanced demo)

## Running the Example

### With PostgreSQL

```bash
# Start PostgreSQL (if using Docker)
docker run --name tutorial-postgres \
  -e POSTGRES_USER=cloacina \
  -e POSTGRES_PASSWORD=cloacina \
  -e POSTGRES_DB=cloacina \
  -p 5432:5432 \
  -d postgres:15

# Run the tutorial
cd examples/tutorial-06
cargo run
```

### Environment Variables

- `DATABASE_URL` - PostgreSQL connection string (optional)

## What You'll See

The example demonstrates two approaches:

1. **Basic Multi-Tenancy** - Using schema isolation with shared credentials
2. **Advanced Admin API** - Using dedicated tenant credentials and provisioning

## Key Concepts

- **Schema Isolation** - Each tenant gets their own PostgreSQL schema
- **Data Security** - Complete isolation prevents cross-tenant data access
- **Scalability** - Native PostgreSQL performance with tenant separation
- **Admin API** - Automated tenant provisioning and management

## Related Documentation

- [Tutorial 06: Multi-Tenancy](/tutorials/06-multi-tenancy/)
- [Multi-Tenancy Architecture](/explanation/multi-tenancy/)
- [Database Admin API](/reference/database-admin/)
