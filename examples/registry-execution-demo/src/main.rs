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
# Registry Execution Demo

This example demonstrates the complete workflow lifecycle:
1. Build a .cloacina package
2. Register it to the workflow registry
3. Execute it using DefaultRunner

## Usage

```bash
cd examples/registry-execution-demo
cargo run
```
*/

use std::path::PathBuf;
use tempfile::TempDir;

use cloacina::database::Database;
use cloacina::registry::storage::FilesystemRegistryStorage;
use cloacina::registry::traits::WorkflowRegistry;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{init_logging, Context, PipelineExecutor};

// Import cloacina-ctl for package building
use cloacina_ctl::commands::package_workflow;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_logging(None);

    println!("🚀 Cloacina Registry Execution Demo\n");

    // Step 1: Build the .cloacina package
    println!("📦 Building workflow package...");
    let package_data = build_package().await?;
    println!("✅ Package built: {} bytes\n", package_data.len());

    // Step 2: Set up shared database and storage
    println!("📋 Setting up shared storage and database...");
    let temp_storage = TempDir::new()?;
    let storage = FilesystemRegistryStorage::new(temp_storage.path().to_path_buf())?;

    // Use a temporary file-based database that both registry and runner can share
    let temp_db = tempfile::NamedTempFile::new()?;
    let db_path = temp_db.path().to_string_lossy();
    let db_url = format!("sqlite://{}?mode=rwc", db_path);

    let database = Database::new(&db_url, "", 5);
    let conn = database.pool().get().await?;
    conn.interact(move |conn| cloacina::database::run_migrations(conn))
        .await??;

    // Step 3: Register the workflow package
    println!("📋 Registering workflow package...");
    let mut registry = WorkflowRegistryImpl::new(storage, database)?;
    let package_id = registry.register_workflow(package_data).await?;
    println!("✅ Package registered with ID: {}\n", package_id);

    // Step 4: List available workflows
    println!("🔍 Available workflows:");
    let workflows = registry.list_workflows().await?;
    for workflow in &workflows {
        println!(
            "  - {} (v{}) - {} tasks",
            workflow.package_name,
            workflow.version,
            workflow.tasks.len()
        );
    }
    println!();

    // Step 5: Get workflow from registry
    println!("📥 Getting workflow from registry...");
    let first_workflow = &workflows[0];
    let loaded_workflow = registry
        .get_workflow(&first_workflow.package_name, &first_workflow.version)
        .await?;
    if let Some(workflow) = &loaded_workflow {
        println!(
            "✅ Workflow loaded: {} v{}\n",
            workflow.metadata.package_name, workflow.metadata.version
        );
    } else {
        println!("❌ Failed to load workflow");
        return Ok(());
    }

    // Step 6: Set up DefaultRunner with shared database and storage
    println!("▶️  Setting up execution environment with shared database...");

    // Configure DefaultRunner with registry reconciler enabled and storage path
    let mut config = DefaultRunnerConfig::default();
    config.enable_registry_reconciler = true;
    config.registry_storage_path = Some(temp_storage.path().to_path_buf());

    let runner = DefaultRunner::with_config(&db_url, config).await?;

    // Step 7: Wait for registry reconciler to load the workflow
    println!("⏳ Waiting for registry reconciler to load workflow...");
    let workflow_name = "analytics_workflow"; // Use the workflow name, not package name

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

    // Cleanup
    runner.shutdown().await?;

    println!("\n🎉 Demo complete!");
    Ok(())
}

async fn build_package() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Build the package using the same approach as the tests
    let workspace_root = find_workspace_root()?;
    let project_path = workspace_root.join("examples/packaged-workflow-example");

    let temp_dir = TempDir::new()?;
    let output_path = temp_dir.path().join("package.cloacina");

    // Create minimal CLI for the package function (following test pattern)
    let cli = cloacina_ctl::cli::Cli {
        target: None,
        profile: "debug".to_string(),
        verbose: false,
        quiet: false,
        color: "auto".to_string(),
        jobs: None,
        command: cloacina_ctl::cli::Commands::Package {
            project_path: project_path.clone(),
            output: output_path.clone(),
            cargo_flags: vec![
                "--no-default-features".to_string(),
                "--features".to_string(),
                "sqlite".to_string(),
            ],
        },
    };

    // Build the package
    package_workflow(
        project_path,
        output_path.clone(),
        None,
        "debug".to_string(),
        vec![
            "--no-default-features".to_string(),
            "--features".to_string(),
            "sqlite".to_string(),
        ],
        &cli,
    )?;

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
