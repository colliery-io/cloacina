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
  [Database Backends]({{< ref "/platform/explanation/database-backends" >}}).

The backend is chosen by the connection URL at runtime — no recompile.

## Size the runner for your load

`DefaultRunnerConfig` exposes the knobs that matter under load. The defaults are
sensible for small embedded use; raise them deliberately:

| Field | Default | Raise it when… |
|-------|---------|----------------|
| `max_concurrent_tasks` | 4 | tasks are I/O-bound and you have headroom |
| `db_pool_size` | 10 | concurrency or replica count is high |
| `task_timeout_seconds` | 300 | legitimate tasks run longer |
| `workflow_timeout_seconds` | 3600 | whole workflows legitimately run longer |
| `enable_recovery` | true | keep on in production (reclaims stalled work) |

{{< tabs "embed-prod-config" >}}
{{< tab "Rust" >}}
```rust
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};

let config = DefaultRunnerConfig {
    max_concurrent_tasks: 16,
    db_pool_size: 24,
    ..DefaultRunnerConfig::default()
};
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

The embedded runner emits the same execution events as the server. Wire your logs
and metrics around workflow/task lifecycle. See
[Observe Execution State]({{< ref "/workflows/how-to-guides/observe-execution-state" >}}).

## Shut down cleanly

Always call `shutdown()` (Rust: `.shutdown().await?`; Python: `runner.shutdown()`,
or use the `DefaultRunner` context manager) so the connection pool drains and
in-flight bookkeeping completes. Tie it to your service's graceful-shutdown path.

## Multiple replicas

Running several instances of your app against **one Postgres** is supported — the
runners coordinate through the database (claiming work atomically). Use Postgres
(not SQLite), keep `enable_recovery` on, and ensure tasks are idempotent. See
[Horizontal Scaling]({{< ref "/platform/explanation/horizontal-scaling" >}}) for the
coordination model.

## See also

- [Runner]({{< ref "/engine/workflows/runner" >}}) · [Reference · Configuration]({{< ref "/reference" >}})
- [Database Backends]({{< ref "/platform/explanation/database-backends" >}})
