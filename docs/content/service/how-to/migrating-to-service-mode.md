---
title: "Migrating from Library to Service Mode"
description: "How to convert an embedded Rust workflow into a packaged workflow for deployment to the daemon or server"
weight: 55
aliases:
  - "/workflows/how-to-guides/migrating-to-service-mode/"

---

# Migrating from Library to Service Mode

This guide walks through converting an existing embedded Rust workflow into a packaged workflow for deployment. **Library mode** (embedded) means your application owns the Tokio runtime and calls Cloacina directly. **Service mode** (packaged) means the workflow is compiled as a shared library and loaded by the daemon or server.

## Prerequisites

- An existing workflow using the library/embedded tutorials (1-4)
- Familiarity with [Packaged Workflows]({{< ref "/service/tutorials/03-packaged-workflows" >}})

## What Changes

| Aspect | Library Mode | Service Mode |
|--------|-------------|--------------|
| Macro | `#[workflow]` | `#[workflow]` (same) plus a one-line `cloacina_workflow_plugin::package!()` shell in `lib.rs` |
| Crate type | `bin` or `lib` | `cdylib` — but you don't declare it; the compiler injects the `cdylib` crate-type + `packaged` feature at build time |
| Dependencies | `cloacina` (full crate) | `cloacina-workflow` (with `packaged`, `macros` features) + `cloacina-workflow-plugin` |
| Registration | `inventory::submit!` entries seeded into `Runtime` at startup via `seed_from_inventory()` | FFI vtable exports (9 methods, indices 0–8) loaded dynamically; the unified [`cloacina_workflow_plugin::package!()`]({{< ref "/reference/package-shell-macro" >}}) shell macro emits the entry points |
| Runtime | Your `#[tokio::main]` | Daemon or server loads and runs it |
| Build | `cargo build` | `cloacinactl package pack` archives the source; the compiler compiles it |

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
├── package.toml    # package manifest (name/version + [metadata])
└── src/
    └── lib.rs      # contains package!() + #[workflow]
```

There is **no `build.rs`** and no `[lib] crate-type` / `[features]` wiring to add — the compiler injects the `cdylib` crate-type and the `packaged` feature when it builds the package.

## Step 2: Update Cargo.toml

Swap the full `cloacina` crate for the packaged dependencies. This is the **whole** shell — no `[lib] crate-type`, no `[features]` table, no `build.rs`, no `cloacina-build` build-dependency. The compiler adds the `cdylib` crate-type and `packaged` feature at build time, and the shell macro routes its runtime companions (async-trait, chrono, computation-graph) — you hand-add none of them.

**Before:**
```toml
[package]
name = "my-workflow"
version = "0.1.0"
edition = "2021"

[dependencies]
cloacina = { version = "0.7.0", features = ["macros", "sqlite"] }
async-trait = "0.1"
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
```

**After:**
```toml
[package]
name = "my-workflow"
version = "0.1.0"
edition = "2021"

[dependencies]
cloacina-workflow = { version = "0.7", features = ["packaged", "macros"] }
cloacina-workflow-plugin = "0.7"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

Key changes:
- **`cloacina-workflow` with `"packaged"` and `"macros"` features** — the packaged authoring surface plus the workflow/task macros
- **`cloacina-workflow-plugin`** — provides the `package!()` shell macro
- **Removed** `cloacina`, `tokio`, `async-trait`, and any `[lib]`/`[features]`/`[build-dependencies]` ceremony — the host provides the runtime and the compiler injects the build wiring

## Step 3: Add a package.toml

Packages carry a `package.toml` manifest alongside `Cargo.toml`. The minimal form is just a name/version and a `[metadata]` block — the resolver defaults the interface header and infers the language from the crate layout:

```toml
[package]
name = "my-workflow"
version = "0.1.0"

[metadata]
workflow_name = "data_processing"
description = "Data processing pipeline"
```

## Step 4: Update the Workflow Code

Add the `cloacina_workflow_plugin::package!()` shell macro at the crate root, remove the `main()` function, and keep the `#[workflow]` module:

**Before** (`main.rs`):
```rust
use cloacina::*;

#[workflow(
    name = "data_processing",
    description = "Data processing pipeline"
)]
mod data_processing {
    use super::*;

    #[task]
    async fn extract(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("data", serde_json::json!(42))?;
        Ok(())
    }

    #[task(dependencies = ["extract"])]
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

// Turns this crate into a fully-formed Cloacina plugin. Un-gated —
// the compiler injects the `packaged` feature it expands under.
cloacina_workflow_plugin::package!();

#[workflow(
    name = "data_processing",
    description = "Data processing pipeline"
)]
pub mod data_processing {
    use super::*;

    #[task]
    pub async fn extract(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("data", serde_json::json!(42))?;
        Ok(())
    }

    #[task(dependencies = ["extract"])]
    pub async fn transform(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let data = context.get("data").unwrap().as_i64().unwrap();
        context.insert("result", serde_json::json!(data * 2))?;
        Ok(())
    }
}
```

Key differences:
- Add `cloacina_workflow_plugin::package!()` at the crate root
- Import from `cloacina_workflow` instead of `cloacina`
- Module and functions are `pub` (required for FFI visibility)
- No `main()` — the daemon/server provides the runtime
- No `DefaultRunner` — execution is managed by the host

## Step 5: Package the Source

Archive the source directory into a `.cloacina` package:

```bash
cloacinactl package pack ./my-workflow
```

This produces `my-workflow.cloacina` — a source archive carrying the resolved `package.toml`. The compiler compiles it (injecting the `cdylib` crate-type and `packaged` feature) when it is uploaded. See the [Packaged Workflows Tutorial]({{< ref "/service/tutorials/03-packaged-workflows" >}}) for the full flow.

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

- [ ] `cloacina` swapped for `cloacina-workflow` (`packaged`, `macros`) + `cloacina-workflow-plugin`
- [ ] No `build.rs`, no `[lib] crate-type`, no `[features]` table (the compiler injects them)
- [ ] `cloacina_workflow_plugin::package!()` added at the crate root
- [ ] `package.toml` present with `[metadata].workflow_name`
- [ ] Module and functions are `pub`
- [ ] No `main()` in `lib.rs`
- [ ] `cloacinactl package pack ./my-workflow` produces a `.cloacina` archive
- [ ] Integration tests pass with `cargo test`

## See Also

- [Packaged Workflows Tutorial]({{< ref "/service/tutorials/03-packaged-workflows" >}}) — step-by-step packaging guide
- [Workflow Registry Tutorial]({{< ref "/embed/tutorials/09-workflow-registry" >}}) — managing packages in the registry
- [FFI System]({{< ref "/engine/explanation/ffi-system" >}}) — how dynamic loading works
- [Packaged Workflow Architecture]({{< ref "/engine/explanation/packaged-workflow-architecture" >}}) — design rationale
