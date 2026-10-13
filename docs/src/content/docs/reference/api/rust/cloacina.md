---
title: "cloacina"
description: "API reference of the Rust module cloacina."
editUrl: false
sidebar:
  order: 2
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Cloacina: Embedded Workflow Framework for Rust**

Cloacina is a **library** for building resilient task workflows directly within your Rust applications.
Unlike standalone orchestration services (Airflow, Prefect), Cloacina embeds into your existing
applications to manage complex multi-step workflows with automatic retry, state persistence,
and dependency resolution.

**What Cloacina Is**

- **Embedded Framework**: Integrates directly into your Rust applications
- **Resilient Execution**: Automatic retries, failure recovery, and state persistence
- **Type-Safe Workflows**: Compile-time validation of task dependencies and data flow
- **Database-Backed**: Uses PostgreSQL or SQLite for reliable state management
- **Multi-Tenant Ready**: PostgreSQL schema-based isolation for complete tenant separation
- **Async-First**: Built on tokio for high-performance concurrent execution
- **Content-Versioned**: Automatic workflow versioning based on task code and structure

**What Cloacina Is Not**

- **Standalone Service**: Not a separate process you deploy and manage
- **Distributed Scheduler**: Doesn't coordinate tasks across multiple machines
- **Web Platform**: No built-in UI or REST API (though you can build one)
- **General-Purpose Cron**: Built-in cron triggers schedule *workflows* (see
  [`DefaultRunner::register_cron_workflow`](../../rust/cloacina/registry/reconciler/#CronWorkflowRegistrar.register_cron_workflow)),
  not arbitrary system jobs
- **Message Queue**: Not designed for high-throughput message processing
- **State Machine**: Focuses on task execution rather than state transitions

**Perfect For**

- **Data Processing Applications**: ETL workflows within your data services
- **Background Job Processing**: Complex multi-step jobs in web applications
- **Batch Processing**: Resilient batch operations with dependency management
- **Integration Workflows**: Multi-step API integrations with error recovery
- **Report Generation**: Multi-step report creation with data dependencies
- **Data Migration**: Complex data transformation and loading operations

**Quick Start Tutorial**

**Your First Task**

Define a task using the `#[task]` macro:

```rust
use cloacina::*;

#[task(
    id = "process_data",
    dependencies = []
)]
async fn process_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Your business logic here
    context.insert("processed", serde_json::json!(true))?;
    println!("Data processed successfully!");
    Ok(())
}
```

**Building a Workflow**

Group related tasks into a workflow by applying the `#[workflow]` attribute
macro to a module. The workflow registers itself by name; you execute it
through a runner (next section):

```rust
use cloacina::*;

#[workflow(
    name = "etl_pipeline",
    description = "Extract, Transform, Load workflow"
)]
pub mod etl_pipeline {
    use super::*;

    #[task(id = "extract", dependencies = [])]
    pub async fn extract_data(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        ctx.insert("raw_data", serde_json::json!({"users": [1, 2, 3]}))?;
        Ok(())
    }

    #[task(id = "transform", dependencies = ["extract"])]
    pub async fn transform_data(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        if let Some(data) = ctx.get("raw_data") {
            ctx.insert("transformed_data", serde_json::json!({"processed": data}))?;
        }
        Ok(())
    }

    #[task(id = "load", dependencies = ["transform"])]
    pub async fn load_data(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        println!("Loading data to warehouse...");
        Ok(())
    }
}
```

**Execution with Database Persistence**

```rust
use cloacina::executor::WorkflowExecutor;
use cloacina::runner::DefaultRunner;
use cloacina::Context;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the runner with a database connection
    let runner = DefaultRunner::new("postgresql://user:pass@localhost/mydb").await?;

    // Execute workflow with automatic state persistence
    let context = Context::new();
    let result = runner.execute("etl_pipeline", context).await?;

    println!("Workflow completed: {:?}", result.status);
    runner.shutdown().await?;
    Ok(())
}
```

**Multi-Tenant Support**

Cloacina provides complete tenant isolation with zero collision risk:

**PostgreSQL Schema-Based Multi-Tenancy**

```rust
use cloacina::runner::DefaultRunner;

// Each tenant gets their own PostgreSQL schema
let tenant_a = DefaultRunner::with_schema(
    "postgresql://user:pass@localhost/cloacina",
    "tenant_a"
).await?;

let tenant_b = DefaultRunner::with_schema(
    "postgresql://user:pass@localhost/cloacina",
    "tenant_b"
).await?;

// Or using the builder pattern
let runner = DefaultRunner::builder()
    .database_url("postgresql://user:pass@localhost/cloacina")
    .schema("my_tenant")
    .build()
    .await?;
```

**SQLite File-Based Multi-Tenancy**

```rust
// Each tenant gets their own database file
let tenant_a = DefaultRunner::new("sqlite://./tenant_a.db").await?;
let tenant_b = DefaultRunner::new("sqlite://./tenant_b.db").await?;
```

Benefits:
- **Zero collision risk** - Impossible for tenants to access each other's data
- **No query changes** - All existing DAL code works unchanged
- **Performance** - No overhead from filtering every query
- **Clean separation** - Each tenant can even have different schema versions

**Core Concepts**

**Tasks**

Tasks are the fundamental units of work. Use the `#[task]` macro for the most convenient definition:

```rust
#[task(
    id = "my_task",
    dependencies = ["other_task_id"],
    retry_attempts = 3,
    retry_backoff = "exponential"
)]
async fn my_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Task implementation
    Ok(())
}
```

Task attributes:
- `id`: Unique identifier for the task (defaults to the function name)
- `dependencies`: List of task IDs that must complete before this task runs
- `retry_attempts`, `retry_delay_ms`, `retry_max_delay_ms`, `retry_jitter`,
  `retry_backoff` (`"fixed"` | `"linear"` | `"exponential"`),
  `retry_condition`: Optional configuration for automatic retries
- `trigger_rules`: Optional conditions for task execution

**Context**

The Context is a type-safe container for sharing data between tasks:

```rust
// Insert data
context.insert("user_id", serde_json::json!(12345))?;
context.insert("config", serde_json::json!({"env": "prod"}))?;

// Read data in later tasks
if let Some(user_id) = context.get("user_id") {
    println!("Processing for user: {}", user_id);
}
```

Context features:
- Type-safe serialization with serde_json
- Atomic updates for data consistency
- Automatic persistence to database
- Thread-safe access patterns

**Dependency Management**

Cloacina automatically resolves task dependencies and executes them in the correct order:

```rust
#[task(id = "a", dependencies = [])]
async fn task_a(_: &mut Context<serde_json::Value>) -> Result<(), TaskError> { Ok(()) }

#[task(id = "b", dependencies = ["a"])]  // Runs after A
async fn task_b(_: &mut Context<serde_json::Value>) -> Result<(), TaskError> { Ok(()) }

#[task(id = "c", dependencies = ["a", "b"])]  // Runs after A and B
async fn task_c(_: &mut Context<serde_json::Value>) -> Result<(), TaskError> { Ok(()) }

// Execution order: A → B → C
```

Dependency features:
- Automatic cycle detection
- Parallel execution of independent tasks
- Runtime validation of dependency chains
- Visual dependency graph generation

**Architecture Overview**

```mermaid
graph TB
    subgraph "Your Application"
        A[Task Definitions]
        B[Workflow Builder]
        C[Workflow Execution]
    end

    subgraph "Cloacina Core"
        D[Task Registry]
        E[Context Management]
        F[Dependency Resolution]
        G[Scheduler Engine]
    end

    subgraph "Persistence Layer"
        H[Data Access Layer]
        I[Database Models]
        J[PostgreSQL Database]
    end

    A --> D
    B --> F
    C --> G
    D --> H
    E --> H
    F --> H
    G --> H
    H --> I
    I --> J
```

**Task Execution Lifecycle**

```mermaid
sequenceDiagram
    participant App as Your App
    participant W as Workflow
    participant S as Scheduler
    participant T as Task
    participant DB as Database

    App->>W: Build workflow
    W->>W: Validate dependencies
    App->>S: Execute workflow
    S->>DB: Load/create execution state
    S->>S: Plan execution order
    loop For each task level
        S->>T: Execute task(s)
        T->>T: Process data
        T->>DB: Persist context updates
        T->>S: Signal completion
    end
    S->>App: Return results
```

**Error Handling and Retries**

Configure retry policies for resilient execution:

```rust
use std::time::Duration;

#[task(
    id = "network_task",
    dependencies = [],
    retry_policy = RetryPolicy::builder()
        .max_attempts(3)
        .initial_delay(Duration::from_secs(1))
        .backoff_strategy(BackoffStrategy::Exponential { base: 2.0, multiplier: 1.0 })
        .retry_condition(RetryCondition::TransientOnly)
        .build()
)]
async fn network_task(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Network operation that might fail and will be retried
    Ok(())
}
```

Retry features:
- Configurable backoff strategies
- Conditional retry based on error type
- Maximum attempt limits
- Custom retry conditions
- Exponential and linear backoff

**Conditional Execution**

Use trigger rules for conditional task execution:

```rust
#[task(
    id = "conditional_task",
    dependencies = ["validation_task"],
    trigger_rules = serde_json::json!({
        "type": "Conditional",
        "condition": {
            "field": "validation_passed",
            "operator": "Equals",
            "value": true
        }
    })
)]
async fn conditional_task(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Only runs if validation_passed == true in context
    Ok(())
}
```

Trigger rule features:
- Field-based conditions
- Multiple operators (Equals, NotEquals, GreaterThan, etc.)
- Complex boolean expressions
- Context-aware evaluation

**Database Setup**

Cloacina requires PostgreSQL for state persistence:

```bash
# Create database
createdb myapp_pipelines

# Set connection string
export DATABASE_URL="postgresql://user:password@localhost/myapp_pipelines"
```

Database features:
- Automatic schema migrations
- Transaction support
- Connection pooling
- Execution history tracking
- State persistence

**Feature Overview**

- **Type Safety**: Leverages Rust's type system for compile-time guarantees
- **Content-Based Versioning**: Automatic workflow versioning based on task code and structure
- **Async/Await**: Built on tokio for high-performance async execution
- **Database Integration**: PostgreSQL support with Diesel ORM
- **Dependency Resolution**: Automatic topological sorting and cycle detection
- **Error Recovery**: Comprehensive error types and checkpoint support
- **Macro Support**: Convenient procedural macros for task definition with code fingerprinting
- **Logging**: Structured logging with configurable levels
- **Metrics**: Built-in performance monitoring
- **Testing**: Comprehensive test utilities and mocks

**Documentation Navigation**

**Learn Cloacina (Tutorials)**

- [Your First Workflow](../../rust/cloacina/task/) - Start here for macro-based task creation
- [Multi-Task Workflows](../../rust/cloacina/workflow/) - Building complex dependency chains
- [Working with Data](../../rust/cloacina/context/) - Context management and serialization
- [Error Handling](../../rust/cloacina/retry/) - Retry policies and failure recovery

**Solve Problems (How-To Guides)**

- [Task Patterns](../../rust/cloacina/task/) - Common task implementation patterns
- [Testing Strategies] - Unit and integration testing approaches
- [Database Operations](../../rust/cloacina/dal/) - Working with execution history
- [Error Recovery](../../rust/cloacina/error/) - Handling partial failures and recovery

**API Reference**

- [`Task`](../../rust/cloacina-workflow/task/#trait.Task) - Core task trait and macro
- [`Workflow`](../../rust/cloacina/workflow/#struct.Workflow) - Workflow construction and management
- [`Context`](../../rust/cloacina-workflow/context/#struct.Context) - Data container for inter-task communication
- [`TaskScheduler`](../../rust/cloacina/execution_planner/#struct.TaskScheduler) - Execution engine and scheduling
- [Error Types](../../rust/cloacina/error/) - Complete error hierarchy

**Understand the Design (Explanations)**

- [Architecture Decisions](../../rust/cloacina/) - Why Cloacina works the way it does
- [Execution Model](../../rust/cloacina/executor/) - How tasks are scheduled and run
- [Versioning Strategy](../../rust/cloacina/workflow/) - Content-based workflow versioning
- [Recovery Mechanisms](../../rust/cloacina/execution_planner/) - Checkpoint and restart concepts

**Modules**

- [`context`](../../rust/cloacina/context/): Context management for sharing data between tasks
- [`task`](../../rust/cloacina/task/): Core task trait and registry functionality
- [`workflow`](../../rust/cloacina/workflow/): Workflow construction and dependency management
- [`registry`](../../rust/cloacina/registry/): Dynamic workflow package loading and storage
- [`database`](../../rust/cloacina/database/): Database connection and persistence
- [`error`](../../rust/cloacina/error/): Comprehensive error types
- [`models`](../../rust/cloacina/models/): Database models and schemas
- [`dal`](../../rust/cloacina/dal/): Data access layer
- [`execution_planner`](../../rust/cloacina/execution_planner/): Task scheduler for persistent workflow execution
- [`executor`](../../rust/cloacina/executor/): Unified execution engine
- [`logging`](../../rust/cloacina/logging/): Structured logging setup
- [`retry`](../../rust/cloacina/retry/): Retry policies and backoff strategies

## Re-exports

- [`cloacina_computation_graph`](../../rust/cloacina-computation-graph/)
- `inventory` (`inventory`)
- `serde_json` (`serde_json`)
- [`init_logging`](../../rust/cloacina/logging/#fn.init_logging)
- [`var`](../../rust/cloacina/var/#fn.var)
- [`var_or`](../../rust/cloacina/var/#fn.var_or)
- [`Database`](../../rust/cloacina/database/connection/#struct.Database)
- [`Graph`](../../rust/cloacina-computation-graph/#trait.Graph)
- [`ComputationReactionMode`](../../rust/cloacina-computation-graph/#enum.ReactionMode) (`cloacina_computation_graph::ReactionMode`)
- [`Reactor`](../../rust/cloacina-computation-graph/#trait.Reactor)
- [`ReactorConstructor`](../../rust/cloacina-computation-graph/#type.ReactorConstructor)
- [`ReactorConstructorRef`](../../rust/cloacina-computation-graph/#struct.ReactorConstructorRef)
- [`ReactorRegistration`](../../rust/cloacina-computation-graph/#struct.ReactorRegistration)
- [`ComputationGraphRegistration`](../../rust/cloacina-computation-graph/#struct.ComputationGraphRegistration)
- `TriggerlessGraph` (`computation_graph::TriggerlessGraph`)
- `TriggerlessGraphFn` (`computation_graph::TriggerlessGraphFn`)
- `TriggerlessGraphRegistration` (`computation_graph::TriggerlessGraphRegistration`)
- [`Context`](../../rust/cloacina-workflow/context/#struct.Context)
- [`SecretAccessError`](../../rust/cloacina-workflow/secret/#enum.SecretAccessError)
- [`SecretResolver`](../../rust/cloacina-workflow/secret/#trait.SecretResolver)
- [`SecretResolverError`](../../rust/cloacina-workflow/secret/#enum.SecretResolverError)
- [`CronError`](../../rust/cloacina-workflow/cron_evaluator/#enum.CronError)
- [`CronEvaluator`](../../rust/cloacina-workflow/cron_evaluator/#struct.CronEvaluator)
- [`CronRecoveryConfig`](../../rust/cloacina/cron_recovery/#struct.CronRecoveryConfig)
- [`CronRecoveryService`](../../rust/cloacina/cron_recovery/#struct.CronRecoveryService)
- [`Scheduler`](../../rust/cloacina/cron_trigger_scheduler/#struct.Scheduler)
- [`SchedulerConfig`](../../rust/cloacina/cron_trigger_scheduler/#struct.SchedulerConfig)
- `AdminError` (`database::AdminError`)
- `DatabaseAdmin` (`database::DatabaseAdmin`)
- `TenantConfig` (`database::TenantConfig`)
- `TenantCredentials` (`database::TenantCredentials`)
- [`UniversalBool`](../../rust/cloacina/database/universal_types/#struct.UniversalBool)
- [`UniversalTimestamp`](../../rust/cloacina/database/universal_types/#struct.UniversalTimestamp)
- [`UniversalUuid`](../../rust/cloacina/database/universal_types/#struct.UniversalUuid)
- [`DefaultDispatcher`](../../rust/cloacina/dispatcher/default/#struct.DefaultDispatcher)
- [`DispatchError`](../../rust/cloacina/dispatcher/types/#enum.DispatchError)
- [`Dispatcher`](../../rust/cloacina/dispatcher/traits/#trait.Dispatcher)
- [`ExecutionResult`](../../rust/cloacina/dispatcher/types/#struct.ExecutionResult)
- [`ExecutionStatus`](../../rust/cloacina/dispatcher/types/#enum.ExecutionStatus)
- [`ExecutorMetrics`](../../rust/cloacina/dispatcher/types/#struct.ExecutorMetrics)
- [`TaskExecutor`](../../rust/cloacina/dispatcher/traits/#trait.TaskExecutor)
- [`TaskReadyEvent`](../../rust/cloacina/dispatcher/types/#struct.TaskReadyEvent)
- [`CheckpointError`](../../rust/cloacina-workflow/error/#enum.CheckpointError)
- [`ContextError`](../../rust/cloacina/error/#enum.ContextError)
- [`ExecutorError`](../../rust/cloacina/error/#enum.ExecutorError)
- [`RegistrationError`](../../rust/cloacina/error/#enum.RegistrationError)
- [`SubgraphError`](../../rust/cloacina/error/#enum.SubgraphError)
- [`TaskError`](../../rust/cloacina-workflow/error/#enum.TaskError)
- [`ValidationError`](../../rust/cloacina/error/#enum.ValidationError)
- [`WorkflowError`](../../rust/cloacina/error/#enum.WorkflowError)
- [`TaskScheduler`](../../rust/cloacina/execution_planner/#struct.TaskScheduler)
- [`TriggerCondition`](../../rust/cloacina/execution_planner/trigger_rules/#enum.TriggerCondition)
- [`TriggerRule`](../../rust/cloacina/execution_planner/trigger_rules/#enum.TriggerRule)
- [`ValueOperator`](../../rust/cloacina/execution_planner/trigger_rules/#enum.ValueOperator)
- [`ExecutorConfig`](../../rust/cloacina/executor/types/#struct.ExecutorConfig)
- [`TaskHandle`](../../rust/cloacina/executor/task_handle/#struct.TaskHandle)
- [`TaskResult`](../../rust/cloacina/executor/workflow_executor/#struct.TaskResult)
- [`ThreadTaskExecutor`](../../rust/cloacina/executor/thread_task_executor/#struct.ThreadTaskExecutor)
- [`WorkflowExecution`](../../rust/cloacina/executor/workflow_executor/#struct.WorkflowExecution)
- [`WorkflowExecutionError`](../../rust/cloacina/executor/workflow_executor/#enum.WorkflowExecutionError)
- [`WorkflowExecutionResult`](../../rust/cloacina/executor/workflow_executor/#struct.WorkflowExecutionResult)
- [`WorkflowExecutor`](../../rust/cloacina/executor/workflow_executor/#trait.WorkflowExecutor)
- [`WorkflowStatus`](../../rust/cloacina/executor/workflow_executor/#enum.WorkflowStatus)
- [`return_task_handle`](../../rust/cloacina/executor/task_handle/#fn.return_task_handle)
- [`take_task_handle`](../../rust/cloacina/executor/task_handle/#fn.take_task_handle)
- [`with_task_handle`](../../rust/cloacina/executor/task_handle/#fn.with_task_handle)
- [`DependencyEdge`](../../rust/cloacina/graph/#struct.DependencyEdge)
- [`GraphEdge`](../../rust/cloacina/graph/#struct.GraphEdge)
- [`GraphMetadata`](../../rust/cloacina/graph/#struct.GraphMetadata)
- [`GraphNode`](../../rust/cloacina/graph/#struct.GraphNode)
- [`TaskNode`](../../rust/cloacina/graph/#struct.TaskNode)
- [`WorkflowGraph`](../../rust/cloacina/graph/#struct.WorkflowGraph)
- [`WorkflowGraphData`](../../rust/cloacina/graph/#struct.WorkflowGraphData)
- `ComputationGraphEntry` (`inventory_entries::ComputationGraphEntry`)
- `ReactorEntry` (`inventory_entries::ReactorEntry`)
- [`StreamBackendEntry`](../../rust/cloacina/inventory_entries/#struct.StreamBackendEntry)
- [`StreamBackendFactoryFn`](../../rust/cloacina/inventory_entries/#type.StreamBackendFactoryFn)
- `TaskEntry` (`inventory_entries::TaskEntry`)
- `TriggerEntry` (`inventory_entries::TriggerEntry`)
- `TriggerlessGraphEntry` (`inventory_entries::TriggerlessGraphEntry`)
- [`WorkflowEntry`](../../rust/cloacina/inventory_entries/#struct.WorkflowEntry)
- [`BackoffStrategy`](../../rust/cloacina-workflow/retry/#enum.BackoffStrategy)
- [`RetryCondition`](../../rust/cloacina-workflow/retry/#enum.RetryCondition)
- [`RetryPolicy`](../../rust/cloacina-workflow/retry/#struct.RetryPolicy)
- [`RetryPolicyBuilder`](../../rust/cloacina-workflow/retry/#struct.RetryPolicyBuilder)
- [`DefaultRunnerBuilder`](../../rust/cloacina/runner/default_runner/config/#struct.DefaultRunnerBuilder)
- [`DefaultRunner`](../../rust/cloacina/runner/default_runner/#struct.DefaultRunner)
- [`DefaultRunnerConfig`](../../rust/cloacina/runner/default_runner/config/#struct.DefaultRunnerConfig)
- [`Runtime`](../../rust/cloacina/runtime/#struct.Runtime)
- [`parse_namespace`](../../rust/cloacina-workflow/namespace/#fn.parse_namespace)
- [`Task`](../../rust/cloacina/registry/loader/constructor_loader/#enum.ConstructorBinding)
- [`TaskNamespace`](../../rust/cloacina-workflow/namespace/#struct.TaskNamespace)
- [`TaskRegistry`](../../rust/cloacina/task/#struct.TaskRegistry)
- [`TaskState`](../../rust/cloacina-workflow/task/#enum.TaskState)
- [`TenantKey`](../../rust/cloacina/tenant_scope/#struct.TenantKey)
- [`TenantOwner`](../../rust/cloacina/tenant_scope/#struct.TenantOwner)
- [`TenantResolveMiss`](../../rust/cloacina/tenant_scope/#enum.TenantResolveMiss)
- [`TenantScope`](../../rust/cloacina/tenant_scope/#struct.TenantScope)
- [`Trigger`](../../rust/cloacina-workflow/trigger/#trait.Trigger)
- [`TriggerConfig`](../../rust/cloacina/trigger/#struct.TriggerConfig)
- [`TriggerError`](../../rust/cloacina/trigger/#enum.TriggerError)
- [`TriggerResult`](../../rust/cloacina-workflow/trigger/#enum.TriggerResult)
- [`DependencyGraph`](../../rust/cloacina/workflow/graph/#struct.DependencyGraph)
- [`Workflow`](../../rust/cloacina/workflow/#struct.Workflow)
- [`WorkflowBuilder`](../../rust/cloacina/workflow/builder/#struct.WorkflowBuilder)
- [`WorkflowMetadata`](../../rust/cloacina/workflow/metadata/#struct.WorkflowMetadata)
- `batch_accumulator` (`cloacina_macros::batch_accumulator`)
- `computation_graph` (`cloacina_macros::computation_graph`)
- `passthrough_accumulator` (`cloacina_macros::passthrough_accumulator`)
- `polling_accumulator` (`cloacina_macros::polling_accumulator`)
- [`reactor`](../../rust/cloacina-python/reactor/#fn.reactor)
- `state_accumulator` (`cloacina_macros::state_accumulator`)
- `stream_accumulator` (`cloacina_macros::stream_accumulator`)
- [`task`](../../rust/cloacina-python/task/#fn.task)
- [`trigger`](../../rust/cloacina/models/schedule/#NewSchedule.trigger)
- `workflow` (`cloacina_macros::workflow`)
