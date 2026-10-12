---
title: "cloacina::runtime"
description: "Scoped runtime unifying all cloacina registries."
sidebar:
  order: 204
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Scoped runtime unifying all cloacina registries.

[`Runtime`] owns the registries for tasks, workflows, triggers, computation
graphs, and stream backends. Every entry can be registered and unregistered
at runtime, which is the mechanism the reconciler uses to hot-swap packages.
The process-global static registries that predated `Runtime` were deleted
in CLOACI-T-0509. [`Runtime::new`] seeds itself from the `inventory` entries
emitted by the macros; the reconciler and Python bindings push into it
directly via [`Runtime::register_task`], [`Runtime::register_workflow`], etc.
```rust
use cloacina::Runtime;
let runtime = Runtime::new(); // seeded from inventory
runtime.register_task(namespace, || Arc::new(my_task()));
runtime.unregister_workflow("obsolete_workflow");
```

## Structs

### <span id="struct.ScopedEntry"></span>`struct ScopedEntry<V>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


One entry in a [`ScopedRegistry`]: the constructor plus who claimed it.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `owner` | `TenantOwner` |  |
| `value` | `V` |  |



### <span id="struct.ScopedRegistry"></span>`struct ScopedRegistry<V>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


A `(tenant, name)`-keyed registry with owner-checked replacement.

Shared by the five bare-name registries so they have exactly one keying and
one resolution rule between them.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `kind` | `& 'static str` | Operator-facing noun used in conflict errors ("workflow", "reactor", …). |
| `entries` | `RwLock < HashMap < TenantKey , ScopedEntry < V > > >` |  |



### <span id="struct.Runtime"></span>`struct Runtime`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Clone`

A scoped runtime holding the registries for every cloacina extension point.

All five namespaces — tasks, workflows, triggers, computation graphs, and
stream backends — are registered and unregistered through the same surface.
`Runtime` is cheap to clone: it shares its registries via `Arc`.
A clone carries the same tenant scope; use [`Runtime::scoped_to_tenant`],
[`Runtime::untenanted_view`] or [`Runtime::admin_view`] to get a
differently-scoped view over the same registries.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `Arc < RuntimeInner >` |  |
| `scope` | `RuntimeScope` |  |

#### Methods

##### <span id="Runtime.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="Runtime.empty"></span>`empty`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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
                workflows: ScopedRegistry::new("workflow"),
                triggers: ScopedRegistry::new("trigger"),
                computation_graphs: ScopedRegistry::new("computation graph"),
                triggerless_graphs: ScopedRegistry::new("trigger-less computation graph"),
                reactors: ScopedRegistry::new("reactor"),
                stream_backends: RwLock::new(HashMap::new()),
            }),
            scope: RuntimeScope::default(),
        }
    }
```

</details>



##### <span id="Runtime.scope"></span>`scope`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn scope (& self) -> TenantScope < '_ >
```

The scope this handle registers and resolves under.

<details>
<summary>Source</summary>

```rust
    fn scope(&self) -> TenantScope<'_> {
        self.scope.as_tenant_scope()
    }
```

</details>



##### <span id="Runtime.scoped_to_tenant"></span>`scoped_to_tenant`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn scoped_to_tenant (& self , tenant_id : impl Into < String >) -> Self
```

A view over the *same* registries bound to `tenant_id`.

Registrations made through the returned handle land under
`(tenant_id, name)`; lookups try that tenant first, then fall back to
the untenanted (inventory / embedded) entries. This is what
`DefaultRunner` calls with `config.tenant_id()`.

<details>
<summary>Source</summary>

```rust
    pub fn scoped_to_tenant(&self, tenant_id: impl Into<String>) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            scope: RuntimeScope {
                tenant_id: Some(tenant_id.into()),
                is_admin: false,
            },
        }
    }
```

</details>



##### <span id="Runtime.untenanted_view"></span>`untenanted_view`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn untenanted_view (& self) -> Self
```

A view over the same registries with no tenant — the embedded / pre-multi-tenancy scope, and the scope `seed_from_inventory` writes under.

<details>
<summary>Source</summary>

```rust
    pub fn untenanted_view(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            scope: RuntimeScope::default(),
        }
    }
```

</details>



##### <span id="Runtime.admin_view"></span>`admin_view`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn admin_view (& self) -> Self
```

A view that may resolve any tenant's entry, but only when the name is unique across tenants. Intended for operator/diagnostic surfaces.

<details>
<summary>Source</summary>

```rust
    pub fn admin_view(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            scope: RuntimeScope {
                tenant_id: None,
                is_admin: true,
            },
        }
    }
```

</details>



##### <span id="Runtime.tenant_id"></span>`tenant_id`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn tenant_id (& self) -> Option < & str >
```

The tenant this handle is bound to, if any.

<details>
<summary>Source</summary>

```rust
    pub fn tenant_id(&self) -> Option<&str> {
        self.scope.tenant_id.as_deref()
    }
```

</details>



##### <span id="Runtime.is_admin"></span>`is_admin`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn is_admin (& self) -> bool
```

Whether this handle is an admin view.

<details>
<summary>Source</summary>

```rust
    pub fn is_admin(&self) -> bool {
        self.scope.is_admin
    }
```

</details>



##### <span id="Runtime.shares_registries_with"></span>`shares_registries_with`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn shares_registries_with (& self , other : & Runtime) -> bool
```

`true` when both handles are views over the same registry allocation.

Replaces `Arc::ptr_eq` comparisons of `Arc<Runtime>`: two per-tenant
runners share one `RuntimeInner` but hold differently-scoped handles.

<details>
<summary>Source</summary>

```rust
    pub fn shares_registries_with(&self, other: &Runtime) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
```

</details>



##### <span id="Runtime.seed_from_inventory"></span>`seed_from_inventory`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn seed_from_inventory (& self)
```

Populate the runtime from the `inventory` entries emitted by the macros.

`inventory`'s linker-section collection works across `dlopen`'d cdylibs
on Linux/macOS, so the reconciler calls this again after loading a new
workflow package to pick up the entries emitted by that cdylib.
Inventory entries are always registered **untenanted**, whatever scope
this handle carries (CLOACI-T-0924). They are host-binary, compile-time
declarations — never tenant-authored — and the reconciler re-runs this
after every `dlopen`, so scoping them would cross-stamp one tenant's
cdylib entries onto whichever tenant happened to load next. Untenanted
entries stay reachable from every tenant via the resolution fallback,
which is exactly the pre-CLOACI-T-0924 behaviour.

<details>
<summary>Source</summary>

```rust
    pub fn seed_from_inventory(&self) {
        use crate::inventory_entries::{
            ComputationGraphEntry, ReactorEntry, StreamBackendEntry, TaskEntry, TriggerEntry,
            TriggerlessGraphEntry, WorkflowEntry,
        };

        let global = self.untenanted_view();

        for entry in inventory::iter::<TaskEntry> {
            let ns = (entry.namespace)();
            let ctor = entry.constructor;
            self.register_task(ns, move || ctor());
        }

        for entry in inventory::iter::<WorkflowEntry> {
            global.register_workflow(entry.name.to_string(), entry.constructor);
        }

        for entry in inventory::iter::<TriggerEntry> {
            global.register_trigger(entry.name.to_string(), entry.constructor);
        }

        for entry in inventory::iter::<ComputationGraphEntry> {
            global.register_computation_graph(entry.name.to_string(), entry.constructor);
        }

        for entry in inventory::iter::<TriggerlessGraphEntry> {
            global.register_triggerless_graph(entry.name.to_string(), entry.constructor);
        }

        for entry in inventory::iter::<ReactorEntry> {
            global.register_reactor(entry.name.to_string(), entry.constructor);
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



##### <span id="Runtime.register_task"></span>`register_task`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn register_task < F > (& self , namespace : TaskNamespace , factory : F) where F : Fn () -> Arc < dyn Task > + Send + Sync + 'static ,
```

Register a task constructor for the given namespace.

<details>
<summary>Source</summary>

```rust
    pub fn register_task<F>(&self, namespace: TaskNamespace, factory: F)
    where
        F: Fn() -> Arc<dyn Task> + Send + Sync + 'static,
    {
        self.inner
            .tasks
            .write()
            .insert(namespace, Box::new(factory));
    }
```

</details>



##### <span id="Runtime.unregister_task"></span>`unregister_task`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="Runtime.get_task"></span>`get_task`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="Runtime.has_task"></span>`has_task`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">pub(crate)</span>


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



##### <span id="Runtime.task_namespaces"></span>`task_namespaces`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="Runtime.register_workflow"></span>`register_workflow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn register_workflow < F > (& self , name : String , constructor : F) where F : Fn () -> Workflow + Send + Sync + 'static ,
```

Register a workflow constructor by name, under this handle's tenant.

Unattributed registration: it never conflicts, so it cannot fail. Use
[`Runtime::try_register_workflow`] from the package loader, where the
owning package is known and a same-tenant collision must be loud.

<details>
<summary>Source</summary>

```rust
    pub fn register_workflow<F>(&self, name: String, constructor: F)
    where
        F: Fn() -> Workflow + Send + Sync + 'static,
    {
        let _ = self.inner.workflows.insert(
            self.scope(),
            name,
            TenantOwner::unknown(),
            Box::new(constructor),
        );
    }
```

</details>



##### <span id="Runtime.try_register_workflow"></span>`try_register_workflow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn try_register_workflow < F > (& self , owner : & TenantOwner , name : String , constructor : F ,) -> Result < () , RuntimeRegistrationError > where F : Fn () -> Workflow + Send + Sync + 'static ,
```

Register a workflow constructor attributed to `owner`, rejecting a same-tenant claim on a name a *different* package already owns.

<details>
<summary>Source</summary>

```rust
    pub fn try_register_workflow<F>(
        &self,
        owner: &TenantOwner,
        name: String,
        constructor: F,
    ) -> Result<(), RuntimeRegistrationError>
    where
        F: Fn() -> Workflow + Send + Sync + 'static,
    {
        self.inner
            .workflows
            .insert(self.scope(), name, owner.clone(), Box::new(constructor))
    }
```

</details>



##### <span id="Runtime.unregister_workflow"></span>`unregister_workflow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn unregister_workflow (& self , name : & str) -> bool
```

Remove a workflow constructor. Returns true if the entry existed.

<details>
<summary>Source</summary>

```rust
    pub fn unregister_workflow(&self, name: &str) -> bool {
        self.inner.workflows.remove(self.scope(), name)
    }
```

</details>



##### <span id="Runtime.get_workflow"></span>`get_workflow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn get_workflow (& self , name : & str) -> Option < Workflow >
```

Look up and instantiate a workflow by name, within this handle's scope.

<details>
<summary>Source</summary>

```rust
    pub fn get_workflow(&self, name: &str) -> Option<Workflow> {
        self.inner.workflows.with(self.scope(), name, |ctor| ctor())
    }
```

</details>



##### <span id="Runtime.workflow_names"></span>`workflow_names`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn workflow_names (& self) -> Vec < String >
```

Get every workflow name visible to this handle's scope.

<details>
<summary>Source</summary>

```rust
    pub fn workflow_names(&self) -> Vec<String> {
        self.inner.workflows.names(self.scope())
    }
```

</details>



##### <span id="Runtime.workflow_keys"></span>`workflow_keys`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn workflow_keys (& self) -> Vec < TenantKey >
```

Every `(tenant, workflow)` key in the process, unfiltered. Diagnostics and cross-tenant isolation tests.

<details>
<summary>Source</summary>

```rust
    pub fn workflow_keys(&self) -> Vec<TenantKey> {
        self.inner.workflows.all_keys()
    }
```

</details>



##### <span id="Runtime.register_trigger"></span>`register_trigger`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn register_trigger < F > (& self , name : String , factory : F) where F : Fn () -> Arc < dyn Trigger > + Send + Sync + 'static ,
```

Register a trigger constructor by name, under this handle's tenant.

<details>
<summary>Source</summary>

```rust
    pub fn register_trigger<F>(&self, name: String, factory: F)
    where
        F: Fn() -> Arc<dyn Trigger> + Send + Sync + 'static,
    {
        let _ = self.inner.triggers.insert(
            self.scope(),
            name,
            TenantOwner::unknown(),
            Box::new(factory),
        );
    }
```

</details>



##### <span id="Runtime.try_register_trigger"></span>`try_register_trigger`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn try_register_trigger < F > (& self , owner : & TenantOwner , name : String , factory : F ,) -> Result < () , RuntimeRegistrationError > where F : Fn () -> Arc < dyn Trigger > + Send + Sync + 'static ,
```

Register a trigger constructor attributed to `owner`, rejecting a same-tenant claim on a name a *different* package already owns.

<details>
<summary>Source</summary>

```rust
    pub fn try_register_trigger<F>(
        &self,
        owner: &TenantOwner,
        name: String,
        factory: F,
    ) -> Result<(), RuntimeRegistrationError>
    where
        F: Fn() -> Arc<dyn Trigger> + Send + Sync + 'static,
    {
        self.inner
            .triggers
            .insert(self.scope(), name, owner.clone(), Box::new(factory))
    }
```

</details>



##### <span id="Runtime.may_claim_trigger"></span>`may_claim_trigger`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn may_claim_trigger (& self , owner : & TenantOwner , name : & str ,) -> Result < bool , RuntimeRegistrationError >
```

Ask whether `owner` may claim the trigger `name` in this handle's scope *before* registering it (CLOACI-T-0924).

The package loader has a "reuse whatever the cdylib's `inventory`
already registered" fast path, which would otherwise let a second
package silently adopt (and later tear down) a first package's trigger.
`Ok(true)` — free, register it. `Ok(false)` — an entry this owner may
reuse or replace exists. `Err` — a different package owns the name in
this tenant.

<details>
<summary>Source</summary>

```rust
    pub fn may_claim_trigger(
        &self,
        owner: &TenantOwner,
        name: &str,
    ) -> Result<bool, RuntimeRegistrationError> {
        self.inner.triggers.check_claim(self.scope(), name, owner)
    }
```

</details>



##### <span id="Runtime.unregister_trigger"></span>`unregister_trigger`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn unregister_trigger (& self , name : & str) -> bool
```

Remove a trigger constructor. Returns true if the entry existed.

<details>
<summary>Source</summary>

```rust
    pub fn unregister_trigger(&self, name: &str) -> bool {
        self.inner.triggers.remove(self.scope(), name)
    }
```

</details>



##### <span id="Runtime.get_trigger"></span>`get_trigger`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn get_trigger (& self , name : & str) -> Option < Arc < dyn Trigger > >
```

Look up and instantiate a trigger by name, within this handle's scope.

<details>
<summary>Source</summary>

```rust
    pub fn get_trigger(&self, name: &str) -> Option<Arc<dyn Trigger>> {
        self.inner.triggers.with(self.scope(), name, |ctor| ctor())
    }
```

</details>



##### <span id="Runtime.trigger_names"></span>`trigger_names`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn trigger_names (& self) -> Vec < String >
```

Get every trigger name visible to this handle's scope.

<details>
<summary>Source</summary>

```rust
    pub fn trigger_names(&self) -> Vec<String> {
        self.inner.triggers.names(self.scope())
    }
```

</details>



##### <span id="Runtime.trigger_keys"></span>`trigger_keys`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn trigger_keys (& self) -> Vec < TenantKey >
```

Every `(tenant, trigger)` key in the process, unfiltered.

<details>
<summary>Source</summary>

```rust
    pub fn trigger_keys(&self) -> Vec<TenantKey> {
        self.inner.triggers.all_keys()
    }
```

</details>



##### <span id="Runtime.register_computation_graph"></span>`register_computation_graph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn register_computation_graph < F > (& self , name : String , constructor : F) where F : Fn () -> ComputationGraphRegistration + Send + Sync + 'static ,
```

Register a computation graph constructor by graph name, under this handle's tenant.

<details>
<summary>Source</summary>

```rust
    pub fn register_computation_graph<F>(&self, name: String, constructor: F)
    where
        F: Fn() -> ComputationGraphRegistration + Send + Sync + 'static,
    {
        let _ = self.inner.computation_graphs.insert(
            self.scope(),
            name,
            TenantOwner::unknown(),
            Box::new(constructor),
        );
    }
```

</details>



##### <span id="Runtime.try_register_computation_graph"></span>`try_register_computation_graph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn try_register_computation_graph < F > (& self , owner : & TenantOwner , name : String , constructor : F ,) -> Result < () , RuntimeRegistrationError > where F : Fn () -> ComputationGraphRegistration + Send + Sync + 'static ,
```

Register a computation graph attributed to `owner`, rejecting a same-tenant claim on a name a *different* package already owns.

<details>
<summary>Source</summary>

```rust
    pub fn try_register_computation_graph<F>(
        &self,
        owner: &TenantOwner,
        name: String,
        constructor: F,
    ) -> Result<(), RuntimeRegistrationError>
    where
        F: Fn() -> ComputationGraphRegistration + Send + Sync + 'static,
    {
        self.inner.computation_graphs.insert(
            self.scope(),
            name,
            owner.clone(),
            Box::new(constructor),
        )
    }
```

</details>



##### <span id="Runtime.unregister_computation_graph"></span>`unregister_computation_graph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn unregister_computation_graph (& self , name : & str) -> bool
```

Remove a computation graph constructor. Returns true if the entry existed.

<details>
<summary>Source</summary>

```rust
    pub fn unregister_computation_graph(&self, name: &str) -> bool {
        self.inner.computation_graphs.remove(self.scope(), name)
    }
```

</details>



##### <span id="Runtime.get_computation_graph"></span>`get_computation_graph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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
            .with(self.scope(), name, |ctor| ctor())
    }
```

</details>



##### <span id="Runtime.computation_graph_names"></span>`computation_graph_names`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn computation_graph_names (& self) -> Vec < String >
```

Get every computation graph name visible to this handle's scope.

<details>
<summary>Source</summary>

```rust
    pub fn computation_graph_names(&self) -> Vec<String> {
        self.inner.computation_graphs.names(self.scope())
    }
```

</details>



##### <span id="Runtime.computation_graph_keys"></span>`computation_graph_keys`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn computation_graph_keys (& self) -> Vec < TenantKey >
```

Every `(tenant, graph)` key in the process, unfiltered.

<details>
<summary>Source</summary>

```rust
    pub fn computation_graph_keys(&self) -> Vec<TenantKey> {
        self.inner.computation_graphs.all_keys()
    }
```

</details>



##### <span id="Runtime.register_triggerless_graph"></span>`register_triggerless_graph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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
        let _ = self.inner.triggerless_graphs.insert(
            self.scope(),
            name,
            TenantOwner::unknown(),
            Box::new(constructor),
        );
    }
```

</details>



##### <span id="Runtime.try_register_triggerless_graph"></span>`try_register_triggerless_graph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn try_register_triggerless_graph < F > (& self , owner : & TenantOwner , name : String , constructor : F ,) -> Result < () , RuntimeRegistrationError > where F : Fn () -> TriggerlessGraphRegistration + Send + Sync + 'static ,
```

Register a trigger-less graph attributed to `owner`, rejecting a same-tenant claim on a name a *different* package already owns.

<details>
<summary>Source</summary>

```rust
    pub fn try_register_triggerless_graph<F>(
        &self,
        owner: &TenantOwner,
        name: String,
        constructor: F,
    ) -> Result<(), RuntimeRegistrationError>
    where
        F: Fn() -> TriggerlessGraphRegistration + Send + Sync + 'static,
    {
        self.inner.triggerless_graphs.insert(
            self.scope(),
            name,
            owner.clone(),
            Box::new(constructor),
        )
    }
```

</details>



##### <span id="Runtime.may_claim_triggerless_graph"></span>`may_claim_triggerless_graph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn may_claim_triggerless_graph (& self , owner : & TenantOwner , name : & str ,) -> Result < bool , RuntimeRegistrationError >
```

Ask whether `owner` may claim the trigger-less graph `name` in this handle's scope before registering it. See [`Runtime::may_claim_trigger`] for the three outcomes.

<details>
<summary>Source</summary>

```rust
    pub fn may_claim_triggerless_graph(
        &self,
        owner: &TenantOwner,
        name: &str,
    ) -> Result<bool, RuntimeRegistrationError> {
        self.inner
            .triggerless_graphs
            .check_claim(self.scope(), name, owner)
    }
```

</details>



##### <span id="Runtime.unregister_triggerless_graph"></span>`unregister_triggerless_graph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn unregister_triggerless_graph (& self , name : & str) -> bool
```

Remove a trigger-less graph constructor. Returns true if the entry existed.

<details>
<summary>Source</summary>

```rust
    pub fn unregister_triggerless_graph(&self, name: &str) -> bool {
        self.inner.triggerless_graphs.remove(self.scope(), name)
    }
```

</details>



##### <span id="Runtime.get_triggerless_graph"></span>`get_triggerless_graph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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
            .with(self.scope(), name, |ctor| ctor())
    }
```

</details>



##### <span id="Runtime.triggerless_graph_names"></span>`triggerless_graph_names`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn triggerless_graph_names (& self) -> Vec < String >
```

Get every trigger-less graph name visible to this handle's scope.

<details>
<summary>Source</summary>

```rust
    pub fn triggerless_graph_names(&self) -> Vec<String> {
        self.inner.triggerless_graphs.names(self.scope())
    }
```

</details>



##### <span id="Runtime.triggerless_graph_keys"></span>`triggerless_graph_keys`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn triggerless_graph_keys (& self) -> Vec < TenantKey >
```

Every `(tenant, trigger-less graph)` key in the process, unfiltered.

<details>
<summary>Source</summary>

```rust
    pub fn triggerless_graph_keys(&self) -> Vec<TenantKey> {
        self.inner.triggerless_graphs.all_keys()
    }
```

</details>



##### <span id="Runtime.register_reactor"></span>`register_reactor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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
        let _ = self.inner.reactors.insert(
            self.scope(),
            name,
            TenantOwner::unknown(),
            Box::new(constructor),
        );
    }
```

</details>



##### <span id="Runtime.try_register_reactor"></span>`try_register_reactor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn try_register_reactor < F > (& self , owner : & TenantOwner , name : String , constructor : F ,) -> Result < () , RuntimeRegistrationError > where F : Fn () -> ReactorRegistration + Send + Sync + 'static ,
```

Register a reactor attributed to `owner`, rejecting a same-tenant claim on a name a *different* package already owns.

<details>
<summary>Source</summary>

```rust
    pub fn try_register_reactor<F>(
        &self,
        owner: &TenantOwner,
        name: String,
        constructor: F,
    ) -> Result<(), RuntimeRegistrationError>
    where
        F: Fn() -> ReactorRegistration + Send + Sync + 'static,
    {
        self.inner
            .reactors
            .insert(self.scope(), name, owner.clone(), Box::new(constructor))
    }
```

</details>



##### <span id="Runtime.unregister_reactor"></span>`unregister_reactor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn unregister_reactor (& self , name : & str) -> bool
```

Remove a reactor constructor. Returns true if the entry existed.

<details>
<summary>Source</summary>

```rust
    pub fn unregister_reactor(&self, name: &str) -> bool {
        self.inner.reactors.remove(self.scope(), name)
    }
```

</details>



##### <span id="Runtime.get_reactor"></span>`get_reactor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn get_reactor (& self , name : & str) -> Option < ReactorRegistration >
```

Look up and instantiate a reactor registration by name.

<details>
<summary>Source</summary>

```rust
    pub fn get_reactor(&self, name: &str) -> Option<ReactorRegistration> {
        self.inner.reactors.with(self.scope(), name, |ctor| ctor())
    }
```

</details>



##### <span id="Runtime.reactor_names"></span>`reactor_names`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn reactor_names (& self) -> Vec < String >
```

Get every reactor name visible to this handle's scope.

<details>
<summary>Source</summary>

```rust
    pub fn reactor_names(&self) -> Vec<String> {
        self.inner.reactors.names(self.scope())
    }
```

</details>



##### <span id="Runtime.reactor_keys"></span>`reactor_keys`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn reactor_keys (& self) -> Vec < TenantKey >
```

Every `(tenant, reactor)` key in the process, unfiltered.

<details>
<summary>Source</summary>

```rust
    pub fn reactor_keys(&self) -> Vec<TenantKey> {
        self.inner.reactors.all_keys()
    }
```

</details>



##### <span id="Runtime.register_stream_backend"></span>`register_stream_backend`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="Runtime.unregister_stream_backend"></span>`unregister_stream_backend`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="Runtime.has_stream_backend"></span>`has_stream_backend`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">pub(crate)</span>


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



##### <span id="Runtime.create_stream_backend"></span>`create_stream_backend`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="Runtime.stream_backend_names"></span>`stream_backend_names`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">pub(crate)</span>


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





### <span id="struct.RuntimeScope"></span>`struct RuntimeScope`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


**Derives:** `Clone`, `Debug`, `Default`, `PartialEq`, `Eq`

The owned form of [`TenantScope`] carried on a [`Runtime`] handle.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `Option < String >` |  |
| `is_admin` | `bool` |  |

#### Methods

##### <span id="RuntimeScope.as_tenant_scope"></span>`as_tenant_scope`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn as_tenant_scope (& self) -> TenantScope < '_ >
```

<details>
<summary>Source</summary>

```rust
    fn as_tenant_scope(&self) -> TenantScope<'_> {
        TenantScope {
            tenant_id: self.tenant_id.as_deref(),
            is_admin: self.is_admin,
        }
    }
```

</details>





### <span id="struct.RuntimeInner"></span>`struct RuntimeInner`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tasks` | `RwLock < HashMap < TaskNamespace , TaskConstructorFn > >` | Already `tenant::package::workflow::task`-keyed — see the module docs
for why this one keeps [`TaskNamespace`] rather than [`TenantKey`]. |
| `workflows` | `ScopedRegistry < WorkflowConstructorFn >` |  |
| `triggers` | `ScopedRegistry < TriggerConstructorFn >` |  |
| `computation_graphs` | `ScopedRegistry < ComputationGraphConstructor >` |  |
| `triggerless_graphs` | `ScopedRegistry < TriggerlessGraphConstructor >` |  |
| `reactors` | `ScopedRegistry < ReactorConstructor >` |  |
| `stream_backends` | `RwLock < HashMap < String , StreamBackendFactory > >` | Keyed by backend *kind* (`"kafka"`, `"mock"`), not by a tenant-authored
entity name — CLOACI-T-0921's audit classified this as collision-safe by
construction, so it stays a plain name map. |



## Enums

### <span id="enum.RuntimeRegistrationError"></span>`enum RuntimeRegistrationError`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


A registration was refused because the name is already claimed inside the same tenant by a different package (CLOACI-T-0924).

This mirrors `RegistryError::EndpointOwnershipConflict` from
CLOACI-T-0921: silently replacing the incumbent would cross-wire two
packages' entities, so the second package must rename.

#### Variants

- **`OwnershipConflict`**



