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

//! Multi-tenant workflow execution example
//!
//! This example demonstrates how to use Cloacina's multi-tenant capabilities
//! with PostgreSQL schema-based isolation.

use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::PipelineError;
use std::env;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("multi_tenant=info,cloacina=info")
        .init();

    // Get database URL from environment or use default Docker PostgreSQL
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        warn!("DATABASE_URL not set, using default Docker PostgreSQL connection");
        "postgresql://cloacina:cloacina@localhost:5432/cloacina".to_string()
    });

    info!("Starting multi-tenant workflow example");

    // Demonstrate different ways to create multi-tenant executors
    demonstrate_multi_tenant_setup(&database_url).await?;

    // Demonstrate recovery and migration scenarios
    demonstrate_recovery_scenarios(&database_url).await?;

    info!("Multi-tenant example completed successfully");
    Ok(())
}

async fn demonstrate_multi_tenant_setup(database_url: &str) -> Result<(), PipelineError> {
    info!("Creating multi-tenant executors with schema isolation");

    // Method 1: Using convenience method
    info!("Creating tenant 'acme_corp' using convenience method");
    let acme_runner = DefaultRunner::with_schema(database_url, "acme_corp").await?;

    // Method 2: Using builder pattern with config
    info!("Creating tenant 'globex_inc' using config pattern");
    let globex_runner = DefaultRunner::with_schema(database_url, "globex_inc").await?;

    // Method 3: Single-tenant (uses default schema)
    info!("Creating single-tenant runner (default schema)");
    let single_tenant = DefaultRunner::with_config(database_url, DefaultRunnerConfig::default()).await?;

    info!("All runners created successfully!");

    // In a real application, each runner would be used by different
    // services or application instances, providing complete data isolation

    // Shutdown all runners
    info!("Shutting down runners");
    acme_runner.shutdown().await?;
    globex_runner.shutdown().await?;
    single_tenant.shutdown().await?;

    info!("All runners shut down successfully");
    Ok(())
}

/// Demonstrates recovery scenarios for multi-tenant systems
async fn demonstrate_recovery_scenarios(database_url: &str) -> Result<(), PipelineError> {
    info!("=== Demonstrating Multi-Tenant Recovery ===");

    // Demonstrate automatic recovery across restarts
    info!("Recovery is enabled by default for all executors");
    info!("Creating executor for tenant 'persistent_tenant'...");

    // First creation - will create schema and run migrations
    let first_runner = DefaultRunner::with_schema(database_url, "persistent_tenant").await?;
    info!("First runner created - schema and tables initialized");

    // Simulate some work would happen here...
    info!("Simulating work in progress...");

    // Shutdown the runner (simulating a crash or restart)
    first_runner.shutdown().await?;
    info!("First runner shut down");

    // Create a new runner for the same tenant
    info!("Creating new runner for same tenant after shutdown...");
    let second_runner = DefaultRunner::with_schema(database_url, "persistent_tenant").await?;
    info!("Second runner created successfully!");
    info!("- Schema was NOT recreated (already exists)");
    info!("- Migrations were NOT re-run (already applied)");
    info!("- Recovery automatically started (enabled by default)");
    info!("- Any orphaned tasks would be recovered");
    info!("- Each tenant's recovery is isolated");

    second_runner.shutdown().await?;

    // Basic migration example
    info!("\nMigration tip: To migrate from single-tenant to multi-tenant:");
    info!("1. Create DefaultRunner with schema for new tenant");
    info!("2. Existing data remains in 'public' schema");
    info!("3. New tenant data is isolated in its own schema");
    info!("4. Both can run side-by-side during transition");

    Ok(())
}
