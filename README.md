# Cloacina

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Crates.io](https://img.shields.io/crates/v/cloacina.svg)](https://crates.io/crates/cloacina)


<div align="center">
  <img src="https://github.com/colliery-io/cloacina/raw/main/docs/static/images/image.png" alt="Cloacina Logo" width="400">
</div>

Cloacina is a workflow orchestration platform for Rust, built by [Colliery Software](https://colliery.io). It runs in two complementary modes — as a **library embedded inside your application** for the simplest deployments, or as a **standalone server** (`cloacina-server`) that loads packaged workflows uploaded via a CLI and HTTP API. The same engine, the same packaging format, the same multi-tenant model — you pick the deployment shape that matches the team and the workload.

Cloacina exposes **two execution primitives**:

- **Workflows** — Durable, DB-backed DAGs with retries, recovery, and multi-tenancy. Pick this when work needs to survive process restart.
- **Computation graphs** — In-process, deterministic, event-driven DAGs that fire on accumulator boundaries via reactors. Pick this when work is event-driven and latency-sensitive.

Both surfaces share the runtime and compose: workflows can subscribe to reactor firings, and workflow tasks can invoke embedded computation graphs.

**Cloaca** is the Python wheel that ships full parity with the Rust surface for both primitives — first-class, not a feature flag.

New here? Start with [When to use Cloacina (and when not)](https://colliery-io.github.io/cloacina/quick-start/when-to-use/), then the [Features overview](https://colliery-io.github.io/cloacina/quick-start/features/).

> Why "Cloacina" and "Cloaca" ? Named after the Roman goddess of sewers and drainage systems, Cloacina reflects the library's purpose: efficiently moving data through processing pipelines, just as ancient Roman infrastructure managed the flow of sewage out of the city. Cloaca is the latin noun for the drain, the Cloaca Maxima is the system Cloacina presided over. (Don't read too much into it, apparently there aren't many deities of "plumbing"!)

## Features

- **Two deployment modes** — Embedded library inside your app, or `cloacina-server` loading packaged `.cloacina` archives over HTTP / WebSocket.
- **Two execution primitives** — Durable workflows and in-process computation graphs; pick one or compose both on the same firing.
- **Resilient execution** — Automatic retries, failure recovery, atomic task-completion commits, heartbeat-driven stale-claim recovery.
- **Type-safe workflows** — Compile-time validation of task dependencies and data flow via the `#[task]` / `workflow!` macros.
- **Database-backed** — PostgreSQL or SQLite, selected at runtime by connection URL.
- **Multi-tenant** — PostgreSQL schema-based isolation; fail-closed `search_path` enforcement; 4-step decommission orchestration on the server.
- **Packaged workflows** — Ship `.cloacina` packages (Rust compiled to a cdylib on load, Python as a source module tree); scaffold/validate/pack with `cloacinactl package`; load via HTTP API, signed (optional `--require-signatures`) or unsigned.
- **First-class Python** — `cloaca` PyPI wheel exposes the full surface; not a feature flag.
- **Client SDKs** — Rust, Python, and TypeScript clients for calling a running server over HTTP/WebSocket.
- **Web UI** — Operate and observe a server: workflows, executions (live event stream), triggers, computation-graph health, package upload, and API-key management.
- **Horizontal scaling** — A `cloacina-compiler` build service and a `cloacina-agent` execution fleet scale the server out; stateless schedulers coordinate through the database.
- **Observability** — Prometheus `/metrics` endpoint with the `cloacina_*` namespace, plus structured logs.
- **Async-first** — Built on tokio for high-performance concurrent execution.
- **Content-versioned** — Automatic workflow versioning based on task code and structure.

## Installation

### Rust library

Add Cloacina to your `Cargo.toml`:

```toml
[dependencies]
cloacina = "0.7.0"

async-trait = "0.1"    # Required for async task definitions
serde_json = "1.0"    # Required for context data serialization
```

Cloacina supports both PostgreSQL and SQLite backends. The backend is selected automatically at runtime based on your connection URL - no compile-time configuration needed.

### Single-Backend Builds (Optional)

For smaller binaries, you can compile with only the backend you need:

```toml
# PostgreSQL only
cloacina = { version = "0.7.0", default-features = false, features = ["postgres", "macros"] }

# SQLite only
cloacina = { version = "0.7.0", default-features = false, features = ["sqlite", "macros"] }
```

### Python bindings (`cloaca`)

```sh
pip install cloaca               # default (both backends)
pip install cloaca[sqlite]       # SQLite only
pip install cloaca[postgres]     # PostgreSQL only
```

See the [Python quick start](https://colliery-io.github.io/cloacina/python/quick-start/) for usage.

### `cloacinactl` CLI

The operator + developer CLI (bundling the daemon as `cloacinactl daemon`):

```sh
curl -fsSL https://get.cloacina.dev/install.sh | bash
```

See [Installing cloacinactl](https://colliery-io.github.io/cloacina/quick-start/install/) for version pinning, system-wide installs, and supported platforms.

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

// Initialize the runner with a database
let runner = DefaultRunner::new("postgresql://user:pass@localhost/dbname").await?;

// Execute the workflow (await blocks until terminal state or timeout)
let result = runner.execute("my_workflow", Context::new()).await?;
```

For service-mode usage (running `cloacina-server`, uploading packaged workflows, executing over the HTTP API), see the [platform tutorials](https://colliery-io.github.io/cloacina/platform/tutorials/).

## Multi-Tenancy

Cloacina supports multi-tenant deployments with complete data isolation. PostgreSQL is the supported backend for production multi-tenancy.

### Embedded mode — per-tenant runner

When you're embedding Cloacina as a library, construct one `DefaultRunner` per tenant pinned to a dedicated schema:

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
let runner = DefaultRunner::builder()
    .database_url("postgresql://user:pass@localhost/cloacina")
    .schema("my_tenant")
    .build()
    .await?;
```

### Server mode — provisioned tenants over the HTTP API

When you're running `cloacina-server`, tenants are provisioned and decommissioned via the CLI and HTTP API. The server's `TenantRunnerCache` keeps a runner per tenant, with fail-closed `search_path` enforcement at the DAL.

```sh
# Create a tenant (schema + admin key)
cloacinactl --profile prod tenant create acme

# Decommission a tenant (4-step teardown:
#   revoke keys → evict runner → evict DB cache → drop schema)
cloacinactl --profile prod tenant delete acme
```

See [Configure a Multi-Tenant Deployment](https://colliery-io.github.io/cloacina/platform/how-to-guides/configure-multi-tenant-deployment/) for the operational surface and [Decommission a Tenant](https://colliery-io.github.io/cloacina/platform/how-to-guides/decommission-a-tenant/) for the teardown recipe.

### SQLite file-based isolation (single-tenant per file)

For non-production setups, SQLite gives you isolation by file path:

```rust
let tenant_a = DefaultRunner::new("sqlite://./tenant_a.db").await?;
let tenant_b = DefaultRunner::new("sqlite://./tenant_b.db").await?;
```

### Properties

- **Zero collision risk** — Impossible for tenants to access each other's data.
- **No query changes** — All existing DAL code works unchanged; multi-tenancy is enforced at the connection level.
- **Performance** — No overhead from filtering every query.
- **Clean separation** — Each tenant can run different schema versions, and decommissioning a tenant drops the schema cleanly.

## Repository Structure

```
cloacina/
  crates/                          # 11 Rust crates
    cloacina/                      # Core workflow + computation graph engine
    cloacina-macros/               # Procedural macros (#[task], #[workflow], #[reactor], ...)
    cloacina-build/                # Build-time helpers shared across packaged-workflow crates
    cloacina-compiler/             # cloacina-compiler service (compiles .cloacina archives)
    cloacina-computation-graph/    # CG runtime helpers
    cloacina-python/               # PyO3 bindings (PyPI: cloaca)
    cloacina-server/               # cloacina-server HTTP+WS service
    cloacina-testing/              # Shared test fixtures
    cloacina-workflow/             # Workflow plugin trait + types (host-side)
    cloacina-workflow-plugin/      # Workflow plugin trait + 9-method FFI vtable
    cloacinactl/                   # CLI (operator + developer + bundled daemon)
  charts/cloacina-server/          # Helm chart (with embedded local Postgres subchart)
  examples/
    tutorials/                     # Step-by-step learning path
    features/                      # Feature showcases (filtered-reactor, multi-tenant, ...)
    performance/                   # Benchmarks
  tests/python/                    # Python integration tests
  docs/                            # Documentation site
  install.sh                       # One-line installer (served at get.cloacina.dev)
```

## Documentation

**[Complete Documentation & User Guide](https://colliery-io.github.io/cloacina/)**

Start here:

- [Quick Start](https://colliery-io.github.io/cloacina/quick-start/) — Pick the right tutorial track for your goal.
- [Installing cloacinactl](https://colliery-io.github.io/cloacina/quick-start/install/) — CLI one-liner + Docker + Helm.
- [Workflows · Tutorials](https://colliery-io.github.io/cloacina/workflows/tutorials/) — Rust library tutorials.
- [Computation Graphs · Tutorials](https://colliery-io.github.io/cloacina/computation-graphs/tutorials/) — Event-driven DAG tutorials.
- [Python · Tutorials](https://colliery-io.github.io/cloacina/python/workflows/tutorials/) — Python-side tutorials (mirror Rust 1:1).
- [Platform · How-to Guides](https://colliery-io.github.io/cloacina/platform/how-to-guides/) — Multi-tenant, signed packages, CLI profiles, Helm.

Additional resources:
- [API Reference](https://docs.rs/cloacina) (Rust docs).
- [Tutorial sources](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials).
- [Feature examples](https://github.com/colliery-io/cloacina/tree/main/examples/features) — including `filtered-reactor`, `multi-tenant`, `packaged-graph`.
- [Compiler + Server Deployment Runbook](https://colliery-io.github.io/cloacina/platform/how-to-guides/compiler-deployment-runbook/).
- [Glossary](https://colliery-io.github.io/cloacina/glossary/) — Every term in one place.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.
