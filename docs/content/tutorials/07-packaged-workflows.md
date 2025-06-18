---
title: "07 - Packaged Workflows"
description: "Create distributable workflow packages with the packaged workflow system"
weight: 17
reviewer: "dstorey"
review_date: "2025-01-17"
---

Welcome to the packaged workflows tutorial! In this guide, you'll learn how to create distributable workflow packages that can be compiled to shared libraries and dynamically loaded at runtime. Packaged workflows enable you to distribute complex workflows as standalone packages that can be shared, version-controlled, and deployed independently from the main application.

## Prerequisites

- Completion of [Tutorial 1]({{< ref "/tutorials/01-first-workflow/" >}})
- Basic understanding of Rust and Cargo projects
- Rust toolchain installed (rustc, cargo)
- cloacina-ctl installed (for packaging commands)
- A code editor of your choice

## Time Estimate
15-20 minutes

## What Are Packaged Workflows?

Before we start building, let's understand what packaged workflows are and when to use them:

**Embedded Workflows** (from previous tutorials):
- Defined directly in your application code
- Compiled into your binary
- Great for application-specific business logic

**Packaged Workflows** (this tutorial):
- Defined in separate Cargo projects
- Compiled to shared libraries (.so/.dylib/.dll)
- Packaged into .cloacina archives for distribution
- Dynamically loaded at runtime
- Perfect for reusable workflows and multi-tenant scenarios

{{< hint type=info title="When to Use Packaged Workflows" >}}
Choose packaged workflows when you need:
- **Distribution**: Share workflows between teams or applications
- **Versioning**: Independent workflow lifecycle management
- **Multi-tenancy**: Different workflows per tenant
- **Hot-swapping**: Update workflows without restarting the application
- **Modularity**: Separate workflow development from application development
{{< /hint >}}

## Setting Up Your Project

For this tutorial, we'll work with the example project that's already set up in the Cloacina repository. Let's examine the `simple-packaged-demo`:

```bash
# Navigate to the Cloacina repository
cd cloacina/examples/simple-packaged-demo
ls -la
```

Your directory structure should look like this:
```
examples/simple-packaged-demo/
├── Cargo.toml
├── src/
│   └── lib.rs           # Note: lib.rs, not main.rs!
├── examples/
│   ├── end_to_end_demo.rs
│   └── package_workflow.rs
└── tests/
    ├── ffi_tests.rs
    └── host_managed_registry_tests.rs
```

Let's examine the `Cargo.toml` configuration for packaged workflows:

```toml
[package]
name = "simple-packaged-demo"
version = "1.0.0"
edition = "2021"

# Required for packaged workflows - generates shared library
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
cloacina = { path = "../../cloacina", default-features = false, features = ["macros", "sqlite"] }
cloacina-macros = { path = "../../cloacina-macros" }
serde_json = "1.0"
tokio = { version = "1.35", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"
```

{{< hint type=warning title="Important Configuration Differences" >}}
Packaged workflows have different requirements:

1. **Library crate**: Use `lib.rs` instead of `main.rs`
2. **Crate type**: Must include `"cdylib"` for shared library generation
3. **Database features**: Include sqlite/postgres features as needed for your runtime
4. **cloacina-macros**: Explicit dependency required for packaged workflows

This configuration allows the workflow to be compiled as both a regular library (`rlib`) and a shared library (`cdylib`) for dynamic loading.
{{< /hint >}}

## Understanding the Packaged Workflow

Let's examine the workflow definition in `src/lib.rs`:

```rust
/*!
# Simple Packaged Workflow Demo

This example demonstrates the complete end-to-end lifecycle of packaged workflows:

1. **Define** - Create a packaged workflow with tasks
2. **Compile** - Build to shared library (.so/.dylib/.dll)
3. **Package** - Create .cloacina archive
4. **Load** - Dynamically load via registry
5. **Execute** - Run tasks through scheduler
*/

use cloacina::{packaged_workflow, task, Context, TaskError};

/// Simple Data Processing Workflow
///
/// A minimal workflow that demonstrates the complete packaged workflow lifecycle
/// with data processing, validation, and reporting.
#[packaged_workflow(
    name = "data_processing",
    package = "simple_demo",
    description = "Simple data processing workflow for demonstration",
    author = "Cloacina Demo Team"
)]
pub mod data_processing {
    use super::*;

    /// Step 1: Collect input data
    #[task(
        id = "collect_data",
        dependencies = [],
        retry_attempts = 2
    )]
    pub async fn collect_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        println!("🔍 Collecting data...");

        // Simulate data collection
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let data = serde_json::json!({
            "records": 1000,
            "source": "demo_database",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        context.insert("raw_data", data)?;
        println!("✅ Collected 1000 records");
        Ok(())
    }

    /// Step 2: Process the collected data
    #[task(
        id = "process_data",
        dependencies = ["collect_data"],
        retry_attempts = 3
    )]
    pub async fn process_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        println!("⚙️  Processing data...");

        // Get input data
        let raw_data = context
            .get("raw_data")
            .ok_or_else(|| TaskError::ValidationFailed {
                message: "Missing raw_data".to_string(),
            })?;

        // Simulate processing
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let processed = serde_json::json!({
            "processed_records": 950,  // Some records filtered out
            "original_count": raw_data["records"],
            "processing_time_ms": 200,
            "status": "completed"
        });

        context.insert("processed_data", processed)?;
        println!("✅ Processed 950 valid records");
        Ok(())
    }

    /// Step 3: Generate summary report
    #[task(
        id = "generate_report",
        dependencies = ["process_data"],
        retry_attempts = 1
    )]
    pub async fn generate_report(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        println!("📊 Generating report...");

        // Get processed data
        let processed_data =
            context
                .get("processed_data")
                .ok_or_else(|| TaskError::ValidationFailed {
                    message: "Missing processed_data".to_string(),
                })?;

        // Simulate report generation
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let report = serde_json::json!({
            "report_id": format!("RPT_{}", chrono::Utc::now().timestamp()),
            "summary": {
                "total_processed": processed_data["processed_records"],
                "success_rate": "95%",
                "processing_time": processed_data["processing_time_ms"]
            },
            "generated_at": chrono::Utc::now().to_rfc3339()
        });

        context.insert("final_report", report)?;
        println!("✅ Report generated successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_execution() {
        let mut context = Context::new();

        // Execute workflow steps in order
        data_processing::collect_data(&mut context).await.unwrap();
        data_processing::process_data(&mut context).await.unwrap();
        data_processing::generate_report(&mut context)
            .await
            .unwrap();

        // Verify final state
        let report = context.get("final_report").unwrap();
        assert!(report["report_id"].as_str().unwrap().starts_with("RPT_"));
        assert_eq!(report["summary"]["total_processed"], 950);
    }
}
```

## Understanding Packaged Workflow Code

Let's examine the key differences from embedded workflows:

### 1. The `#[packaged_workflow]` Macro

```rust
#[packaged_workflow(
    name = "data_processing",
    package = "simple_demo",
    description = "Simple data processing workflow for demonstration",
    author = "Cloacina Demo Team"
)]
```

This macro:
- **Generates FFI exports** for dynamic loading
- **Creates metadata** for package identification
- **Enables dynamic registration** with workflow registries
- **Provides namespace isolation** for multi-tenant scenarios

### 2. Module Structure

```rust
pub mod data_processing {
    // Tasks go inside the module
}
```

The workflow tasks must be defined inside the module created by the `#[packaged_workflow]` macro. This ensures proper namespacing and registration.

### 3. Task Dependencies and Context Flow

Our workflow demonstrates a typical data pipeline:
- `collect_data` → `process_data` → `generate_report`
- Each task receives data through context from previous tasks
- Error handling ensures the pipeline fails gracefully if data is missing

## Building and Testing Your Packaged Workflow

Let's build and test the simple-packaged-demo:

```bash
# From the simple-packaged-demo directory
cargo build --release
```

This creates a shared library in your target directory:
- **Linux**: `target/release/libsimple_packaged_demo.so`
- **macOS**: `target/release/libsimple_packaged_demo.dylib`
- **Windows**: `target/release/simple_packaged_demo.dll`

## Running the Examples

The demo includes several examples to demonstrate different aspects of packaged workflows:

### 1. Testing the Workflow Logic

First, run the unit tests to verify the workflow logic:

```bash
cargo test
```

### 2. Package Creation Demo

Run the packaging example to see how .cloacina packages are created:

```bash
cargo run --example package_workflow
```

This example:
- Builds the workflow to a shared library
- Creates a .cloacina package archive
- Demonstrates the packaging lifecycle

### 3. End-to-End Demo

Run the complete end-to-end demo:

```bash
cargo run --example end_to_end_demo
```

This demonstrates:
- Building and packaging the workflow
- Dynamic loading through the registry
- Task execution with full context flow
- Complete lifecycle from package to execution

## Understanding the Output

When you run the end-to-end demo, you should see output similar to:

```
🚀 Simple Packaged Workflow Demo
===============================

Step 1: Building workflow package...
✅ Package built: 1234567 bytes

Step 2: Setting up registry and loading package...
✅ Package registered and loaded

Step 3: Executing workflow...
🔍 Collecting data...
✅ Collected 1000 records
⚙️  Processing data...
✅ Processed 950 valid records
📊 Generating report...
✅ Report generated successfully

📈 Final Report:
   Report ID: RPT_1705123456
   Records Processed: 950
   Success Rate: 95%
   Generated: 2025-01-17T10:30:45.123456+00:00

✅ Demo completed successfully!
```

## Creating a Package with cloacina-ctl

You can also create packages manually using cloacina-ctl:

```bash
# Create a .cloacina package
cloacina-ctl package . -o simple-demo.cloacina
```

This creates a `.cloacina` file that contains:
- The shared library for your platform
- Metadata about the workflow and its tasks
- Package information for registry systems

## Inspecting Your Package

You can inspect the package contents:

```bash
# Inspect the package
cloacina-ctl inspect simple-demo.cloacina
```

You should see output showing:
- Package metadata (name, version, author)
- Workflow information (data_processing)
- Task definitions and dependencies (collect_data → process_data → generate_report)
- Platform and architecture information

## What's Different from Embedded Workflows?

| Aspect | Embedded Workflows | Packaged Workflows |
|--------|-------------------|-------------------|
| **Distribution** | Part of application binary | Standalone .cloacina packages |
| **Loading** | Compile-time registration | Dynamic runtime loading |
| **Versioning** | Application version | Independent package versioning |
| **Deployment** | Requires application redeployment | Hot-swappable without downtime |
| **Multi-tenancy** | Shared across all tenants | Per-tenant workflow packages |
| **Testing** | Application integration tests | Independent package tests |

## Next Steps

Congratulations! You've created and tested your first packaged workflow. Next, you'll learn how to work with the workflow registry for dynamic loading and execution:

- [**Tutorial 08: Working with the Workflow Registry**]({{< ref "/tutorials/08-workflow-registry/" >}}) - Register and execute workflows dynamically
- **Multi-tenant Deployments**: Different workflows per tenant
- **Continuous Deployment**: CI/CD pipelines for workflow packages
- **Advanced Packaging**: Complex dependencies and cross-compilation

## Related Resources

- [Tutorial 08: Working with the Workflow Registry]({{< ref "/tutorials/08-workflow-registry/" >}})
- [Explanation: Packaged Workflow Architecture]({{< ref "/explanation/packaged-workflow-architecture/" >}})
- [How-to: Debug Packaged Workflows]({{< ref "/how-to-guides/debug-packaged-workflows/" >}})
- [API Documentation]({{< ref "/reference/api/" >}})

## Download the Example

You can find the complete example code in our [GitHub repository](https://github.com/colliery-io/cloacina/tree/main/examples/simple-packaged-demo).
