---
title: "03 - Complex Workflows"
description: "Build sophisticated workflows with advanced dependency patterns"
weight: 13
reviewer: "dstorey"
review_date: "2024-03-19"
---


Welcome to the third tutorial in our Cloacina series! In this tutorial, you'll learn how to create sophisticated workflows with complex dependencies, parallel processing, and advanced patterns. We'll build on the concepts from [Tutorial 2]({{< ref "/tutorials/02-context-handling/" >}}) and show you how to handle expensive operations efficiently and create complex data processing pipelines.

## Prerequisites

- Completion of [Tutorial 2]({{< ref "/tutorials/02-context-handling/" >}})
- Basic understanding of Rust
- Rust toolchain installed (rustc, cargo)
- A code editor of your choice

## Time Estimate
25-30 minutes

## Setting Up Your Project

Let's start by creating a new Rust project. We'll create it in a directory that's a sibling to the Cloacina repository:

```bash
# Assuming you're in the parent directory of the Cloacina repository
mkdir -p my-cloacina-projects
cd my-cloacina-projects
cargo new parallel-workflow
cd parallel-workflow
```

Your directory structure should look like this:
```
.
├── cloacina/              # The Cloacina repository
└── my-cloacina-projects/  # Your projects directory
    └── parallel-workflow/ # Your new project
        ├── Cargo.toml
        └── src/
            └── main.rs
```

Now, add Cloacina and its dependencies to your `Cargo.toml`:

```toml
[dependencies]
cloacina = { path = "../../cloacina", default-features = false, features = ["macros", "sqlite"] }
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

## Understanding Parallel Processing

In Cloacina, tasks can run in parallel when they don't depend on each other. This is particularly useful for handling expensive operations like:
- Long-running network calls
- CPU-intensive computations
- External API requests
- Database operations
- File I/O operations

By splitting these operations into independent tasks, we may significantly reduce the total execution time of our workflow.

### Key Concepts

1. **Task Parallelization**: Running multiple tasks simultaneously when they don't depend on each other
2. **Task Partitioning**: Splitting expensive operations into independent tasks that can run in parallel
3. **Context Synchronization**: Managing data flow and state between parallel tasks
4. **Result Combination**: Merging results from parallel tasks into a coherent output

### Workflow Structure

Here's a visual representation of our parallel processing workflow:

```mermaid
graph TD
    A[generate_data] --> B[partition_data]
    B --> C[process_partition_1]
    B --> D[process_partition_2]
    B --> E[process_partition_3]
    C --> F[combine_results]
    D --> F
    E --> F
    F --> G[generate_report]
    F --> H[send_notifications]
    G --> I[cleanup]
    H --> I
```

The diagram shows:
- Initial data generation
- Data partitioning into three streams
- Parallel processing of each partition
- Final combination of results

Notice how `process_partition_1`, `process_partition_2`, and `process_partition_3` can run simultaneously since they only depend on `partition_data` and not on each other.

{{< hint warning >}}
**Important Note About Task Parallelization**

While we use a product catalog as an example, these principles apply to any expensive operations like API calls, file processing, or database queries.

The key is to identify operations that:
1. Can be split into independent tasks
2. Are expensive enough to justify parallelization overhead
3. Don't have interdependencies preventing parallel execution
{{< /hint >}}

## Building a Parallel Processing Pipeline

Let's create a workflow that processes a product catalog in parallel. We'll:
1. Generate a dataset of products
2. Split the processing into three independent tasks
3. Process each partition in parallel
4. Combine the results
5. Generate reports and send notifications

First, let's define our data structures:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Product {
    id: u32,
    name: String,
    category: String,
    price: f64,
    stock: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct CategoryStats {
    total_value: f64,
    total_stock: u32,
    product_count: u32,
}
```

Now, let's create our complete main.rs file:

```rust
//! # Parallel Processing Example
//!
//! This example demonstrates Cloacina's ability to execute tasks in parallel
//! when they have no dependencies on each other, and then converge the results.

use cloacina::{task, workflow, Context, TaskError};
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use tracing::info;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Product {
    id: u32,
    name: String,
    category: String,
    price: f64,
    stock: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct CategoryStats {
    total_value: f64,
    total_stock: u32,
    product_count: u32,
}

/// Generate a large dataset of products
#[task(
    id = "generate_data",
    dependencies = [],
    retry_attempts = 2
)]
async fn generate_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("🚀 Generating product dataset");

    // Simulate loading a large dataset
    tokio::time::sleep(Duration::from_millis(500)).await;

    let total_products = 10000;
    let products = (1..=total_products)
        .map(|id| Product {
            id,
            name: format!("Product {}", id),
            category: format!("Category {}", (id % 10) + 1),
            price: (id as f64 * 1.5) % 100.0,
            stock: (id * 10) % 1000,
        })
        .collect::<Vec<_>>();

    context.insert("total_products", json!(total_products))?;
    context.insert("products", json!(products))?;

    info!("Generated {} products across 10 categories", total_products);
    Ok(())
}

/// Partition the data into three chunks for parallel processing
#[task(
    id = "partition_data",
    dependencies = ["generate_data"],
    retry_attempts = 2
)]
async fn partition_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Partitioning product data");

    let products: Vec<Product> = context
        .get("products")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Missing products in context".to_string(),
        })?
        .as_array()
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Products is not an array".to_string(),
        })?
        .iter()
        .map(|v| serde_json::from_value(v.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| TaskError::ValidationFailed {
            message: format!("Failed to deserialize products: {}", e),
        })?;

    let chunk_size = products.len() / 3;

    let (chunk1, remainder) = products.split_at(chunk_size);
    let (chunk2, chunk3) = remainder.split_at(chunk_size);

    context.insert("partition_1", json!(chunk1.to_vec()))?;
    context.insert("partition_2", json!(chunk2.to_vec()))?;
    context.insert("partition_3", json!(chunk3.to_vec()))?;

    info!("Data partitioned into 3 chunks of {} products each", chunk_size);
    Ok(())
}

/// Process the first partition of products
#[task(
    id = "process_partition_1",
    dependencies = ["partition_data"],
    retry_attempts = 3,
    retry_delay_ms = 1000
)]
async fn process_partition_1(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let products: Vec<Product> = context
        .get("partition_1")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Missing partition_1 in context".to_string(),
        })?
        .as_array()
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Partition_1 is not an array".to_string(),
        })?
        .iter()
        .map(|v| serde_json::from_value(v.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| TaskError::ValidationFailed {
            message: format!("Failed to deserialize partition_1: {}", e),
        })?;

    info!("Processing partition 1: {} products", products.len());

    // Simulate CPU-intensive processing
    let processing_time = rand::thread_rng().gen_range(1000..3000);
    tokio::time::sleep(Duration::from_millis(processing_time)).await;

    let mut stats = HashMap::new();
    for product in &products {
        let entry = stats
            .entry(product.category.clone())
            .or_insert(CategoryStats {
                total_value: 0.0,
                total_stock: 0,
                product_count: 0,
            });

        entry.total_value += product.price * product.stock as f64;
        entry.total_stock += product.stock;
        entry.product_count += 1;
    }

    context.insert("stats_1", json!(stats))?;
    context.insert("processing_time_1", json!(processing_time))?;

    info!(
        "Partition 1 complete: processed {} products in {}ms",
        products.len(),
        processing_time
    );
    Ok(())
}

/// Process the second partition of products
#[task(
    id = "process_partition_2",
    dependencies = ["partition_data"],
    retry_attempts = 3,
    retry_delay_ms = 1000
)]
async fn process_partition_2(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let products: Vec<Product> = context
        .get("partition_2")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Missing partition_2 in context".to_string(),
        })?
        .as_array()
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Partition_2 is not an array".to_string(),
        })?
        .iter()
        .map(|v| serde_json::from_value(v.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| TaskError::ValidationFailed {
            message: format!("Failed to deserialize partition_2: {}", e),
        })?;

    info!("Processing partition 2: {} products", products.len());

    // Simulate CPU-intensive processing
    let processing_time = rand::thread_rng().gen_range(1500..4000);
    tokio::time::sleep(Duration::from_millis(processing_time)).await;

    let mut stats = HashMap::new();
    for product in &products {
        let entry = stats
            .entry(product.category.clone())
            .or_insert(CategoryStats {
                total_value: 0.0,
                total_stock: 0,
                product_count: 0,
            });

        entry.total_value += product.price * product.stock as f64;
        entry.total_stock += product.stock;
        entry.product_count += 1;
    }

    context.insert("stats_2", json!(stats))?;
    context.insert("processing_time_2", json!(processing_time))?;

    info!(
        "Partition 2 complete: processed {} products in {}ms",
        products.len(),
        processing_time
    );
    Ok(())
}

/// Process the third partition of products
#[task(
    id = "process_partition_3",
    dependencies = ["partition_data"],
    retry_attempts = 3,
    retry_delay_ms = 1000
)]
async fn process_partition_3(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let products: Vec<Product> = context
        .get("partition_3")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Missing partition_3 in context".to_string(),
        })?
        .as_array()
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Partition_3 is not an array".to_string(),
        })?
        .iter()
        .map(|v| serde_json::from_value(v.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| TaskError::ValidationFailed {
            message: format!("Failed to deserialize partition_3: {}", e),
        })?;

    info!("Processing partition 3: {} products", products.len());

    // Simulate CPU-intensive processing
    let processing_time = rand::thread_rng().gen_range(800..2500);
    tokio::time::sleep(Duration::from_millis(processing_time)).await;

    let mut stats = HashMap::new();
    for product in &products {
        let entry = stats
            .entry(product.category.clone())
            .or_insert(CategoryStats {
                total_value: 0.0,
                total_stock: 0,
                product_count: 0,
            });

        entry.total_value += product.price * product.stock as f64;
        entry.total_stock += product.stock;
        entry.product_count += 1;
    }

    context.insert("stats_3", json!(stats))?;
    context.insert("processing_time_3", json!(processing_time))?;

    info!(
        "Partition 3 complete: processed {} products in {}ms",
        products.len(),
        processing_time
    );
    Ok(())
}

/// Combine results from all parallel processing tasks
#[task(
    id = "combine_results",
    dependencies = ["process_partition_1", "process_partition_2", "process_partition_3"],
    retry_attempts = 2
)]
async fn combine_results(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("🔀 Combining results from parallel processing");

    let stats_1: HashMap<String, CategoryStats> = context
        .get("stats_1")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Missing stats_1 in context".to_string(),
        })?
        .as_object()
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "stats_1 is not an object".to_string(),
        })?
        .iter()
        .map(|(k, v)| serde_json::from_value(v.clone()).map(|v| (k.clone(), v)))
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|e| TaskError::ValidationFailed {
            message: format!("Failed to deserialize stats_1: {}", e),
        })?;

    let stats_2: HashMap<String, CategoryStats> = context
        .get("stats_2")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Missing stats_2 in context".to_string(),
        })?
        .as_object()
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "stats_2 is not an object".to_string(),
        })?
        .iter()
        .map(|(k, v)| serde_json::from_value(v.clone()).map(|v| (k.clone(), v)))
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|e| TaskError::ValidationFailed {
            message: format!("Failed to deserialize stats_2: {}", e),
        })?;

    let stats_3: HashMap<String, CategoryStats> = context
        .get("stats_3")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Missing stats_3 in context".to_string(),
        })?
        .as_object()
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "stats_3 is not an object".to_string(),
        })?
        .iter()
        .map(|(k, v)| serde_json::from_value(v.clone()).map(|v| (k.clone(), v)))
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|e| TaskError::ValidationFailed {
            message: format!("Failed to deserialize stats_3: {}", e),
        })?;

    let processing_time_1: u64 = context
        .get("processing_time_1")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Missing processing_time_1 in context".to_string(),
        })?
        .as_u64()
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "processing_time_1 is not a number".to_string(),
        })?;

    let processing_time_2: u64 = context
        .get("processing_time_2")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Missing processing_time_2 in context".to_string(),
        })?
        .as_u64()
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "processing_time_2 is not a number".to_string(),
        })?;

    let processing_time_3: u64 = context
        .get("processing_time_3")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Missing processing_time_3 in context".to_string(),
        })?
        .as_u64()
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "processing_time_3 is not a number".to_string(),
        })?;

    // Combine all stats
    let mut combined_stats = stats_1;

    for (category, stats) in stats_2 {
        let entry = combined_stats.entry(category).or_insert(CategoryStats {
            total_value: 0.0,
            total_stock: 0,
            product_count: 0,
        });

        entry.total_value += stats.total_value;
        entry.total_stock += stats.total_stock;
        entry.product_count += stats.product_count;
    }

    for (category, stats) in stats_3 {
        let entry = combined_stats.entry(category).or_insert(CategoryStats {
            total_value: 0.0,
            total_stock: 0,
            product_count: 0,
        });

        entry.total_value += stats.total_value;
        entry.total_stock += stats.total_stock;
        entry.product_count += stats.product_count;
    }

    // Calculate parallel efficiency
    let total_processing_time = processing_time_1 + processing_time_2 + processing_time_3;
    let max_parallel_time = std::cmp::max(
        std::cmp::max(processing_time_1, processing_time_2),
        processing_time_3,
    );
    let parallel_efficiency = (total_processing_time as f64 / max_parallel_time as f64) * 100.0;

    context.insert("final_stats", json!(combined_stats))?;
    context.insert("total_processing_time_ms", json!(total_processing_time))?;
    context.insert("actual_parallel_time_ms", json!(max_parallel_time))?;
    context.insert("parallel_efficiency_percent", json!(parallel_efficiency))?;

    info!(
        "Results combined: {:.1}% parallel efficiency",
        parallel_efficiency
    );
    Ok(())
}

/// Generate final report
#[task(
    id = "generate_report",
    dependencies = ["combine_results"],
    retry_attempts = 2
)]
async fn generate_report(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Generating processing report");

    let final_stats: HashMap<String, CategoryStats> = context
        .get("final_stats")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Missing final_stats in context".to_string(),
        })?
        .as_object()
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "final_stats is not an object".to_string(),
        })?
        .iter()
        .map(|(k, v)| serde_json::from_value(v.clone()).map(|v| (k.clone(), v)))
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|e| TaskError::ValidationFailed {
            message: format!("Failed to deserialize final_stats: {}", e),
        })?;

    let report = final_stats
        .iter()
        .map(|(category, stats)| {
            format!(
                "Category {}: {} products, {} total stock, ${:.2} total value",
                category, stats.product_count, stats.total_stock, stats.total_value
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    context.insert("report", json!(report))?;
    info!("Report generated for {} categories", final_stats.len());
    Ok(())
}

/// Send notifications
#[task(
    id = "send_notifications",
    dependencies = ["combine_results"],
    retry_attempts = 2
)]
async fn send_notifications(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Sending completion notifications");

    let final_stats: HashMap<String, CategoryStats> = context
        .get("final_stats")
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "Missing final_stats in context".to_string(),
        })?
        .as_object()
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "final_stats is not an object".to_string(),
        })?
        .iter()
        .map(|(k, v)| serde_json::from_value(v.clone()).map(|v| (k.clone(), v)))
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|e| TaskError::ValidationFailed {
            message: format!("Failed to deserialize final_stats: {}", e),
        })?;

    let total_products: u32 = final_stats.values().map(|s| s.product_count).sum();
    let total_value: f64 = final_stats.values().map(|s| s.total_value).sum();

    info!(
        "Processing complete: {} products processed, total value: ${:.2}",
        total_products, total_value
    );
    Ok(())
}

/// Clean up temporary data
#[task(
    id = "cleanup",
    dependencies = ["generate_report", "send_notifications"],
    retry_attempts = 2
)]
async fn cleanup(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    info!("Cleaning up resources");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("tutorial_03=debug,cloacina=info")
        .init();

    info!("Starting Parallel Processing Example");

    // Initialize runner with SQLite database using WAL mode for better concurrency
    let runner = DefaultRunner::with_config(
        "sqlite://tutorial-03.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
        DefaultRunnerConfig::default(),
    )
    .await?;

    // Create the parallel processing workflow
    let _workflow = workflow! {
        name: "parallel_processing",
        description: "Parallel product data processing pipeline",
        tasks: [
            generate_data,
            partition_data,
            process_partition_1,
            process_partition_2,
            process_partition_3,
            combine_results,
            generate_report,
            send_notifications,
            cleanup
        ]
    };

    // Create input context
    let input_context = Context::new();

    info!("Executing parallel processing workflow");
    let result = runner.execute("parallel_processing", input_context).await?;

    info!("Workflow completed with status: {:?}", result.status);
    info!("Final context: {:?}", result.final_context);

    // Shutdown the runner
    runner.shutdown().await?;

    info!("Parallel processing example completed!");
    Ok(())
}
```

{{< hint type=info title="Parallel Processing Power" >}}
Cloacina's parallel processing capabilities enable you to:

- **Task Independence**: Run multiple tasks simultaneously when they don't depend on each other
- **Resource Efficiency**: Process expensive operations in parallel to reduce total execution time
- **Result Coordination**: Automatically combine results from parallel tasks
- **Error Isolation**: Handle failures in parallel tasks without affecting others
- **Scalability**: Process more work in the same time by utilizing parallel execution

In the next tutorials, you'll learn how to build more complex parallel processing patterns.
{{< /hint >}}

## Running Your Workflow

You can run this tutorial in two ways:

### Option 1: Using Angreal (Recommended)

If you're following along with the Cloacina repository, you can use angreal to run the tutorial:

```bash
# From the Cloacina repository root
angreal demos tutorial-03
```

This will run the tutorial code with all necessary dependencies.

### Option 2: Manual Setup

If you're building the project manually, simply run your workflow with:

```bash
cargo run
```

## What's Next?

Congratulations! You've created a parallel processing workflow. In the next tutorial, we'll explore:
- More complex parallel processing patterns
- Advanced task coordination
- Error handling in parallel workflows
- Resource management strategies

## Related Resources

- [API Documentation]({{< ref "/reference/api/" >}})
- [Task Reference]({{< ref "/reference/api/" >}})
- [Context Reference]({{< ref "/reference/api/" >}})
- [Workflow Reference]({{< ref "/reference/api/" >}})
