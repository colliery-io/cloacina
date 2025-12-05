---
title: "Packaged Workflow Architecture"
description: "High-level system design and integration with cloacina core"
weight: 22
reviewer: "dstorey"
review_date: "2025-01-17"
---

This article explains the overall architecture of packaged workflows and how they integrate with the Cloacina core system. Understanding this architecture is essential for deploying packaged workflows in production and designing systems that leverage both packaged and embedded workflows.

## System Overview

Packaged workflows extend Cloacina's core architecture by adding dynamic loading capabilities while maintaining full compatibility with the existing embedded workflow system. The architecture enables seamless mixing of both workflow types within the same application.

### Core Design Principles

1. **Unified Execution**: Both embedded and packaged workflows use the same execution engine
2. **Registry Integration**: Packaged workflows integrate through a registry layer for lifecycle management
3. **Namespace Isolation**: Task namespaces prevent conflicts between packages and tenants
4. **Persistent Storage**: Workflows and execution state persist across restarts
5. **Hot-swapping**: Packaged workflows can be updated without application restarts
6. **Minimal Compilation Dependencies**: Packaged workflows can use `cloacina-workflow` (minimal types only) instead of the full `cloacina` crate, enabling fast compilation without database drivers

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Cloacina Application                         │
├─────────────────────────────────────────────────────────────────────┤
│                          DefaultRunner                              │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │
│  │   Task          │  │   Pipeline      │  │   Background    │     │
│  │   Scheduler     │  │   Executor      │  │   Services      │     │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘     │
├─────────────────────────────────────────────────────────────────────┤
│                      Global Task Registry                           │
│         (Unified namespace for embedded + packaged tasks)           │
├─────────────────────────────────────────────────────────────────────┤
│  Embedded Workflows        │        Packaged Workflows              │
│  ┌─────────────────┐      │      ┌─────────────────┐                │
│  │ workflow! macro │      │      │ Workflow        │                │
│  │ task! macro     │      │      │ Registry        │                │
│  │ Compile-time    │      │      │ - Storage       │                │
│  │ registration    │      │      │ - Loader        │                │
│  └─────────────────┘      │      │ - Validator     │                │
│                           │      │ - Reconciler    │                │
│                           │      └─────────────────┘                │
├─────────────────────────────────────────────────────────────────────┤
│                        Database Layer                               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │
│  │ Execution State │  │ Package         │  │ Cron Schedules  │     │
│  │ - Pipelines     │  │ Metadata        │  │ - Time-based    │     │
│  │ - Tasks         │  │ - Binary Store  │  │ - Recovery      │     │
│  │ - Context       │  │ - Registry      │  │ - Missed runs   │     │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
```

## Core Components

### DefaultRunner - Unified Execution Engine

The `DefaultRunner` is the central orchestrator that manages both embedded and packaged workflows:

```rust
pub struct DefaultRunner {
    /// Database connection for persistence and state management
    database: Database,
    /// Configuration parameters for the runner
    config: DefaultRunnerConfig,
    /// Task scheduler for managing workflow execution scheduling
    scheduler: Arc<TaskScheduler>,
    /// Task executor for running individual tasks
    executor: Arc<dyn TaskExecutorTrait>,
    /// Optional workflow registry for packaged workflows
    workflow_registry: Arc<RwLock<Option<Arc<WorkflowRegistryImpl<FilesystemRegistryStorage>>>>>,
    /// Optional registry reconciler for packaged workflows
    registry_reconciler: Arc<RwLock<Option<Arc<RegistryReconciler>>>>,
    /// Optional cron scheduler for time-based workflow execution
    cron_scheduler: Arc<RwLock<Option<Arc<CronScheduler>>>>,
}
```

### WorkflowRegistryImpl - Package Management

The workflow registry provides comprehensive package lifecycle management:

```rust
pub struct WorkflowRegistryImpl<S: RegistryStorage> {
    /// Storage backend for binary data (.cloacina files)
    storage: S,
    /// Database for metadata storage
    database: Database,
    /// Package loader for metadata extraction
    loader: PackageLoader,
    /// Task registrar for global registry integration
    registrar: TaskRegistrar,
    /// Package validator for safety checks
    validator: PackageValidator,
    /// Map of package IDs to registered task namespaces
    loaded_packages: HashMap<Uuid, Vec<TaskNamespace>>,
}
```

### Global Task Registry

Both embedded and packaged workflows register tasks in the same global registry, providing a unified view:

- **Embedded tasks**: Registered at compile-time via macro expansion
- **Packaged tasks**: Registered at runtime via `TaskRegistrar`
- **Namespace format**: `tenant.package.workflow.task_id`
- **Unified lookup**: All tasks discoverable through same interface

## Integration Points

### Task Registration

Both workflow types populate the same global task registry but through different mechanisms:

**Embedded Workflow Registration:**
```rust
// Generated by workflow! macro at compile-time
register_task_constructor(
    "embedded_workflow::collect_data",
    Box::new(|| Box::new(CollectDataTask::new()))
);
```

**Packaged Workflow Registration:**
```rust
// Runtime registration via TaskRegistrar
registrar.register_package_tasks(
    &package_metadata,
    &task_namespaces,
    library_handle
).await?;
```

### Workflow Execution

Both workflow types use identical execution paths:

1. **Task Discovery**: Lookup in global task registry
2. **Dependency Resolution**: Same graph algorithms via `TaskScheduler`
3. **Execution**: Same `ThreadTaskExecutor` runs tasks
4. **Context Management**: Same `Context` object for data flow
5. **Persistence**: Same database schema for execution state

### Configuration and Setup

The `DefaultRunner` can be configured with or without packaged workflow support:

```rust
// Embedded workflows only
let runner = DefaultRunner::new(&database_url).await?;

// With packaged workflow support
let mut config = DefaultRunnerConfig::default();
config.enable_registry_reconciler = true;
config.registry_storage_path = Some(PathBuf::from("/path/to/storage"));

let runner = DefaultRunner::with_config(&database_url, config).await?;
```

## Database Schema Integration

### Workflow Package Tables

Packaged workflows add dedicated tables to the schema:

**workflow_packages:**
```sql
CREATE TABLE workflow_packages (
    id UUID PRIMARY KEY,
    package_name TEXT NOT NULL,
    version TEXT NOT NULL,
    description TEXT,
    author TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    UNIQUE(package_name, version)
);
```

**workflow_registry:**
```sql
CREATE TABLE workflow_registry (
    id UUID PRIMARY KEY,
    package_id UUID NOT NULL REFERENCES workflow_packages(id),
    binary_data BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);
```

### Shared Execution Tables

Both workflow types use the same execution state tables:

- **pipelines**: Workflow execution instances
- **tasks**: Individual task execution records
- **cron_schedules**: Time-based scheduling
- **pipeline_context**: Execution context data

## Execution Lifecycle

### Embedded Workflow Lifecycle

1. **Compile-time**: Macros generate workflow structures and register tasks
2. **Runtime**: `runner.execute(workflow, context)` starts execution
3. **Scheduling**: `TaskScheduler` resolves dependencies
4. **Execution**: `ThreadTaskExecutor` runs tasks in dependency order
5. **Persistence**: Results saved to database

### Packaged Workflow Lifecycle

1. **Package Registration**: `.cloacina` file loaded into registry
2. **Task Registration**: Tasks extracted and registered in global registry
3. **Runtime**: Same execution path as embedded workflows
4. **Reconciliation**: Background service monitors for package changes
5. **Hot-swapping**: New package versions can replace existing ones

### Unified Execution Flow

```rust
// Same execution interface for both types
impl DefaultRunner {
    pub async fn execute<T>(&self, workflow_name: &str, context: Context<T>) -> Result<()>
    where T: Clone + Send + Sync + 'static {
        // 1. Create pipeline execution record
        let pipeline_id = self.pipeline_executor.create_pipeline(workflow_name, context).await?;

        // 2. Task discovery (embedded or packaged)
        let tasks = self.scheduler.get_workflow_tasks(workflow_name).await?;

        // 3. Dependency resolution and scheduling
        let execution_plan = self.scheduler.create_execution_plan(tasks).await?;

        // 4. Execute tasks (same executor for both types)
        self.executor.execute_plan(pipeline_id, execution_plan).await?;

        Ok(())
    }
}
```

## Namespace Management

### Task Namespacing

Packaged workflows use hierarchical namespaces to prevent conflicts:

**Format**: `{tenant}.{package}.{workflow}.{task_id}`

**Examples**:
- `acme.data_processor.etl_pipeline.extract_data`
- `acme.data_processor.etl_pipeline.transform_data`
- `beta_corp.ml_trainer.model_pipeline.train_model`

### Namespace Isolation

- **Tenant isolation**: Different tenants can use same package names
- **Package isolation**: Multiple packages can define same task names
- **Workflow isolation**: Multiple workflows in same package are isolated
- **Version isolation**: Different package versions maintain separate namespaces

## Background Services

### Registry Reconciler

Monitors the storage backend for package changes and automatically updates the registry:

```rust
pub struct RegistryReconciler {
    registry: Arc<WorkflowRegistryImpl<FilesystemRegistryStorage>>,
    config: ReconcilerConfig,
}

impl RegistryReconciler {
    pub async fn run(&self, mut shutdown: broadcast::Receiver<()>) {
        loop {
            // Scan storage for new/updated packages
            self.scan_and_reconcile().await;

            // Wait for next poll interval or shutdown
            tokio::select! {
                _ = shutdown.recv() => break,
                _ = tokio::time::sleep(self.config.poll_interval) => continue,
            }
        }
    }
}
```

### Cron Scheduler

Provides time-based execution for both workflow types:

- **Unified scheduling**: Works with embedded and packaged workflows
- **Recovery support**: Handles missed executions during downtime
- **Persistent schedules**: Cron expressions stored in database
- **Multi-tenant aware**: Schedules isolated by tenant namespace

## Multi-Tenancy Support

### Schema-Based Isolation (PostgreSQL)

```rust
// Tenant A
let runner_a = DefaultRunnerBuilder::new()
    .database_url("postgresql://user:pass@localhost/cloacina")
    .schema("tenant_a")
    .build()
    .await?;

// Tenant B
let runner_b = DefaultRunnerBuilder::new()
    .database_url("postgresql://user:pass@localhost/cloacina")
    .schema("tenant_b")
    .build()
    .await?;
```

### Storage Isolation

Each tenant can have isolated storage for packaged workflows:

```rust
// Tenant-specific storage paths
config.registry_storage_path = Some(PathBuf::from("/storage/tenant_a"));
```

## Crate Structure

Cloacina is organized into separate crates to support both embedded and packaged workflow development:

```
cloacina/
  cloacina-workflow/     # Minimal types for workflow compilation
  cloacina-macros/       # Procedural macros (#[task], #[packaged_workflow])
  cloacina/              # Full runtime (executor, scheduler, database)
```

### cloacina-workflow (Minimal Crate)

Contains only the types needed to compile workflows:
- `Context<T>` - Data container for task communication
- `Task` trait - Interface for task implementations
- `TaskError`, `ContextError` - Error types
- `RetryPolicy`, `BackoffStrategy` - Retry configuration
- `TaskNamespace` - Namespace utilities

**Dependencies**: `async-trait`, `serde`, `serde_json`, `thiserror`, `chrono`

**Does NOT include**: Database drivers (diesel), connection pools, executor, scheduler, libloading

### cloacina (Full Crate)

Re-exports everything from `cloacina-workflow` plus:
- Database backends (PostgreSQL, SQLite)
- `DefaultRunner` and execution engine
- Workflow registry and package loading
- Cron scheduling
- Multi-tenancy support

### Usage

| Use Case | Crate |
|----------|-------|
| Packaged workflows | `cloacina-workflow` (includes macros) |
| Embedded workflows | `cloacina` |
| Host application | `cloacina` |

## Comparison: Embedded vs Packaged

| Aspect | Embedded Workflows | Packaged Workflows |
|--------|-------------------|-------------------|
| **Definition** | `workflow!` and `task!` macros | `.cloacina` archives |
| **Registration** | Compile-time via macro expansion | Runtime via registry loading |
| **Distribution** | Part of application binary | Separate distributable files |
| **Loading** | Static linking | Dynamic loading via `libloading` |
| **Validation** | Compile-time dependency checking | Runtime validation during registration |
| **Hot-swapping** | Requires recompilation and restart | Runtime replacement possible |
| **Storage** | In-memory function pointers | Database + filesystem storage |
| **Deployment** | Application deployment | Independent package deployment |
| **Versioning** | Application version | Independent package versioning |
| **Multi-tenancy** | Shared across tenants | Tenant-specific packages possible |
| **Performance** | Direct function calls | FFI overhead |
| **Development** | Integrated development cycle | Independent development cycle |
| **Compile Time** | Full crate dependencies | Minimal dependencies with `cloacina-workflow` |

## Production Deployment Patterns

### Hybrid Architecture

Most production systems use both workflow types strategically:

**Embedded Workflows for:**
- Core business logic
- Application-specific workflows
- Performance-critical paths
- Workflows that rarely change

**Packaged Workflows for:**
- Customer-specific customizations
- Frequently updated workflows
- Multi-tenant scenarios
- External integrations

### Deployment Strategy

```rust
// Production configuration
let mut config = DefaultRunnerConfig::default();
config.max_concurrent_tasks = 50;
config.task_timeout = Duration::from_mins(30);
config.enable_registry_reconciler = true;
config.enable_cron_scheduling = true;
config.enable_recovery = true;

let runner = DefaultRunner::with_config(&database_url, config).await?;
```

### Operational Considerations

1. **Monitoring**: Same monitoring for both workflow types
2. **Logging**: Unified logging with namespace identification
3. **Metrics**: Combined metrics collection and reporting
4. **Backup**: Include both database and package storage
5. **Security**: Package validation and namespace isolation

## Best Practices

### Architecture Design

1. **Start with embedded workflows** for core functionality
2. **Add packaged workflows** for customization and extensions
3. **Use consistent naming** across embedded and packaged workflows
4. **Plan namespace hierarchy** before deployment
5. **Consider tenant isolation** requirements early

### Performance Optimization

1. **Minimize FFI calls** in packaged workflows
2. **Cache package metadata** to avoid repeated loading
3. **Use appropriate buffer sizes** for task communication
4. **Monitor resource usage** of both workflow types
5. **Tune database connection pools** for expected load

### Operational Excellence

1. **Version package dependencies** explicitly
2. **Test package compatibility** before deployment
3. **Monitor package storage** disk usage
4. **Implement gradual rollouts** for package updates
5. **Maintain rollback procedures** for package issues

## Related Resources

- [Tutorial: Creating Your First Packaged Workflow]({{< ref "/tutorials/07-packaged-workflows/" >}})
- [Explanation: Package Format]({{< ref "/explanation/package-format/" >}})
- [Explanation: FFI System]({{< ref "/explanation/ffi-system/" >}})
