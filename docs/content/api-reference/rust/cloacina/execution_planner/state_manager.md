# cloacina::execution_planner::state_manager <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Task state management and dependency checking.

This module handles checking task dependencies and updating task readiness
based on dependency states and trigger rules. Dispatch of Ready tasks is
handled separately by the scheduler loop.

## Structs

### `cloacina::execution_planner::state_manager::StateManager`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


State management operations for the scheduler.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
| `runtime` | `Arc < Runtime >` |  |
