# Cloacina

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Crates.io](https://img.shields.io/crates/v/cloacina.svg)](https://crates.io/crates/cloacina)


<div align="center">
  <img src="https://github.com/colliery-io/cloacina/raw/main/docs/static/images/image.png" alt="Cloacina Logo" width="400">
</div>

Cloacina is a Rust library for building resilient task pipelines directly within your Rust applications, built by [Colliery Software](https://colliery.io). Unlike standalone orchestration services, Cloacina embeds into your existing applications to manage complex multi-step workflows with automatic retry, state persistence, and dependency resolution.

Cloaca are the python bindings for Cloacina, providing a familiar interface for developers from the python ecosystem.

> Why "Cloacina" and "Cloaca" ? Named after the Roman goddess of sewers and drainage systems, Cloacina reflects the library's purpose: efficiently moving data through processing pipelines, just as ancient Roman infrastructure managed the flow of sewage out of the city. Cloaca is the latin noun for the drain, the Cloaca Maxima is the system Cloacina presided over. (Don't read too much into it, apparently there aren't many deities of "plumbing"!)

## Features

- **Embedded Framework**: Integrates directly into your Rust applications
- **Resilient Execution**: Automatic retries, failure recovery, and state persistence
- **Type-Safe Workflows**: Compile-time validation of task dependencies and data flow
- **Database-Backed**: Uses PostgreSQL or SQLite for reliable state management
- **Multi-Tenant Ready**: PostgreSQL schema-based isolation for complete tenant separation
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

Cloacina supports both PostgreSQL and SQLite backends. The backend is selected automatically at runtime based on your connection URL - no compile-time configuration needed.

### Single-Backend Builds (Optional)

For smaller binaries, you can compile with only the backend you need:

```toml
# PostgreSQL only
cloacina = { version = "0.1.0", default-features = false, features = ["postgres", "macros"] }

# SQLite only
cloacina = { version = "0.1.0", default-features = false, features = ["sqlite", "macros"] }
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
let executor = DefaultRunner::new("postgresql://user:pass@localhost/dbname").await?;

// Execute the workflow
let result = executor.execute("my_workflow", Context::new()).await?;
```

## Multi-Tenancy

Cloacina supports multi-tenant deployments with complete data isolation:

### PostgreSQL Schema-Based Isolation

```rust
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
let executor = DefaultRunner::builder()
    .database_url("postgresql://user:pass@localhost/cloacina")
    .schema("my_tenant")
    .build()
    .await?;
```

### SQLite File-Based Isolation

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

## Repository Structure

```
cloacina/
  crates/                    # Rust library crates
    cloacina/                # Core workflow engine
    cloacina-macros/         # Procedural macros
  bindings/
    cloaca-backend/          # Python bindings (PyPI: cloaca)
  examples/
    tutorials/               # Step-by-step learning path
    features/                # Feature showcases
    performance/             # Benchmarks
  tests/python/              # Python integration tests
  docs/                      # Documentation site
```

## Documentation

**[Complete Documentation & User Guide](https://colliery-io.github.io/cloacina/)**

Additional resources:
- [API Reference](https://docs.rs/cloacina) (Rust docs)
- [Tutorials](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials)
- [Feature Examples](https://github.com/colliery-io/cloacina/tree/main/examples/features)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.
