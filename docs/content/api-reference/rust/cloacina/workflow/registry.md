# cloacina::workflow::registry <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Global workflow registry for automatic workflow registration.

This module provides the global registry used by the `workflow!` macro
to automatically register workflows at startup.

## Functions

### `cloacina::workflow::registry::register_workflow_constructor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_workflow_constructor < F > (workflow_name : String , constructor : F) where F : Fn () -> Workflow + Send + Sync + 'static ,
```

Register a workflow constructor function globally

This is used internally by the `workflow!` macro to automatically register workflows.
Most users won't call this directly.

<details>
<summary>Source</summary>

```rust
pub fn register_workflow_constructor<F>(workflow_name: String, constructor: F)
where
    F: Fn() -> Workflow + Send + Sync + 'static,
{
    let mut registry = GLOBAL_WORKFLOW_REGISTRY.write();
    registry.insert(workflow_name, Box::new(constructor));
    tracing::debug!("Successfully registered workflow constructor");
}
```

</details>



### `cloacina::workflow::registry::global_workflow_registry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn global_workflow_registry () -> GlobalWorkflowRegistry
```

Get the global workflow registry

This provides access to the global workflow registry used by the macro system.
Most users won't need to call this directly.

<details>
<summary>Source</summary>

```rust
pub fn global_workflow_registry() -> GlobalWorkflowRegistry {
    GLOBAL_WORKFLOW_REGISTRY.clone()
}
```

</details>



### `cloacina::workflow::registry::get_all_workflows`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_all_workflows () -> Vec < Workflow >
```

Get all workflows from the global registry

Returns instances of all workflows registered with the `workflow!` macro.

**Examples:**

```rust
use cloacina::*;

let all_workflows = get_all_workflows();
for workflow in all_workflows {
    println!("Found workflow: {}", workflow.name());
}
```

<details>
<summary>Source</summary>

```rust
pub fn get_all_workflows() -> Vec<Workflow> {
    let registry = GLOBAL_WORKFLOW_REGISTRY.read();
    registry.values().map(|constructor| constructor()).collect()
}
```

</details>
