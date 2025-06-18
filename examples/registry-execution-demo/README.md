# Registry Execution Demo

This example demonstrates the end-to-end workflow execution using Cloacina's workflow registry system.

## What it does

1. **Builds a .cloacina package** - Uses cloacina-ctl to compile the packaged-workflow-example into a distributable package
2. **Registers the package** - Loads the package into the workflow registry with metadata extraction
3. **Lists available workflows** - Shows what workflows are available in the registry
4. **Loads the workflow** - Dynamically loads the workflow from the registry
5. **Executes the workflow** - Runs the workflow using DefaultRunner with full scheduling and execution

## Running the demo

```bash
cd examples/registry-execution-demo
cargo run
```

## Expected output

```
🚀 Cloacina Registry Execution Demo

📦 Building workflow package...
✅ Package built: 1048576 bytes

📋 Registering workflow package...
✅ Package registered with ID: 550e8400-e29b-41d4-a716-446655440000

🔍 Available workflows:
  - analytics_pipeline (v2.1.0) - 4 tasks

📥 Loading workflow from registry...
✅ Workflow loaded into namespace: analytics_pipeline_v2_1_0

▶️  Executing workflow...
✅ Workflow executed successfully!
   Pipeline ID: 550e8400-e29b-41d4-a716-446655440001
   Status: Completed
   Extracted records: 2600
   Generated 4 reports

🎉 Demo complete!
```

## Key features demonstrated

- **Package compilation**: Shows how to build distributable workflow packages
- **Registry management**: Demonstrates package registration and metadata extraction
- **Dynamic loading**: Loads workflows from the registry at runtime
- **Complete execution**: Full workflow execution through scheduler/executor pipeline
- **Context passing**: Shows how data flows through the workflow tasks

## Architecture

The demo uses:
- `FilesystemRegistryStorage` for binary package storage
- SQLite in-memory database for metadata and execution tracking
- `DefaultRunner` which manages both scheduler and executor
- The packaged-workflow-example as the workflow to execute

This demonstrates how Cloacina can dynamically load and execute workflows from packages, enabling a plugin-style architecture for workflow distribution.
