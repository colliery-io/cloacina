---
title: "Custom Task Routing"
description: "How to route tasks to different executors based on name patterns using RoutingConfig"
weight: 50
---

# Custom Task Routing

This guide explains how to route tasks to different executor backends based on name patterns. An executor is a named processing backend (e.g., a thread pool, GPU cluster, or remote runner) that the dispatcher assigns tasks to. Use routing when different tasks in your workflow need different resource profiles — for example, routing ML training tasks to GPU-equipped executors and ETL tasks to high-memory nodes.

{{< hint type="note" >}}
Routing is currently configured programmatically in Rust via `RoutingConfig`. There is no config-file or CLI equivalent.
{{< /hint >}}

## Prerequisites

- A multi-task workflow with tasks of different resource requirements
- Familiarity with the [Dispatcher Architecture]({{< ref "/workflows/explanation/dispatcher-architecture" >}})

## Configuring Routes

Routes are evaluated in order — the first matching rule wins. If no rules match, the default executor handles the task.

Use `RoutingConfig` and `RoutingRule` to define your routing table:

```rust
use cloacina::dispatcher::types::{RoutingConfig, RoutingRule};

let config = RoutingConfig::new("default")
    .with_rule(RoutingRule::new("ml::*", "gpu"))
    .with_rule(RoutingRule::new("etl::heavy_*", "high_memory"))
    .with_rule(RoutingRule::new("**::audit_*", "low_priority"));
```

You can also add multiple rules at once with `with_rules`:

```rust
let config = RoutingConfig::new("default")
    .with_rules([
        RoutingRule::new("ml::*", "gpu"),
        RoutingRule::new("etl::heavy_*", "high_memory"),
    ]);
```

Then pass the routing config to the runner:

```rust
let runner_config = DefaultRunnerConfig::builder()
    .routing_config(Some(config))
    .build();
```

## Pattern Syntax

Routing patterns match against fully qualified task names (e.g., `tenant::package::workflow::task_id`).

| Pattern | Matches | Does Not Match |
|---------|---------|----------------|
| `ml::train` | `ml::train` (exact) | `ml::predict` |
| `ml::*` | `ml::train`, `ml::predict` | `etl::extract` |
| `heavy_*` | `heavy_compute`, `heavy_load` | `light_compute` |
| `*_gpu` | `train_gpu`, `infer_gpu` | `train_cpu` |
| `**` | Any task name | — |
| `**::heavy_*` | `ml::heavy_train`, `etl::data::heavy_load` | `ml::light_train` |

### Wildcards

- **`*`** — matches any characters within a single namespace segment (does not cross `::` boundaries)
- **`**`** — matches any number of segments, including `::` separators

### Rule Priority

Rules are evaluated **in order**. The first match wins. Place specific rules before general ones:

```rust
// Correct: specific before general
let config = RoutingConfig::new("default")
    .with_rule(RoutingRule::new("ml::train", "gpu_dedicated"))   // specific
    .with_rule(RoutingRule::new("ml::*", "gpu_shared"));          // general

// ml::train → gpu_dedicated (first rule matches)
// ml::predict → gpu_shared (second rule matches)
```

If you reverse the order, `ml::train` would match `ml::*` first and go to `gpu_shared`.

## Full Example

```rust
use cloacina::dispatcher::types::{RoutingConfig, RoutingRule};
use cloacina::runner::default_runner::DefaultRunnerConfig;

// Define executor routing
let routing = RoutingConfig::new("cpu_general")
    .with_rule(RoutingRule::new("ml::train_*", "gpu_cluster"))
    .with_rule(RoutingRule::new("ml::*", "gpu_shared"))
    .with_rule(RoutingRule::new("etl::heavy_*", "high_memory"))
    .with_rule(RoutingRule::new("**::audit_*", "low_priority"));

let config = DefaultRunnerConfig::builder()
    .max_concurrent_tasks(16)
    .routing_config(Some(routing))
    .build();

// Tasks route as follows:
// "ml::train_model"    → gpu_cluster    (rule 1)
// "ml::predict"        → gpu_shared     (rule 2)
// "etl::heavy_load"    → high_memory    (rule 3)
// "etl::data::audit_log" → low_priority (rule 4)
// "etl::extract"       → cpu_general    (default)
```

## Adding Rules Dynamically

You can add rules after creating the router:

```rust
use cloacina::dispatcher::router::Router;

let mut router = Router::new(config);
router.add_rule(RoutingRule::new("batch::*", "batch_executor"));
```

## Common Patterns

### GPU Tasks

```rust
.with_rule(RoutingRule::new("**::train_*", "gpu"))
.with_rule(RoutingRule::new("**::infer_*", "gpu"))
```

### Background / Low-Priority

```rust
.with_rule(RoutingRule::new("**::cleanup_*", "background"))
.with_rule(RoutingRule::new("**::archive_*", "background"))
```

### Tenant Isolation

```rust
.with_rule(RoutingRule::new("premium_tenant::**", "dedicated"))
.with_rule(RoutingRule::new("free_tier::**", "shared"))
```

## See Also

- [Dispatcher Architecture]({{< ref "/workflows/explanation/dispatcher-architecture" >}}) — how the dispatcher works
- [Horizontal Scaling]({{< ref "/platform/explanation/horizontal-scaling" >}}) — multi-runner coordination
- [Production Deployment]({{< ref "/platform/how-to-guides/production-deployment" >}}) — deploying with multiple executors
