# Cloacina

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Crates.io](https://img.shields.io/crates/v/cloacina.svg)](https://crates.io/crates/cloacina)


<div align="center">
  <img src="https://github.com/colliery-io/cloacina/raw/main/docs/static/images/image.png" alt="Cloacina Logo" width="400">
</div>

Cloacina is a **workflow orchestration engine for Rust and Python**, built by [Colliery Software](https://colliery.io). Its only hard dependency is a database — PostgreSQL or SQLite. No broker, no queue, no coordinator.

You run the same engine **two co-equal ways**:

- **Embed the library** — add `cloacina` (Rust) or `cloaca` (Python) as a dependency and run orchestration inside your application, against a database you already operate. Embedding is a permanent, production-legitimate end-state — not a starter mode you graduate from.
- **Run the service** — operate `cloacina-server` as a multi-tenant control plane with an HTTP/WebSocket API and web UI, and ship workflows to it as `.cloacina` packages.

Cloacina is embedded-first by design: the engine is a genuine standalone library, and the server is built on top of it — not the other way around.

Cloacina exposes **two execution primitives**:

- **Workflows** — Durable, DB-backed DAGs with retries, recovery, and multi-tenancy. Pick this when work needs to survive process restart.
- **Computation graphs** — In-process, deterministic, event-driven DAGs that fire on accumulator boundaries via reactors. Pick this when work is event-driven and latency-sensitive.

Both surfaces share the runtime and compose: workflows can subscribe to reactor firings, and workflow tasks can invoke embedded computation graphs.

**Cloaca** is the Python package — the same engine with a first-class Python authoring and runtime surface, not a wrapper around a subset.

New here? Start with [Is Cloacina for you?](https://colliery-io.github.io/cloacina/start/is-cloacina-for-you/), then the [Features Overview](https://colliery-io.github.io/cloacina/start/features/).

> Why "Cloacina" and "Cloaca" ? Named after the Roman goddess of sewers and drainage systems, Cloacina reflects the library's purpose: efficiently moving data through processing pipelines, just as ancient Roman infrastructure managed the flow of sewage out of the city. Cloaca is the latin noun for the drain, the Cloaca Maxima is the system Cloacina presided over. (Don't read too much into it, apparently there aren't many deities of "plumbing"!)

## Features

- **Two ways to run it** — Embedded library inside your app, or `cloacina-server` loading packaged `.cloacina` archives over HTTP / WebSocket.
- **Two execution primitives** — Durable workflows and in-process computation graphs; pick one or compose both on the same firing.
- **Resilient execution** — Automatic retries, failure recovery, atomic task-completion commits, heartbeat-driven stale-claim recovery.
- **Type-safe workflows** — Compile-time validation of task dependencies and data flow via the `#[task]` / `#[workflow]` attribute macros.
- **Database-backed** — PostgreSQL or SQLite, selected at runtime by connection URL.
- **Multi-tenant** — PostgreSQL schema-based isolation; fail-closed `search_path` enforcement; 4-step decommission orchestration on the server.
- **Packaged workflows** — Ship `.cloacina` source packages (Rust compiled server-side by `cloacina-compiler`, Python loaded as a source module tree); scaffold/validate/pack with `cloacinactl package`; upload via CLI or HTTP API.
- **First-class Python** — the `cloaca` PyPI wheel embeds the engine in your Python process; not a feature flag.
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
cloacina = "0.10"
cloacina-workflow = "0.10"   # macro-generated task code references this crate directly
tokio = { version = "1", features = ["full"] }
serde_json = "1.0"
```

Cloacina supports both PostgreSQL and SQLite backends. The backend is selected automatically at runtime based on your connection URL - no compile-time configuration needed.

### Single-Backend Builds (Optional)

For smaller binaries, you can compile with only the backend you need:

```toml
# PostgreSQL only
cloacina = { version = "0.10", default-features = false, features = ["postgres", "macros"] }

# SQLite only
cloacina = { version = "0.10", default-features = false, features = ["sqlite", "macros"] }
```

### Python bindings (`cloaca`)

```sh
pip install cloaca
```

The wheel bundles both database backends; the one you use is chosen at runtime by connection URL, same as Rust. See the [embedded quick start](https://colliery-io.github.io/cloacina/embed/quick-start/) for usage.

### `cloacinactl` CLI

The operator + developer CLI (bundling the daemon as `cloacinactl daemon`):

```sh
curl -fsSL https://get.cloacina.dev/install.sh | bash
```

See [Installing cloacinactl](https://colliery-io.github.io/cloacina/start/install/) for version pinning, system-wide installs, and supported platforms.

## Quick Start

A one-task workflow, run in-process against SQLite:

```rust
use cloacina::executor::WorkflowExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError};

#[workflow(name = "my_workflow", description = "A simple workflow")]
pub mod my_workflow {
    use super::*;

    #[task(id = "process_data", dependencies = [])]
    pub async fn process_data(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        // Your business logic here
        context.insert("processed", serde_json::json!(true))?;
        println!("Data processed successfully!");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Backend chosen by URL scheme: sqlite://... or postgresql://...
    let runner = DefaultRunner::with_config(
        "sqlite://my_app.db",
        DefaultRunnerConfig::default(),
    )
    .await?;

    // Blocks until the workflow reaches a terminal state (or times out)
    let result = runner.execute("my_workflow", Context::new()).await?;
    println!("status: {:?}", result.status);

    runner.shutdown().await?;
    Ok(())
}
```

The same workflow in Python:

```python
import cloaca

with cloaca.WorkflowBuilder("my_workflow") as builder:
    builder.description("A simple workflow")

    @cloaca.task(id="process_data")
    def process_data(context):
        context.set("processed", True)
        return context

if __name__ == "__main__":
    runner = cloaca.DefaultRunner("sqlite://my_app.db")
    result = runner.execute("my_workflow", cloaca.Context())
    print("status:", result.status)
    runner.shutdown()
```

For service-mode usage (running `cloacina-server`, uploading packaged workflows, executing over the HTTP API), see [Run the Service](https://colliery-io.github.io/cloacina/service/).

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

See [Configure a Multi-Tenant Deployment](https://colliery-io.github.io/cloacina/service/how-to/configure-multi-tenant-deployment/) for the operational surface and [Decommission a Tenant](https://colliery-io.github.io/cloacina/service/how-to/decommission-a-tenant/) for the teardown recipe.

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
  crates/                          # 15 Rust crates
    cloacina/                      # Core workflow + computation graph engine
    cloacina-agent/                # Execution-agent fleet worker
    cloacina-api-types/            # Shared API types
    cloacina-build/                # build.rs helper for crates that link pyo3
    cloacina-client/               # Rust client SDK for the server API
    cloacina-compiler/             # cloacina-compiler service (compiles .cloacina archives)
    cloacina-computation-graph/    # CG runtime types
    cloacina-constructor-contract/ # Constructor-provider contract types
    cloacina-macros/               # Procedural macros (#[task], #[workflow], #[reactor], ...)
    cloacina-python/               # PyO3 bindings (PyPI: cloaca)
    cloacina-server/               # cloacina-server HTTP+WS service
    cloacina-testing/              # Test harness (TestRunner, assertions)
    cloacina-workflow/             # Task/workflow authoring types (host-side)
    cloacina-workflow-plugin/      # FFI plugin interface for .cloacina packages
    cloacinactl/                   # CLI (operator + developer + bundled daemon)
  providers/                       # Constructor provider crates (fs, kafka, ...)
  clients/                         # Python + TypeScript client SDKs
  charts/cloacina-server/          # Helm chart (with embedded local Postgres subchart)
  ui/                              # Web UI (ships embedded in the server)
  examples/
    tutorials/                     # Step-by-step learning path
    features/                      # Feature showcases (filtered-reactor, multi-tenant, ...)
    performance/                   # Benchmarks
  tests/python/                    # Python integration tests
  docs/                            # Documentation site
  scripts/install.sh               # One-line installer (served at get.cloacina.dev)
```

## Documentation

**[Complete Documentation & User Guide](https://colliery-io.github.io/cloacina/)**

Start here:

- [Start Here](https://colliery-io.github.io/cloacina/start/) — what Cloacina is, whether it fits, and which door to pick.
- [Installing cloacinactl](https://colliery-io.github.io/cloacina/start/install/) — CLI one-liner + Docker + Helm.
- [Embed the Library](https://colliery-io.github.io/cloacina/embed/) — quick start and tutorials for Rust and Python, in-process.
- [Run the Service](https://colliery-io.github.io/cloacina/service/) — deploy and operate `cloacina-server`.
- [Engine & Primitives](https://colliery-io.github.io/cloacina/engine/) — workflows, computation graphs, and the objects they're built from.
- [Reference](https://colliery-io.github.io/cloacina/reference/) — APIs, CLI, HTTP/WebSocket, configuration.

Additional resources:
- [API Reference](https://docs.rs/cloacina) (Rust docs).
- [Tutorial sources](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials).
- [Feature examples](https://github.com/colliery-io/cloacina/tree/main/examples/features) — including `filtered-reactor`, `multi-tenant`, `packaged-graph`.
- [Glossary](https://colliery-io.github.io/cloacina/reference/glossary/) — Every term in one place.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.
