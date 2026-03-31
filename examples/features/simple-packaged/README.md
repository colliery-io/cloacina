# Simple Workflow Package Demo

This example provides a **complete end-to-end demonstration** of workflow packages in Cloacina, showing the entire lifecycle from development to execution.

## 🎯 What This Demonstrates

### Complete Workflow Package Lifecycle:
1. **📝 Define** - Create workflow with `#[workflow]` macro
2. **🏗️ Compile** - Build to shared library (`.so`/`.dylib`/`.dll`)
3. **📦 Package** - Create distributable `.cloacina` archive
4. **🔄 Load** - Dynamically load via workflow registry
5. **⚡ Execute** - Run tasks through scheduler with dependency resolution
6. **📊 Monitor** - Track execution progress and results

### Key Features Showcased:
- **Namespace Isolation** - Tasks isolated under `tenant::package::workflow::task`
- **Dependency Resolution** - Automatic task ordering based on dependencies
- **Context Data Flow** - Data passing between tasks via execution context
- **Error Handling** - Retry policies and graceful error recovery
- **FFI Exports** - Standard C-compatible interface for dynamic loading

## 🚀 Quick Start

```bash
# 1. Build the workflow package
cargo build --release

# 2. See the compilation process
cargo run --example package_workflow

# 3. Run the complete end-to-end demo
cargo run --example end_to_end_demo

# 4. Run the tests
cargo test
```

## 📋 Example Workflow

The demo implements a simple **Data Processing Pipeline**:

```
collect_data → process_data → generate_report
```

### Tasks:
- **`collect_data`** - Simulates gathering data from external sources
- **`process_data`** - Validates and transforms the collected data
- **`generate_report`** - Creates summary report from processed data

### Data Flow:
```
raw_data → processed_data → final_report
```

## 🔧 Real-World Usage

### Dependencies

Workflow packages only need `cloacina-workflow`:

```toml
[dependencies]
cloacina-workflow = "0.2"  # Includes macros by default
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
```

### Development:
```rust
use cloacina_workflow::{workflow, task, Context, TaskError};

#[workflow(
    name = "data_processing",
    package = "simple_demo",
    description = "Data processing workflow",
    author = "Your Team"
)]
pub mod data_processing {
    #[task(id = "collect_data", dependencies = [])]
    pub async fn collect_data(context: &mut Context<Value>) -> Result<(), TaskError> {
        // Implementation
    }
}
```

### Compilation:
```bash
# Build as shared library
cargo build --release --target x86_64-unknown-linux-gnu

# Create distributable package
cloacina-ctl package build .
# → Generates: simple_demo.cloacina
```

### Deployment:
```rust
// Load package into registry
let package_data = std::fs::read("simple_demo.cloacina")?;
let package_id = registry.register_workflow(package_data).await?;

// Schedule workflow execution
scheduler.schedule_workflow("data_processing", context).await?;
```

## 🏗️ Architecture

### Workflow Package Structure:
```
simple_demo.cloacina
├── metadata.json           # Package information
├── lib/
│   └── libsimple_demo.so   # Compiled workflow
└── manifest.toml           # Task definitions
```

### Runtime Architecture:
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  .cloacina      │───▶│ Workflow        │───▶│ Task            │
│  Package        │    │ Registry        │    │ Scheduler       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │                       │
                                ▼                       ▼
                       ┌─────────────────┐    ┌─────────────────┐
                       │ Dynamic         │    │ Thread Task     │
                       │ Loader          │    │ Executor        │
                       └─────────────────┘    └─────────────────┘
```

## 🎯 Key Benefits

### For Developers:
- **Independent Development** - Teams can develop workflows separately
- **Language Agnostic** - Standard C ABI enables any language
- **Version Control** - Code fingerprinting for integrity verification
- **Testing** - Unit test individual tasks and full workflows

### For Operations:
- **Horizontal Scaling** - Deploy packages to multiple executors
- **Zero Downtime** - Hot-swap workflows without stopping executors
- **Multi-Tenancy** - Isolate workflows by tenant namespace
- **Observability** - Built-in monitoring and logging

### For Organizations:
- **Workflow Reuse** - Share packages across teams and projects
- **Dependency Management** - Clear task dependency definitions
- **Compliance** - Audit trail for all workflow executions
- **Resource Efficiency** - Shared infrastructure for all workflows

## 📊 Production Considerations

### Performance:
- **Lazy Loading** - Workflows loaded on-demand
- **Connection Pooling** - Efficient database resource usage
- **Parallel Execution** - Independent tasks run concurrently
- **Memory Management** - Automatic cleanup of completed workflows

### Security:
- **Namespace Isolation** - Tenants cannot access each other's data
- **Code Signing** - Verify package integrity before loading
- **Permission Control** - Fine-grained access controls
- **Audit Logging** - Complete execution history

### Monitoring:
- **Execution Metrics** - Task duration, success rates, error counts
- **Resource Usage** - Memory, CPU, database connections
- **Dependency Tracking** - Understand workflow bottlenecks
- **Alerting** - Automated notifications for failures

## 🔗 Related Examples

- `examples/tutorial-*` - Basic workflow development
- `examples/multi_tenant` - Multi-tenancy without packaging
- `examples/cron-scheduling` - Time-based workflow execution
- `examples/registry-execution-demo` - Advanced registry usage

## 📚 Further Reading

- [Workflow Package Architecture](../../docs/architecture/packaged-workflows.md)
- [Deployment Guide](../../docs/deployment/packaged-workflows.md)
- [Best Practices](../../docs/best-practices/workflow-design.md)
