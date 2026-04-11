# cloacina::runtime <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Scoped runtime for isolated task, workflow, and trigger registries.

[`Runtime`] replaces direct access to process-global static registries,
enabling multiple isolated workflow environments in the same process and
parallel test execution without `#[serial]`.

## Structs

### `cloacina::runtime::Runtime`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

A scoped runtime holding isolated registries for tasks, workflows, and triggers.

`Runtime` enables multiple independent workflow environments in the same process.
Each runtime has its own set of registered tasks, workflows, and triggers that
do not interfere with other runtimes or the process-global registries.
Two modes:
- [`Runtime::new()`] — isolated, no fallback (for tests)
- [`Runtime::from_global()`] — delegates to global registries for dynamic
package loading (for the server)

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `Arc < RuntimeInner >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new () -> Self
```

Create an empty runtime with no registered tasks, workflows, or triggers.

Lookups only check the local maps — no fallback to globals.
Use this for test isolation.

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RuntimeInner {
                tasks: RwLock::new(HashMap::new()),
                workflows: RwLock::new(HashMap::new()),
                triggers: RwLock::new(HashMap::new()),
                use_globals: false,
            }),
        }
    }
```

</details>



##### `from_global` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_global () -> Self
```

Create a runtime that delegates to the process-global registries.

Lookups check the local maps first, then fall back to the global
task/workflow/trigger registries. This supports dynamic package
loading — packages registered after startup are visible immediately.

<details>
<summary>Source</summary>

```rust
    pub fn from_global() -> Self {
        Self {
            inner: Arc::new(RuntimeInner {
                tasks: RwLock::new(HashMap::new()),
                workflows: RwLock::new(HashMap::new()),
                triggers: RwLock::new(HashMap::new()),
                use_globals: true,
            }),
        }
    }
```

</details>



##### `register_task` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_task < F > (& self , namespace : TaskNamespace , constructor : F) where F : Fn () -> Arc < dyn Task > + Send + Sync + 'static ,
```

Register a task constructor for the given namespace.

<details>
<summary>Source</summary>

```rust
    pub fn register_task<F>(&self, namespace: TaskNamespace, constructor: F)
    where
        F: Fn() -> Arc<dyn Task> + Send + Sync + 'static,
    {
        let mut guard = self.inner.tasks.write();
        guard.insert(namespace, Box::new(constructor));
    }
```

</details>



##### `get_task` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_task (& self , namespace : & TaskNamespace) -> Option < Arc < dyn Task > >
```

Look up and instantiate a task by namespace.

Checks local registry first, then falls back to the global registry
if `use_globals` is enabled (i.e., created via `from_global()`).

<details>
<summary>Source</summary>

```rust
    pub fn get_task(&self, namespace: &TaskNamespace) -> Option<Arc<dyn Task>> {
        // Check local first
        {
            let guard = self.inner.tasks.read();
            if let Some(ctor) = guard.get(namespace) {
                return Some(ctor());
            }
        }
        // Fall back to globals
        if self.inner.use_globals {
            return crate::task::get_task(namespace);
        }
        None
    }
```

</details>



##### `has_task` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn has_task (& self , namespace : & TaskNamespace) -> bool
```

Check if a task is registered for the given namespace.

<details>
<summary>Source</summary>

```rust
    pub fn has_task(&self, namespace: &TaskNamespace) -> bool {
        let guard = self.inner.tasks.read();
        if guard.contains_key(namespace) {
            return true;
        }
        if self.inner.use_globals {
            let global = crate::task::global_task_registry();
            let g = global.read();
            return g.contains_key(namespace);
        }
        false
    }
```

</details>



##### `register_workflow` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_workflow < F > (& self , name : String , constructor : F) where F : Fn () -> Workflow + Send + Sync + 'static ,
```

Register a workflow constructor by name.

<details>
<summary>Source</summary>

```rust
    pub fn register_workflow<F>(&self, name: String, constructor: F)
    where
        F: Fn() -> Workflow + Send + Sync + 'static,
    {
        let mut guard = self.inner.workflows.write();
        guard.insert(name, Box::new(constructor));
    }
```

</details>



##### `get_workflow` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_workflow (& self , name : & str) -> Option < Workflow >
```

Look up and instantiate a workflow by name.

Checks local registry first, then falls back to the global registry
if `use_globals` is enabled.

<details>
<summary>Source</summary>

```rust
    pub fn get_workflow(&self, name: &str) -> Option<Workflow> {
        {
            let guard = self.inner.workflows.read();
            if let Some(ctor) = guard.get(name) {
                return Some(ctor());
            }
        }
        if self.inner.use_globals {
            let global = crate::workflow::global_workflow_registry();
            let g = global.read();
            return g.get(name).map(|ctor| ctor());
        }
        None
    }
```

</details>



##### `workflow_names` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn workflow_names (& self) -> Vec < String >
```

Get all registered workflow names.

<details>
<summary>Source</summary>

```rust
    pub fn workflow_names(&self) -> Vec<String> {
        let guard = self.inner.workflows.read();
        guard.keys().cloned().collect()
    }
```

</details>



##### `all_workflows` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn all_workflows (& self) -> Vec < Workflow >
```

Get all registered workflows (instantiated).

<details>
<summary>Source</summary>

```rust
    pub fn all_workflows(&self) -> Vec<Workflow> {
        let guard = self.inner.workflows.read();
        guard.values().map(|ctor| ctor()).collect()
    }
```

</details>



##### `register_trigger` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_trigger < F > (& self , name : String , constructor : F) where F : Fn () -> Arc < dyn Trigger > + Send + Sync + 'static ,
```

Register a trigger constructor by name.

<details>
<summary>Source</summary>

```rust
    pub fn register_trigger<F>(&self, name: String, constructor: F)
    where
        F: Fn() -> Arc<dyn Trigger> + Send + Sync + 'static,
    {
        let mut guard = self.inner.triggers.write();
        guard.insert(name, Box::new(constructor));
    }
```

</details>



##### `get_trigger` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_trigger (& self , name : & str) -> Option < Arc < dyn Trigger > >
```

Look up and instantiate a trigger by name.

Checks local registry first, then falls back to the global registry
if `use_globals` is enabled.

<details>
<summary>Source</summary>

```rust
    pub fn get_trigger(&self, name: &str) -> Option<Arc<dyn Trigger>> {
        {
            let guard = self.inner.triggers.read();
            if let Some(ctor) = guard.get(name) {
                return Some(ctor());
            }
        }
        if self.inner.use_globals {
            let global = crate::trigger::global_trigger_registry();
            let g = global.read();
            return g.get(name).map(|ctor| ctor());
        }
        None
    }
```

</details>



##### `trigger_names` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn trigger_names (& self) -> Vec < String >
```

Get all registered trigger names.

<details>
<summary>Source</summary>

```rust
    pub fn trigger_names(&self) -> Vec<String> {
        let guard = self.inner.triggers.read();
        guard.keys().cloned().collect()
    }
```

</details>



##### `all_triggers` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn all_triggers (& self) -> HashMap < String , Arc < dyn Trigger > >
```

Get all registered triggers (instantiated).

<details>
<summary>Source</summary>

```rust
    pub fn all_triggers(&self) -> HashMap<String, Arc<dyn Trigger>> {
        let guard = self.inner.triggers.read();
        guard
            .iter()
            .map(|(name, ctor)| (name.clone(), ctor()))
            .collect()
    }
```

</details>





### `cloacina::runtime::RuntimeInner`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tasks` | `RwLock < HashMap < TaskNamespace , TaskConstructorFn > >` |  |
| `workflows` | `RwLock < HashMap < String , WorkflowConstructorFn > >` |  |
| `triggers` | `RwLock < HashMap < String , TriggerConstructorFn > >` |  |
| `use_globals` | `bool` | When true, `get_*()` falls back to the process-global registries
if the local map doesn't contain the entry. This enables dynamic
package loading (reconciler registers in globals after startup). |
