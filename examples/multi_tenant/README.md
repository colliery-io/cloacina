# Multi-Tenant Workflow Execution Example

This example demonstrates Cloacina's multi-tenant capabilities, showing how to achieve complete data isolation for different tenants or services.

## Multi-Tenancy Approaches

### PostgreSQL: Schema-Based Isolation

For PostgreSQL, Cloacina uses schema-based multi-tenancy for complete data isolation:

```rust
// Create tenant-specific runners
let tenant_a = DefaultRunner::with_schema(
    "postgresql://user:pass@localhost/cloacina",
    "tenant_a"
).await?;

let tenant_b = DefaultRunner::with_schema(
    "postgresql://user:pass@localhost/cloacina",
    "tenant_b"
).await?;
```

Each schema provides:
- Complete data isolation - no cross-tenant data access possible
- Zero collision risk - each tenant operates in their own namespace
- Native PostgreSQL feature - battle-tested and performant
- Automatic schema creation and migration on first use

### SQLite: File-Based Isolation

For SQLite, simply use different database files:

```rust
// Each tenant gets their own database file
let tenant_a = DefaultRunner::with_config(
    "sqlite://./tenant_a.db",
    DefaultRunnerConfig::default()
).await?;
let tenant_b = DefaultRunner::with_config(
    "sqlite://./tenant_b.db",
    DefaultRunnerConfig::default()
).await?;
```

## Running the Example

### Prerequisites

1. **Docker** (for PostgreSQL):
   The example uses the same Docker PostgreSQL setup as the rest of Cloacina.

   ```bash
   # From the project root
   angreal services up  # or docker-compose up -d
   ```

   This will start PostgreSQL with:
   - User: `cloacina`
   - Password: `cloacina`
   - Database: `cloacina`

2. **Environment Variables** (optional):
   ```bash
   # Default uses Docker PostgreSQL, but you can override:
   export DATABASE_URL="postgresql://cloacina:cloacina@localhost:5432/cloacina"
   ```

### Run the Example

```bash
cd examples/multi_tenant
cargo run

# Or using angreal (from project root)
angreal examples multi-tenant
```

## Key Benefits

1. **Zero Collision Risk**: Impossible for tenants to access each other's data
2. **No Query Changes**: All existing DAL code works unchanged
3. **Performance**: No overhead from filtering every query
4. **Clean Separation**: Each tenant can even have different schema versions
5. **Simple Migration**: Can run side-by-side with existing single-tenant deployments

## Production Usage

### Environment-Based Configuration

```rust
let tenant_id = env::var("TENANT_ID")?;
let database_url = env::var("DATABASE_URL")?;

let runner = DefaultRunner::with_schema(&database_url, &tenant_id).await?;
```

### Service-Based Isolation

```rust
// API service
let api_runner = DefaultRunner::with_schema(db_url, "api_service").await?;

// Background job processor
let batch_runner = DefaultRunner::with_schema(db_url, "batch_processor").await?;

// Analytics service
let analytics_runner = DefaultRunner::with_schema(db_url, "analytics").await?;
```

## Schema Naming Rules

Schema names must contain only:
- Alphanumeric characters (a-z, A-Z, 0-9)
- Underscores (_)

Examples:
- ✅ `tenant_123`
- ✅ `acme_corp`
- ✅ `production_api`
- ❌ `tenant-123` (hyphens not allowed)
- ❌ `tenant 123` (spaces not allowed)
- ❌ `tenant@123` (special characters not allowed)
