# Packaged Workflow Example

This example demonstrates how to create distributable workflow packages using the `#[packaged_workflow]` macro introduced in Phase 2 of the Cloacina packaged workflow system.

## Overview

Packaged workflows enable teams to:
- **Develop workflows independently** and deploy them as shared libraries
- **Scale executor pools horizontally** without recompiling/redeploying
- **Maintain namespace isolation** between different workflow packages
- **Support multi-tenancy** with proper task isolation
- **Version and fingerprint** workflows for integrity verification

## What This Example Demonstrates

### 1. Analytics Pipeline Package
A complete data processing workflow with:
- **Data Extraction** from multiple sources
- **Validation and Cleaning** with quality checks
- **Transformation** into analytics-ready format
- **Report Generation** with multiple output formats

### 2. Marketing Campaign Package
An automated marketing workflow with:
- **Customer Segmentation** based on behavior/value
- **Campaign Creation** with personalized content

### 3. Namespace Isolation
Both packages can have tasks with potentially conflicting names, but they're isolated:
```
public::analytics_pipeline::analytics_workflow::extract_data
public::marketing_campaigns::marketing_workflow::segment_customers
```

## Usage

### Building as a Library
```bash
# Build for local development
cargo build

# Build as shared library for distribution
cargo build --release
```

### Testing the Example
```bash
# Run all tests including workflow execution
cargo test

# Run a specific test
cargo test test_analytics_workflow_tasks

# Run with output to see task execution
cargo test test_analytics_workflow_tasks -- --nocapture
```

### Using in Production

1. **Compile to Shared Library**:
   ```bash
   cargo build --release --target x86_64-unknown-linux-gnu
   ```

2. **Deploy to Executors**:
   The resulting `.so` file can be loaded by Cloacina executors using the standard ABI:
   ```rust
   // Load package
   let package = load_workflow_package("analytics_pipeline.so")?;

   // Register tasks for tenant
   package.register_tasks("tenant_123", "analytics_workflow")?;

   // Tasks are now available under namespace:
   // "tenant_123::analytics_pipeline::analytics_workflow::extract_data"
   ```

## Package Metadata

Each packaged workflow includes metadata:

```rust
let metadata = analytics_workflow::get_package_metadata();
println!("Package: {} v{}", metadata.package, metadata.version);
println!("Description: {}", metadata.description);
println!("Fingerprint: {}", metadata.fingerprint);
```

## ABI Compatibility

Packages expose standard C-compatible entry points for dynamic loading:

```c
// Register all tasks in package
extern "C" fn register_tasks_abi(tenant_id: *const c_char, workflow_id: *const c_char);

// Get package metadata
extern "C" fn get_package_metadata_abi() -> *const PackageMetadata;
```

## Integration with Cloacina Core

When loaded dynamically, tasks are registered using the namespace isolation system:

```rust
// Tasks registered as:
TaskNamespace::new("tenant_id", "package_name", "workflow_id", "task_id")

// Enables precise task resolution:
let task = get_task(&TaskNamespace::new(
    "acme_corp",
    "analytics_pipeline",
    "analytics_workflow",
    "extract_data"
))?;
```

## Best Practices

1. **Use Descriptive Package Names**: Choose unique, descriptive names to avoid conflicts
2. **Version Appropriately**: Follow semantic versioning for package versions
3. **Document Dependencies**: Clearly document task dependencies within workflows
4. **Test Thoroughly**: Include comprehensive tests for all task scenarios
5. **Handle Errors Gracefully**: Use proper retry policies and error handling

## Next Steps

This example represents Phase 2 of the packaged workflow system. Future phases will include:

- **Phase 3**: Compiler crate for automated .so generation
- **Phase 4**: Dynamic loading and registry system
- **Phase 5**: Hot-swapping and version management

## Related Examples

- `examples/tutorial-*`: Basic workflow examples without packaging
- `examples/multi_tenant`: Multi-tenancy without packaging
- `docs/tutorials/`: Comprehensive workflow development guides
