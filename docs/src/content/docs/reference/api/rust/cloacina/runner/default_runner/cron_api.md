---
title: "cloacina::runner::default_runner::cron_api"
description: "Cron scheduling API for the DefaultRunner."
editUrl: false
sidebar:
  order: 180
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Cron scheduling API for the DefaultRunner.

This module provides methods for managing cron-scheduled workflow executions.

## Structs

### <span id="struct.DalCronRegistrar"></span><code style="font-size: 0.85em">struct DalCronRegistrar</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Adapter that lets the registry reconciler register/unregister cron workflow schedules without holding a `DefaultRunner` reference back (which would form a cycle, given the runner OWNS the reconciler). Holds an `Arc<Database>` and replicates the schedule-CRUD logic from the runner's `register_cron_workflow` / `delete_cron_schedule` methods. Constructed by `services.rs` only when cron scheduling is enabled in the runner config; otherwise the reconciler runs without a registrar and cron triggers warn loudly at load.

#### Methods

##### <span id="DalCronRegistrar.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    database: crate::database::Database,
    cron_change: Arc<tokio::sync::Notify>,
) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(database: crate::database::Database, cron_change: Arc<tokio::sync::Notify>) -> Self {
        Self {
            database,
            cron_change,
        }
    }
```

</details>
