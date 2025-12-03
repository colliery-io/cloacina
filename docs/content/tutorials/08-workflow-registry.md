---
title: "08 - Working with the Workflow Registry"
description: "Set up and use the workflow registry for dynamic workflow execution with cron scheduling"
weight: 18
reviewer: "dstorey"
review_date: "2025-01-17"
---

Welcome to the workflow registry tutorial! Building on the packaged workflows from Tutorial 07, you'll now learn how to set up a workflow registry, register packaged workflows, and execute them dynamically with cron scheduling. The registry system enables you to manage workflows independently from your application code.

## Prerequisites

- Completion of [Tutorial 07: Packaged Workflows]({{< ref "/tutorials/07-packaged-workflows/" >}})
- Understanding of databases (SQLite or PostgreSQL)
- Basic understanding of cron expressions
- A code editor of your choice

## Time Estimate
20-25 minutes

## What is the Workflow Registry?

The workflow registry is a system that allows you to:

- **Store and manage** packaged workflows (.cloacina files)
- **Dynamically load** workflows at runtime without application restarts
- **Execute workflows** on-demand or on schedules
- **Version and update** workflows independently
- **Isolate workflows** per tenant in multi-tenant scenarios

{{< hint type=info title="Registry vs Direct Execution" >}}
**Without Registry** (Tutorial 07):
- Workflows are part of your application binary
- Updates require recompilation and deployment
- All tenants share the same workflows

**With Registry** (This tutorial):
- Workflows are stored as .cloacina packages
- Runtime loading and hot-swapping
- Per-tenant workflow isolation
- Scheduled execution support
{{< /hint >}}

## Setting Up the Registry Demo

Let's work with the complete registry demonstration that's already set up in the Cloacina repository:

```bash
# Navigate to the registry execution demo
cd cloacina/examples/registry-execution-demo
ls -la
```

Your directory structure should look like this:
```
examples/registry-execution-demo/
├── Cargo.toml
├── README.md
└── src/
    └── main.rs
```

Let's examine the `Cargo.toml` configuration:

```toml
[package]
name = "registry-execution-demo"
version = "0.1.0"
edition = "2021"

[dependencies]
cloacina = { path = "../../cloacina" }
cloacina-ctl = { path = "../../cloacina-ctl" }
tokio = { version = "1.35", features = ["full"] }
serde_json = "1.0"
tempfile = "3.8"
uuid = { version = "1.6", features = ["v4"] }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
chrono = "0.4"
```

{{< hint type=warning title="Key Dependencies" >}}
The registry demo requires:

1. **cloacina** for workflow execution and database storage
2. **cloacina-ctl** for package building functionality
3. **tokio** for async runtime
4. **tracing-subscriber** for detailed logging
5. **chrono** for cron scheduling support

Cloacina automatically detects the database backend based on your connection URL. This example uses SQLite for simplicity (`sqlite://` URLs), but PostgreSQL (`postgresql://` or `postgres://` URLs) is also supported for production deployments.
{{< /hint >}}

## Understanding the Registry Demo

The registry execution demo demonstrates the complete workflow lifecycle:

1. **Package Building** - Creates a .cloacina package from simple-packaged-demo
2. **Registry Setup** - Initializes storage and database
3. **Package Registration** - Stores the package in the registry
4. **Reconciliation** - Loads packages into the runtime
5. **Execution** - Runs workflows through DefaultRunner
6. **Cron Scheduling** - Sets up automated execution

Let's examine the key parts of the demo:

## Registry Setup and Storage

```rust
// Step 2: Set up shared database and storage
println!("📋 Setting up shared storage and database...");
let storage_path = "/tmp/cloacina_demo_storage";
std::fs::create_dir_all(storage_path)?;
let storage = FilesystemRegistryStorage::new(storage_path)?;
println!("📋 Storage directory: {}", storage_path);

// Use a persistent file-based database that both registry and runner can share
let db_path = "/tmp/cloacina_debug.db";
let db_url = format!("sqlite://{}?mode=rwc", db_path);
println!("📋 Database will be saved to: {}", db_path);

let database = Database::new(&db_url, "", 5);
let conn = database.pool().get().await?;
conn.interact(move |conn| cloacina::database::run_migrations(conn))
    .await??;
```

This sets up:
- **Persistent storage** at `/tmp/cloacina_demo_storage` for .cloacina files
- **SQLite database** at `/tmp/cloacina_debug.db` for metadata
- **Database migrations** to ensure proper schema

## Package Registration

```rust
// Step 3: Register the workflow package
println!("📋 Registering workflow package...");
let mut registry = WorkflowRegistryImpl::new(storage, database)?;
match registry.register_workflow(package_data).await {
    Ok(package_id) => {
        println!("✅ Package registered with ID: {}", package_id);
    }
    Err(RegistryError::PackageExists { package_name, version }) => {
        println!("⚠️  Package already exists: {} v{} - continuing with existing package", package_name, version);
        // For demo purposes, we'll continue with the existing package
        // In production, you might want to check versions or handle differently
    }
    Err(e) => return Err(e.into()),
};
```

The registration process:
- **Validates** the package format and metadata
- **Stores binary data** in filesystem storage
- **Saves metadata** to the database
- **Handles collisions** gracefully by warning and continuing

## Runner Configuration with Registry

```rust
// Configure DefaultRunner with registry reconciler enabled and storage path
let mut config = DefaultRunnerConfig::default();
config.enable_registry_reconciler = true;
config.registry_storage_path = Some(PathBuf::from(storage_path));

// Enable cron scheduling for automatic workflow execution
config.enable_cron_scheduling = true;
config.cron_enable_recovery = true;
config.cron_poll_interval = Duration::from_secs(5); // Check every 5 seconds for demo
config.cron_recovery_interval = Duration::from_secs(30); // Recovery check every 30 seconds

let runner = DefaultRunner::with_config(&db_url, config).await?;
```

This configuration:
- **Enables registry reconciliation** to automatically load packages
- **Sets storage path** for the reconciler to find packages
- **Enables cron scheduling** for automated workflow execution
- **Configures polling intervals** for demo responsiveness

## Cron Scheduling

```rust
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

println!("✅ Cron schedule registered (ID: {}) - workflow will run every 30 seconds", schedule_id);
```

The cron scheduling:
- **Registers workflows** for automatic execution
- **Uses cron expressions** for flexible scheduling
- **Supports timezones** for global deployments
- **Returns schedule IDs** for management

## Running the Registry Demo

Let's run the complete demo:

```bash
# From the registry-execution-demo directory
cargo run
```

## Understanding the Demo Output

You should see output similar to this:

```
🚀 Cloacina Registry Execution Demo

📦 Building workflow package...
Packaging workflow project: "/path/to/cloacina/examples/simple-packaged-demo"
✅ Package built: 1065567 bytes

📋 Setting up shared storage and database...
📋 Storage directory: /tmp/cloacina_demo_storage
📋 Database will be saved to: /tmp/cloacina_debug.db
Database connection pool initialized

📋 Registering workflow package...
✅ Package registered with ID: 12345678-1234-5678-9abc-123456789012

🔍 Available workflows:
  - data_processing from package simple_demo (v1.0.0) - 3 tasks

📥 Getting workflow from registry...
✅ Workflow loaded: simple_demo v1.0.0

▶️  Setting up execution environment with shared database...
⏳ Waiting for registry reconciler to load workflow...
   Waiting for reconciler startup and task registration (10 seconds)...

🔍 Checking if workflow is available for execution...
✅ Workflow is available for execution

🚀 Executing workflow from registry...
🔍 Collecting data...
✅ Collected 1000 records
⚙️  Processing data...
✅ Processed 950 valid records
📊 Generating report...
✅ Report generated successfully

✅ Workflow executed successfully!
   Execution ID: 87654321-4321-8765-cba9-876543210987
   Status: Completed
   Extracted records: 1000
   Generated 1 reports

⏰ Setting up cron scheduling for automated workflow execution...
✅ Cron schedule registered (ID: abcdef12-3456-7890-abcd-ef1234567890) - workflow will run every 30 seconds

🕐 Running scheduled executions for 2 minutes (you can monitor the logs)...
   Press Ctrl+C to shutdown gracefully before the 2 minutes are up

[2025-06-17T23:37:00.000000Z] Successfully executed and audited workflow data_processing
[2025-06-17T23:37:30.000000Z] Successfully executed and audited workflow data_processing
[2025-06-17T23:38:00.000000Z] Successfully executed and audited workflow data_processing

📊 Gathering execution statistics...
📈 Execution Statistics (last hour):
   Total executions: 5
   Successful executions: 5
   Failed executions: 0
   Success rate: 100.0%

🔧 Shutting down gracefully...
🎉 Registry Execution Demo with Cron Scheduling complete!
   Database saved at: /tmp/cloacina_debug.db
```

## Key Demo Features Demonstrated

### 1. Package Lifecycle Management

The demo shows the complete package lifecycle:
- **Building** from source code to .cloacina package
- **Registration** in the workflow registry
- **Loading** through reconciliation
- **Execution** via the scheduler

### 2. Collision Handling

If you run the demo multiple times, you'll see:
```
⚠️  Package already exists: simple_demo v1.0.0 - continuing with existing package
```

This demonstrates graceful handling of package collisions.

### 3. Registry Reconciliation

The reconciler automatically:
- **Scans** the registry for available packages
- **Loads** workflow metadata and task definitions
- **Registers** tasks in the runtime registry
- **Makes workflows available** for execution

### 4. Cron Scheduling

The demo sets up automated execution:
- **Schedules** the workflow to run every 30 seconds
- **Monitors** execution statistics
- **Provides** execution history and success rates

### 5. Persistent Storage

Both the database and storage are persistent:
- **Database**: `/tmp/cloacina_debug.db` - Contains metadata
- **Storage**: `/tmp/cloacina_demo_storage` - Contains .cloacina files

This means packages remain available across restarts.

## Production Considerations

### Storage Backends

For production deployments, consider:

- **PostgreSQL**: Better for high-throughput and multi-instance deployments
- **Object Storage**: S3-compatible storage for package binaries
- **Network Storage**: Shared storage for multi-instance setups

### Cron Scheduling

Production cron scheduling should use:
- **Realistic intervals**: Avoid overly frequent execution
- **Timezone awareness**: Consider global user bases
- **Recovery mechanisms**: Handle missed executions
- **Load balancing**: Distribute scheduled work

### Registry Management

Production registries need:
- **Version management**: Handle workflow updates
- **Access control**: Secure package access
- **Monitoring**: Track execution and performance
- **Backup strategies**: Protect registry data

## Next Steps

Congratulations! You've successfully set up and used the workflow registry. In future tutorials and guides, you'll learn:

- **Multi-tenant Registries**: Different workflows per tenant
- **Production Deployment**: Scaling and monitoring registries
- **Advanced Scheduling**: Complex cron patterns and dependencies
- **Registry Security**: Access control and package validation

## Related Resources

- [Tutorial 07: Packaged Workflows]({{< ref "/tutorials/07-packaged-workflows/" >}})
- [API Documentation]({{< ref "/reference/api/" >}})

## Download the Example

You can find the complete example code in our [GitHub repository](https://github.com/colliery-io/cloacina/tree/main/examples/registry-execution-demo).
