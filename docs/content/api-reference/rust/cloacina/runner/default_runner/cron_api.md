# cloacina::runner::default_runner::cron_api <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Cron scheduling API for the DefaultRunner.

This module provides methods for managing cron-scheduled workflow executions.

## Structs

### `cloacina::runner::default_runner::cron_api::DalCronRegistrar`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Adapter that lets the registry reconciler register/unregister cron workflow schedules without holding a `DefaultRunner` reference back (which would form a cycle, given the runner OWNS the reconciler). Holds an `Arc<Database>` and replicates the schedule-CRUD logic from the runner's `register_cron_workflow` / `delete_cron_schedule` methods. Constructed by `services.rs` only when cron scheduling is enabled in the runner config; otherwise the reconciler runs without a registrar and cron triggers warn loudly at load.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `database` | `crate :: database :: Database` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (database : crate :: database :: Database) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(database: crate::database::Database) -> Self {
        Self { database }
    }
```

</details>
