/*
 *  Copyright 2025 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Tutorial 06: Multi-Tenancy
//!
//! This tutorial demonstrates how to deploy isolated workflows for multiple tenants
//! using PostgreSQL schema-based multi-tenancy and the Database Admin API.

use cloacina::database::{Database, DatabaseAdmin, TenantConfig};
use cloacina::executor::PipelineExecutor;
use cloacina::runner::DefaultRunner;
use cloacina::{task, workflow, Context, PipelineStatus, TaskError};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use tracing::{error, info, warn};

#[task(
    id = "process_customer_data",
    dependencies = []
)]
async fn process_customer_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
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

#[task(
    id = "tenant_onboarding",
    dependencies = []
)]
async fn tenant_onboarding(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
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
        info!("  ✓ Completed: {}", task);
    }

    context.insert("onboarding_completed", json!(true))?;
    context.insert("setup_tasks_count", json!(setup_tasks.len()))?;
    context.insert("tenant_status", json!("active"))?;

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
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        warn!("DATABASE_URL not set, using default PostgreSQL connection");
        "postgresql://cloacina:cloacina@localhost:5432/cloacina".to_string()
    });

    // Basic multi-tenant setup without admin API
    basic_multi_tenant_demo(&database_url).await?;

    // Advanced demo with admin API (if PostgreSQL is available)
    if database_url.starts_with("postgresql://") {
        info!("\n{}", "=".repeat(60));
        advanced_admin_demo(&database_url).await.unwrap_or_else(|e| {
            warn!("Advanced admin demo skipped: {}", e);
            info!("This is expected if you don't have admin privileges or PostgreSQL isn't running");
        });
    }

    info!("\n✅ Tutorial 06 completed successfully!");
    Ok(())
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

        info!(
            "✓ Tenant {} runner created with schema isolation",
            tenant_id
        );
    }

    // Execute workflows for each tenant
    // Create workflows using the macro system
    let _customer_workflow = workflow! {
        name: "customer_processing",
        description: "Process customer data in isolated tenant environment",
        tasks: [
            process_customer_data
        ]
    };

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
                "✓ Tenant {} completed: {} records processed",
                tenant_id, records
            );
        } else {
            warn!("✗ Tenant {} failed: {:?}", tenant_id, result.status);
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
            info!("✓ Tenant provisioned successfully!");
            info!("  Schema: {}", credentials.schema_name);
            info!("  Username: {}", credentials.username);
            info!("  Connection ready");

            // Create runner with tenant-specific credentials
            info!("Creating runner with dedicated credentials...");
            let tenant_runner = DefaultRunner::new(&credentials.connection_string).await?;

            // Create onboarding workflow
            let _onboarding_workflow = workflow! {
                name: "tenant_onboarding",
                description: "Complete tenant onboarding process",
                tasks: [
                    tenant_onboarding
                ]
            };

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
                    "✓ Tenant onboarded successfully with {} setup tasks",
                    task_count
                );
            } else {
                error!("✗ Tenant onboarding failed: {:?}", result.status);
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
