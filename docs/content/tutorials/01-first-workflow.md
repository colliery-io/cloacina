---
title: "01 - Your First Workflow"
description: "Create your first Cloacina workflow"
weight: 11
reviewer: "dstorey"
review_date: "2024-03-19"
---


Welcome to your first Cloacina tutorial! In this guide, you'll learn how to create and execute a simple workflow using Cloacina's macro system. By the end of this tutorial, you'll understand the basic concepts of tasks, workflows, context, and execution in Cloacina.

## Prerequisites

- Basic knowledge of Rust
- Rust toolchain installed (rustc, cargo)
- A code editor of your choice

## Time Estimate
10-15 minutes

## Setting Up Your Project

Let's start by creating a new Rust project. We'll create it in a directory that's a sibling to the Cloacina repository:

```bash
# Assuming you're in the parent directory of the Cloacina repository
mkdir -p my-cloacina-projects
cd my-cloacina-projects
cargo new first-workflow
cd first-workflow
```

Your directory structure should look like this:
```
.
├── cloacina/              # The Cloacina repository
└── my-cloacina-projects/  # Your projects directory
    └── first-workflow/    # Your new project
        ├── Cargo.toml
        └── src/
            └── main.rs
```

Now, add Cloacina and its dependencies to your `Cargo.toml`. Note that we're using a relative path to the Cloacina repository:

```toml
[dependencies]
cloacina = { path = "../../cloacina" }
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
async-trait = "0.1"
ctor = "0.2"
chrono = "0.4"
```

{{< hint type=warning title=Important >}}
Normally you'd use `cloacina = "0.1.0"` in Cargo.toml. For these tutorials, we're using path dependencies to vendor code locally.

The path must be relative to your project. Examples:
- Next to Cloacina: `path = "../cloacina"`
- In subdirectory: `path = "../../../cloacina"`

Note: Use `version = "0.1.0"` when available on crates.io.
{{< /hint >}}

Cloacina supports both PostgreSQL and SQLite backends. The backend is selected automatically at runtime based on your connection URL - no feature flags needed.

### Understanding the Dependencies

Each dependency serves a specific purpose in the Cloacina macro system:

- `async-trait`: Required for async functions in traits (macro expansion)
- `ctor`: Enables static initialization before `main()`
- `chrono`: Timestamp handling for execution metadata
- `serde_json`: Context serialization

These dependencies must be explicit because macro expansion happens at compile time, where transitive dependencies aren't available.

## Creating Your First Workflow

Let's create a simple workflow with a single task that prints a greeting message. Create a new file `src/main.rs` with the following content:

```rust
//! Simple Cloacina Example
//!
//! This example demonstrates the most basic usage of Cloacina with a single task.

use cloacina::{task, workflow, Context, TaskError};
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use serde_json::json;
use tracing::info;

/// A simple task that just logs a message
#[task(
    id = "hello_world",
    dependencies = []
)]
async fn hello_world(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Hello from Cloacina!");

    // Add some data to context for demonstration
    context.insert("message", json!("Hello World!"))?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("tutorial_01=info,cloacina=debug")
        .init();

    info!("Starting Simple Cloacina Example");

    // Initialize runner with SQLite database using WAL mode for better concurrency
    let runner = DefaultRunner::with_config(
        "sqlite://tutorial-01.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
        DefaultRunnerConfig::default(),
    )
    .await?;

    // Create a simple workflow (automatically registers in global registry)
    let _workflow = workflow! {
        name: "simple_workflow",
        description: "A simple workflow with one task",
        tasks: [
            hello_world
        ]
    };

    // Create input context
    let input_context = Context::new();

    info!("Executing workflow");

    // Execute the workflow (scheduler and runner managed automatically)
    let result = runner.execute("simple_workflow", input_context).await?;

    info!("Workflow completed with status: {:?}", result.status);
    info!("Final context: {:?}", result.final_context);

    // Shutdown the runner
    runner.shutdown().await?;

    info!("Simple example completed!");

    Ok(())
}
```

## Understanding the Code

Let's walk through the code in execution order and understand why each component needs to be set up in this specific sequence:

1. **Imports and Dependencies**: First, we import all necessary components from Cloacina:
   ```rust
   use cloacina::{task, workflow, Context, TaskError};
   use cloacina::runner::DefaultRunner;
   ```
   These imports are needed because they define the core types and traits we'll use throughout the program. `DefaultRunner` provides the interface for executing workflows and managing task pipelines.

2. **Task Definition**: We define our task:
   ```rust
   #[task(id = "hello_world", dependencies = [])]
   async fn hello_world(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>
   ```
   The task definition includes its ID and dependencies, which are used by the workflow system to build the execution graph.

3. **Main Function Setup**: The main function follows a specific sequence:
   ```rust
   // 1. Initialize logging first - needed for all subsequent operations
   tracing_subscriber::fmt()
       .with_env_filter("tutorial_01=info,cloacina=debug")
       .init();

   // 2. Create the runner - this must happen before any workflow definition
   // because the workflow! macro registers workflows in a global registry
   // that the runner needs to access
   let runner = DefaultRunner::with_config(
       "sqlite://tutorial-01.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
       DefaultRunnerConfig::default(),
   )
   .await?;

   // 3. Define the workflow - the workflow! macro will automatically register
   // it in the global registry that the executor uses
   let _workflow = workflow! {
       name: "simple_workflow",
       description: "A simple workflow with one task",
       tasks: [hello_world]
   };
   ```
   This sequence is important because:
   - Logging must be initialized first to capture all subsequent operations
   - The runner must be created before workflows because it manages the workflow registry
   - The workflow! macro automatically registers workflows in the global registry that the runner uses

4. **Workflow Execution**: Only after all components are set up can we execute the workflow:
   ```rust
   // Create and execute with input context
   let input_context = Context::new();
   let result = runner.execute("simple_workflow", input_context).await?;
   ```
   The execution requires:
   - A properly initialized runner
   - A registered workflow
   - An input context

5. **Cleanup**: Finally, we properly shut down the runner:
   ```rust
   runner.shutdown().await?;
   ```
   This ensures all resources are properly released and the database connection is closed gracefully.

This ordered approach ensures that each component has its dependencies available when needed, and resources are properly managed throughout the workflow's lifecycle.

{{< hint type=info  title="Workflow Power" >}}
While this example shows a single task, Cloacina's workflows are designed to handle complex business processes through:

- **Task Dependencies**: Define clear relationships between tasks, ensuring they run in the correct order
- **Data Management**: Share and transform data between tasks using the Context system
- **Error Handling**: Consistent error handling and recovery across all tasks
- **Parallel Execution**: Automatically run independent tasks in parallel
- **Retry Management**: Configure and manage task retries with:
  - Custom retry policies
  - Automatic retry scheduling
  - State preservation between attempts

In the next tutorials, you'll learn how to build these features into your workflows.
{{< /hint >}}

## Running Your Workflow

You can run this tutorial in two ways:

### Option 1: Using Angreal (Recommended)

If you're following along with the Cloacina repository, you can use angreal to run the tutorial:

```bash
# From the Cloacina repository root
angreal demos tutorial-01
```

This will run the tutorial code with all necessary dependencies.

### Option 2: Manual Setup

If you're building the project manually, simply run your workflow with:

```bash
cargo run
```

You should see output similar to:

```
INFO  tutorial_01 > Starting Simple Cloacina Example
INFO  tutorial_01 > Executing workflow
INFO  tutorial_01 > Hello from Cloacina!
INFO  tutorial_01 > Workflow completed with status: Success
INFO  tutorial_01 > Final context: {"message": "Hello World!"}
INFO  tutorial_01 > Simple example completed!
```

## What's Next?

Congratulations! You've created and executed your first Cloacina workflow. In the next tutorial, we'll explore:
- Adding dependencies between tasks
- Working with different types of context data
- Error handling and recovery

## Related Resources

- [API Documentation]({{< ref "/reference/api/" >}})
- [Task Reference]({{< ref "/reference/api/" >}})
- [Context Reference]({{< ref "/reference/api/" >}})

## Download the Example

You can download the complete example code from our [GitHub repository](https://github.com/colliery-io/cloacina/tree/main/examples/tutorial-01).
