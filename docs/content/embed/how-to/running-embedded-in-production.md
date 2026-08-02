---
title: "Running embedded in production"
description: "Operate the embedded engine as a long-lived production component: backend, runner sizing, recovery, observability, and shutdown."
weight: 11
---

# Running embedded in production

Embedding Cloacina is a **production-legitimate** way to run it — not a stepping
stone. This guide covers running the library as a long-lived component of your own
service. (If you'd rather operate a standalone control plane, that's the
[service door]({{< ref "/service" >}}) — a different choice, not a graduation.)

## Choose the backend for your posture

- **SQLite** — single process. Great for embedding in a single-instance app, CLIs,
  and local/dev. No multi-replica coordination.
- **PostgreSQL** — required for **multiple replicas** of your app sharing
  orchestration state, and for schema-isolated multi-tenancy. See
  [Database Backends]({{< ref "/service/explanation/database-backends" >}}).

The backend is chosen by the connection URL at runtime — no recompile.

## Size the runner for your load

`DefaultRunnerConfig` exposes the knobs that matter under load. The defaults are
sensible for small embedded use; raise them deliberately:

| Field | Default | Raise it when… |
|-------|---------|----------------|
| `max_concurrent_tasks` | 4 | tasks are I/O-bound and you have headroom |
| `db_pool_size` | 10 | concurrency or replica count is high |
| `task_timeout` (Python: `task_timeout_seconds`) | 300 s | legitimate tasks run longer |
| `workflow_timeout` (Python: `workflow_timeout_seconds`) | 3600 s | whole workflows legitimately run longer |
| `enable_recovery` | true | keep on in production (reclaims stalled work) |

The Rust struct's fields are private (`#[non_exhaustive]`) — construct it through
its builder:

{{< tabs "embed-prod-config" >}}
{{< tab "Rust" >}}
```rust
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};

let config = DefaultRunnerConfig::builder()
    .max_concurrent_tasks(16)
    .db_pool_size(24)
    .build()?;
let runner = DefaultRunner::with_config(
    "postgresql://user:pass@db:5432/app",
    config,
).await?;
```
{{< /tab >}}
{{< tab "Python" >}}
```python
import cloaca

config = cloaca.DefaultRunnerConfig(max_concurrent_tasks=16, db_pool_size=24)
runner = cloaca.DefaultRunner.with_config(
    "postgresql://user:pass@db:5432/app", config,
)
```
{{< /tab >}}
{{< /tabs >}}

See the full field list in [Reference · Configuration]({{< ref "/reference" >}}).

## Build for at-least-once

Execution is **at-least-once with recovery** — after a crash, in-flight work is
reclaimed and may re-run. Make tasks **idempotent**: writing the same row twice,
re-sending the same message, etc., must be safe. This is the single most important
production property to design for.

## Observe it

The embedded runner records the same execution state the server reads — poll it,
subscribe to status callbacks, and query cron/trigger history in-process. See
[Monitoring Executions]({{< ref "/embed/how-to/monitoring-executions" >}}) for the
embedded observation APIs, and
[Observe Execution State]({{< ref "/embed/how-to/observe-execution-state" >}}) for
the metrics/logs/tracing surfaces of server and daemon deployments.

## Shut down cleanly

Always call `shutdown()` (Rust: `.shutdown().await?`; Python: `runner.shutdown()`,
or use the `DefaultRunner` context manager) so the connection pool drains and
in-flight bookkeeping completes. Tie it to your service's graceful-shutdown path.

## If you load packaged workflows

Two registry defaults are fine for development and wrong for production:

- **Set an explicit registry path.** With the default `"filesystem"` storage
  backend and no `registry_storage_path`, packages are stored under
  `std::env::temp_dir()/cloacina_registry` — a temp directory that can be wiped
  on reboot or by temp-cleaners, silently losing your registered packages. Set
  `.registry_storage_path(Some("/var/lib/myapp/cloacina_registry".into()))` on
  the config builder (or use the `"sqlite"` / `"postgres"` database-backed
  storage via `.registry_storage_backend(...)`).
- **Registry construction failure is non-fatal.** If the registry backend fails
  to construct at startup (bad path, permissions, unknown backend name), the
  runner logs `Failed to create workflow registry: ...` at ERROR and **keeps
  running without a registry** — packaged workflows silently never load. If you
  depend on packaged workflows, alert on that log line or verify after startup
  that `runner.get_workflow_registry().await` returns `Some`.

## If you register cron schedules programmatically

`runner.register_cron_workflow(name, expr, tz)` **creates a new schedule row on
every call** — it does not upsert. Calling it in your service's startup path
means every restart adds a duplicate schedule, and the workflow starts running
N times per tick. Either register once (out-of-band), or check
`list_cron_schedules` for an existing entry before registering. (The registry
reconciler's own path for packaged `#[trigger(cron = ...)]` declarations *is*
upsert-idempotent — this asymmetry only bites programmatic registration.)

## Multiple replicas

Running several instances of your app against **one Postgres** is supported — the
runners coordinate through the database (claiming work atomically). Use Postgres
(not SQLite), keep `enable_recovery` on, and ensure tasks are idempotent. See
[Horizontal Scaling]({{< ref "/service/explanation/horizontal-scaling" >}}) for the
coordination model.

## See also

- [Runner]({{< ref "/engine/workflows/runner" >}}) · [Reference · Configuration]({{< ref "/reference" >}})
- [Database Backends]({{< ref "/service/explanation/database-backends" >}})
