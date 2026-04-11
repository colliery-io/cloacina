# cloacina::execution_planner::context_manager <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Context management for task execution.

This module handles loading and merging contexts for tasks based on
their dependencies.

## Structs

### `cloacina::execution_planner::context_manager::ContextManager`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Context management operations for the scheduler.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
| `runtime` | `Arc < Runtime >` |  |
