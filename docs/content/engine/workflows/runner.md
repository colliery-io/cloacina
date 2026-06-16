---
title: "Runner"
description: "The host that executes workflows against a database — the embedded engine's entry point."
weight: 14
---

# Runner

The **Runner** (`DefaultRunner`) is the host that executes
[Workflows]({{< ref "/engine/workflows/workflow" >}}) against a database. It is the
**entry point** for the embedded engine and an elevated, first-class operational
primitive: it owns the database connection pool, the scheduler, and task dispatch.
You create one, execute workflows on it, and shut it down.

## Mental model

- The **database URL selects the backend** — `sqlite://…` or `postgresql://…` —
  at runtime, no recompile.
- One runner manages all shared state (pool, scheduler, recovery).
- Execution is **at-least-once with recovery**; the runner reclaims stalled work.

## Interfaces

{{< tabs "runner-use" >}}
{{< tab "Rust" >}}
```rust
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};

let runner = DefaultRunner::with_config(
    "sqlite://app.db?mode=rwc&_journal_mode=WAL",
    DefaultRunnerConfig::default(),
).await?;

let result = runner.execute("greeting", cloacina::Context::new()).await?;
println!("{:?}", result.status);

runner.shutdown().await?;
```
{{< /tab >}}
{{< tab "Python" >}}
```python
import cloaca

runner = cloaca.DefaultRunner("sqlite:///app.db")
result = runner.execute("greeting", cloaca.Context())
print(result.status)
runner.shutdown()
```
The constructor takes only a database URL. To pass a configuration use
`DefaultRunner.with_config(url, config)`; for a PostgreSQL schema-isolated tenant
use `DefaultRunner.with_schema(url, schema)`.
{{< /tab >}}
{{< /tabs >}}

## Key facts

- **Backend by URL:** SQLite (single-process; embedding/dev) or PostgreSQL
  (multi-tenant/scale).
- **Config:** tuned via `DefaultRunnerConfig` (concurrency, timeouts, pool size,
  cron/recovery). See [Reference · Configuration]({{< ref "/reference" >}}).
- **Multi-tenant (Postgres):** `with_schema` pins the runner to one tenant schema.
- **Lifecycle:** always `shutdown()` to release the pool cleanly (Python
  `DefaultRunner` is also a context manager).

## See also

- [Workflow]({{< ref "/engine/workflows/workflow" >}}) · [Context]({{< ref "/engine/workflows/context" >}})
- Pick a way to run it: [Embed the Library]({{< ref "/embed" >}}) · [Run the Service]({{< ref "/service" >}})
