---
title: "Setting Up Multi-Tenant Workflows"
description: "Step-by-step guide to implementing multi-tenant workflow execution"
weight: 40
---

# Setting Up Multi-Tenant Workflows

This guide walks you through setting up multi-tenant workflow execution in Cloacina, covering both PostgreSQL schema-based and SQLite file-based approaches.

## Prerequisites

- Cloacina with PostgreSQL or SQLite features enabled
- Database server running (PostgreSQL) or file system access (SQLite)
- Basic understanding of Cloacina workflows

## PostgreSQL Schema-Based Setup

### Step 1: Enable PostgreSQL Features

Add Cloacina with PostgreSQL support to your `Cargo.toml`:

```toml
[dependencies]
cloacina = { version = "0.1.0", features = ["postgres"] }
tokio = { version = "1.0", features = ["full"] }
```

### Step 2: Create Multi-Tenant Executors

```rust
use cloacina::executor::unified_executor::UnifiedExecutor;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = "postgresql://user:pass@localhost/cloacina";

    // Create executors for different tenants
    let mut executors = HashMap::new();

    // Tenant A
    let tenant_a = UnifiedExecutor::with_schema(database_url, "tenant_a").await?;
    executors.insert("tenant_a", tenant_a);

    // Tenant B
    let tenant_b = UnifiedExecutor::with_schema(database_url, "tenant_b").await?;
    executors.insert("tenant_b", tenant_b);

    Ok(())
}
```

### Step 3: Dynamic Tenant Management

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct TenantManager {
    database_url: String,
    executors: Arc<RwLock<HashMap<String, UnifiedExecutor>>>,
}

impl TenantManager {
    pub fn new(database_url: String) -> Self {
        Self {
            database_url,
            executors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_executor(&self, tenant_id: &str) -> Result<UnifiedExecutor, Box<dyn std::error::Error>> {
        // Check if executor already exists
        {
            let executors = self.executors.read().await;
            if let Some(executor) = executors.get(tenant_id) {
                return Ok(executor.clone());
            }
        }

        // Create new executor for tenant
        let executor = UnifiedExecutor::with_schema(&self.database_url, tenant_id).await?;

        // Store for reuse
        {
            let mut executors = self.executors.write().await;
            executors.insert(tenant_id.to_string(), executor.clone());
        }

        Ok(executor)
    }

    pub async fn execute_for_tenant(
        &self,
        tenant_id: &str,
        workflow_name: &str,
        context: Context<serde_json::Value>,
    ) -> Result<PipelineResult, Box<dyn std::error::Error>> {
        let executor = self.get_executor(tenant_id).await?;
        let result = executor.execute(workflow_name, context).await?;
        Ok(result)
    }
}
```

### Step 4: Environment-Based Configuration

```rust
use std::env;

async fn create_tenant_executor() -> Result<UnifiedExecutor, Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL")?;
    let tenant_id = env::var("TENANT_ID")?;

    // Validate tenant ID format
    if !tenant_id.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Invalid tenant ID format".into());
    }

    let executor = UnifiedExecutor::with_schema(&database_url, &tenant_id).await?;
    Ok(executor)
}
```

## Enhanced Security: Per-Tenant Database Credentials

For PostgreSQL deployments requiring database-level user isolation, Cloacina provides the `DatabaseAdmin` utility for creating tenants with their own database users.

### Setting Up Per-Tenant Credentials

```rust
use cloacina::database::{Database, DatabaseAdmin, TenantConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Create admin connection
    let admin_db = Database::new(
        "postgresql://admin:admin_pass@localhost/cloacina",
        "cloacina",
        10
    );
    let admin = DatabaseAdmin::new(admin_db);

    // Step 2: Create tenant with auto-generated password
    let tenant_creds = admin.create_tenant(TenantConfig {
        schema_name: "tenant_secure".to_string(),
        username: "secure_user".to_string(),
        password: "".to_string(), // Empty = secure auto-generation
    })?;

    println!("Tenant created:");
    println!("  Username: {}", tenant_creds.username);
    println!("  Password: {}", tenant_creds.password);
    println!("  Connection: {}", tenant_creds.connection_string);

    // Step 3: Save credentials securely (e.g., secrets manager)
    store_in_secrets_manager(&tenant_creds)?;

    // Step 4: Tenant application uses their credentials
    let executor = UnifiedExecutor::with_schema(
        &tenant_creds.connection_string,
        &tenant_creds.schema_name
    ).await?;

    Ok(())
}
```

### Multi-Tenant Service with Per-Tenant Credentials

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SecureTenantManager {
    admin: DatabaseAdmin,
    // In production, load from secrets manager
    credentials: Arc<RwLock<HashMap<String, TenantCredentials>>>,
}

impl SecureTenantManager {
    pub async fn create_tenant(&self, tenant_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Generate secure schema and username
        let schema_name = format!("tenant_{}", tenant_id);
        let username = format!("user_{}", tenant_id);

        // Create tenant with auto-generated password
        let creds = self.admin.create_tenant(TenantConfig {
            schema_name,
            username,
            password: "".to_string(),
        })?;

        // Store credentials securely
        self.credentials.write().await.insert(tenant_id.to_string(), creds);

        Ok(())
    }

    pub async fn get_executor(&self, tenant_id: &str) -> Result<UnifiedExecutor, Box<dyn std::error::Error>> {
        let creds = self.credentials
            .read()
            .await
            .get(tenant_id)
            .ok_or("Tenant not found")?
            .clone();

        let executor = UnifiedExecutor::with_schema(
            &creds.connection_string,
            &creds.schema_name
        ).await?;

        Ok(executor)
    }

    pub async fn remove_tenant(&self, tenant_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let creds = self.credentials
            .read()
            .await
            .get(tenant_id)
            .ok_or("Tenant not found")?
            .clone();

        // Remove from database
        self.admin.remove_tenant(&creds.schema_name, &creds.username)?;

        // Remove from cache
        self.credentials.write().await.remove(tenant_id);

        Ok(())
    }
}
```

### Benefits of Per-Tenant Credentials

- **Database-level isolation**: Each tenant can only access their schema
- **Audit compliance**: PostgreSQL logs show which tenant performed operations
- **Independent credential rotation**: Change passwords without affecting other tenants
- **Defense in depth**: Additional security layer beyond application-level controls

### Requirements

- PostgreSQL database (not available for SQLite)
- Admin user with `CREATEDB` and `CREATEROLE` privileges
- Secure credential storage system (e.g., HashiCorp Vault, AWS Secrets Manager)

## Recovery

For information about how recovery works in multi-tenant deployments, including automatic recovery and migration considerations, see the [Multi-Tenant Recovery Guide]({{< ref "multi-tenant-recovery" >}}).

## SQLite File-Based Setup

### Step 1: Enable SQLite Features

```toml
[dependencies]
cloacina = { version = "0.1.0", features = ["sqlite"] }
tokio = { version = "1.0", features = ["full"] }
```

### Step 2: File-Based Tenant Management

```rust
use std::path::Path;
use std::fs;

pub struct SqliteTenantManager {
    data_dir: String,
}

impl SqliteTenantManager {
    pub fn new(data_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Ensure data directory exists
        fs::create_dir_all(data_dir)?;

        Ok(Self {
            data_dir: data_dir.to_string(),
        })
    }

    pub async fn get_executor(&self, tenant_id: &str) -> Result<UnifiedExecutor, Box<dyn std::error::Error>> {
        // Validate tenant ID
        if !tenant_id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err("Invalid tenant ID format".into());
        }

        let db_path = format!("sqlite://{}/{}.db", self.data_dir, tenant_id);
        let executor = UnifiedExecutor::new(&db_path).await?;

        Ok(executor)
    }

    pub async fn list_tenants(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut tenants = Vec::new();

        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("db") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    tenants.push(name.to_string());
                }
            }
        }

        Ok(tenants)
    }

    pub async fn delete_tenant(&self, tenant_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let db_path = format!("{}/{}.db", self.data_dir, tenant_id);
        fs::remove_file(db_path)?;
        Ok(())
    }
}
```

## Web Service Integration

### Axum Example with Multi-Tenancy

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ExecutionResponse {
    execution_id: String,
    status: String,
}

#[derive(Deserialize)]
struct WorkflowRequest {
    workflow_name: String,
    context: serde_json::Value,
}

async fn execute_workflow(
    Path(tenant_id): Path<String>,
    State(manager): State<Arc<TenantManager>>,
    Json(request): Json<WorkflowRequest>,
) -> Result<Json<ExecutionResponse>, StatusCode> {
    let context = Context::from_value(request.context)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    match manager.execute_for_tenant(&tenant_id, &request.workflow_name, context).await {
        Ok(result) => Ok(Json(ExecutionResponse {
            execution_id: result.execution_id.to_string(),
            status: format!("{:?}", result.status),
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_execution_status(
    Path((tenant_id, execution_id)): Path<(String, String)>,
    State(manager): State<Arc<TenantManager>>,
) -> Result<Json<ExecutionResponse>, StatusCode> {
    let executor = manager.get_executor(&tenant_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let execution_id = execution_id.parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    match executor.get_execution_status(execution_id).await {
        Ok(status) => Ok(Json(ExecutionResponse {
            execution_id: execution_id.to_string(),
            status: format!("{:?}", status),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

#[tokio::main]
async fn main() {
    let manager = Arc::new(TenantManager::new(
        "postgresql://user:pass@localhost/cloacina".to_string()
    ));

    let app = Router::new()
        .route("/tenants/:tenant_id/workflows", post(execute_workflow))
        .route("/tenants/:tenant_id/executions/:execution_id", get(get_execution_status))
        .with_state(manager);

    // Start server...
}
```

## Configuration Management

### Environment Variables

Create a `.env` file for tenant configuration:

```bash
# Database configuration
DATABASE_URL=postgresql://user:pass@localhost/cloacina

# Tenant configuration
TENANT_ID=production_tenant_123
DEFAULT_SCHEMA=public

# Performance tuning
MAX_CONCURRENT_TASKS=8
TASK_TIMEOUT_SECONDS=300
DB_POOL_SIZE=10
```

### Configuration Structure

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct TenantConfig {
    pub tenant_id: String,
    pub database_url: String,
    pub schema: Option<String>,
    pub max_concurrent_tasks: Option<usize>,
    pub task_timeout_seconds: Option<u64>,
    pub db_pool_size: Option<u32>,
}

impl TenantConfig {
    pub async fn create_executor(&self) -> Result<UnifiedExecutor, Box<dyn std::error::Error>> {
        let mut builder = UnifiedExecutor::builder()
            .database_url(&self.database_url);

        if let Some(ref schema) = self.schema {
            builder = builder.schema(schema);
        }

        if let Some(max_tasks) = self.max_concurrent_tasks {
            builder = builder.max_concurrent_tasks(max_tasks);
        }

        if let Some(timeout) = self.task_timeout_seconds {
            builder = builder.task_timeout(Duration::from_secs(timeout));
        }

        if let Some(pool_size) = self.db_pool_size {
            builder = builder.db_pool_size(pool_size);
        }

        builder.build().await
    }
}
```

## Testing Multi-Tenant Setup

### Integration Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tenant_isolation() {
        let database_url = "postgresql://test:test@localhost/cloacina_test";

        // Create two tenant executors
        let tenant_a = UnifiedExecutor::with_schema(database_url, "test_tenant_a").await.unwrap();
        let tenant_b = UnifiedExecutor::with_schema(database_url, "test_tenant_b").await.unwrap();

        // Execute workflows in each tenant
        let context_a = Context::new();
        let context_b = Context::new();

        let result_a = tenant_a.execute_async("test_workflow", context_a).await.unwrap();
        let result_b = tenant_b.execute_async("test_workflow", context_b).await.unwrap();

        // Verify executions are isolated
        assert_ne!(result_a.execution_id, result_b.execution_id);

        // Cleanup
        tenant_a.shutdown().await.unwrap();
        tenant_b.shutdown().await.unwrap();
    }
}
```

## Performance Optimization

### Connection Pool Tuning

```rust
let executor = UnifiedExecutor::builder()
    .database_url(&database_url)
    .schema(&tenant_id)
    .db_pool_size(20)  // Tune based on tenant load
    .max_concurrent_tasks(10)  // Limit concurrent execution
    .build()
    .await?;
```

### Monitoring

```rust
use std::time::Duration;

pub struct TenantMetrics {
    pub active_executions: usize,
    pub total_executions: u64,
    pub avg_execution_time: Duration,
}

impl TenantManager {
    pub async fn get_tenant_metrics(&self, tenant_id: &str) -> Result<TenantMetrics, Box<dyn std::error::Error>> {
        let executor = self.get_executor(tenant_id).await?;
        let executions = executor.list_executions().await?;

        let total = executions.len() as u64;
        let avg_duration = executions.iter()
            .filter_map(|e| e.duration)
            .sum::<Duration>() / (total.max(1) as u32);

        Ok(TenantMetrics {
            active_executions: executions.iter()
                .filter(|e| matches!(e.status, PipelineStatus::Running))
                .count(),
            total_executions: total,
            avg_execution_time: avg_duration,
        })
    }
}
```

This setup provides a robust foundation for multi-tenant workflow execution with complete data isolation and scalable architecture.
