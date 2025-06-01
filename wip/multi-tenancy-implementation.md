# Cloacina Multi-Tenancy Implementation Plan

**Project**: PostgreSQL Schema-Based Multi-Tenancy
**Issue**: https://github.com/colliery-io/cloacina/issues/10
**Status**: Planning Phase
**Created**: 2025-05-31
**Updated**: 2025-05-31

## Executive Summary

Enable multiple independent executors to share a single PostgreSQL database using schema isolation. Each executor operates in its own PostgreSQL schema, providing complete isolation with zero collision risk. SQLite users should simply use separate database files.

## Core Design

**PostgreSQL only**: Use native schema support for perfect isolation
**SQLite approach**: Use separate database files (no code changes needed)

## Implementation

### 1. Update UnifiedExecutorBuilder

```rust
pub struct UnifiedExecutorBuilder {
    database_url: Option<String>,
    schema: Option<String>,  // New field
}

impl UnifiedExecutorBuilder {
    pub fn database_url(mut self, url: &str) -> Self {
        self.database_url = Some(url.to_string());
        self
    }

    pub fn schema(mut self, schema: &str) -> Self {
        self.schema = Some(schema.to_string());
        self
    }

    pub async fn build(self) -> Result<UnifiedExecutor, PipelineError> {
        let database_url = self.database_url.ok_or(PipelineError::Configuration)?;

        // Validate schema is only used with PostgreSQL
        if self.schema.is_some() && !database_url.starts_with("postgresql://") {
            return Err(PipelineError::Configuration(
                "Schema isolation is only supported with PostgreSQL. \
                 For SQLite, use separate database files instead.".to_string()
            ));
        }

        let dal = Arc::new(DAL::new(&database_url).await?);

        // Set schema if provided
        if let Some(schema) = &self.schema {
            dal.set_schema(schema).await?;
        }

        // Create components - no changes needed!
        let scheduler = Arc::new(Scheduler::new(dal.clone()));
        let task_executor = Arc::new(TaskExecutor::new(dal.clone()));
        let pipeline_executor = Arc::new(PipelineExecutor::new(dal.clone()));

        Ok(UnifiedExecutor {
            dal,
            scheduler,
            task_executor,
            pipeline_executor,
        })
    }
}
```

### 2. Add Schema Support to DAL

```rust
impl DAL {
    pub async fn set_schema(&self, schema: &str) -> Result<(), DatabaseError> {
        // Validate schema name (alphanumeric + underscore only)
        if !schema.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(DatabaseError::InvalidSchema(
                "Schema name must be alphanumeric with underscores only".to_string()
            ));
        }

        let conn = self.pool.get()?;

        // Create schema if it doesn't exist
        conn.execute(&format!(
            "CREATE SCHEMA IF NOT EXISTS {}",
            schema
        ))?;

        // Set search path for this connection pool
        // Include 'public' for extensions like uuid-ossp
        conn.execute(&format!(
            "SET search_path TO {}, public",
            schema
        ))?;

        // Run migrations in this schema
        self.run_migrations_in_schema(schema)?;

        Ok(())
    }

    fn run_migrations_in_schema(&self, schema: &str) -> Result<(), DatabaseError> {
        // Set search path before running migrations
        let conn = self.pool.get()?;
        conn.execute(&format!("SET search_path TO {}, public", schema))?;

        // Run normal migrations - they'll create tables in the current schema
        run_migrations(&*conn)?;

        Ok(())
    }
}
```

### 3. Connection Pool Configuration

```rust
// Ensure all connections in the pool use the correct schema
fn create_pool_with_schema(database_url: &str, schema: Option<&str>) -> Result<DbPool, DatabaseError> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .connection_customizer(Box::new(SchemaCustomizer {
            schema: schema.map(String::from),
        }))
        .build(manager)?;
    Ok(pool)
}

#[derive(Debug, Clone)]
struct SchemaCustomizer {
    schema: Option<String>,
}

impl CustomizeConnection<PgConnection, r2d2_diesel::Error> for SchemaCustomizer {
    fn on_acquire(&self, conn: &mut PgConnection) -> Result<(), r2d2_diesel::Error> {
        if let Some(schema) = &self.schema {
            conn.execute(&format!("SET search_path TO {}, public", schema))
                .map_err(r2d2_diesel::Error::QueryError)?;
        }
        Ok(())
    }
}
```

### 4. Backward Compatibility

```rust
impl UnifiedExecutor {
    /// Existing API remains unchanged
    pub async fn new(database_url: &str) -> Result<Self, PipelineError> {
        Self::builder()
            .database_url(database_url)
            .build()
            .await
    }

    /// Convenience constructor for schema-based multi-tenancy
    pub async fn with_schema(database_url: &str, schema: &str) -> Result<Self, PipelineError> {
        Self::builder()
            .database_url(database_url)
            .schema(schema)
            .build()
            .await
    }
}
```

## Usage Examples

### PostgreSQL with Schemas
```rust
// Each service gets its own schema
let api_executor = UnifiedExecutor::builder()
    .database_url("postgresql://user:pass@localhost/cloacina")
    .schema("api_service")
    .build()
    .await?;

let batch_executor = UnifiedExecutor::builder()
    .database_url("postgresql://user:pass@localhost/cloacina")
    .schema("batch_processor")
    .build()
    .await?;

// Complete isolation - no data visible between executors
```

### SQLite with Separate Files
```rust
// Each service gets its own database file
let api_executor = UnifiedExecutor::new("sqlite://./api_service.db").await?;
let batch_executor = UnifiedExecutor::new("sqlite://./batch_processor.db").await?;

// Natural isolation - completely separate databases
```

### Environment-Based Configuration
```rust
let schema = env::var("SERVICE_NAME").unwrap_or_else(|_| "default".to_string());

let executor = UnifiedExecutor::builder()
    .database_url(&env::var("DATABASE_URL")?)
    .schema(&schema)
    .build()
    .await?;
```

## Migration Strategy

### For New PostgreSQL Deployments
No migration needed - just start using schemas:
```rust
let executor = UnifiedExecutor::with_schema(db_url, "my_service").await?;
```

### For Existing PostgreSQL Deployments

Option 1: Move existing data to a schema
```sql
BEGIN;
-- Create new schema
CREATE SCHEMA legacy_data;

-- Move all tables
ALTER TABLE pipeline_executions SET SCHEMA legacy_data;
ALTER TABLE task_executions SET SCHEMA legacy_data;
ALTER TABLE contexts SET SCHEMA legacy_data;
ALTER TABLE task_execution_metadata SET SCHEMA legacy_data;
-- ... repeat for all tables

-- Update search_path for legacy executor
-- Use schema("legacy_data") in code
COMMIT;
```

Option 2: Keep existing data in public schema
```rust
// Existing executors continue to work unchanged
let legacy = UnifiedExecutor::new(db_url).await?;

// New executors use schemas
let new_service = UnifiedExecutor::with_schema(db_url, "new_service").await?;
```

### For SQLite Users
No changes needed - continue using separate files:
```bash
# Instead of schemas, use multiple database files
./api_service.db
./batch_processor.db
./test_runner.db
```

## Testing

```rust
#[tokio::test]
async fn test_schema_isolation() {
    let db_url = "postgresql://test@localhost/cloacina_test";

    // Create two executors with different schemas
    let executor1 = UnifiedExecutor::with_schema(db_url, "tenant_1").await?;
    let executor2 = UnifiedExecutor::with_schema(db_url, "tenant_2").await?;

    // Create workflow in schema 1
    let result1 = executor1.execute("test_workflow", Context::new()).await?;

    // Executor 2 cannot see executor 1's data
    let workflows = executor2.list_workflows().await?;
    assert_eq!(workflows.len(), 0);

    // Each schema is completely isolated
}

#[tokio::test]
async fn test_sqlite_rejection() {
    let result = UnifiedExecutor::builder()
        .database_url("sqlite://test.db")
        .schema("not_allowed")
        .build()
        .await;

    assert!(matches!(result, Err(PipelineError::Configuration(_))));
}

#[tokio::test]
async fn test_invalid_schema_name() {
    let db_url = "postgresql://test@localhost/cloacina_test";

    // Schema names with special characters should fail
    let result = UnifiedExecutor::with_schema(db_url, "tenant-1-prod").await;
    assert!(result.is_err());

    // Alphanumeric + underscore should work
    let result = UnifiedExecutor::with_schema(db_url, "tenant_1_prod").await;
    assert!(result.is_ok());
}
```

## Implementation Timeline

### Day 1: Core Schema Support
- [ ] Add schema field to UnifiedExecutorBuilder
- [ ] Implement DAL::set_schema() method
- [ ] Update connection pool configuration
- [ ] Test schema creation and switching

### Day 2: Migration Support
- [ ] Update migration runner for schema context
- [ ] Test migrations in non-public schemas
- [ ] Handle schema-specific migration tracking
- [ ] Document migration strategies

### Day 3: Testing & Edge Cases
- [ ] Comprehensive isolation tests
- [ ] Schema name validation
- [ ] Connection pool behavior verification
- [ ] Performance impact assessment

### Day 4: Documentation & Examples
- [ ] Update API documentation
- [ ] Create multi-tenant example
- [ ] Write migration guide
- [ ] Update README with multi-tenancy section

## Benefits of This Approach

1. **Zero collision risk** - Impossible to access wrong schema
2. **No query changes** - All existing DAL code works unchanged
3. **Native PostgreSQL feature** - Battle-tested isolation
4. **Simple migration** - Can run side-by-side with existing deployment
5. **Clean separation** - Each schema can even have different table versions
6. **Performance** - No overhead from filtering every query

## Limitations

1. **PostgreSQL only** - But SQLite users have a natural alternative
2. **No cross-schema queries** - But that's probably desired
3. **Schema name validation** - Must be valid PostgreSQL identifiers

## Security Considerations

- Schema names should be validated (alphanumeric + underscore only)
- Each schema requires migration permissions on first use
- Consider using database roles per schema for additional isolation
- Schema names might be visible to other schemas (metadata)

---

**Estimated effort**: 3-4 days
**Risk**: Very low - using native PostgreSQL features
**Breaking changes**: None - opt-in feature only
