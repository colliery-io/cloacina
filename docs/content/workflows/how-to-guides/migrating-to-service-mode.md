---
title: "Migrating from Library to Service Mode"
description: "How to convert an embedded Rust workflow into a packaged workflow for deployment to the daemon or server"
weight: 55
---

# Migrating from Library to Service Mode

This guide walks through converting an existing embedded Rust workflow into a packaged workflow for deployment. **Library mode** (embedded) means your application owns the Tokio runtime and calls Cloacina directly. **Service mode** (packaged) means the workflow is compiled as a shared library and loaded by the daemon or server.

## Prerequisites

- An existing workflow using the library/embedded tutorials (1-4)
- Familiarity with [Packaged Workflows]({{< ref "/workflows/tutorials/service/07-packaged-workflows" >}})

## What Changes

| Aspect | Library Mode | Service Mode |
|--------|-------------|--------------|
| Macro | `#[workflow]` | `#[workflow]` (same — packaging is handled by `build.rs` and Cargo features) |
| Crate type | `bin` or `lib` | `cdylib` (shared library) |
| Dependencies | `cloacina` (full crate) | `cloacina-workflow` + `cloacina-macros` + `cloacina-workflow-plugin` |
| Registration | `#[ctor]` at startup | FFI entry points for dynamic loading |
| Runtime | Your `#[tokio::main]` | Daemon or server loads and runs it |
| Build | `cargo build` | `cloacina_build::configure()` in `build.rs` |

## Step 1: Restructure as a Library Crate

Convert your binary crate to a library crate. Move your workflow module from `main.rs` to `lib.rs`:

**Before** (library mode):
```
my-workflow/
├── Cargo.toml
└── src/
    └── main.rs     # contains #[workflow] + #[tokio::main]
```

**After** (service mode):
```
my-workflow/
├── Cargo.toml
├── build.rs
└── src/
    └── lib.rs      # contains #[workflow] only
```

## Step 2: Update Cargo.toml

Change the crate type to `cdylib` and swap dependencies:

**Before:**
```toml
[package]
name = "my-workflow"
version = "0.1.0"
edition = "2021"

[dependencies]
cloacina = { version = "0.5.0", features = ["macros", "sqlite"] }
async-trait = "0.1"
serde_json = "1.0"
ctor = "0.2"
tokio = { version = "1", features = ["full"] }
```

**After:**
```toml
[package]
name = "my-workflow"
version = "0.1.0"
edition = "2021"

[features]
default = ["packaged"]
packaged = []

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
cloacina-macros = "0.5.0"
cloacina-workflow = { version = "0.5.0", features = ["packaged"] }
cloacina-workflow-plugin = "0.5.0"
async-trait = "0.1"
serde_json = "1.0"
ctor = "0.2"

[build-dependencies]
cloacina-build = "0.5.0"

# Optional: keep cloacina for local testing
[dev-dependencies]
cloacina = { version = "0.5.0", default-features = false, features = ["macros", "sqlite"] }
```

Key changes:
- **`crate-type = ["cdylib", "rlib"]`** — `cdylib` produces a shared library for dynamic loading; `rlib` allows `cargo test` to work
- **`cloacina-workflow` with `"packaged"` feature** — enables FFI export generation
- **`cloacina-build`** — generates the correct linker flags via `build.rs`
- **Removed** `cloacina` and `tokio` from runtime dependencies (the host provides the runtime)

## Step 3: Add build.rs

Create `build.rs` at the crate root:

```rust
fn main() {
    cloacina_build::configure();
}
```

This sets the linker flags needed for the shared library to expose FFI entry points.

## Step 4: Update the Workflow Code

The workflow code itself barely changes. Remove the `main()` function and keep the `#[workflow]` module:

**Before** (`main.rs`):
```rust
use cloacina::*;

#[workflow(
    name = "data_processing",
    description = "Data processing pipeline"
)]
mod data_processing {
    use super::*;

    #[task(id = "extract", dependencies = [])]
    async fn extract(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("data", serde_json::json!(42))?;
        Ok(())
    }

    #[task(id = "transform", dependencies = ["extract"])]
    async fn transform(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let data = context.get("data").unwrap().as_i64().unwrap();
        context.insert("result", serde_json::json!(data * 2))?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runner = DefaultRunner::new(":memory:").await?;
    let result = runner.execute("data_processing", Context::new()).await?;
    println!("Result: {:?}", result.status);
    Ok(())
}
```

**After** (`lib.rs`):
```rust
use cloacina_workflow::{task, workflow, Context, TaskError};

#[workflow(
    name = "data_processing",
    description = "Data processing pipeline"
)]
pub mod data_processing {
    use super::*;

    #[task(id = "extract", dependencies = [])]
    pub async fn extract(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("data", serde_json::json!(42))?;
        Ok(())
    }

    #[task(id = "transform", dependencies = ["extract"])]
    pub async fn transform(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let data = context.get("data").unwrap().as_i64().unwrap();
        context.insert("result", serde_json::json!(data * 2))?;
        Ok(())
    }
}
```

Key differences:
- Import from `cloacina_workflow` instead of `cloacina`
- Module and functions are `pub` (required for FFI visibility)
- No `main()` — the daemon/server provides the runtime
- No `DefaultRunner` — execution is managed by the host

## Step 5: Build the Package

Compile the shared library:

```bash
cargo build --release
```

This produces a shared library at `target/release/libmy_workflow.so` (Linux) or `target/release/libmy_workflow.dylib` (macOS).

To create a `.cloacina` package from the compiled library, use the packaging tools described in [Packaged Workflows Tutorial]({{< ref "/workflows/tutorials/service/07-packaged-workflows" >}}).

## Step 6: Deploy

Copy the `.cloacina` package to the daemon's watch directory:

```bash
cp my-workflow.cloacina ~/.cloacina/packages/
```

Or upload to the server:

```bash
curl -X POST \
  -H "Authorization: Bearer $API_KEY" \
  -F "package=@my-workflow.cloacina" \
  https://cloacina.example.com/v1/tenants/my_tenant/workflows
```

## Step 7: Keep Local Tests Working

Add integration tests that use the full `cloacina` crate (via `dev-dependencies`):

```rust
#[cfg(test)]
mod tests {
    use cloacina::DefaultRunner;
    use cloacina_workflow::Context;

    #[tokio::test]
    async fn test_workflow_executes() {
        let runner = DefaultRunner::new(":memory:").await.unwrap();
        let result = runner
            .execute("data_processing", Context::new())
            .await
            .unwrap();
        assert_eq!(result.status.to_string(), "completed");
        runner.shutdown().await;
    }
}
```

## Checklist

- [ ] Crate type set to `["cdylib", "rlib"]`
- [ ] `build.rs` calls `cloacina_build::configure()`
- [ ] `cloacina-workflow` has `"packaged"` feature enabled
- [ ] Module and functions are `pub`
- [ ] No `main()` in `lib.rs`
- [ ] `cargo build --release` produces a shared library
- [ ] Integration tests pass with `cargo test`

## See Also

- [Packaged Workflows Tutorial]({{< ref "/workflows/tutorials/service/07-packaged-workflows" >}}) — step-by-step packaging guide
- [Workflow Registry Tutorial]({{< ref "/workflows/tutorials/service/08-workflow-registry" >}}) — managing packages in the registry
- [FFI System]({{< ref "/platform/explanation/ffi-system" >}}) — how dynamic loading works
- [Packaged Workflow Architecture]({{< ref "/platform/explanation/packaged-workflow-architecture" >}}) — design rationale
