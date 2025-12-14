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

//! # Cloacina: Embedded Pipeline Framework for Rust
//!
//! Cloacina is a **library** for building resilient task pipelines directly within your Rust applications.
//! Unlike standalone orchestration services (Airflow, Prefect), Cloacina embeds into your existing
//! applications to manage complex multi-step workflows with automatic retry, state persistence,
//! and dependency resolution.
//!
//! ## What Cloacina Is
//!
//! - **Embedded Framework**: Integrates directly into your Rust applications
//! - **Resilient Execution**: Automatic retries, failure recovery, and state persistence
//! - **Type-Safe Workflows**: Compile-time validation of task dependencies and data flow
//! - **Database-Backed**: Uses PostgreSQL or SQLite for reliable state management
//! - **Multi-Tenant Ready**: PostgreSQL schema-based isolation for complete tenant separation
//! - **Async-First**: Built on tokio for high-performance concurrent execution
//! - **Content-Versioned**: Automatic workflow versioning based on task code and structure
//!
//! ## What Cloacina Is Not
//!
//! - **Standalone Service**: Not a separate process you deploy and manage
//! - **Distributed Scheduler**: Doesn't coordinate tasks across multiple machines
//! - **Web Platform**: No built-in UI or REST API (though you can build one)
//! - **Cron Replacement**: Use proper schedulers for time-based triggering
//! - **Message Queue**: Not designed for high-throughput message processing
//! - **State Machine**: Focuses on task execution rather than state transitions
//!
//! ## Perfect For
//!
//! - **Data Processing Applications**: ETL pipelines within your data services
//! - **Background Job Processing**: Complex multi-step jobs in web applications
//! - **Batch Processing**: Resilient batch operations with dependency management
//! - **Integration Workflows**: Multi-step API integrations with error recovery
//! - **Report Generation**: Multi-step report creation with data dependencies
//! - **Data Migration**: Complex data transformation and loading operations
//!
//! ## Quick Start Tutorial
//!
//! ### Your First Task
//!
//! Define a task using the `#[task]` macro:
//!
//! ```rust,ignore
//! use cloacina::*;
//!
//! #[task(
//!     id = "process_data",
//!     dependencies = []
//! )]
//! async fn process_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     // Your business logic here
//!     context.insert("processed", serde_json::json!(true))?;
//!     println!("Data processed successfully!");
//!     Ok(())
//! }
//! ```
//!
//! ### Building a Workflow
//!
//! Create workflows with the `workflow!` macro:
//!
//! ```rust,ignore
//! use cloacina::*;
//!
//! #[task(id = "extract", dependencies = [])]
//! async fn extract_data(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     ctx.insert("raw_data", serde_json::json!({"users": [1, 2, 3]}))?;
//!     Ok(())
//! }
//!
//! #[task(id = "transform", dependencies = ["extract"])]
//! async fn transform_data(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     if let Some(data) = ctx.get("raw_data") {
//!         ctx.insert("transformed_data", serde_json::json!({"processed": data}))?;
//!     }
//!     Ok(())
//! }
//!
//! #[task(id = "load", dependencies = ["transform"])]
//! async fn load_data(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     println!("Loading data to warehouse...");
//!     Ok(())
//! }
//!
//! // Create the workflow
//! let workflow = workflow! {
//!     name: "etl_pipeline",
//!     description: "Extract, Transform, Load pipeline",
//!     tasks: [extract_data, transform_data, load_data]
//! };
//! ```
//!
//! ### Execution with Database Persistence
//!
//! ```rust,ignore
//! use cloacina::runner::{DefaultRunner, PipelineExecutor};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize executor with database connection
//!     let runner = DefaultRunner::new("postgresql://user:pass@localhost/mydb").await?;
//!
//!     // Execute workflow with automatic state persistence
//!     let context = Context::new();
//!     let result = executor.execute("etl_pipeline", context).await?;
//!
//!     println!("Pipeline completed: {:?}", result.status);
//!     Ok(())
//! }
//! ```
//!
//! ## Multi-Tenant Support
//!
//! Cloacina provides complete tenant isolation with zero collision risk:
//!
//! ### PostgreSQL Schema-Based Multi-Tenancy
//!
//! ```rust,ignore
//! use cloacina::runner::DefaultRunner;
//!
//! // Each tenant gets their own PostgreSQL schema
//! let tenant_a = DefaultRunner::with_schema(
//!     "postgresql://user:pass@localhost/cloacina",
//!     "tenant_a"
//! ).await?;
//!
//! let tenant_b = DefaultRunner::with_schema(
//!     "postgresql://user:pass@localhost/cloacina",
//!     "tenant_b"
//! ).await?;
//!
//! // Or using the builder pattern
//! let runner = DefaultRunner::builder()
//!     .database_url("postgresql://user:pass@localhost/cloacina")
//!     .schema("my_tenant")
//!     .build()
//!     .await?;
//! ```
//!
//! ### SQLite File-Based Multi-Tenancy
//!
//! ```rust,ignore
//! // Each tenant gets their own database file
//! let tenant_a = DefaultRunner::new("sqlite://./tenant_a.db").await?;
//! let tenant_b = DefaultRunner::new("sqlite://./tenant_b.db").await?;
//! ```
//!
//! Benefits:
//! - **Zero collision risk** - Impossible for tenants to access each other's data
//! - **No query changes** - All existing DAL code works unchanged
//! - **Performance** - No overhead from filtering every query
//! - **Clean separation** - Each tenant can even have different schema versions
//!
//! ## Core Concepts
//!
//! ### Tasks
//!
//! Tasks are the fundamental units of work. Use the `#[task]` macro for the most convenient definition:
//!
//! ```rust,ignore
//! #[task(
//!     id = "my_task",
//!     dependencies = ["other_task_id"],
//!     retry_policy = RetryPolicy::builder()
//!         .max_attempts(3)
//!         .build()
//! )]
//! async fn my_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     // Task implementation
//!     Ok(())
//! }
//! ```
//!
//! Task attributes:
//! - `id`: Unique identifier for the task
//! - `dependencies`: List of task IDs that must complete before this task runs
//! - `retry_policy`: Optional configuration for automatic retries
//! - `trigger_rules`: Optional conditions for task execution
//!
//! ### Context
//!
//! The Context is a type-safe container for sharing data between tasks:
//!
//! ```rust,ignore
//! // Insert data
//! context.insert("user_id", serde_json::json!(12345))?;
//! context.insert("config", serde_json::json!({"env": "prod"}))?;
//!
//! // Read data in later tasks
//! if let Some(user_id) = context.get("user_id") {
//!     println!("Processing for user: {}", user_id);
//! }
//! ```
//!
//! Context features:
//! - Type-safe serialization with serde_json
//! - Atomic updates for data consistency
//! - Automatic persistence to database
//! - Thread-safe access patterns
//!
//! ### Dependency Management
//!
//! Cloacina automatically resolves task dependencies and executes them in the correct order:
//!
//! ```rust,ignore
//! #[task(id = "a", dependencies = [])]
//! async fn task_a(_: &mut Context<serde_json::Value>) -> Result<(), TaskError> { Ok(()) }
//!
//! #[task(id = "b", dependencies = ["a"])]  // Runs after A
//! async fn task_b(_: &mut Context<serde_json::Value>) -> Result<(), TaskError> { Ok(()) }
//!
//! #[task(id = "c", dependencies = ["a", "b"])]  // Runs after A and B
//! async fn task_c(_: &mut Context<serde_json::Value>) -> Result<(), TaskError> { Ok(()) }
//!
//! // Execution order: A → B → C
//! ```
//!
//! Dependency features:
//! - Automatic cycle detection
//! - Parallel execution of independent tasks
//! - Runtime validation of dependency chains
//! - Visual dependency graph generation
//!
//! ## Architecture Overview
//!
//! ```mermaid
//! graph TB
//!     subgraph "Your Application"
//!         A[Task Definitions]
//!         B[Workflow Builder]
//!         C[Pipeline Execution]
//!     end
//!
//!     subgraph "Cloacina Core"
//!         D[Task Registry]
//!         E[Context Management]
//!         F[Dependency Resolution]
//!         G[Scheduler Engine]
//!     end
//!
//!     subgraph "Persistence Layer"
//!         H[Data Access Layer]
//!         I[Database Models]
//!         J[PostgreSQL Database]
//!     end
//!
//!     A --> D
//!     B --> F
//!     C --> G
//!     D --> H
//!     E --> H
//!     F --> H
//!     G --> H
//!     H --> I
//!     I --> J
//! ```
//!
//! ## Task Execution Lifecycle
//!
//! ```mermaid
//! sequenceDiagram
//!     participant App as Your App
//!     participant W as Workflow
//!     participant S as Scheduler
//!     participant T as Task
//!     participant DB as Database
//!
//!     App->>W: Build workflow
//!     W->>W: Validate dependencies
//!     App->>S: Execute workflow
//!     S->>DB: Load/create execution state
//!     S->>S: Plan execution order
//!     loop For each task level
//!         S->>T: Execute task(s)
//!         T->>T: Process data
//!         T->>DB: Persist context updates
//!         T->>S: Signal completion
//!     end
//!     S->>App: Return results
//! ```
//!
//! ## Error Handling and Retries
//!
//! Configure retry policies for resilient execution:
//!
//! ```rust,ignore
//! use std::time::Duration;
//!
//! #[task(
//!     id = "network_task",
//!     dependencies = [],
//!     retry_policy = RetryPolicy::builder()
//!         .max_attempts(3)
//!         .initial_delay(Duration::from_secs(1))
//!         .backoff_strategy(BackoffStrategy::Exponential { base: 2.0, multiplier: 1.0 })
//!         .retry_condition(RetryCondition::TransientOnly)
//!         .build()
//! )]
//! async fn network_task(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     // Network operation that might fail and will be retried
//!     Ok(())
//! }
//! ```
//!
//! Retry features:
//! - Configurable backoff strategies
//! - Conditional retry based on error type
//! - Maximum attempt limits
//! - Custom retry conditions
//! - Exponential and linear backoff
//!
//! ## Conditional Execution
//!
//! Use trigger rules for conditional task execution:
//!
//! ```rust,ignore
//! #[task(
//!     id = "conditional_task",
//!     dependencies = ["validation_task"],
//!     trigger_rules = serde_json::json!({
//!         "type": "Conditional",
//!         "condition": {
//!             "field": "validation_passed",
//!             "operator": "Equals",
//!             "value": true
//!         }
//!     })
//! )]
//! async fn conditional_task(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     // Only runs if validation_passed == true in context
//!     Ok(())
//! }
//! ```
//!
//! Trigger rule features:
//! - Field-based conditions
//! - Multiple operators (Equals, NotEquals, GreaterThan, etc.)
//! - Complex boolean expressions
//! - Context-aware evaluation
//!
//! ## Database Setup
//!
//! Cloacina requires PostgreSQL for state persistence:
//!
//! ```bash
//! # Create database
//! createdb myapp_pipelines
//!
//! # Set connection string
//! export DATABASE_URL="postgresql://user:password@localhost/myapp_pipelines"
//! ```
//!
//! Database features:
//! - Automatic schema migrations
//! - Transaction support
//! - Connection pooling
//! - Execution history tracking
//! - State persistence
//!
//! ## Feature Overview
//!
//! - **Type Safety**: Leverages Rust's type system for compile-time guarantees
//! - **Content-Based Versioning**: Automatic workflow versioning based on task code and structure
//! - **Async/Await**: Built on tokio for high-performance async execution
//! - **Database Integration**: PostgreSQL support with Diesel ORM
//! - **Dependency Resolution**: Automatic topological sorting and cycle detection
//! - **Error Recovery**: Comprehensive error types and checkpoint support
//! - **Macro Support**: Convenient procedural macros for task definition with code fingerprinting
//! - **Logging**: Structured logging with configurable levels
//! - **Metrics**: Built-in performance monitoring
//! - **Testing**: Comprehensive test utilities and mocks
//!
//! ## Documentation Navigation
//!
//! ### Learn Cloacina (Tutorials)
//! - [Your First Pipeline](crate::task) - Start here for macro-based task creation
//! - [Multi-Task Workflows](crate::workflow) - Building complex dependency chains
//! - [Working with Data](crate::context) - Context management and serialization
//! - [Error Handling](crate::retry) - Retry policies and failure recovery
//!
//! ### Solve Problems (How-To Guides)
//! - [Task Patterns](crate::task) - Common task implementation patterns
//! - [Testing Strategies] - Unit and integration testing approaches
//! - [Database Operations](crate::dal) - Working with execution history
//! - [Error Recovery](crate::error) - Handling partial failures and recovery
//!
//! ### API Reference
//! - [`Task`] - Core task trait and macro
//! - [`Workflow`] - Pipeline construction and management
//! - [`Context`] - Data container for inter-task communication
//! - [`TaskScheduler`] - Execution engine and scheduling
//! - [Error Types](crate::error) - Complete error hierarchy
//!
//! ### Understand the Design (Explanations)
//! - [Architecture Decisions](crate) - Why Cloacina works the way it does
//! - [Execution Model](crate::executor) - How tasks are scheduled and run
//! - [Versioning Strategy](crate::workflow) - Content-based workflow versioning
//! - [Recovery Mechanisms](crate::task_scheduler) - Checkpoint and restart concepts
//!
//! ## Modules
//!
//! - [`context`]: Context management for sharing data between tasks
//! - [`task`]: Core task trait and registry functionality
//! - [`workflow`]: Workflow construction and dependency management
//! - [`registry`]: Dynamic workflow package loading and storage
//! - [`database`]: Database connection and persistence
//! - [`error`]: Comprehensive error types
//! - [`models`]: Database models and schemas
//! - [`dal`]: Data access layer
//! - [`task_scheduler`]: Task scheduler for persistent workflow execution
//! - [`executor`]: Unified execution engine
//! - [`logging`]: Structured logging setup
//! - [`retry`]: Retry policies and backoff strategies

// Re-export cloacina_workflow crate for macro-generated code compatibility
// This makes cloacina_workflow available to any crate that depends on cloacina
pub extern crate cloacina_workflow;

/// Prelude module for convenient imports.
///
/// The prelude provides convenient access to the most commonly used types
/// in Cloacina. Import everything with:
///
/// ```rust
/// use cloacina::prelude::*;
/// ```
///
/// This gives you access to:
/// - Core types: [`Context`], [`Task`], [`Workflow`], [`WorkflowBuilder`]
/// - Error types: [`TaskError`], [`WorkflowError`], [`ExecutorError`]
/// - Retry configuration: [`RetryPolicy`], [`BackoffStrategy`]
/// - Task state and scheduling: [`TaskState`], [`TriggerRule`]
/// - Execution: [`DefaultRunner`], [`PipelineExecutor`]
/// - Macros: `#[task]` and `workflow!` (when "macros" feature is enabled)
pub mod prelude {
    // Core types
    pub use crate::context::Context;
    pub use crate::task::{Task, TaskRegistry, TaskState};
    pub use crate::workflow::{DependencyGraph, Workflow, WorkflowBuilder, WorkflowMetadata};

    // Trigger types
    pub use crate::trigger::{Trigger, TriggerConfig, TriggerError, TriggerResult};

    // Error types
    pub use crate::error::{ExecutorError, TaskError, WorkflowError};

    // Retry configuration
    pub use crate::retry::{BackoffStrategy, RetryCondition, RetryPolicy, RetryPolicyBuilder};

    // Task scheduling
    pub use crate::task_scheduler::{TaskScheduler, TriggerCondition, TriggerRule, ValueOperator};

    // Execution
    pub use crate::executor::{
        PipelineExecution, PipelineExecutor, PipelineResult, PipelineStatus,
    };
    pub use crate::runner::{DefaultRunner, DefaultRunnerBuilder, DefaultRunnerConfig};

    // Universal types for database interop
    pub use crate::database::{UniversalBool, UniversalTimestamp, UniversalUuid};

    // Re-export macros when feature is enabled
    #[cfg(feature = "macros")]
    pub use cloacina_macros::{task, workflow};
}

// #[cfg(feature = "auth")]
// pub mod auth;
pub mod context;
pub mod cron_evaluator;
pub mod cron_recovery;
pub mod cron_scheduler;
pub mod dal;
pub mod database;
pub mod dispatcher;
pub mod error;
pub mod executor;
pub mod graph;
pub mod logging;
pub mod models;
pub mod packaging;
pub mod registry;
pub mod retry;
pub mod runner;
pub mod task;
pub mod task_scheduler;
pub mod trigger;
pub mod trigger_scheduler;
pub mod workflow;

pub use logging::init_logging;

#[cfg(test)]
pub use logging::init_test_logging;

#[cfg(test)]
pub fn setup_test() {
    init_test_logging();
}

pub use database::connection::Database;

// Re-export key types for convenience
pub use context::Context;
pub use cron_evaluator::{CronError, CronEvaluator};
pub use cron_recovery::{CronRecoveryConfig, CronRecoveryService};
pub use cron_scheduler::{CronScheduler, CronSchedulerConfig};
#[cfg(feature = "postgres")]
pub use database::{AdminError, DatabaseAdmin, TenantConfig, TenantCredentials};
pub use database::{UniversalBool, UniversalTimestamp, UniversalUuid};
pub use dispatcher::{
    DefaultDispatcher, DispatchError, Dispatcher, ExecutionResult, ExecutionStatus,
    ExecutorMetrics, RoutingConfig, RoutingRule, TaskExecutor, TaskReadyEvent,
};
pub use error::{
    CheckpointError, ContextError, ExecutorError, RegistrationError, SubgraphError, TaskError,
    ValidationError, WorkflowError,
};
pub use executor::{
    ExecutorConfig, PipelineError, PipelineExecution, PipelineExecutor, PipelineResult,
    PipelineStatus, TaskResult, ThreadTaskExecutor,
};
pub use graph::{
    DependencyEdge, GraphEdge, GraphMetadata, GraphNode, TaskNode, WorkflowGraph, WorkflowGraphData,
};
pub use retry::{BackoffStrategy, RetryCondition, RetryPolicy, RetryPolicyBuilder};
pub use runner::DefaultRunnerBuilder;
pub use runner::{DefaultRunner, DefaultRunnerConfig};
pub use task::namespace::parse_namespace;
pub use task::{
    global_task_registry, register_task_constructor, Task, TaskNamespace, TaskRegistry, TaskState,
};
pub use task_scheduler::{TaskScheduler, TriggerCondition, TriggerRule, ValueOperator};
pub use trigger::{
    get_trigger, global_trigger_registry, register_trigger, register_trigger_constructor, Trigger,
    TriggerConfig, TriggerError, TriggerResult,
};
pub use trigger_scheduler::{TriggerScheduler, TriggerSchedulerConfig};
pub use workflow::{
    get_all_workflows, global_workflow_registry, register_workflow_constructor, DependencyGraph,
    Workflow, WorkflowBuilder, WorkflowMetadata,
};

// Re-export the macros from cloacina-macros
#[cfg(feature = "macros")]
pub use cloacina_macros::{packaged_workflow, task, workflow};
