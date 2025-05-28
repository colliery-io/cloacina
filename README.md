# Cloacina

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Crates.io](https://img.shields.io/crates/v/cloacina.svg)](https://crates.io/crates/cloacina)
[![Documentation](https://docs.rs/cloacina/badge.svg)](https://docs.rs/cloacina)

<div align="center">
  <img src="https://github.com/colliery-io/cloacina/blob/main/docs/static/images/image.png" alt="Cloacina Logo" width="400">
</div>

Cloacina is a Rust library for building resilient task pipelines directly within your Rust applications. Unlike standalone orchestration services, Cloacina embeds into your existing applications to manage complex multi-step workflows with automatic retry, state persistence, and dependency resolution.

## Features

- **Embedded Framework**: Integrates directly into your Rust applications
- **Resilient Execution**: Automatic retries, failure recovery, and state persistence
- **Type-Safe Workflows**: Compile-time validation of task dependencies and data flow
- **Database-Backed**: Uses PostgreSQL for reliable state management
- **Async-First**: Built on tokio for high-performance concurrent execution
- **Content-Versioned**: Automatic workflow versioning based on task code and structure

## Installation

Add Cloacina to your `Cargo.toml`:

```toml
[dependencies]
cloacina = "0.1.0"
async-trait = "0.1"    # Required for async task definitions
ctor = "0.2"          # Required for task registration
serde_json = "1.0"    # Required for context data serialization
```

## Quick Start

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
    name: "my_workflow",
    description: "A simple workflow",
    tasks: [process_data]
};

// Initialize executor with database
let executor = UnifiedExecutor::new("postgresql://user:pass@localhost/dbname").await?;

// Execute the workflow
let result = executor.execute("my_workflow", Context::new()).await?;
```

## Documentation

- [User Guide](https://colliery-io.github.io/cloacina/)
- [API Reference](https://docs.rs/cloacina)
- [Examples](https://github.com/collier-io/cloacina/tree/main/examples)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.
