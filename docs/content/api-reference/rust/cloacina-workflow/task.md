# cloacina-workflow::task <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>




## Enums

### `cloacina-workflow::task::TaskState` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Represents the execution state of a task throughout its lifecycle.

Tasks progress through these states during execution, providing visibility
into the current status and enabling proper error handling and recovery.

#### Variants

- **`Pending`** - Task is registered but not yet started
- **`Running`** - Task is currently executing
- **`Completed`** - Task finished successfully
- **`Failed`** - Task encountered an error
- **`Skipped`** - Task was skipped (e.g., trigger rule not satisfied)
