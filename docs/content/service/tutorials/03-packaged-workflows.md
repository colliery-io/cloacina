---
title: "03 — Packaged Workflows"
description: "Create distributable workflow packages with the packaged workflow system"
weight: 13
aliases:
  - "/workflows/tutorials/service/07-packaged-workflows/"

---

Welcome to the workflow packages tutorial! In this guide, you'll learn how to create distributable workflow packages that can be compiled to shared libraries and dynamically loaded at runtime. Workflow packages enable you to distribute complex workflows as standalone packages that can be shared, version-controlled, and deployed independently from the main application.

## Prerequisites

- Completion of [Tutorial 1]({{< ref "/embed/tutorials/01-first-workflow/" >}})
- Basic understanding of Rust and Cargo projects
- Rust toolchain installed (rustc, cargo)
- cloacinactl installed (for packaging commands)
- A code editor of your choice

## Time Estimate
15-20 minutes

## What Are Workflow Packages?

Workflow packages are workflows compiled to shared libraries and distributed as `.cloacina` archives that a server loads dynamically at runtime — see [Packaged Workflow Architecture]({{< ref "/engine/explanation/packaged-workflow-architecture" >}}) for the full rationale and trade-offs.

## Setting Up Your Project

For this tutorial, we'll work with the example project that's already set up in the Cloacina repository. Let's examine the `simple-packaged-demo`:

```bash
# Navigate to the Cloacina repository
cd cloacina/examples/features/workflows/simple-packaged
ls -la
```

Your directory structure should look like this:
```
examples/features/workflows/simple-packaged/
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

Let's examine the `Cargo.toml` configuration for workflow packages:

```toml
[package]
name = "simple-packaged-demo"
version = "1.0.0"
edition = "2021"

# Required for workflow packages - generates shared library
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
# The "packaged" feature emits the FFI exports a server loads at runtime;
# "macros" pulls in the #[workflow]/#[task] attribute macros.
cloacina-workflow = { version = "0.7.0", features = ["packaged", "macros"] }
serde_json = "1.0"
tokio = { version = "1.35", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"
```

Packages depend on **cloacina-workflow** (compile-only workflow types), not the full
`cloacina` crate — see [Packaged Workflow Architecture]({{< ref "/engine/explanation/packaged-workflow-architecture" >}}).

{{< hint type=warning title="Important Configuration Differences" >}}
Workflow packages have different requirements:

1. **Library crate**: Use `lib.rs` instead of `main.rs`
2. **Crate type**: Must include `"cdylib"` for shared library generation
3. **`features = ["packaged", "macros"]`**: Enable `packaged` on `cloacina-workflow` for FFI export generation, plus `macros` for the `#[workflow]`/`#[task]` attribute macros

This configuration allows the workflow to be compiled as both a regular library (`rlib`) and a shared library (`cdylib`) for dynamic loading. The database backend (PostgreSQL or SQLite) is detected automatically at runtime based on the connection URL.
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

use cloacina_workflow::{workflow, task, Context, TaskError};

/// Simple Data Processing Workflow
///
/// A minimal workflow that demonstrates the complete workflow package lifecycle
/// with data processing, validation, and reporting.
#[workflow(
    name = "data_processing",
    package = "simple_demo",
    description = "Simple data processing workflow for demonstration",
    author = "Cloacina Demo Team"
)]
pub mod data_processing {
    use super::*;

    /// Step 1: Collect input data
    #[task(
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
```

The key difference from an embedded workflow is the `package = "..."` argument on
`#[workflow]` plus the `cdylib` crate type — together they make the macro emit the
FFI exports a server needs to load the workflow at runtime. The task definitions
inside the module are identical to the embedded form. See
[Packaged Workflow Architecture]({{< ref "/engine/explanation/packaged-workflow-architecture" >}})
for why packages are built this way.

## Build and run

Build the package to a shared library, then run the end-to-end demo, which packages
it, loads it through the registry, and executes it:

```bash
# From the simple-packaged directory
cargo build --release
cargo run --example end_to_end_demo
```

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

## Variations

- **Package it by hand** instead of via the demo: `cloacinactl package pack . --out simple-demo.cloacina`, then `cloacinactl package validate simple-demo.cloacina` to check the archive against the canonical format without uploading. See the [CLI Reference]({{< ref "/reference/cli" >}}).
- **Run the unit tests** for the workflow logic alone: `cargo test`.

## Next Steps

You've built a workflow package and run it end to end.

Next: [04 — Packaging a Computation Graph]({{< ref "/service/tutorials/04-packaging" >}})

## Related Resources

- [Tutorial 09: Working with the Workflow Registry]({{< ref "/embed/tutorials/09-workflow-registry/" >}})
- [Explanation: Packaged Workflow Architecture]({{< ref "/engine/explanation/packaged-workflow-architecture/" >}})
- [API Documentation]({{< ref "/reference/" >}})

## Download the Example

You can find the complete example code in our [GitHub repository](https://github.com/colliery-io/cloacina/tree/main/examples/features/workflows/simple-packaged).
