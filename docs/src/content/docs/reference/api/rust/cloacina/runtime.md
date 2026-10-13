---
title: "cloacina::runtime"
description: "Scoped runtime unifying all cloacina registries."
editUrl: false
sidebar:
  order: 186
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Scoped runtime unifying all cloacina registries.

[`Runtime`](../../../rust/cloacina/runtime/#struct.Runtime) owns the registries for tasks, workflows, triggers, computation
graphs, and stream backends. Every entry can be registered and unregistered
at runtime, which is the mechanism the reconciler uses to hot-swap packages.

The process-global static registries that predated `Runtime` were deleted
in CLOACI-T-0509. [`Runtime::new`](../../../rust/cloacina/runtime/#Runtime.new) seeds itself from the `inventory` entries
emitted by the macros; the reconciler and Python bindings push into it
directly via [`Runtime::register_task`](../../../rust/cloacina/runtime/#Runtime.register_task), [`Runtime::register_workflow`](../../../rust/cloacina/runtime/#Runtime.register_workflow), etc.

```rust
use cloacina::Runtime;

let runtime = Runtime::new(); // seeded from inventory
runtime.register_task(namespace, || Arc::new(my_task()));
runtime.unregister_workflow("obsolete_workflow");
```

**Tenant scoping (CLOACI-T-0924)**

One `Runtime` allocation is shared by every tenant's runner in
`cloacina-server` (`TenantRunnerCache::shared_runtime`), so the name-keyed
registries below carry the tenant **in the key**: they are
[`TenantKey`](../../../rust/cloacina/tenant_scope/#struct.TenantKey)`(tenant, name)` maps, matching the `EndpointKey` convention
CLOACI-T-0921 established for the `EndpointRegistry`. (`tasks` stays keyed
by [`TaskNamespace`](../../../rust/cloacina-workflow/namespace/#struct.TaskNamespace) — `tenant::package::workflow::task` is persisted on
`task_executions` rows, so its readers genuinely have all four components.
Every reader of the other five registries addresses them by *bare name*.)

The tenant is not threaded through every `get_workflow(&str)` call site.
Instead the **handle** carries it: `Runtime` is a cheap `Arc` share plus a
scope, and [`Runtime::scoped_to_tenant`](../../../rust/cloacina/runtime/#Runtime.scoped_to_tenant) / [`Runtime::untenanted_view`](../../../rust/cloacina/runtime/#Runtime.untenanted_view) /
[`Runtime::admin_view`](../../../rust/cloacina/runtime/#Runtime.admin_view) produce differently-scoped views over the *same*
registries. `DefaultRunner` binds its handle to `config.tenant_id()` at
construction, so the scheduler, executor, reconciler and cron scheduler it
builds all inherit it.

Resolution order is CLOACI-T-0921's, verbatim: the caller's own tenant, then
the untenanted entry (embedded / pre-multi-tenancy, and what
[`Runtime::seed_from_inventory`](../../../rust/cloacina/runtime/#Runtime.seed_from_inventory) always writes), then — for admin views only
— a *unique* cross-tenant match. Two tenants owning a name is never a guess.
An untenanted deployment therefore behaves exactly as it did before this
change: every entry is untenanted and every lookup hits step 2.

## Structs

### <span id="struct.Runtime"></span><code style="font-size: 0.85em">struct Runtime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

A scoped runtime holding the registries for every cloacina extension point.

All five namespaces — tasks, workflows, triggers, computation graphs, and
stream backends — are registered and unregistered through the same surface.
`Runtime` is cheap to clone: it shares its registries via `Arc`.

A clone carries the same tenant scope; use [`Runtime::scoped_to_tenant`](../../../rust/cloacina/runtime/#Runtime.scoped_to_tenant),
[`Runtime::untenanted_view`](../../../rust/cloacina/runtime/#Runtime.untenanted_view) or [`Runtime::admin_view`](../../../rust/cloacina/runtime/#Runtime.admin_view) to get a
differently-scoped view over the same registries.

#### Methods

##### <span id="Runtime.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new() -> Self
```

Create a runtime seeded with every macro-registered entry from the `inventory` crate (tasks, workflows, triggers, computation graphs, stream backends).

`inventory` collects entries in a linker section and is read lazily
after `main()`, so every entry registered by the `#[task]`,
`#[workflow]`, `#[trigger]`, `#[computation_graph]`, and stream-backend
macros in the current binary is visible here. For a blank-slate runtime
(used by isolation-sensitive tests), use [`Runtime::empty`](../../../rust/cloacina/runtime/#Runtime.empty) instead.

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

##### <span id="Runtime.empty"></span><code>empty</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn empty() -> Self
```

Create an empty runtime with no registered entries in any namespace.

Use this when you want complete isolation — no macro-registered tasks,
workflows, triggers, CGs, or stream backends are installed. Intended
for unit tests; production code should generally use [`Runtime::new`](../../../rust/cloacina/runtime/#Runtime.new).

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

##### <span id="Runtime.scoped_to_tenant"></span><code>scoped_​to_​tenant</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn scoped_to_tenant(&self, tenant_id: impl Into<String>) -> Self
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

##### <span id="Runtime.untenanted_view"></span><code>untenanted_​view</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn untenanted_view(&self) -> Self
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

##### <span id="Runtime.admin_view"></span><code>admin_​view</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn admin_view(&self) -> Self
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

##### <span id="Runtime.tenant_id"></span><code>tenant_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn tenant_id(&self) -> Option<&str>
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

##### <span id="Runtime.is_admin"></span><code>is_​admin</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_admin(&self) -> bool
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

##### <span id="Runtime.shares_registries_with"></span><code>shares_​registries_​with</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn shares_registries_with(&self, other: &Runtime) -> bool
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

##### <span id="Runtime.seed_from_inventory"></span><code>seed_​from_​inventory</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn seed_from_inventory(&self)
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

##### <span id="Runtime.register_task"></span><code>register_​task</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn register_task<F>(&self, namespace: TaskNamespace, factory: F)
where
    F: Fn() -> Arc<dyn Task> + Send + Sync + 'static,
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

##### <span id="Runtime.unregister_task"></span><code>unregister_​task</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn unregister_task(&self, namespace: &TaskNamespace) -> bool
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

##### <span id="Runtime.get_task"></span><code>get_​task</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_task(&self, namespace: &TaskNamespace) -> Option<Arc<dyn Task>>
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

##### <span id="Runtime.task_namespaces"></span><code>task_​namespaces</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn task_namespaces(&self) -> Vec<TaskNamespace>
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

##### <span id="Runtime.register_workflow"></span><code>register_​workflow</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn register_workflow<F>(&self, name: String, constructor: F)
where
    F: Fn() -> Workflow + Send + Sync + 'static,
```

Register a workflow constructor by name, under this handle's tenant.

Unattributed registration: it never conflicts, so it cannot fail. Use
[`Runtime::try_register_workflow`](../../../rust/cloacina/runtime/#Runtime.try_register_workflow) from the package loader, where the
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

##### <span id="Runtime.try_register_workflow"></span><code>try_​register_​workflow</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn try_register_workflow<F>(
    &self,
    owner: &TenantOwner,
    name: String,
    constructor: F,
) -> Result<(), RuntimeRegistrationError>
where
    F: Fn() -> Workflow + Send + Sync + 'static,
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

##### <span id="Runtime.unregister_workflow"></span><code>unregister_​workflow</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn unregister_workflow(&self, name: &str) -> bool
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

##### <span id="Runtime.get_workflow"></span><code>get_​workflow</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_workflow(&self, name: &str) -> Option<Workflow>
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

##### <span id="Runtime.workflow_names"></span><code>workflow_​names</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn workflow_names(&self) -> Vec<String>
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

##### <span id="Runtime.workflow_keys"></span><code>workflow_​keys</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn workflow_keys(&self) -> Vec<TenantKey>
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

##### <span id="Runtime.register_trigger"></span><code>register_​trigger</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn register_trigger<F>(&self, name: String, factory: F)
where
    F: Fn() -> Arc<dyn Trigger> + Send + Sync + 'static,
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

##### <span id="Runtime.try_register_trigger"></span><code>try_​register_​trigger</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn try_register_trigger<F>(
    &self,
    owner: &TenantOwner,
    name: String,
    factory: F,
) -> Result<(), RuntimeRegistrationError>
where
    F: Fn() -> Arc<dyn Trigger> + Send + Sync + 'static,
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

##### <span id="Runtime.may_claim_trigger"></span><code>may_​claim_​trigger</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn may_claim_trigger(
    &self,
    owner: &TenantOwner,
    name: &str,
) -> Result<bool, RuntimeRegistrationError>
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

##### <span id="Runtime.unregister_trigger"></span><code>unregister_​trigger</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn unregister_trigger(&self, name: &str) -> bool
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

##### <span id="Runtime.get_trigger"></span><code>get_​trigger</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_trigger(&self, name: &str) -> Option<Arc<dyn Trigger>>
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

##### <span id="Runtime.trigger_names"></span><code>trigger_​names</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn trigger_names(&self) -> Vec<String>
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

##### <span id="Runtime.trigger_keys"></span><code>trigger_​keys</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn trigger_keys(&self) -> Vec<TenantKey>
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

##### <span id="Runtime.register_computation_graph"></span><code>register_​computation_​graph</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn register_computation_graph<F>(&self, name: String, constructor: F)
where
    F: Fn() -> ComputationGraphRegistration + Send + Sync + 'static,
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

##### <span id="Runtime.try_register_computation_graph"></span><code>try_​register_​computation_​graph</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn try_register_computation_graph<F>(
    &self,
    owner: &TenantOwner,
    name: String,
    constructor: F,
) -> Result<(), RuntimeRegistrationError>
where
    F: Fn() -> ComputationGraphRegistration + Send + Sync + 'static,
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

##### <span id="Runtime.unregister_computation_graph"></span><code>unregister_​computation_​graph</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn unregister_computation_graph(&self, name: &str) -> bool
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

##### <span id="Runtime.get_computation_graph"></span><code>get_​computation_​graph</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_computation_graph(
    &self,
    name: &str,
) -> Option<ComputationGraphRegistration>
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

##### <span id="Runtime.computation_graph_names"></span><code>computation_​graph_​names</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn computation_graph_names(&self) -> Vec<String>
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

##### <span id="Runtime.computation_graph_keys"></span><code>computation_​graph_​keys</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn computation_graph_keys(&self) -> Vec<TenantKey>
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

##### <span id="Runtime.register_triggerless_graph"></span><code>register_​triggerless_​graph</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn register_triggerless_graph<F>(&self, name: String, constructor: F)
where
    F: Fn() -> TriggerlessGraphRegistration + Send + Sync + 'static,
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

##### <span id="Runtime.try_register_triggerless_graph"></span><code>try_​register_​triggerless_​graph</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn try_register_triggerless_graph<F>(
    &self,
    owner: &TenantOwner,
    name: String,
    constructor: F,
) -> Result<(), RuntimeRegistrationError>
where
    F: Fn() -> TriggerlessGraphRegistration + Send + Sync + 'static,
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

##### <span id="Runtime.may_claim_triggerless_graph"></span><code>may_​claim_​triggerless_​graph</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn may_claim_triggerless_graph(
    &self,
    owner: &TenantOwner,
    name: &str,
) -> Result<bool, RuntimeRegistrationError>
```

Ask whether `owner` may claim the trigger-less graph `name` in this handle's scope before registering it. See [`Runtime::may_claim_trigger`](../../../rust/cloacina/runtime/#Runtime.may_claim_trigger) for the three outcomes.

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

##### <span id="Runtime.unregister_triggerless_graph"></span><code>unregister_​triggerless_​graph</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn unregister_triggerless_graph(&self, name: &str) -> bool
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

##### <span id="Runtime.get_triggerless_graph"></span><code>get_​triggerless_​graph</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_triggerless_graph(
    &self,
    name: &str,
) -> Option<TriggerlessGraphRegistration>
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

##### <span id="Runtime.triggerless_graph_names"></span><code>triggerless_​graph_​names</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn triggerless_graph_names(&self) -> Vec<String>
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

##### <span id="Runtime.triggerless_graph_keys"></span><code>triggerless_​graph_​keys</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn triggerless_graph_keys(&self) -> Vec<TenantKey>
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

##### <span id="Runtime.register_reactor"></span><code>register_​reactor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn register_reactor<F>(&self, name: String, constructor: F)
where
    F: Fn() -> ReactorRegistration + Send + Sync + 'static,
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

##### <span id="Runtime.try_register_reactor"></span><code>try_​register_​reactor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn try_register_reactor<F>(
    &self,
    owner: &TenantOwner,
    name: String,
    constructor: F,
) -> Result<(), RuntimeRegistrationError>
where
    F: Fn() -> ReactorRegistration + Send + Sync + 'static,
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

##### <span id="Runtime.unregister_reactor"></span><code>unregister_​reactor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn unregister_reactor(&self, name: &str) -> bool
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

##### <span id="Runtime.get_reactor"></span><code>get_​reactor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_reactor(&self, name: &str) -> Option<ReactorRegistration>
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

##### <span id="Runtime.reactor_names"></span><code>reactor_​names</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn reactor_names(&self) -> Vec<String>
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

##### <span id="Runtime.reactor_keys"></span><code>reactor_​keys</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn reactor_keys(&self) -> Vec<TenantKey>
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

##### <span id="Runtime.register_stream_backend"></span><code>register_​stream_​backend</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn register_stream_backend(
    &self,
    type_name: String,
    factory: StreamBackendFactory,
)
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

##### <span id="Runtime.unregister_stream_backend"></span><code>unregister_​stream_​backend</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn unregister_stream_backend(&self, type_name: &str) -> bool
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

##### <span id="Runtime.create_stream_backend"></span><code>create_​stream_​backend</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn create_stream_backend(
    &self,
    type_name: &str,
    config: StreamConfig,
) -> Option<StreamBackendFuture>
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

## Enums

### <span id="enum.RuntimeRegistrationError"></span><code style="font-size: 0.85em">enum RuntimeRegistrationError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

A registration was refused because the name is already claimed inside the same tenant by a different package (CLOACI-T-0924).

This mirrors `RegistryError::EndpointOwnershipConflict` from
CLOACI-T-0921: silently replacing the incumbent would cross-wire two
packages' entities, so the second package must rename.

#### Variants

- **`OwnershipConflict`**
