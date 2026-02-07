---
title: "Quick Start Guide"
description: "Get up and running with Cloacina quickly"
weight: 10
review_date: "2024-03-19"
reviewer: "dstorey"
---


Welcome to Cloacina! This guide will help you get started with building resilient task pipelines directly within your Rust applications.

## What is Cloacina?

Cloacina is an embedded pipeline framework for Rust that helps you build resilient task pipelines with:
- Automatic retries and failure recovery
- State persistence
- Type-safe workflows
- Database-backed execution
- Async-first design
- Content versioning

## Prerequisites

- Rust (latest stable version)
- SQLite or PostgreSQL database
- Basic understanding of async Rust

## Installation

Add Cloacina and its required dependencies to your `Cargo.toml`:

```toml
[dependencies]
cloacina = "0.1.0"
async-trait = "0.1"    # Required for async task definitions
ctor = "0.2"          # Required for task registration
serde_json = "1.0"    # Required for context data serialization
```

### Why These Dependencies?

- `async-trait`: Required because Cloacina tasks are async functions. This crate provides the necessary support for async functions in traits.
- `ctor`: Used for automatic task registration. This allows Cloacina to discover and register your tasks at compile time.
- `serde_json`: Used for serializing and deserializing data in the task context. This enables type-safe data sharing between tasks.

## Your First Pipeline

Here's a simple example that demonstrates the basic usage:

```rust
use cloacina::*;

// Define a simple task
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

// Create the workflow
let workflow = workflow! {
    name: "simple_pipeline",
    description: "A simple data processing pipeline",
    tasks: [process_data]
};

// Execute the workflow
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize runner with database connection
    let runner = DefaultRunner::new("postgresql://user:pass@localhost/mydb").await?;

    // Execute workflow with automatic state persistence
    let context = Context::new();
    let result = runner.execute("simple_pipeline", context).await?;

    println!("Pipeline completed: {:?}", result.status);
    Ok(())
}
```

## Core Concepts

### Tasks

Tasks are the fundamental units of work. They can have dependencies, retry policies, and trigger rules:

```rust
#[task(
    id = "my_task",
    dependencies = ["other_task_id"],
    retry_attempts = 3,
    retry_backoff = "exponential",
    retry_delay_ms = 1000
)]
async fn my_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Task implementation
    Ok(())
}
```

{{< hint warning >}}
Tasks should be designed to be idempotent. This means that running the same task multiple times with the same input should produce the same result and have no additional side effects. This is crucial for Cloacina's retry and recovery mechanisms to work correctly.
{{< /hint >}}

### Context

The Context is a type-safe container for sharing data between tasks. It's automatically persisted by Cloacina's backend, ensuring your data survives task failures and system restarts:

```rust
// Insert data
context.insert("user_id", serde_json::json!(12345))?;

// Read data in later tasks
if let Some(user_id) = context.get("user_id") {
    println!("Processing for user: {}", user_id);
}
```

The Context is automatically serialized and stored in your database after each task execution. This means:
- Your data persists across task retries and system restarts
- You can inspect the state of your pipeline at any point
- Long-running workflows can safely pause and resume
- Task failures won't result in complete data loss

### Workflows

Workflows define the structure of your pipeline and its tasks. They support complex task dependencies that can model any workflow pattern, from simple linear sequences to sophisticated branching pipelines:

```rust
let workflow = workflow! {
    name: "my_workflow",
    description: "Description of the workflow",
    tasks: [
        task1,
        task2,
        task3
    ]
};
```

Workflows are expressive and can:
- Model complex task dependencies and relationships
- Support parallel task execution where dependencies allow
- Handle conditional task execution based on context
- Maintain clear task ordering and execution flow
- Enable efficient pipeline execution through smart dependency management

## Next Steps

- Check out our [Tutorials]({{< ref "/tutorials/" >}}) for more complex examples
- Read the [How-to Guides]({{< ref "/how-to-guides/" >}}) for specific use cases
- Explore the [Reference]({{< ref "/reference/" >}}) for detailed API documentation

## Need Help?

- Open an [issue](https://github.com/colliery-io/cloacina/issues) for bug reports
