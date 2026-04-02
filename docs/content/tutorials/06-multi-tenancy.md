---
title: "06 - Multi-Tenancy"
description: "Deploy isolated workflows for multiple tenants using PostgreSQL schemas"
weight: 16
reviewer: "dstorey"
review_date: "2026-04-02"
---

# Multi-Tenancy

Welcome to the multi-tenancy tutorial! In this tutorial, you'll learn how to deploy workflows for multiple tenants with complete data isolation using PostgreSQL schema-based multi-tenancy. This is essential for building SaaS applications where each customer needs isolated workflow execution and data storage.

## Learning Objectives

- Understand schema-based multi-tenancy architecture
- Implement tenant-specific workflow runners
- Use the Database Admin API for tenant provisioning
- Manage tenant isolation and security
- Handle tenant lifecycle and recovery
- Design scalable multi-tenant systems

## Prerequisites

- Completion of [Tutorial 5]({{< ref "/tutorials/05-cron-scheduling/" >}})
- Access to PostgreSQL database
- Understanding of database schemas
- Basic knowledge of SaaS architecture concepts
- Familiarity with Rust async programming

## Time Estimate
30-35 minutes

## Multi-Tenancy Overview

Cloacina implements multi-tenancy using PostgreSQL schemas, providing complete data isolation between tenants without requiring separate databases or query filtering.

### Key Benefits

- **Complete isolation**: Each tenant has their own PostgreSQL schema
- **Zero data leakage**: No cross-tenant access possible
- **Native performance**: PostgreSQL handles isolation efficiently
- **Simplified queries**: No tenant filtering required in application code
- **Admin API**: Built-in tenant provisioning and management

## Setting Up Your Project

Create a new Rust project for this tutorial:

```bash
cargo new multi-tenant-tutorial
cd multi-tenant-tutorial
```

Add the required dependencies to your `Cargo.toml`:

```toml
[package]
name = "multi-tenant-tutorial"
version = "0.1.0"
edition = "2021"

[dependencies]
cloacina = { path = "../../cloacina" }
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

Cloacina automatically selects the database backend based on your connection URL at runtime. No feature flags are needed.

## Basic Multi-Tenant Setup

Let's start with a basic multi-tenant application:

```rust
// src/main.rs
use cloacina::database::{Database, DatabaseAdmin, TenantConfig};
use cloacina::executor::PipelineExecutor;
use cloacina::runner::DefaultRunner;
use cloacina::{task, workflow, Context, PipelineStatus, TaskError};
use serde_json::json;
use std::collections::HashMap;
use tracing::{info, warn};

#[workflow(
    name = "customer_processing",
    description = "Process customer data in isolated tenant environment"
)]
pub mod customer_processing {
    use super::*;

    #[task(
        id = "process_customer_data",
        dependencies = []
    )]
    pub async fn process_customer_data(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        let tenant_id = context
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let customer_name = context
            .get("customer_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        info!(
            "Processing data for customer: {} (tenant: {})",
            customer_name, tenant_id
        );

        // Simulate tenant-specific processing
        let processed_records = match tenant_id.as_str() {
            "acme_corp" => 1250,
            "globex_inc" => 890,
            "initech" => 430,
            _ => 100,
        };

        info!(
            "Processed {} records for {}",
            processed_records, customer_name
        );

        context.insert("processed_records", json!(processed_records))?;
        context.insert("processing_completed", json!(true))?;

        Ok(())
    }
}

async fn basic_multi_tenant_demo(database_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Basic Multi-Tenant Demo ===");

    // Create tenant-specific runners using schema isolation
    let mut tenant_runners = HashMap::new();
    let tenants = vec!["acme_corp", "globex_inc", "initech"];

    for tenant_id in &tenants {
        info!("Creating runner for tenant: {}", tenant_id);

        // Create runner with tenant-specific schema
        let runner = DefaultRunner::with_schema(database_url, tenant_id).await?;
        tenant_runners.insert(tenant_id.to_string(), runner);

        info!("Tenant {} runner created with schema isolation", tenant_id);
    }

    // Execute workflows for each tenant
    // Workflows are auto-registered by the #[workflow] macro

    for (tenant_id, runner) in &tenant_runners {
        info!("Executing workflow for tenant: {}", tenant_id);

        let mut context = Context::new();
        context.insert("tenant_id", json!(tenant_id.clone()))?;
        context.insert("customer_name", json!(format!("{} Customer", tenant_id)))?;

        let result = runner.execute("customer_processing", context).await?;

        if matches!(result.status, PipelineStatus::Completed) {
            let records = result
                .final_context
                .get("processed_records")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            info!(
                "Tenant {} completed: {} records processed",
                tenant_id, records
            );
        } else {
            warn!("Tenant {} failed: {:?}", tenant_id, result.status);
        }
    }

    // Shutdown all runners
    for (tenant_id, runner) in tenant_runners {
        info!("Shutting down runner for tenant: {}", tenant_id);
        runner.shutdown().await?;
    }

    info!("Basic multi-tenant demo completed");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("tutorial_06=info,cloacina=info")
        .init();

    info!("Starting Tutorial 06: Multi-Tenancy");

    // Get database URL from environment or use default
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        warn!("DATABASE_URL not set, using default PostgreSQL connection");
        "postgresql://cloacina:cloacina@localhost:5432/cloacina".to_string()
    });

    basic_multi_tenant_demo(&database_url).await?;

    info!("\nTutorial 06 completed successfully!");
    Ok(())
}
```

## Advanced Multi-Tenant Patterns with Admin API

Now let's explore advanced patterns using the Database Admin API:

```rust
// src/advanced.rs
use cloacina::database::{Database, DatabaseAdmin, TenantConfig};
use cloacina::executor::PipelineExecutor;
use cloacina::runner::DefaultRunner;
use cloacina::{task, workflow, Context, PipelineStatus, TaskError};
use serde_json::json;
use std::collections::HashMap;
use tracing::{error, info, warn};

#[workflow(
    name = "tenant_onboarding",
    description = "Complete tenant onboarding process"
)]
pub mod tenant_onboarding_workflow {
    use super::*;

    #[task(
        id = "tenant_onboarding",
        dependencies = []
    )]
    pub async fn tenant_onboarding(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        let tenant_name = context
            .get("tenant_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        let tenant_type = context
            .get("tenant_type")
            .and_then(|v| v.as_str())
            .unwrap_or("starter")
            .to_string();

        info!(
            "Onboarding new tenant: {} (type: {})",
            tenant_name, tenant_type
        );

        // Simulate tenant-specific setup
        let setup_tasks = match tenant_type.as_str() {
            "enterprise" => vec![
                "provision_resources",
                "setup_integrations",
                "configure_billing",
                "setup_support",
            ],
            "professional" => vec![
                "provision_resources",
                "setup_integrations",
                "configure_billing",
            ],
            "starter" => vec!["provision_resources", "configure_billing"],
            _ => vec!["provision_resources"],
        };

        info!(
            "Executing {} setup tasks for {}",
            setup_tasks.len(),
            tenant_name
        );

        for task in &setup_tasks {
            info!("  Completed: {}", task);
        }

        context.insert("onboarding_completed", json!(true))?;
        context.insert("setup_tasks_count", json!(setup_tasks.len()))?;
        context.insert("tenant_status", json!("active"))?;

        Ok(())
    }
}

#[workflow(
    name = "tenant_data_processing",
    description = "Process tenant-specific data with isolation"
)]
pub mod tenant_data_processing_workflow {
    use super::*;

    #[task(
        id = "process_tenant_data",
        dependencies = []
    )]
    pub async fn process_tenant_data(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        let tenant_id = context
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let data_volume = context
            .get("data_volume")
            .and_then(|v| v.as_str())
            .unwrap_or("small")
            .to_string();

        info!("Processing {} data for tenant: {}", data_volume, tenant_id);

        // Simulate data processing based on volume
        let (processing_time, records_processed) = match data_volume.as_str() {
            "large" => (5000, 50000),
            "medium" => (2000, 15000),
            "small" => (500, 2000),
            _ => (100, 100),
        };

        // Simulate processing delay
        tokio::time::sleep(tokio::time::Duration::from_millis(processing_time / 10)).await;

        info!("Processed {} records in {}ms for {}", records_processed, processing_time, tenant_id);

        context.insert("records_processed", json!(records_processed))?;
        context.insert("processing_time_ms", json!(processing_time))?;
        context.insert("processing_status", json!("completed"))?;

        Ok(())
    }
}

async fn advanced_admin_demo(admin_database_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Advanced Multi-Tenant Demo with Admin API ===");

    // Create database admin
    let admin_db = Database::new(admin_database_url, "cloacina", 10);
    let admin = DatabaseAdmin::new(admin_db);

    // Provision new tenant using admin API
    let tenant_config = TenantConfig {
        schema_name: "tenant_demo".to_string(),
        username: "demo_user".to_string(),
        password: String::new(), // Auto-generate secure password
    };

    info!("Provisioning tenant with admin API...");

    match admin.create_tenant(tenant_config).await {
        Ok(credentials) => {
            info!("Tenant provisioned successfully!");
            info!("  Schema: {}", credentials.schema_name);
            info!("  Username: {}", credentials.username);
            info!("  Connection ready");

            // Create runner with tenant-specific credentials
            info!("Creating runner with dedicated credentials...");
            let tenant_runner = DefaultRunner::new(&credentials.connection_string).await?;

            // Workflow is auto-registered by the #[workflow] macro

            // Execute onboarding workflow
            let mut context = Context::new();
            context.insert("tenant_id", json!("demo"))?;
            context.insert("tenant_name", json!("Demo Tenant"))?;
            context.insert("tenant_type", json!("professional"))?;

            let result = tenant_runner.execute("tenant_onboarding", context).await?;

            if matches!(result.status, PipelineStatus::Completed) {
                let task_count = result
                    .final_context
                    .get("setup_tasks_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                info!(
                    "Tenant onboarded successfully with {} setup tasks",
                    task_count
                );
            } else {
                error!("Tenant onboarding failed: {:?}", result.status);
            }

            tenant_runner.shutdown().await?;
            info!("Advanced admin demo completed");
        }
        Err(e) => {
            return Err(format!("Failed to provision tenant: {}", e).into());
        }
    }

    Ok(())
}
```

To run the advanced demo, update your `main` function to call it when PostgreSQL is available:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("tutorial_06=info,cloacina=info")
        .init();

    info!("Starting Tutorial 06: Multi-Tenancy");

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        warn!("DATABASE_URL not set, using default PostgreSQL connection");
        "postgresql://cloacina:cloacina@localhost:5432/cloacina".to_string()
    });

    basic_multi_tenant_demo(&database_url).await?;

    // Advanced demo with admin API (if PostgreSQL is available)
    if database_url.starts_with("postgresql://") {
        info!("\n{}", "=".repeat(60));
        advanced_admin_demo(&database_url).await.unwrap_or_else(|e| {
            warn!("Advanced admin demo skipped: {}", e);
        });
    }

    info!("\nTutorial 06 completed successfully!");
    Ok(())
}
```

## Running the Tutorial

1. **Start PostgreSQL** (using Docker):
```bash
docker run --name tutorial-postgres \
  -e POSTGRES_USER=cloacina \
  -e POSTGRES_PASSWORD=cloacina \
  -e POSTGRES_DB=cloacina \
  -p 5432:5432 \
  -d postgres:15
```

2. **Run the tutorial**:
```bash
cargo run
```

## Security and Isolation Benefits

### Complete Data Isolation

```rust
// Each tenant's data is completely isolated:
// tenant_acme_corp.task_executions
// tenant_globex_inc.task_executions
// tenant_initech.task_executions

// No cross-tenant queries possible!
```

### Database-Level Security

```rust
// With admin API, each tenant gets:
// - Dedicated PostgreSQL user
// - Access only to their schema
// - No access to other tenant data
// - PostgreSQL enforces all access controls
```

## Production Considerations

### Environment Configuration

```rust
use std::env;

let admin_db_url = env::var("ADMIN_DATABASE_URL")
    .unwrap_or_else(|_| "postgresql://admin:admin@localhost/cloacina".to_string());

let tenant_config = TenantConfig {
    schema_name: format!("tenant_{}", tenant_id),
    username: format!("{}_user", tenant_id),
    password: env::var("TENANT_PASSWORD").unwrap_or_default(), // Or auto-generate
};
```

### Resource Management

```rust
// Limit concurrent tenant runners
const MAX_TENANT_RUNNERS: usize = 50;

// Use connection pooling per tenant
let runner = DefaultRunner::with_config(
    &credentials.connection_string,
    DefaultRunnerConfig::builder()
        .db_pool_size(5)  // Limit per tenant
        .build()
).await?;
```

## Best Practices

{{< tabs "mt-best-practices" >}}
{{< tab "Tenant Naming" >}}
**Use consistent naming conventions:**
```rust
// Good: Predictable, URL-safe, database-safe
let schema_name = format!("tenant_{}", tenant_id.to_lowercase());
let username = format!("{}_user", tenant_id.to_lowercase());

// Validate tenant IDs
fn is_valid_tenant_id(id: &str) -> bool {
    id.chars().all(|c| c.is_alphanumeric() || c == '_')
        && id.len() >= 3
        && id.len() <= 30
}
```
{{< /tab >}}

{{< tab "Error Handling" >}}
**Handle tenant operations robustly:**
```rust
match tenant_manager.provision_tenant("new_customer", "New Customer Inc").await {
    Ok(_) => info!("Tenant provisioned successfully"),
    Err(e) => {
        error!("Tenant provisioning failed: {}", e);
        // Implement cleanup, alerting, fallback
        return Err(e);
    }
}
```
{{< /tab >}}

{{< tab "Performance" >}}
**Optimize for scale:**
```rust
// Use async for concurrent tenant operations
let futures: Vec<_> = tenants.iter()
    .map(|tenant_id| async move {
        tenant_manager.process_workload(tenant_id, "medium").await
    })
    .collect();

// Process with controlled concurrency
let results = futures::future::join_all(futures).await;
```
{{< /tab >}}
{{< /tabs >}}

## What You've Learned

Congratulations! You now understand:

- **Schema-based multi-tenancy** with complete data isolation
- **Database Admin API** for tenant provisioning
- **Tenant lifecycle management** from onboarding to processing
- **Security benefits** of database-level isolation
- **Production considerations** for scalable multi-tenant systems

## Next Steps

With multi-tenancy mastered, you're ready to:

1. **[API Reference]({{< ref "/reference/" >}})** - Explore advanced multi-tenant APIs
2. **[How-to Guides]({{< ref "/how-to-guides/" >}})** - Deploy multi-tenant applications

## Related Resources

- [Explanation: Multi-Tenancy Architecture]({{< ref "/explanation/multi-tenancy/" >}}) - Deep dive into design
- [How-to: Multi-Tenant Setup]({{< ref "/how-to-guides/multi-tenant-setup/" >}}) - Production deployment
- [Database Admin API]({{< ref "/reference/database-admin/" >}}) - Complete API reference
- [Python Multi-Tenancy Tutorial]({{< ref "/python-bindings/tutorials/06-multi-tenancy/" >}}) - Python equivalent
