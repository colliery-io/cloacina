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

/*!
# Registry Execution Demo - New DAL System

This example demonstrates the complete workflow lifecycle using the new DAL system:
1. Build a .cloacina package
2. Register it using WorkflowRegistryDAL with database storage
3. Execute it using DefaultRunner

## Features Demonstrated

- **New DAL System**: Uses WorkflowRegistryDAL instead of manual registry management
- **Database Storage**: Stores binary package data directly in the database workflow_registry table
- **Atomic Operations**: Registration uses atomic transactions across workflow_registry and workflow_packages tables
- **Storage Options**: Shows both database and filesystem storage backend options

## Usage

```bash
cd examples/registry-execution-demo
cargo run --features sqlite  # or --features postgres
```
*/

use std::path::PathBuf;
use tempfile::TempDir;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use chrono::{Duration as ChronoDuration, Utc};
use cloacina::database::Database;
use cloacina::registry::error::RegistryError;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{Context, PipelineExecutor};
use std::sync::Arc;
use std::time::Duration;

// Import cloacina library packaging functions (like cloacina-app)
use cloacina::packaging::{package_workflow, CompileOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with info level
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_line_number(true))
        .with(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    println!("🚀 Cloacina Registry Execution Demo\n");

    // Step 1: Build the .cloacina package
    println!("📦 Building workflow package...");
    let package_data = build_package().await?;
    println!("✅ Package built: {} bytes\n", package_data.len());

    // Step 2: Set up shared database
    println!("📋 Setting up shared database...");

    // Use a persistent file-based database that both registry and runner can share
    let db_path = "/tmp/cloacina_debug.db";
    let db_url = format!("sqlite://{}?mode=rwc", db_path);
    println!("📋 Database will be saved to: {}", db_path);

    let database = Database::new(&db_url, "", 5);
    let conn = database.pool().get().await?;
    conn.interact(move |conn| cloacina::database::run_migrations(conn))
        .await??;

    // Step 3: Register the workflow package using the new DAL system
    println!("📋 Registering workflow package using DAL system...");

    // Create DAL and choose storage backend
    let dal = cloacina::dal::DAL::new(database.clone());

    // Option 1: Database storage (SQLite) - stores binary data in workflow_registry table
    let storage = Arc::new(cloacina::dal::SqliteRegistryStorage::new(database.clone()));

    // Option 2: Filesystem storage - stores binary data as files
    // let storage_path = "/tmp/cloacina_demo_storage";
    // std::fs::create_dir_all(storage_path)?;
    // let storage = Arc::new(cloacina::dal::FilesystemRegistryStorage::new(storage_path));

    let mut registry_dal = dal.workflow_registry(storage);

    match registry_dal.register_workflow_package(package_data).await {
        Ok(package_id) => {
            println!("✅ Package registered with DAL ID: {}", package_id);
        }
        Err(RegistryError::PackageExists {
            package_name,
            version,
        }) => {
            println!(
                "⚠️  Package already exists: {} v{} - continuing with existing package",
                package_name, version
            );
            // For demo purposes, we'll continue with the existing package
            // In production, you might want to check versions or handle differently
        }
        Err(e) => return Err(e.into()),
    };
    println!();

    // Step 4: List available workflows using DAL
    println!("🔍 Available workflows:");
    let packages = registry_dal.list_packages().await?;
    for package_info in &packages {
        println!(
            "  - Package {} (v{}) - ID: {}",
            package_info.package_name, package_info.version, package_info.id
        );
    }
    println!();

    // Step 5: Get workflow from registry using DAL
    println!("📥 Getting workflow from registry...");
    if !packages.is_empty() {
        let first_package = &packages[0];
        let loaded_workflow = registry_dal
            .get_workflow_package_by_name(&first_package.package_name, &first_package.version)
            .await?;
        if let Some((metadata, _binary_data)) = &loaded_workflow {
            println!(
                "✅ Workflow loaded: {} v{}\n",
                metadata.package_name, metadata.version
            );
        } else {
            println!("❌ Failed to load workflow");
            return Ok(());
        }
    } else {
        println!("❌ No packages found in registry");
        return Ok(());
    }

    // Step 6: Set up DefaultRunner with shared database
    println!("▶️  Setting up execution environment with shared database...");

    // Configure DefaultRunner with registry reconciler enabled (uses database storage now)
    let mut config = DefaultRunnerConfig::default();
    config.enable_registry_reconciler = true;
    // Configure registry to use SQLite database storage (matching our registration method)
    config.registry_storage_backend = "sqlite".to_string();
    // Enable cron scheduling for automatic workflow execution
    config.enable_cron_scheduling = true;
    config.cron_enable_recovery = true;
    config.cron_poll_interval = Duration::from_secs(5); // Check every 5 seconds for demo
    config.cron_recovery_interval = Duration::from_secs(30); // Recovery check every 30 seconds

    let runner = DefaultRunner::with_config(&db_url, config).await?;

    // Step 7: Wait for registry reconciler to load the workflow
    println!("⏳ Waiting for registry reconciler to load workflow...");
    let workflow_name = "data_processing"; // Use the workflow name from simple-packaged-demo

    // Give the reconciler some time to complete startup reconciliation and register tasks
    println!("   Waiting for reconciler startup and task registration (10 seconds)...");
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    // Test if workflow is available before trying to execute
    println!("🔍 Checking if workflow is available for execution...");
    let mut test_context = Context::new();
    test_context.insert("test", serde_json::json!("availability_check"))?;

    match runner.execute(workflow_name, test_context).await {
        Ok(_) => println!("✅ Workflow is available for execution"),
        Err(e) => {
            println!("❌ Workflow still not available: {}", e);
            println!("   This indicates an issue with the reconciler → task registry integration");
            return Err(e.into());
        }
    }

    // Step 8: Execute the workflow from the registry
    println!("🚀 Executing workflow from registry...");

    let mut context = Context::new();
    context.insert("demo", serde_json::json!("registry-execution"))?;

    let result = runner.execute(workflow_name, context).await?;

    println!("✅ Workflow executed successfully!");
    println!("   Execution ID: {}", result.execution_id);
    println!("   Status: {:?}", result.status);

    let final_context = &result.final_context;
    if let Some(extracted) = final_context.get("extracted_records") {
        println!("   Extracted records: {}", extracted);
    }
    if let Some(reports) = final_context.get("generated_reports") {
        if let Some(arr) = reports.as_array() {
            println!("   Generated {} reports", arr.len());
        }
    }

    // Step 9: Set up cron scheduling for automated execution
    println!("\n⏰ Setting up cron scheduling for automated workflow execution...");

    // Register a cron schedule to run the workflow every 30 seconds for demo purposes
    let schedule_id = runner
        .register_cron_workflow(
            workflow_name,
            "*/30 * * * * *", // Every 30 seconds for demo visibility
            "UTC",
        )
        .await?;

    println!(
        "✅ Cron schedule registered (ID: {}) - workflow will run every 30 seconds",
        schedule_id
    );

    // Step 10: Let the scheduled executions run for a demo period
    println!("🕐 Running scheduled executions for 2 minutes (you can monitor the logs)...");
    println!("   Press Ctrl+C to shutdown gracefully before the 2 minutes are up");

    let runtime_duration = Duration::from_secs(20); // 2 minutes demo

    // Sleep for demo duration or until interrupted
    tokio::select! {
        _ = tokio::time::sleep(runtime_duration) => {
            println!("⏰ Demo time completed");
        }
        _ = tokio::signal::ctrl_c() => {
            println!("🛑 Received shutdown signal");
        }
    }

    // Step 11: Show execution statistics before shutdown
    println!("\n📊 Gathering execution statistics...");
    let stats = runner
        .get_cron_execution_stats(Utc::now() - ChronoDuration::try_hours(1).unwrap())
        .await?;

    println!("📈 Execution Statistics (last hour):");
    println!("   Total executions: {}", stats.total_executions);
    println!("   Successful executions: {}", stats.successful_executions);
    println!(
        "   Failed executions: {}",
        stats.total_executions - stats.successful_executions
    );
    if stats.total_executions > 0 {
        println!(
            "   Success rate: {:.1}%",
            (stats.successful_executions as f64 / stats.total_executions as f64) * 100.0
        );
    }

    // Cleanup
    println!("\n🔧 Shutting down gracefully...");
    runner.shutdown().await?;

    println!("🎉 Registry Execution Demo with Cron Scheduling complete!");
    println!("   Database saved at: {}", db_path);
    Ok(())
}

async fn build_package() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Build the package using the library function directly (like cloacina-app)
    let workspace_root = find_workspace_root()?;
    let project_path = workspace_root.join("examples/simple-packaged-demo");

    let temp_dir = TempDir::new()?;
    let output_path = temp_dir.path().join("package.cloacina");

    // Create compile options
    let options = cloacina::packaging::CompileOptions {
        target: None,
        profile: "debug".to_string(),
        cargo_flags: vec![],
        jobs: None,
    };

    // Build the package using the library function directly
    cloacina::packaging::package_workflow(project_path, output_path.clone(), options)?;

    Ok(tokio::fs::read(output_path).await?)
}

async fn create_database() -> Result<Database, Box<dyn std::error::Error>> {
    let database = Database::new(":memory:", "", 5);

    let conn = database.pool().get().await?;
    conn.interact(move |conn| cloacina::database::run_migrations(conn))
        .await??;

    Ok(database)
}

fn find_workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = std::env::current_dir()?;

    loop {
        if path.join("Cargo.toml").exists() && path.join("examples").exists() {
            return Ok(path);
        }

        match path.parent() {
            Some(parent) => path = parent.to_path_buf(),
            None => return Err("Could not find workspace root".into()),
        }
    }
}
