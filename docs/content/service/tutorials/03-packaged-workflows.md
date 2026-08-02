---
title: "03 — Packaged Workflows"
description: "Create distributable workflow packages with the packaged workflow system"
weight: 13
aliases:
  - "/workflows/tutorials/service/07-packaged-workflows/"

---

Welcome to the workflow packages tutorial! In this guide, you'll author a
Rust workflow package from the canonical scaffold, understand the
compiler-injected shell model, and take the package all the way through
pack → upload → server-side compile → execution.

## Prerequisites

- Completion of [01 — Deploy a Server]({{< ref "/service/tutorials/01-deploy-a-server" >}})
  — you'll need the running server, the `acme` tenant, and the profile
  from that tutorial.
- Basic understanding of Rust and Cargo projects.
- `cloacinactl`, `cloacina-server`, and `cloacina-compiler` binaries on
  your `PATH`.
- A code editor of your choice.

## Time Estimate
15-20 minutes

## What Are Workflow Packages?

A `.cloacina` package is a **source archive** (tar + bzip2). For Rust
packages, the shared library is compiled **server-side** by the
`cloacina-compiler` service after upload — you never build or ship a
cdylib yourself. See
[Packaged Workflow Architecture]({{< ref "/engine/explanation/packaged-workflow-architecture" >}})
for the full rationale and trade-offs.

## Scaffold the Package

`cloacinactl package new` emits the canonical source tree:

```bash
cloacinactl package new data-pipeline --lang rust
# created rust workflow package at data-pipeline
# next: cloacinactl package validate data-pipeline
```

Your directory structure looks like this:

```
data-pipeline/
├── Cargo.toml
├── package.toml
└── src/
    └── lib.rs           # Note: lib.rs, not main.rs!
```

Look at the generated `Cargo.toml` — this is the **whole** build
configuration:

```toml
[package]
name = "data-pipeline"
version = "0.1.0"
edition = "2021"

[dependencies]
cloacina-workflow = { version = "0.10", features = ["packaged", "macros"] }
cloacina-workflow-plugin = "0.10"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

{{< hint type=important title="The compiler injects the shell" >}}
Notice what is **not** here: no `[lib] crate-type`, no `[features]`
section, no `build.rs`, no build-dependencies. The `cloacina-compiler`
service injects `crate-type = ["cdylib", "rlib"]` and the `packaged`
feature at build time. Older documentation showed packages declaring
these by hand — that model is retired. A package is just the two
dependencies above plus your workflow source.
{{< /hint >}}

Alongside `Cargo.toml` sits `package.toml`, the package manifest the
server reads at upload:

```toml
[package]
name = "data-pipeline"
version = "0.1.0"

[metadata]
workflow_name = "data_pipeline"
description = "data-pipeline workflow"
```

## Understanding the Packaged Workflow

Open `src/lib.rs`. The scaffold generates a two-task workflow:

```rust
use cloacina_workflow::{task, workflow};
use cloacina_workflow::{Context, TaskError};

cloacina_workflow_plugin::package!();

#[workflow(name = "data_pipeline", description = "data-pipeline workflow")]
pub mod data_pipeline_wf {
    use super::*;

    #[task(id = "hello", dependencies = [])]
    pub async fn hello(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("hello", serde_json::json!("world"))?;
        Ok(())
    }

    #[task(id = "goodbye", dependencies = ["hello"])]
    pub async fn goodbye(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("done", serde_json::json!(true))?;
        Ok(())
    }
}
```

Two things distinguish this from an embedded workflow:

1. **`cloacina_workflow_plugin::package!();`** at the crate root — this
   macro emits the FFI vtable a server uses to load the compiled
   library at runtime. Every Rust package has exactly this one line.
2. The workflow and task definitions themselves are **identical** to
   the embedded form — same `#[workflow]` module, same `#[task]`
   attributes, same `Context` API.

## Extend the Workflow

Add a third task so the package does something visible. Replace the
`goodbye` task's dependency chain with a small pipeline — edit
`src/lib.rs` so the module body reads:

```rust
    #[task(id = "collect_data", dependencies = [], retry_attempts = 2)]
    pub async fn collect_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("records", serde_json::json!(1000))?;
        Ok(())
    }

    #[task(id = "process_data", dependencies = ["collect_data"], retry_attempts = 3)]
    pub async fn process_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let records = context.get("records").cloned().unwrap_or_default();
        context.insert("processed", records)?;
        Ok(())
    }

    #[task(id = "generate_report", dependencies = ["process_data"])]
    pub async fn generate_report(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("report_done", serde_json::json!(true))?;
        Ok(())
    }
```

Check it compiles locally (optional — the server compiles it anyway,
but a local build catches errors before upload):

```bash
cloacinactl package build data-pipeline
```

## Validate and Pack

```bash
cloacinactl package validate data-pipeline
cloacinactl package pack data-pipeline
# data-pipeline/data-pipeline.cloacina
```

`validate` checks the source tree against the canonical format without
uploading; `pack` produces the `.cloacina` source archive.

## Start a Compiler and Upload

Rust packages need a running `cloacina-compiler` service — without one,
an uploaded package sits at `build_status = pending` forever. Start one
against the same database as your server:

```bash
cloacinactl compiler start --database-url "$DATABASE_URL"
```

Upload the package (uses the profile from tutorial 01):

```bash
cloacinactl package upload data-pipeline/data-pipeline.cloacina --tenant acme
```

The compiler claims the pending row, runs the cargo build with the
injected shell, and writes the result back. Watch it:

```bash
cloacinactl compiler status
```

Once the build succeeds and the server's reconciler runs (a few
seconds), the workflow appears:

```bash
cloacinactl workflow list --tenant acme
# data_pipeline  v0.1.0 ...
```

## Run It

```bash
echo '{}' > /tmp/ctx.json
cloacinactl workflow run data_pipeline --tenant acme --context /tmp/ctx.json
# <execution-id>

cloacinactl execution status <execution-id> --tenant acme
# Status: Completed
```

## Variations

- **`package publish`** collapses build + pack + upload into one
  command: `cloacinactl package publish data-pipeline`.
- **Validate an archive** (not just a source dir):
  `cloacinactl package validate data-pipeline/data-pipeline.cloacina`.
- **Python packages** skip the compiler entirely — the server loads
  them directly. Scaffold one with `cloacinactl package new <name>`
  (Python is the default `--lang`).

## Next Steps

You've authored a Rust workflow package from the canonical scaffold and
run it through the full upload → compile → reconcile → execute
pipeline.

Next: [04 — Packaging a Computation Graph]({{< ref "/service/tutorials/04-packaging" >}})

## Related Resources

- [Creating Your First Package]({{< ref "/service/how-to/creating-your-first-package" >}}) — the task-focused how-to version of this flow
- [Use cloacina-compiler Locally]({{< ref "/service/how-to/use-cloacina-compiler-locally" >}})
- [Explanation: Packaged Workflow Architecture]({{< ref "/engine/explanation/packaged-workflow-architecture/" >}})
- [CLI Reference]({{< ref "/reference/cli" >}})
