# cloacina::runtime <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Scoped runtime unifying all cloacina registries.

[`Runtime`] owns the registries for tasks, workflows, triggers, computation
graphs, and stream backends. Every entry can be registered and unregistered
at runtime, which is the mechanism the reconciler uses to hot-swap packages.
The process-global static registries that predated `Runtime` were deleted
in CLOACI-T-0509. [`Runtime::new`] seeds itself from the `inventory` entries
emitted by the macros; the reconciler and Python bindings push into it
directly via [`Runtime::register_task`], [`Runtime::register_workflow`], etc.
```rust,ignore
use cloacina::Runtime;
let runtime = Runtime::new(); // seeded from inventory
runtime.register_task(namespace, || Arc::new(my_task()));
runtime.unregister_workflow("obsolete_workflow");
```

## Structs

### `cloacina::runtime::Runtime`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

A scoped runtime holding the registries for every cloacina extension point.

All five namespaces — tasks, workflows, triggers, computation graphs, and
stream backends — are registered and unregistered through the same surface.
`Runtime` is cheap to clone: it shares its registries via `Arc`.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `Arc < RuntimeInner >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new () -> Self
```

Create a runtime seeded with every macro-registered entry from the `inventory` crate (tasks, workflows, triggers, computation graphs, stream backends).

`inventory` collects entries in a linker section and is read lazily
after `main()`, so every entry registered by the `#[task]`,
`#[workflow]`, `#[trigger]`, `#[computation_graph]`, and stream-backend
macros in the current binary is visible here. For a blank-slate runtime
(used by isolation-sensitive tests), use [`Runtime::empty`] instead.

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        let rt = Self::empty();
        rt.seed_from_inventory();
        rt
    }
```

</details>



##### `empty` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn empty () -> Self
```

Create an empty runtime with no registered entries in any namespace.

Use this when you want complete isolation — no macro-registered tasks,
workflows, triggers, CGs, or stream backends are installed. Intended
for unit tests; production code should generally use [`Runtime::new`].

<details>
<summary>Source</summary>

```rust
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(RuntimeInner {
                tasks: RwLock::new(HashMap::new()),
                workflows: RwLock::new(HashMap::new()),
                triggers: RwLock::new(HashMap::new()),
                computation_graphs: RwLock::new(HashMap::new()),
                triggerless_graphs: RwLock::new(HashMap::new()),
                reactors: RwLock::new(HashMap::new()),
                stream_backends: RwLock::new(HashMap::new()),
            }),
        }
    }
```

</details>



##### `seed_from_inventory` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn seed_from_inventory (& self)
```

Populate the runtime from the `inventory` entries emitted by the macros.

`inventory`'s linker-section collection works across `dlopen`'d cdylibs
on Linux/macOS, so the reconciler calls this again after loading a new
workflow package to pick up the entries emitted by that cdylib.

<details>
<summary>Source</summary>

```rust
    pub fn seed_from_inventory(&self) {
        use crate::inventory_entries::{
            ComputationGraphEntry, ReactorEntry, StreamBackendEntry, TaskEntry, TriggerEntry,
            TriggerlessGraphEntry, WorkflowEntry,
        };

        for entry in inventory::iter::<TaskEntry> {
            let ns = (entry.namespace)();
            let ctor = entry.constructor;
            self.register_task(ns, move || ctor());
        }

        for entry in inventory::iter::<WorkflowEntry> {
            self.register_workflow(entry.name.to_string(), entry.constructor);
        }

        for entry in inventory::iter::<TriggerEntry> {
            self.register_trigger(entry.name.to_string(), entry.constructor);
        }

        for entry in inventory::iter::<ComputationGraphEntry> {
            self.register_computation_graph(entry.name.to_string(), entry.constructor);
        }

        for entry in inventory::iter::<TriggerlessGraphEntry> {
            self.register_triggerless_graph(entry.name.to_string(), entry.constructor);
        }

        for entry in inventory::iter::<ReactorEntry> {
            self.register_reactor(entry.name.to_string(), entry.constructor);
        }

        for entry in inventory::iter::<StreamBackendEntry> {
            let factory = entry.factory;
            self.register_stream_backend(
                entry.type_name.to_string(),
                Box::new(move |config| factory(config)),
            );
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
        self.inner
            .tasks
            .write()
            .insert(namespace, Box::new(constructor));
    }
```

</details>



##### `unregister_task` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn unregister_task (& self , namespace : & TaskNamespace) -> bool
```

Remove a task constructor. Returns true if the entry existed.

<details>
<summary>Source</summary>

```rust
    pub fn unregister_task(&self, namespace: &TaskNamespace) -> bool {
        self.inner.tasks.write().remove(namespace).is_some()
    }
```

</details>



##### `get_task` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_task (& self , namespace : & TaskNamespace) -> Option < Arc < dyn Task > >
```

Look up and instantiate a task by namespace.

<details>
<summary>Source</summary>

```rust
    pub fn get_task(&self, namespace: &TaskNamespace) -> Option<Arc<dyn Task>> {
        self.inner.tasks.read().get(namespace).map(|ctor| ctor())
    }
```

</details>



##### `has_task` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">pub(crate)</span>


```rust
fn has_task (& self , namespace : & TaskNamespace) -> bool
```

Check if a task is registered for the given namespace.

<details>
<summary>Source</summary>

```rust
    pub(crate) fn has_task(&self, namespace: &TaskNamespace) -> bool {
        self.inner.tasks.read().contains_key(namespace)
    }
```

</details>



##### `task_namespaces` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn task_namespaces (& self) -> Vec < TaskNamespace >
```

Snapshot of every currently-registered task namespace. Used by code that needs to enumerate tasks (e.g. collecting all tasks belonging to a specific tenant/package/workflow triple during Python import).

<details>
<summary>Source</summary>

```rust
    pub fn task_namespaces(&self) -> Vec<TaskNamespace> {
        self.inner.tasks.read().keys().cloned().collect()
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
        self.inner
            .workflows
            .write()
            .insert(name, Box::new(constructor));
    }
```

</details>



##### `unregister_workflow` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn unregister_workflow (& self , name : & str) -> bool
```

Remove a workflow constructor. Returns true if the entry existed.

<details>
<summary>Source</summary>

```rust
    pub fn unregister_workflow(&self, name: &str) -> bool {
        self.inner.workflows.write().remove(name).is_some()
    }
```

</details>



##### `get_workflow` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_workflow (& self , name : & str) -> Option < Workflow >
```

Look up and instantiate a workflow by name.

<details>
<summary>Source</summary>

```rust
    pub fn get_workflow(&self, name: &str) -> Option<Workflow> {
        self.inner.workflows.read().get(name).map(|ctor| ctor())
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
        self.inner.workflows.read().keys().cloned().collect()
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
        self.inner
            .triggers
            .write()
            .insert(name, Box::new(constructor));
    }
```

</details>



##### `unregister_trigger` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn unregister_trigger (& self , name : & str) -> bool
```

Remove a trigger constructor. Returns true if the entry existed.

<details>
<summary>Source</summary>

```rust
    pub fn unregister_trigger(&self, name: &str) -> bool {
        self.inner.triggers.write().remove(name).is_some()
    }
```

</details>



##### `get_trigger` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_trigger (& self , name : & str) -> Option < Arc < dyn Trigger > >
```

Look up and instantiate a trigger by name.

<details>
<summary>Source</summary>

```rust
    pub fn get_trigger(&self, name: &str) -> Option<Arc<dyn Trigger>> {
        self.inner.triggers.read().get(name).map(|ctor| ctor())
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
        self.inner.triggers.read().keys().cloned().collect()
    }
```

</details>



##### `register_computation_graph` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_computation_graph < F > (& self , name : String , constructor : F) where F : Fn () -> ComputationGraphRegistration + Send + Sync + 'static ,
```

Register a computation graph constructor by graph name.

<details>
<summary>Source</summary>

```rust
    pub fn register_computation_graph<F>(&self, name: String, constructor: F)
    where
        F: Fn() -> ComputationGraphRegistration + Send + Sync + 'static,
    {
        self.inner
            .computation_graphs
            .write()
            .insert(name, Box::new(constructor));
    }
```

</details>



##### `unregister_computation_graph` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn unregister_computation_graph (& self , name : & str) -> bool
```

Remove a computation graph constructor. Returns true if the entry existed.

<details>
<summary>Source</summary>

```rust
    pub fn unregister_computation_graph(&self, name: &str) -> bool {
        self.inner.computation_graphs.write().remove(name).is_some()
    }
```

</details>



##### `get_computation_graph` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_computation_graph (& self , name : & str) -> Option < ComputationGraphRegistration >
```

Look up and instantiate a computation graph registration by name.

<details>
<summary>Source</summary>

```rust
    pub fn get_computation_graph(&self, name: &str) -> Option<ComputationGraphRegistration> {
        self.inner
            .computation_graphs
            .read()
            .get(name)
            .map(|ctor| ctor())
    }
```

</details>



##### `computation_graph_names` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn computation_graph_names (& self) -> Vec < String >
```

Get all registered computation graph names.

<details>
<summary>Source</summary>

```rust
    pub fn computation_graph_names(&self) -> Vec<String> {
        self.inner
            .computation_graphs
            .read()
            .keys()
            .cloned()
            .collect()
    }
```

</details>



##### `register_triggerless_graph` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_triggerless_graph < F > (& self , name : String , constructor : F) where F : Fn () -> TriggerlessGraphRegistration + Send + Sync + 'static ,
```

Register a trigger-less computation graph constructor by graph name.

Trigger-less graphs are declared with `#[computation_graph(graph =
{ ... })]` (no `trigger = reactor(...)` clause) and operate on a
`Context<Value>`. They are invoked directly by workflow tasks
(T-02) and Python decorators (T-03).

<details>
<summary>Source</summary>

```rust
    pub fn register_triggerless_graph<F>(&self, name: String, constructor: F)
    where
        F: Fn() -> TriggerlessGraphRegistration + Send + Sync + 'static,
    {
        self.inner
            .triggerless_graphs
            .write()
            .insert(name, Box::new(constructor));
    }
```

</details>



##### `unregister_triggerless_graph` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn unregister_triggerless_graph (& self , name : & str) -> bool
```

Remove a trigger-less graph constructor. Returns true if the entry existed.

<details>
<summary>Source</summary>

```rust
    pub fn unregister_triggerless_graph(&self, name: &str) -> bool {
        self.inner.triggerless_graphs.write().remove(name).is_some()
    }
```

</details>



##### `get_triggerless_graph` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_triggerless_graph (& self , name : & str) -> Option < TriggerlessGraphRegistration >
```

Look up and instantiate a trigger-less graph registration by name.

<details>
<summary>Source</summary>

```rust
    pub fn get_triggerless_graph(&self, name: &str) -> Option<TriggerlessGraphRegistration> {
        self.inner
            .triggerless_graphs
            .read()
            .get(name)
            .map(|ctor| ctor())
    }
```

</details>



##### `triggerless_graph_names` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn triggerless_graph_names (& self) -> Vec < String >
```

Get every registered trigger-less graph name.

<details>
<summary>Source</summary>

```rust
    pub fn triggerless_graph_names(&self) -> Vec<String> {
        self.inner
            .triggerless_graphs
            .read()
            .keys()
            .cloned()
            .collect()
    }
```

</details>



##### `register_reactor` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_reactor < F > (& self , name : String , constructor : F) where F : Fn () -> ReactorRegistration + Send + Sync + 'static ,
```

Register a reactor constructor by name.

Reactors declared via `#[reactor]` or synthesized by the bundled form
of `#[computation_graph]` land here. Graphs that declare
`trigger = reactor(X)` bind to the named reactor at load time.

<details>
<summary>Source</summary>

```rust
    pub fn register_reactor<F>(&self, name: String, constructor: F)
    where
        F: Fn() -> ReactorRegistration + Send + Sync + 'static,
    {
        self.inner
            .reactors
            .write()
            .insert(name, Box::new(constructor));
    }
```

</details>



##### `unregister_reactor` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn unregister_reactor (& self , name : & str) -> bool
```

Remove a reactor constructor. Returns true if the entry existed.

<details>
<summary>Source</summary>

```rust
    pub fn unregister_reactor(&self, name: &str) -> bool {
        self.inner.reactors.write().remove(name).is_some()
    }
```

</details>



##### `get_reactor` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_reactor (& self , name : & str) -> Option < ReactorRegistration >
```

Look up and instantiate a reactor registration by name.

<details>
<summary>Source</summary>

```rust
    pub fn get_reactor(&self, name: &str) -> Option<ReactorRegistration> {
        self.inner.reactors.read().get(name).map(|ctor| ctor())
    }
```

</details>



##### `reactor_names` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn reactor_names (& self) -> Vec < String >
```

Get every registered reactor name.

<details>
<summary>Source</summary>

```rust
    pub fn reactor_names(&self) -> Vec<String> {
        self.inner.reactors.read().keys().cloned().collect()
    }
```

</details>



##### `register_stream_backend` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_stream_backend (& self , type_name : String , factory : StreamBackendFactory)
```

Register a stream backend factory by type name (e.g. `"kafka"`, `"mock"`).

<details>
<summary>Source</summary>

```rust
    pub fn register_stream_backend(&self, type_name: String, factory: StreamBackendFactory) {
        self.inner
            .stream_backends
            .write()
            .insert(type_name, factory);
    }
```

</details>



##### `unregister_stream_backend` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn unregister_stream_backend (& self , type_name : & str) -> bool
```

Remove a stream backend factory. Returns true if the entry existed.

<details>
<summary>Source</summary>

```rust
    pub fn unregister_stream_backend(&self, type_name: &str) -> bool {
        self.inner
            .stream_backends
            .write()
            .remove(type_name)
            .is_some()
    }
```

</details>



##### `has_stream_backend` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">pub(crate)</span>


```rust
fn has_stream_backend (& self , type_name : & str) -> bool
```

Check if a stream backend is registered for the given type name.

<details>
<summary>Source</summary>

```rust
    pub(crate) fn has_stream_backend(&self, type_name: &str) -> bool {
        self.inner.stream_backends.read().contains_key(type_name)
    }
```

</details>



##### `create_stream_backend` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn create_stream_backend (& self , type_name : & str , config : StreamConfig ,) -> Option < StreamBackendFuture >
```

Get the creation future for a stream backend without holding the lock across await. Returns `None` if the type is not registered.

<details>
<summary>Source</summary>

```rust
    pub fn create_stream_backend(
        &self,
        type_name: &str,
        config: StreamConfig,
    ) -> Option<StreamBackendFuture> {
        let guard = self.inner.stream_backends.read();
        let factory = guard.get(type_name)?;
        Some(factory(config))
    }
```

</details>



##### `stream_backend_names` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">pub(crate)</span>


```rust
fn stream_backend_names (& self) -> Vec < String >
```

Get all registered stream backend type names.

<details>
<summary>Source</summary>

```rust
    pub(crate) fn stream_backend_names(&self) -> Vec<String> {
        self.inner.stream_backends.read().keys().cloned().collect()
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
| `computation_graphs` | `RwLock < HashMap < String , ComputationGraphConstructor > >` |  |
| `triggerless_graphs` | `RwLock < HashMap < String , TriggerlessGraphConstructor > >` |  |
| `reactors` | `RwLock < HashMap < String , ReactorConstructor > >` |  |
| `stream_backends` | `RwLock < HashMap < String , StreamBackendFactory > >` |  |
