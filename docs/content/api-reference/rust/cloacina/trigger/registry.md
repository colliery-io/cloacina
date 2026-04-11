# cloacina::trigger::registry <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>




## Functions

### `cloacina::trigger::registry::register_trigger_constructor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_trigger_constructor < F > (name : impl Into < String > , constructor : F) where F : Fn () -> Arc < dyn Trigger > + Send + Sync + 'static ,
```

Register a trigger constructor function globally.

This is used internally by the `#[trigger]` macro to automatically register triggers.
Most users won't call this directly.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `-` | Unique name for the trigger |
| `constructor` | `-` | Function that creates a new instance of the trigger |


**Examples:**

```rust,ignore
use cloacina::trigger::{register_trigger_constructor, Trigger};
use std::sync::Arc;

register_trigger_constructor("my_trigger", || {
    Arc::new(MyTrigger::new())
});
```

<details>
<summary>Source</summary>

```rust
pub fn register_trigger_constructor<F>(name: impl Into<String>, constructor: F)
where
    F: Fn() -> Arc<dyn Trigger> + Send + Sync + 'static,
{
    let name = name.into();
    let mut registry = GLOBAL_TRIGGER_REGISTRY.write();
    registry.insert(name.clone(), Box::new(constructor));
    tracing::debug!("Registered trigger constructor: {}", name);
}
```

</details>



### `cloacina::trigger::registry::register_trigger`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_trigger < T : Trigger + Clone + 'static > (trigger : T)
```

Register a trigger instance directly.

This is a convenience function for registering a single trigger instance.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `trigger` | `-` | The trigger to register |


<details>
<summary>Source</summary>

```rust
pub fn register_trigger<T: Trigger + Clone + 'static>(trigger: T) {
    let name = trigger.name().to_string();
    register_trigger_constructor(name, move || Arc::new(trigger.clone()));
}
```

</details>



### `cloacina::trigger::registry::get_trigger`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_trigger (name : & str) -> Option < Arc < dyn Trigger > >
```

Get a trigger instance from the global registry by name.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `-` | The name of the trigger to retrieve |


**Returns:**

* `Some(Arc<dyn Trigger>)` - If the trigger exists * `None` - If no trigger with that name is registered

<details>
<summary>Source</summary>

```rust
pub fn get_trigger(name: &str) -> Option<Arc<dyn Trigger>> {
    let registry = GLOBAL_TRIGGER_REGISTRY.read();
    registry.get(name).map(|constructor| constructor())
}
```

</details>



### `cloacina::trigger::registry::global_trigger_registry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn global_trigger_registry () -> GlobalTriggerRegistry
```

Get the global trigger registry.

This provides access to the global trigger registry used by the macro system.
Most users won't need to call this directly.

<details>
<summary>Source</summary>

```rust
pub fn global_trigger_registry() -> GlobalTriggerRegistry {
    GLOBAL_TRIGGER_REGISTRY.clone()
}
```

</details>



### `cloacina::trigger::registry::list_triggers`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn list_triggers () -> Vec < String >
```

Get all registered trigger names.

**Returns:**

A vector of all trigger names currently registered.

<details>
<summary>Source</summary>

```rust
pub fn list_triggers() -> Vec<String> {
    let registry = GLOBAL_TRIGGER_REGISTRY.read();
    registry.keys().cloned().collect()
}
```

</details>



### `cloacina::trigger::registry::get_all_triggers`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_all_triggers () -> Vec < Arc < dyn Trigger > >
```

Get all registered triggers.

**Returns:**

A vector of all trigger instances currently registered.

<details>
<summary>Source</summary>

```rust
pub fn get_all_triggers() -> Vec<Arc<dyn Trigger>> {
    let registry = GLOBAL_TRIGGER_REGISTRY.read();
    registry.values().map(|constructor| constructor()).collect()
}
```

</details>



### `cloacina::trigger::registry::deregister_trigger`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn deregister_trigger (name : & str) -> bool
```

Deregister a trigger by name.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `-` | The name of the trigger to deregister |


**Returns:**

`true` if the trigger was found and removed, `false` otherwise.

<details>
<summary>Source</summary>

```rust
pub fn deregister_trigger(name: &str) -> bool {
    let mut registry = GLOBAL_TRIGGER_REGISTRY.write();
    registry.remove(name).is_some()
}
```

</details>



### `cloacina::trigger::registry::is_trigger_registered`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_trigger_registered (name : & str) -> bool
```

Check if a trigger is registered.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `-` | The name of the trigger to check |


**Returns:**

`true` if the trigger is registered, `false` otherwise.

<details>
<summary>Source</summary>

```rust
pub fn is_trigger_registered(name: &str) -> bool {
    let registry = GLOBAL_TRIGGER_REGISTRY.read();
    registry.contains_key(name)
}
```

</details>



### `cloacina::trigger::registry::clear_triggers`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn clear_triggers ()
```

Clear all registered triggers.

This is primarily useful for testing to reset the registry state.

<details>
<summary>Source</summary>

```rust
pub fn clear_triggers() {
    let mut registry = GLOBAL_TRIGGER_REGISTRY.write();
    registry.clear();
}
```

</details>
