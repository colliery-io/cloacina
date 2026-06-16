---
title: "09 — Working with the Workflow Registry"
description: "Build a .cloacina package, register it through the DAL, and execute it once from a registry-backed runner"
weight: 19
aliases:
  - "/workflows/tutorials/service/08-workflow-registry/"

---

In this tutorial you'll register a packaged workflow into Cloacina's **workflow registry** and execute it once from a runner that loads the package at runtime — no recompilation of your application required. The registry stores `.cloacina` packages and lets a `DefaultRunner` reconcile and run them dynamically. For the bigger picture of how versions are tracked, see [Workflow Versioning]({{< ref "/engine/explanation/workflow-versioning" >}}).

{{< hint type=note title="Shown in Rust" >}}
This tutorial is shown in Rust only. Registry registration and reconciliation are
core engine features exercised here through the Rust API.
{{< /hint >}}

{{< hint type=info title="This is an advanced topic" >}}
This tutorial assumes you can already build a `.cloacina` package. If you can't yet,
work through [Packaging Python Workflows]({{< ref "/embed/how-to/packaging-python-workflows" >}})
or [14 — Packaged Triggers]({{< ref "/embed/tutorials/14-packaged-triggers" >}}) first.
{{< /hint >}}

## What you'll learn

- How to build a `.cloacina` package from a workflow project in code
- How to register a package through the DAL (`workflow_registry().register_workflow_package`)
- How to configure a `DefaultRunner` with the registry reconciler enabled
- How to execute a registry-loaded workflow once and inspect the result

## Prerequisites

- The Cloacina repository checked out, with a Rust toolchain installed
- The ability to build a `.cloacina` package (see the hint above)

## The complete example

The full source lives at [`examples/features/workflows/registry-execution`](https://github.com/colliery-io/cloacina/tree/main/examples/features/workflows/registry-execution).

To run it from the repository root:

```bash
cargo run --features sqlite -p registry-execution-demo
```

The rest of this tutorial walks through the path step by step.

---

## Step 1: Build the `.cloacina` package

The registry stores packaged workflows, so the first step is to produce one. Here we
package the `simple-packaged` example project directly in code with
`cloacina::packaging::package_workflow`, returning the package bytes:

```rust
async fn build_package() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let workspace_root = find_workspace_root()?;
    let project_path = workspace_root.join("examples/features/workflows/simple-packaged");

    let temp_dir = TempDir::new()?;
    let output_path = temp_dir.path().join("package.cloacina");

    cloacina::packaging::package_workflow(project_path, output_path.clone())?;

    Ok(tokio::fs::read(output_path).await?)
}
```

Call it from `main`:

```rust
println!("📦 Building workflow package...");
let package_data = build_package().await?;
println!("✅ Package built: {} bytes", package_data.len());
```

**Result:** the package bytes are in memory, ready to register. You'll see the size printed, e.g. `✅ Package built: 1065567 bytes`.

## Step 2: Set up a database

The registry stores package metadata (and, with database storage, the binary too) in a
Cloacina database. Use a fresh SQLite file so a stale run can't dispatch an outdated
package before the reconciler finishes loading:

```rust
let db_path = "/tmp/cloacina_demo.db";
let _ = std::fs::remove_file(db_path);
let db_url = format!("sqlite://{}?mode=rwc", db_path);

let database = Database::new(&db_url, "", 5);
database
    .run_migrations()
    .await
    .map_err(|e| format!("Migration error: {}", e))?;
```

**Result:** a migrated SQLite database at `/tmp/cloacina_demo.db` that both the registry and the runner will share.

## Step 3: Register the package through the DAL

Registration goes through the DAL. You create a `DAL`, choose a storage backend, obtain
a registry DAL, and call `register_workflow_package` with the package bytes:

```rust
// Create the DAL and choose a storage backend.
let dal = cloacina::dal::DAL::new(database.clone());

// Database storage — stores binary data in the workflow_registry table.
let storage = cloacina::dal::UnifiedRegistryStorage::new(database.clone());

// Filesystem storage is also available:
// let storage = cloacina::dal::FilesystemRegistryStorage::new(storage_path);

let mut registry_dal = dal.workflow_registry(storage);

match registry_dal.register_workflow_package(package_data).await {
    Ok(package_id) => {
        println!("✅ Package registered with DAL ID: {}", package_id);
    }
    Err(RegistryError::PackageExists { package_name, version }) => {
        println!(
            "⚠️  Package already exists: {} v{} - continuing with existing package",
            package_name, version
        );
    }
    Err(e) => return Err(e.into()),
};
```

**Result:** the package is stored in the registry. On the first run you'll see `✅ Package registered with DAL ID: ...`; on later runs the `PackageExists` arm reports that it already exists and continues.

You can confirm it landed by listing packages:

```rust
let packages = registry_dal.list_packages().await?;
for package_info in &packages {
    println!(
        "  - Package {} (v{}) - ID: {}",
        package_info.package_name, package_info.version, package_info.id
    );
}
```

**Result:** at least one package, e.g. `- Package simple_demo (v1.0.0) - ID: ...`.

## Step 4: Build a runner with the registry reconciler

`DefaultRunnerConfig` is built through its builder — its fields are private. Enable the
registry reconciler and point it at the same storage backend you registered with
(`sqlite` here, matching the database storage from Step 3):

```rust
let config = DefaultRunnerConfig::builder()
    .enable_registry_reconciler(true)
    .registry_storage_backend("sqlite")
    .build()
    .unwrap();

let runner = DefaultRunner::with_config(&db_url, config).await?;
```

The reconciler runs in the background, scans the registry, loads workflow metadata, and
registers the workflow's tasks in the runtime. Give it a moment to finish startup
reconciliation before executing:

```rust
let workflow_name = "data_processing"; // the workflow exported by simple-packaged
tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
```

**Result:** a running `DefaultRunner` that knows about the registered workflow.

## Step 5: Execute the workflow once

Build a context and execute the workflow by name. The runner resolves it from the
registry-loaded tasks:

```rust
let mut context = Context::new();
context.insert("demo", serde_json::json!("registry-execution"))?;

let result = runner.execute(workflow_name, context).await?;

println!("✅ Workflow executed successfully!");
println!("   Execution ID: {}", result.execution_id);
println!("   Status: {:?}", result.status);
```

**Result:** the workflow runs end to end from the registry:

```
🚀 Executing workflow from registry...
🔍 Collecting data...
✅ Collected 1000 records
⚙️  Processing data...
✅ Processed 950 valid records
📊 Generating report...
✅ Report generated successfully

✅ Workflow executed successfully!
   Execution ID: 87654321-4321-8765-cba9-876543210987
   Status: Completed
```

## Step 6: Shut down

Always shut the runner down cleanly so background tasks stop:

```rust
runner.shutdown().await?;
```

**Result:** the runner stops, and the database at `/tmp/cloacina_demo.db` remains with the registered package for next time.

---

## Where to go next

You've registered a package and executed it once from the registry. The example source
also demonstrates **cron scheduling** of a registered workflow (via
`runner.register_cron_workflow(...)`) and gathering execution statistics — see the
[full example](https://github.com/colliery-io/cloacina/tree/main/examples/features/workflows/registry-execution)
and [Cron Scheduling]({{< ref "/embed/tutorials/05-cron-scheduling" >}}). For how the
registry tracks workflow versions, see
[Workflow Versioning]({{< ref "/engine/explanation/workflow-versioning" >}}).

---

Prev: [08 — Task Deferral]({{< ref "/embed/tutorials/08-task-deferral" >}}) ·
Next: [10 — Your First Computation Graph]({{< ref "/embed/tutorials/10-computation-graph" >}})
