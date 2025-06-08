---
title: "06 - Multi-Tenancy"
description: "Deploy isolated workflows for multiple tenants using PostgreSQL schemas"
weight: 16
reviewer: "automation"
review_date: "2025-06-08"
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

- Completion of [Tutorial 5](/tutorials/05-cron-scheduling/)
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
cloacina = { path = "../../cloacina", features = ["postgres"] }
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

## Basic Multi-Tenant Setup

Let's start with a basic multi-tenant application:

```rust
// src/main.rs
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, PipelineError};
use std::collections::HashMap;
use tracing::{info, warn};

#[task(id = "process_customer_data")]
async fn process_customer_data(mut context: Context) -> Result<Context, PipelineError> {
    let tenant_id = context.get::<String>("tenant_id").unwrap_or_default();
    let customer_name = context.get::<String>("customer_name").unwrap_or_default();

    info!("Processing data for customer: {} (tenant: {})", customer_name, tenant_id);

    // Simulate customer-specific processing
    let processed_records = match tenant_id.as_str() {
        "acme_corp" => 1250,
        "globex_inc" => 890,
        "initech" => 430,
        _ => 100,
    };

    info!("Processed {} records for {}", processed_records, customer_name);

    context.set("processed_records", processed_records);
    context.set("processing_completed", true);

    Ok(context)
}

#[workflow]
fn customer_processing_workflow() -> cloacina::Workflow {
    cloacina::Workflow::builder("customer_processing")
        .description("Process customer data in isolated tenant environment")
        .task(process_customer_data)
        .build()
}

async fn basic_multi_tenant_demo() -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Basic Multi-Tenant Demo ===");

    let database_url = "postgresql://cloacina:cloacina@localhost:5432/cloacina";

    // Create tenant-specific runners
    let mut tenant_runners = HashMap::new();

    let tenants = vec!["acme_corp", "globex_inc", "initech"];

    for tenant_id in &tenants {
        info!("Creating runner for tenant: {}", tenant_id);

        // Create runner with tenant-specific schema
        let runner = DefaultRunner::with_schema(database_url, tenant_id).await?;
        tenant_runners.insert(tenant_id.to_string(), runner);

        info!("✓ Tenant {} runner created with schema isolation", tenant_id);
    }

    // Execute workflows for each tenant
    for (tenant_id, runner) in &tenant_runners {
        info!("Executing workflow for tenant: {}", tenant_id);

        let context = Context::new()
            .with("tenant_id", tenant_id.clone())
            .with("customer_name", format!("{} Customer", tenant_id));

        let result = runner.execute("customer_processing", context).await?;

        if result.status.is_success() {
            let records = result.final_context.get::<i32>("processed_records").unwrap_or(0);
            info!("✓ Tenant {} completed: {} records processed", tenant_id, records);
        } else {
            warn!("✗ Tenant {} failed: {:?}", tenant_id, result.status);
        }
    }

    // Shutdown all runners
    for (tenant_id, runner) in tenant_runners {
        info!("Shutting down runner for tenant: {}", tenant_id);
        runner.shutdown().await?;
    }

    info!("Basic multi-tenant demo completed successfully");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("multi_tenant_tutorial=info,cloacina=info")
        .init();

    basic_multi_tenant_demo().await?;

    Ok(())
}
```

## Advanced Multi-Tenant Patterns with Admin API

Now let's explore advanced patterns using the Database Admin API:

```rust
// src/advanced.rs
use cloacina::database::{Database, DatabaseAdmin, TenantConfig, TenantCredentials};
use cloacina::runner::DefaultRunner;
use cloacina::{task, workflow, Context, PipelineError};
use std::collections::HashMap;
use tracing::{error, info, warn};

#[task(id = "tenant_onboarding")]
async fn tenant_onboarding(mut context: Context) -> Result<Context, PipelineError> {
    let tenant_name = context.get::<String>("tenant_name").unwrap_or_default();
    let tenant_type = context.get::<String>("tenant_type").unwrap_or_default();

    info!("Onboarding new tenant: {} (type: {})", tenant_name, tenant_type);

    // Simulate tenant-specific setup
    let setup_tasks = match tenant_type.as_str() {
        "enterprise" => vec!["provision_resources", "setup_integrations", "configure_billing", "setup_support"],
        "professional" => vec!["provision_resources", "setup_integrations", "configure_billing"],
        "starter" => vec!["provision_resources", "configure_billing"],
        _ => vec!["provision_resources"],
    };

    info!("Executing {} setup tasks for {}", setup_tasks.len(), tenant_name);

    for task in &setup_tasks {
        info!("  ✓ Completed: {}", task);
    }

    context.set("onboarding_completed", true);
    context.set("setup_tasks_count", setup_tasks.len());
    context.set("tenant_status", "active");

    Ok(context)
}

#[task(id = "process_tenant_data")]
async fn process_tenant_data(mut context: Context) -> Result<Context, PipelineError> {
    let tenant_id = context.get::<String>("tenant_id").unwrap_or_default();
    let data_volume = context.get::<String>("data_volume").unwrap_or_default();

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

    context.set("records_processed", records_processed);
    context.set("processing_time_ms", processing_time);
    context.set("processing_status", "completed");

    Ok(context)
}

#[workflow]
fn tenant_onboarding_workflow() -> cloacina::Workflow {
    cloacina::Workflow::builder("tenant_onboarding")
        .description("Complete tenant onboarding process")
        .task(tenant_onboarding)
        .build()
}

#[workflow]
fn tenant_data_processing_workflow() -> cloacina::Workflow {
    cloacina::Workflow::builder("tenant_data_processing")
        .description("Process tenant-specific data with isolation")
        .task(process_tenant_data)
        .build()
}

pub struct TenantManager {
    admin: DatabaseAdmin,
    tenant_runners: HashMap<String, DefaultRunner>,
    tenant_credentials: HashMap<String, TenantCredentials>,
}

impl TenantManager {
    pub fn new(admin_database_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let admin_db = Database::new(admin_database_url, "cloacina", 10);
        let admin = DatabaseAdmin::new(admin_db);

        Ok(Self {
            admin,
            tenant_runners: HashMap::new(),
            tenant_credentials: HashMap::new(),
        })
    }

    pub async fn provision_tenant(
        &mut self,
        tenant_id: &str,
        tenant_name: &str,
    ) -> Result<&TenantCredentials, Box<dyn std::error::Error>> {
        info!("Provisioning tenant: {} ({})", tenant_name, tenant_id);

        // Create tenant configuration
        let tenant_config = TenantConfig {
            schema_name: format!("tenant_{}", tenant_id),
            username: format!("{}_user", tenant_id),
            password: String::new(), // Auto-generate secure password
        };

        // Create tenant using admin API
        let credentials = self.admin.create_tenant(tenant_config).await?;

        info!("✓ Tenant {} provisioned successfully", tenant_id);
        info!("  Schema: {}", credentials.schema_name);
        info!("  Username: {}", credentials.username);
        info!("  Connection ready");

        self.tenant_credentials.insert(tenant_id.to_string(), credentials);

        Ok(self.tenant_credentials.get(tenant_id).unwrap())
    }

    pub async fn create_tenant_runner(
        &mut self,
        tenant_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let credentials = self.tenant_credentials.get(tenant_id)
            .ok_or_else(|| format!("Tenant {} not provisioned", tenant_id))?;

        info!("Creating runner for tenant: {}", tenant_id);

        // Create runner with tenant-specific credentials
        let runner = DefaultRunner::new(&credentials.connection_string).await?;
        self.tenant_runners.insert(tenant_id.to_string(), runner);

        info!("✓ Runner created for tenant {} with isolated credentials", tenant_id);

        Ok(())
    }

    pub async fn onboard_customer(
        &mut self,
        tenant_id: &str,
        tenant_name: &str,
        tenant_type: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting complete onboarding for: {} ({})", tenant_name, tenant_id);

        // Step 1: Provision tenant infrastructure
        self.provision_tenant(tenant_id, tenant_name).await?;

        // Step 2: Create tenant runner
        self.create_tenant_runner(tenant_id).await?;

        // Step 3: Execute onboarding workflow
        let runner = self.tenant_runners.get(tenant_id).unwrap();

        let context = Context::new()
            .with("tenant_id", tenant_id.to_string())
            .with("tenant_name", tenant_name.to_string())
            .with("tenant_type", tenant_type.to_string());

        let result = runner.execute("tenant_onboarding", context).await?;

        if result.status.is_success() {
            let task_count = result.final_context.get::<usize>("setup_tasks_count").unwrap_or(0);
            info!("✓ Tenant {} onboarded successfully with {} setup tasks", tenant_id, task_count);
        } else {
            error!("✗ Tenant {} onboarding failed: {:?}", tenant_id, result.status);
        }

        Ok(())
    }

    pub async fn process_tenant_workload(
        &self,
        tenant_id: &str,
        data_volume: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let runner = self.tenant_runners.get(tenant_id)
            .ok_or_else(|| format!("No runner for tenant {}", tenant_id))?;

        info!("Processing {} workload for tenant: {}", data_volume, tenant_id);

        let context = Context::new()
            .with("tenant_id", tenant_id.to_string())
            .with("data_volume", data_volume.to_string());

        let result = runner.execute("tenant_data_processing", context).await?;

        if result.status.is_success() {
            let records = result.final_context.get::<i32>("records_processed").unwrap_or(0);
            let time_ms = result.final_context.get::<i32>("processing_time_ms").unwrap_or(0);
            info!("✓ Processed {} records in {}ms for tenant {}", records, time_ms, tenant_id);
        } else {
            error!("✗ Processing failed for tenant {}: {:?}", tenant_id, result.status);
        }

        Ok(())
    }

    pub async fn shutdown_all(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Shutting down all tenant runners...");

        for (tenant_id, runner) in self.tenant_runners.drain() {
            info!("Shutting down runner for tenant: {}", tenant_id);
            runner.shutdown().await?;
        }

        info!("All tenant runners shut down successfully");
        Ok(())
    }
}

pub async fn advanced_multi_tenant_demo() -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Advanced Multi-Tenant Demo with Admin API ===");

    let admin_database_url = "postgresql://cloacina:cloacina@localhost:5432/cloacina";
    let mut tenant_manager = TenantManager::new(admin_database_url)?;

    // Define tenant configurations
    let tenants = vec![
        ("acme_corp", "Acme Corporation", "enterprise"),
        ("globex_inc", "Globex Industries", "professional"),
        ("initech", "Initech Solutions", "starter"),
    ];

    // Phase 1: Tenant Onboarding
    info!("Phase 1: Complete Tenant Onboarding");
    info!("-" .repeat(40));

    for (tenant_id, tenant_name, tenant_type) in &tenants {
        match tenant_manager.onboard_customer(tenant_id, tenant_name, tenant_type).await {
            Ok(()) => info!("✅ {} onboarded successfully", tenant_name),
            Err(e) => error!("❌ Failed to onboard {}: {}", tenant_name, e),
        }
    }

    // Phase 2: Workload Processing
    info!("\nPhase 2: Tenant Workload Processing");
    info!("-" .repeat(40));

    let workloads = vec![
        ("acme_corp", "large"),
        ("globex_inc", "medium"),
        ("initech", "small"),
        ("acme_corp", "medium"), // Multiple workloads for same tenant
    ];

    for (tenant_id, volume) in &workloads {
        match tenant_manager.process_tenant_workload(tenant_id, volume).await {
            Ok(()) => info!("✅ {} workload completed for {}", volume, tenant_id),
            Err(e) => error!("❌ Failed {} workload for {}: {}", volume, tenant_id, e),
        }
    }

    // Phase 3: Demonstrate Isolation
    info!("\nPhase 3: Demonstrating Tenant Isolation");
    info!("-" .repeat(40));

    // Simulate concurrent processing to show isolation
    let mut handles = vec![];

    for (tenant_id, _, _) in &tenants {
        let tenant_id = tenant_id.to_string();
        let runner = tenant_manager.tenant_runners.get(&tenant_id).unwrap();

        // Clone the runner (Arc<> internally)
        let runner_clone = runner.clone();

        let handle = tokio::spawn(async move {
            let context = Context::new()
                .with("tenant_id", tenant_id.clone())
                .with("data_volume", "medium");

            let result = runner_clone.execute("tenant_data_processing", context).await;

            match result {
                Ok(r) if r.status.is_success() => {
                    let records = r.final_context.get::<i32>("records_processed").unwrap_or(0);
                    info!("🔄 Concurrent processing for {}: {} records", tenant_id, records);
                }
                Ok(r) => warn!("⚠️  Concurrent processing failed for {}: {:?}", tenant_id, r.status),
                Err(e) => error!("❌ Concurrent processing error for {}: {}", tenant_id, e),
            }
        });

        handles.push(handle);
    }

    // Wait for all concurrent executions
    for handle in handles {
        handle.await?;
    }

    info!("✅ All concurrent executions completed with full isolation");

    // Cleanup
    tenant_manager.shutdown_all().await?;

    info!("🎉 Advanced multi-tenant demo completed successfully!");

    Ok(())
}
```

Add this to your `src/main.rs` to include the advanced demo:

```rust
// Add to src/main.rs after the existing code

mod advanced;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("multi_tenant_tutorial=info,cloacina=info")
        .init();

    // Run basic demo
    basic_multi_tenant_demo().await?;

    println!("\n" + &"=".repeat(60) + "\n");

    // Run advanced demo
    advanced::advanced_multi_tenant_demo().await?;

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
        .max_connections(5)  // Limit per tenant
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

1. **[API Reference](/reference/)** - Explore advanced multi-tenant APIs
2. **[Examples](/examples/)** - See production multi-tenant patterns
3. **[How-to Guides](/how-to-guides/)** - Deploy multi-tenant applications

## Related Resources

- [Explanation: Multi-Tenancy Architecture](/explanation/multi-tenancy/) - Deep dive into design
- [How-to: Multi-Tenant Setup](/how-to-guides/multi-tenant-setup/) - Production deployment
- [Database Admin API](/reference/database-admin/) - Complete API reference
- [Python Multi-Tenancy Tutorial](/python-bindings/tutorials/05-multi-tenancy/) - Python equivalent
