/*
 *  Copyright 2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Scoped runtime unifying all cloacina registries.
//!
//! [`Runtime`] owns the registries for tasks, workflows, triggers, computation
//! graphs, and stream backends. Every entry can be registered and unregistered
//! at runtime, which is the mechanism the reconciler uses to hot-swap packages.
//!
//! The process-global static registries that predated `Runtime` were deleted
//! in CLOACI-T-0509. [`Runtime::new`] seeds itself from the `inventory` entries
//! emitted by the macros; the reconciler and Python bindings push into it
//! directly via [`Runtime::register_task`], [`Runtime::register_workflow`], etc.
//!
//! ```rust,ignore
//! use cloacina::Runtime;
//!
//! let runtime = Runtime::new(); // seeded from inventory
//! runtime.register_task(namespace, || Arc::new(my_task()));
//! runtime.unregister_workflow("obsolete_workflow");
//! ```
//!
//! # Tenant scoping (CLOACI-T-0924)
//!
//! One `Runtime` allocation is shared by every tenant's runner in
//! `cloacina-server` (`TenantRunnerCache::shared_runtime`), so the name-keyed
//! registries below carry the tenant **in the key**: they are
//! [`TenantKey`]`(tenant, name)` maps, matching the `EndpointKey` convention
//! CLOACI-T-0921 established for the `EndpointRegistry`. (`tasks` stays keyed
//! by [`TaskNamespace`] — `tenant::package::workflow::task` is persisted on
//! `task_executions` rows, so its readers genuinely have all four components.
//! Every reader of the other five registries addresses them by *bare name*.)
//!
//! The tenant is not threaded through every `get_workflow(&str)` call site.
//! Instead the **handle** carries it: `Runtime` is a cheap `Arc` share plus a
//! scope, and [`Runtime::scoped_to_tenant`] / [`Runtime::untenanted_view`] /
//! [`Runtime::admin_view`] produce differently-scoped views over the *same*
//! registries. `DefaultRunner` binds its handle to `config.tenant_id()` at
//! construction, so the scheduler, executor, reconciler and cron scheduler it
//! builds all inherit it.
//!
//! Resolution order is CLOACI-T-0921's, verbatim: the caller's own tenant, then
//! the untenanted entry (embedded / pre-multi-tenancy, and what
//! [`Runtime::seed_from_inventory`] always writes), then — for admin views only
//! — a *unique* cross-tenant match. Two tenants owning a name is never a guess.
//! An untenanted deployment therefore behaves exactly as it did before this
//! change: every entry is untenanted and every lookup hits step 2.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use parking_lot::RwLock;

use crate::computation_graph::stream_backend::{
    StreamBackendFactory, StreamBackendFuture, StreamConfig,
};
use crate::computation_graph::triggerless::TriggerlessGraphRegistration;
use crate::task::{Task, TaskNamespace};
use crate::tenant_scope::{resolve_tenant_key, visible_keys, TenantKey, TenantOwner, TenantScope};
use crate::trigger::Trigger;
use crate::workflow::Workflow;
use cloacina_computation_graph::{
    ComputationGraphConstructor, ComputationGraphRegistration, ReactorConstructor,
    ReactorRegistration,
};

/// Type alias for trigger-less graph constructor functions.
pub(crate) type TriggerlessGraphConstructor =
    Box<dyn Fn() -> TriggerlessGraphRegistration + Send + Sync>;

/// Type alias for task constructor functions.
pub(crate) type TaskConstructorFn = Box<dyn Fn() -> Arc<dyn Task> + Send + Sync>;

/// Type alias for workflow constructor functions.
pub(crate) type WorkflowConstructorFn = Box<dyn Fn() -> Workflow + Send + Sync>;

/// Type alias for trigger constructor functions.
pub(crate) type TriggerConstructorFn = Box<dyn Fn() -> Arc<dyn Trigger> + Send + Sync>;

/// A registration was refused because the name is already claimed inside the
/// same tenant by a different package (CLOACI-T-0924).
///
/// This mirrors `RegistryError::EndpointOwnershipConflict` from
/// CLOACI-T-0921: silently replacing the incumbent would cross-wire two
/// packages' entities, so the second package must rename.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimeRegistrationError {
    #[error(
        "{kind} '{name}' is already registered in tenant '{tenant}' by {existing}; \
         {incoming} cannot claim the same name — rename the {kind} in one of the two packages"
    )]
    OwnershipConflict {
        kind: &'static str,
        name: String,
        tenant: String,
        existing: String,
        incoming: String,
    },
}

/// One entry in a [`ScopedRegistry`]: the constructor plus who claimed it.
struct ScopedEntry<V> {
    owner: TenantOwner,
    value: V,
}

/// A `(tenant, name)`-keyed registry with owner-checked replacement.
///
/// Shared by the five bare-name registries so they have exactly one keying and
/// one resolution rule between them.
struct ScopedRegistry<V> {
    /// Operator-facing noun used in conflict errors ("workflow", "reactor", …).
    kind: &'static str,
    entries: RwLock<HashMap<TenantKey, ScopedEntry<V>>>,
}

impl<V> ScopedRegistry<V> {
    fn new(kind: &'static str) -> Self {
        Self {
            kind,
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Insert under `scope`'s own key, rejecting a claim on a live
    /// `(tenant, name)` held by a *different, named* package.
    fn insert(
        &self,
        scope: TenantScope<'_>,
        name: String,
        owner: TenantOwner,
        value: V,
    ) -> Result<(), RuntimeRegistrationError> {
        let key = scope.own_key(&name);
        let mut guard = self.entries.write();
        if let Some(existing) = guard.get(&key) {
            if !existing.owner.may_replace(&owner) {
                return Err(RuntimeRegistrationError::OwnershipConflict {
                    kind: self.kind,
                    name,
                    tenant: key.tenant_label().to_string(),
                    existing: existing.owner.label(),
                    incoming: owner.label(),
                });
            }
        }
        guard.insert(key, ScopedEntry { owner, value });
        Ok(())
    }

    /// Resolve `name` within `scope` and apply `f` to the stored value while
    /// the read lock is held (the values are constructor closures, so this is
    /// "call the constructor" at every call site).
    fn with<R>(&self, scope: TenantScope<'_>, name: &str, f: impl FnOnce(&V) -> R) -> Option<R> {
        let guard = self.entries.read();
        let key = resolve_tenant_key(&*guard, scope, name).ok()?;
        guard.get(&key).map(|entry| f(&entry.value))
    }

    /// Remove the entry `name` resolves to within `scope`. A non-admin caller
    /// can therefore never unregister another tenant's entry.
    fn remove(&self, scope: TenantScope<'_>, name: &str) -> bool {
        let mut guard = self.entries.write();
        let Ok(key) = resolve_tenant_key(&*guard, scope, name) else {
            return false;
        };
        guard.remove(&key).is_some()
    }

    /// Distinct entry names visible to `scope` (own tenant + untenanted, or
    /// everything for an admin view). Deduplicated, because a tenant entry and
    /// an untenanted entry can share a name.
    fn names(&self, scope: TenantScope<'_>) -> Vec<String> {
        let guard = self.entries.read();
        let mut seen = HashSet::new();
        visible_keys(&*guard, scope)
            .filter(|k| seen.insert(k.name.clone()))
            .map(|k| k.name.clone())
            .collect()
    }

    /// Every `(tenant, name)` key in the registry, unfiltered. Diagnostics and
    /// tests only.
    fn all_keys(&self) -> Vec<TenantKey> {
        self.entries.read().keys().cloned().collect()
    }

    /// Provenance of the entry `name` resolves to within `scope`, if any.
    fn owner(&self, scope: TenantScope<'_>, name: &str) -> Option<TenantOwner> {
        let guard = self.entries.read();
        let key = resolve_tenant_key(&*guard, scope, name).ok()?;
        guard.get(&key).map(|entry| entry.owner.clone())
    }

    /// Build the conflict error for an incoming claim on `name`.
    fn conflict(
        &self,
        scope: TenantScope<'_>,
        name: &str,
        existing: &TenantOwner,
        incoming: &TenantOwner,
    ) -> RuntimeRegistrationError {
        RuntimeRegistrationError::OwnershipConflict {
            kind: self.kind,
            name: name.to_string(),
            tenant: scope.own_key(name).tenant_label().to_string(),
            existing: existing.label(),
            incoming: incoming.label(),
        }
    }

    /// Whether `incoming` may claim `name` in `scope`: `Ok(true)` when the
    /// name is free, `Ok(false)` when an entry `incoming` is allowed to reuse
    /// or replace already exists, `Err` when a different package owns it.
    fn check_claim(
        &self,
        scope: TenantScope<'_>,
        name: &str,
        incoming: &TenantOwner,
    ) -> Result<bool, RuntimeRegistrationError> {
        match self.owner(scope, name) {
            None => Ok(true),
            Some(existing) if existing.may_replace(incoming) => Ok(false),
            Some(existing) => Err(self.conflict(scope, name, &existing, incoming)),
        }
    }

    fn len(&self) -> usize {
        self.entries.read().len()
    }
}

/// A scoped runtime holding the registries for every cloacina extension point.
///
/// All five namespaces — tasks, workflows, triggers, computation graphs, and
/// stream backends — are registered and unregistered through the same surface.
/// `Runtime` is cheap to clone: it shares its registries via `Arc`.
///
/// A clone carries the same tenant scope; use [`Runtime::scoped_to_tenant`],
/// [`Runtime::untenanted_view`] or [`Runtime::admin_view`] to get a
/// differently-scoped view over the same registries.
#[derive(Clone)]
pub struct Runtime {
    inner: Arc<RuntimeInner>,
    scope: RuntimeScope,
}

/// The owned form of [`TenantScope`] carried on a [`Runtime`] handle.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct RuntimeScope {
    tenant_id: Option<String>,
    is_admin: bool,
}

impl RuntimeScope {
    fn as_tenant_scope(&self) -> TenantScope<'_> {
        TenantScope {
            tenant_id: self.tenant_id.as_deref(),
            is_admin: self.is_admin,
        }
    }
}

struct RuntimeInner {
    /// Already `tenant::package::workflow::task`-keyed — see the module docs
    /// for why this one keeps [`TaskNamespace`] rather than [`TenantKey`].
    tasks: RwLock<HashMap<TaskNamespace, TaskConstructorFn>>,
    workflows: ScopedRegistry<WorkflowConstructorFn>,
    triggers: ScopedRegistry<TriggerConstructorFn>,
    computation_graphs: ScopedRegistry<ComputationGraphConstructor>,
    triggerless_graphs: ScopedRegistry<TriggerlessGraphConstructor>,
    reactors: ScopedRegistry<ReactorConstructor>,
    /// Keyed by backend *kind* (`"kafka"`, `"mock"`), not by a tenant-authored
    /// entity name — CLOACI-T-0921's audit classified this as collision-safe by
    /// construction, so it stays a plain name map.
    stream_backends: RwLock<HashMap<String, StreamBackendFactory>>,
}

impl Runtime {
    /// Create a runtime seeded with every macro-registered entry from the
    /// `inventory` crate (tasks, workflows, triggers, computation graphs,
    /// stream backends).
    ///
    /// `inventory` collects entries in a linker section and is read lazily
    /// after `main()`, so every entry registered by the `#[task]`,
    /// `#[workflow]`, `#[trigger]`, `#[computation_graph]`, and stream-backend
    /// macros in the current binary is visible here. For a blank-slate runtime
    /// (used by isolation-sensitive tests), use [`Runtime::empty`] instead.
    pub fn new() -> Self {
        let rt = Self::empty();
        rt.seed_from_inventory();
        rt
    }

    /// Create an empty runtime with no registered entries in any namespace.
    ///
    /// Use this when you want complete isolation — no macro-registered tasks,
    /// workflows, triggers, CGs, or stream backends are installed. Intended
    /// for unit tests; production code should generally use [`Runtime::new`].
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

    // -----------------------------------------------------------------------
    // Tenant scoping (CLOACI-T-0924)
    // -----------------------------------------------------------------------

    /// The scope this handle registers and resolves under.
    fn scope(&self) -> TenantScope<'_> {
        self.scope.as_tenant_scope()
    }

    /// A view over the *same* registries bound to `tenant_id`.
    ///
    /// Registrations made through the returned handle land under
    /// `(tenant_id, name)`; lookups try that tenant first, then fall back to
    /// the untenanted (inventory / embedded) entries. This is what
    /// `DefaultRunner` calls with `config.tenant_id()`.
    pub fn scoped_to_tenant(&self, tenant_id: impl Into<String>) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            scope: RuntimeScope {
                tenant_id: Some(tenant_id.into()),
                is_admin: false,
            },
        }
    }

    /// A view over the same registries with no tenant — the embedded /
    /// pre-multi-tenancy scope, and the scope `seed_from_inventory` writes
    /// under.
    pub fn untenanted_view(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            scope: RuntimeScope::default(),
        }
    }

    /// A view that may resolve any tenant's entry, but only when the name is
    /// unique across tenants. Intended for operator/diagnostic surfaces.
    pub fn admin_view(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            scope: RuntimeScope {
                tenant_id: None,
                is_admin: true,
            },
        }
    }

    /// The tenant this handle is bound to, if any.
    pub fn tenant_id(&self) -> Option<&str> {
        self.scope.tenant_id.as_deref()
    }

    /// Whether this handle is an admin view.
    pub fn is_admin(&self) -> bool {
        self.scope.is_admin
    }

    /// `true` when both handles are views over the same registry allocation.
    ///
    /// Replaces `Arc::ptr_eq` comparisons of `Arc<Runtime>`: two per-tenant
    /// runners share one `RuntimeInner` but hold differently-scoped handles.
    pub fn shares_registries_with(&self, other: &Runtime) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    /// Populate the runtime from the `inventory` entries emitted by the
    /// macros.
    ///
    /// `inventory`'s linker-section collection works across `dlopen`'d cdylibs
    /// on Linux/macOS, so the reconciler calls this again after loading a new
    /// workflow package to pick up the entries emitted by that cdylib.
    ///
    /// Inventory entries are always registered **untenanted**, whatever scope
    /// this handle carries (CLOACI-T-0924). They are host-binary, compile-time
    /// declarations — never tenant-authored — and the reconciler re-runs this
    /// after every `dlopen`, so scoping them would cross-stamp one tenant's
    /// cdylib entries onto whichever tenant happened to load next. Untenanted
    /// entries stay reachable from every tenant via the resolution fallback,
    /// which is exactly the pre-CLOACI-T-0924 behaviour.
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

    // -----------------------------------------------------------------------
    // Task registry
    // -----------------------------------------------------------------------

    /// Register a task constructor for the given namespace.
    pub fn register_task<F>(&self, namespace: TaskNamespace, factory: F)
    where
        F: Fn() -> Arc<dyn Task> + Send + Sync + 'static,
    {
        self.inner
            .tasks
            .write()
            .insert(namespace, Box::new(factory));
    }

    /// Remove a task constructor. Returns true if the entry existed.
    pub fn unregister_task(&self, namespace: &TaskNamespace) -> bool {
        self.inner.tasks.write().remove(namespace).is_some()
    }

    /// Look up and instantiate a task by namespace.
    pub fn get_task(&self, namespace: &TaskNamespace) -> Option<Arc<dyn Task>> {
        self.inner.tasks.read().get(namespace).map(|ctor| ctor())
    }

    /// Check if a task is registered for the given namespace.
    #[cfg(test)]
    pub(crate) fn has_task(&self, namespace: &TaskNamespace) -> bool {
        self.inner.tasks.read().contains_key(namespace)
    }

    /// Snapshot of every currently-registered task namespace. Used by code
    /// that needs to enumerate tasks (e.g. collecting all tasks belonging to
    /// a specific tenant/package/workflow triple during Python import).
    pub fn task_namespaces(&self) -> Vec<TaskNamespace> {
        self.inner.tasks.read().keys().cloned().collect()
    }

    // -----------------------------------------------------------------------
    // Workflow registry
    // -----------------------------------------------------------------------

    /// Register a workflow constructor by name, under this handle's tenant.
    ///
    /// Unattributed registration: it never conflicts, so it cannot fail. Use
    /// [`Runtime::try_register_workflow`] from the package loader, where the
    /// owning package is known and a same-tenant collision must be loud.
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

    /// Register a workflow constructor attributed to `owner`, rejecting a
    /// same-tenant claim on a name a *different* package already owns.
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

    /// Remove a workflow constructor. Returns true if the entry existed.
    pub fn unregister_workflow(&self, name: &str) -> bool {
        self.inner.workflows.remove(self.scope(), name)
    }

    /// Look up and instantiate a workflow by name, within this handle's scope.
    pub fn get_workflow(&self, name: &str) -> Option<Workflow> {
        self.inner.workflows.with(self.scope(), name, |ctor| ctor())
    }

    /// Get every workflow name visible to this handle's scope.
    pub fn workflow_names(&self) -> Vec<String> {
        self.inner.workflows.names(self.scope())
    }

    /// Every `(tenant, workflow)` key in the process, unfiltered. Diagnostics
    /// and cross-tenant isolation tests.
    pub fn workflow_keys(&self) -> Vec<TenantKey> {
        self.inner.workflows.all_keys()
    }

    // -----------------------------------------------------------------------
    // Trigger registry
    // -----------------------------------------------------------------------

    /// Register a trigger constructor by name, under this handle's tenant.
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

    /// Register a trigger constructor attributed to `owner`, rejecting a
    /// same-tenant claim on a name a *different* package already owns.
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

    /// Ask whether `owner` may claim the trigger `name` in this handle's scope
    /// *before* registering it (CLOACI-T-0924).
    ///
    /// The package loader has a "reuse whatever the cdylib's `inventory`
    /// already registered" fast path, which would otherwise let a second
    /// package silently adopt (and later tear down) a first package's trigger.
    /// `Ok(true)` — free, register it. `Ok(false)` — an entry this owner may
    /// reuse or replace exists. `Err` — a different package owns the name in
    /// this tenant.
    pub fn may_claim_trigger(
        &self,
        owner: &TenantOwner,
        name: &str,
    ) -> Result<bool, RuntimeRegistrationError> {
        self.inner.triggers.check_claim(self.scope(), name, owner)
    }

    /// Remove a trigger constructor. Returns true if the entry existed.
    pub fn unregister_trigger(&self, name: &str) -> bool {
        self.inner.triggers.remove(self.scope(), name)
    }

    /// Look up and instantiate a trigger by name, within this handle's scope.
    pub fn get_trigger(&self, name: &str) -> Option<Arc<dyn Trigger>> {
        self.inner.triggers.with(self.scope(), name, |ctor| ctor())
    }

    /// Get every trigger name visible to this handle's scope.
    pub fn trigger_names(&self) -> Vec<String> {
        self.inner.triggers.names(self.scope())
    }

    /// Every `(tenant, trigger)` key in the process, unfiltered.
    pub fn trigger_keys(&self) -> Vec<TenantKey> {
        self.inner.triggers.all_keys()
    }

    // -----------------------------------------------------------------------
    // Computation graph registry
    // -----------------------------------------------------------------------

    /// Register a computation graph constructor by graph name, under this
    /// handle's tenant.
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

    /// Register a computation graph attributed to `owner`, rejecting a
    /// same-tenant claim on a name a *different* package already owns.
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

    /// Remove a computation graph constructor. Returns true if the entry existed.
    pub fn unregister_computation_graph(&self, name: &str) -> bool {
        self.inner.computation_graphs.remove(self.scope(), name)
    }

    /// Look up and instantiate a computation graph registration by name.
    pub fn get_computation_graph(&self, name: &str) -> Option<ComputationGraphRegistration> {
        self.inner
            .computation_graphs
            .with(self.scope(), name, |ctor| ctor())
    }

    /// Get every computation graph name visible to this handle's scope.
    pub fn computation_graph_names(&self) -> Vec<String> {
        self.inner.computation_graphs.names(self.scope())
    }

    /// Every `(tenant, graph)` key in the process, unfiltered.
    pub fn computation_graph_keys(&self) -> Vec<TenantKey> {
        self.inner.computation_graphs.all_keys()
    }

    // -----------------------------------------------------------------------
    // Trigger-less computation graph registry
    // -----------------------------------------------------------------------

    /// Register a trigger-less computation graph constructor by graph name.
    ///
    /// Trigger-less graphs are declared with `#[computation_graph(graph =
    /// { ... })]` (no `trigger = reactor(...)` clause) and operate on a
    /// `Context<Value>`. They are invoked directly by workflow tasks
    /// (T-02) and Python decorators (T-03).
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

    /// Register a trigger-less graph attributed to `owner`, rejecting a
    /// same-tenant claim on a name a *different* package already owns.
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

    /// Ask whether `owner` may claim the trigger-less graph `name` in this
    /// handle's scope before registering it. See
    /// [`Runtime::may_claim_trigger`] for the three outcomes.
    pub fn may_claim_triggerless_graph(
        &self,
        owner: &TenantOwner,
        name: &str,
    ) -> Result<bool, RuntimeRegistrationError> {
        self.inner
            .triggerless_graphs
            .check_claim(self.scope(), name, owner)
    }

    /// Remove a trigger-less graph constructor. Returns true if the entry existed.
    pub fn unregister_triggerless_graph(&self, name: &str) -> bool {
        self.inner.triggerless_graphs.remove(self.scope(), name)
    }

    /// Look up and instantiate a trigger-less graph registration by name.
    pub fn get_triggerless_graph(&self, name: &str) -> Option<TriggerlessGraphRegistration> {
        self.inner
            .triggerless_graphs
            .with(self.scope(), name, |ctor| ctor())
    }

    /// Get every trigger-less graph name visible to this handle's scope.
    pub fn triggerless_graph_names(&self) -> Vec<String> {
        self.inner.triggerless_graphs.names(self.scope())
    }

    /// Every `(tenant, trigger-less graph)` key in the process, unfiltered.
    pub fn triggerless_graph_keys(&self) -> Vec<TenantKey> {
        self.inner.triggerless_graphs.all_keys()
    }

    // -----------------------------------------------------------------------
    // Reactor registry
    // -----------------------------------------------------------------------

    /// Register a reactor constructor by name.
    ///
    /// Reactors declared via `#[reactor]` or synthesized by the bundled form
    /// of `#[computation_graph]` land here. Graphs that declare
    /// `trigger = reactor(X)` bind to the named reactor at load time.
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

    /// Register a reactor attributed to `owner`, rejecting a same-tenant claim
    /// on a name a *different* package already owns.
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

    /// Remove a reactor constructor. Returns true if the entry existed.
    pub fn unregister_reactor(&self, name: &str) -> bool {
        self.inner.reactors.remove(self.scope(), name)
    }

    /// Look up and instantiate a reactor registration by name.
    pub fn get_reactor(&self, name: &str) -> Option<ReactorRegistration> {
        self.inner.reactors.with(self.scope(), name, |ctor| ctor())
    }

    /// Get every reactor name visible to this handle's scope.
    pub fn reactor_names(&self) -> Vec<String> {
        self.inner.reactors.names(self.scope())
    }

    /// Every `(tenant, reactor)` key in the process, unfiltered.
    pub fn reactor_keys(&self) -> Vec<TenantKey> {
        self.inner.reactors.all_keys()
    }

    // -----------------------------------------------------------------------
    // Stream backend registry
    // -----------------------------------------------------------------------

    /// Register a stream backend factory by type name (e.g. `"kafka"`, `"mock"`).
    pub fn register_stream_backend(&self, type_name: String, factory: StreamBackendFactory) {
        self.inner
            .stream_backends
            .write()
            .insert(type_name, factory);
    }

    /// Remove a stream backend factory. Returns true if the entry existed.
    pub fn unregister_stream_backend(&self, type_name: &str) -> bool {
        self.inner
            .stream_backends
            .write()
            .remove(type_name)
            .is_some()
    }

    /// Check if a stream backend is registered for the given type name.
    #[cfg(test)]
    pub(crate) fn has_stream_backend(&self, type_name: &str) -> bool {
        self.inner.stream_backends.read().contains_key(type_name)
    }

    /// Get the creation future for a stream backend without holding the lock
    /// across await. Returns `None` if the type is not registered.
    pub fn create_stream_backend(
        &self,
        type_name: &str,
        config: StreamConfig,
    ) -> Option<StreamBackendFuture> {
        let guard = self.inner.stream_backends.read();
        let factory = guard.get(type_name)?;
        Some(factory(config))
    }

    /// Get all registered stream backend type names.
    #[cfg(test)]
    pub(crate) fn stream_backend_names(&self) -> Vec<String> {
        self.inner.stream_backends.read().keys().cloned().collect()
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tasks = self.inner.tasks.read().len();
        let workflows = self.inner.workflows.len();
        let triggers = self.inner.triggers.len();
        let cgs = self.inner.computation_graphs.len();
        let sbs = self.inner.stream_backends.read().len();
        f.debug_struct("Runtime")
            .field("tenant", &self.scope.tenant_id)
            .field("is_admin", &self.scope.is_admin)
            .field("tasks", &tasks)
            .field("workflows", &workflows)
            .field("triggers", &triggers)
            .field("computation_graphs", &cgs)
            .field("stream_backends", &sbs)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskNamespace;

    #[test]
    fn register_and_unregister_workflow() {
        let rt = Runtime::empty();
        assert!(!rt.unregister_workflow("nope"));

        let wf = crate::workflow::Workflow::new("unit-test-wf");
        rt.register_workflow("unit-test-wf".to_string(), move || wf.clone());
        assert!(rt.get_workflow("unit-test-wf").is_some());
        assert_eq!(rt.workflow_names(), vec!["unit-test-wf".to_string()]);

        assert!(rt.unregister_workflow("unit-test-wf"));
        assert!(rt.get_workflow("unit-test-wf").is_none());
        assert!(rt.workflow_names().is_empty());
    }

    #[test]
    fn register_and_unregister_trigger_by_name() {
        // Triggers need a Trigger trait impl; skip full integration here and
        // cover the lifecycle via the workflow test. The shape of the API is
        // identical across namespaces.
        let rt = Runtime::empty();
        assert!(!rt.unregister_trigger("missing"));
        assert!(rt.get_trigger("missing").is_none());
        assert!(rt.trigger_names().is_empty());
    }

    #[test]
    fn register_and_unregister_task() {
        let rt = Runtime::empty();
        let ns = TaskNamespace::new("t", "p", "w", "task_a");
        assert!(!rt.unregister_task(&ns));
        assert!(!rt.has_task(&ns));
    }

    #[test]
    fn stream_backend_roundtrip_names_only() {
        let rt = Runtime::empty();
        assert!(!rt.has_stream_backend("mock"));
        assert!(rt.stream_backend_names().is_empty());
        assert!(!rt.unregister_stream_backend("mock"));
    }

    #[test]
    fn runtimes_are_independent() {
        let rt1 = Runtime::empty();
        let rt2 = Runtime::empty();
        let wf = crate::workflow::Workflow::new("iso");
        rt1.register_workflow("iso".to_string(), move || wf.clone());

        assert!(rt1.get_workflow("iso").is_some());
        assert!(rt2.get_workflow("iso").is_none());
    }

    #[test]
    fn debug_format_reports_sizes() {
        let rt = Runtime::empty();
        let debug = format!("{:?}", rt);
        assert!(debug.contains("computation_graphs: 0"));
        assert!(debug.contains("stream_backends: 0"));
    }

    // -----------------------------------------------------------------------
    // CLOACI-T-0924: tenant keying
    // -----------------------------------------------------------------------

    fn wf(name: &str) -> crate::workflow::Workflow {
        crate::workflow::Workflow::new(name)
    }

    /// Two tenants register the same workflow name on ONE shared runtime and
    /// each resolves its own — the collision T-0924 was filed for.
    #[test]
    fn two_tenants_same_workflow_name_do_not_collide() {
        let shared = Runtime::empty();
        let acme = shared.scoped_to_tenant("acme");
        let globex = shared.scoped_to_tenant("globex");
        assert!(acme.shares_registries_with(&globex));

        let a = wf("acme-only-desc");
        acme.register_workflow("pipeline".to_string(), move || a.clone());
        let g = wf("globex-only-desc");
        globex.register_workflow("pipeline".to_string(), move || g.clone());

        assert_eq!(
            acme.get_workflow("pipeline").unwrap().name(),
            "acme-only-desc"
        );
        assert_eq!(
            globex.get_workflow("pipeline").unwrap().name(),
            "globex-only-desc"
        );
        // Both entries are physically present under distinct keys.
        assert_eq!(shared.workflow_keys().len(), 2);
    }

    /// Unloading one tenant's entry leaves the other tenant's alive.
    #[test]
    fn unregister_is_tenant_scoped() {
        let shared = Runtime::empty();
        let acme = shared.scoped_to_tenant("acme");
        let globex = shared.scoped_to_tenant("globex");

        let a = wf("a");
        acme.register_workflow("pipeline".to_string(), move || a.clone());
        let g = wf("g");
        globex.register_workflow("pipeline".to_string(), move || g.clone());

        assert!(acme.unregister_workflow("pipeline"));
        assert!(acme.get_workflow("pipeline").is_none());
        assert_eq!(globex.get_workflow("pipeline").unwrap().name(), "g");
        assert_eq!(shared.workflow_keys().len(), 1);
    }

    /// A non-admin handle can never see, resolve, or drop another tenant's
    /// entry.
    #[test]
    fn other_tenants_entries_are_invisible() {
        let shared = Runtime::empty();
        let acme = shared.scoped_to_tenant("acme");
        let a = wf("a");
        acme.register_workflow("private".to_string(), move || a.clone());

        let globex = shared.scoped_to_tenant("globex");
        assert!(globex.get_workflow("private").is_none());
        assert!(globex.workflow_names().is_empty());
        assert!(!globex.unregister_workflow("private"));
        // …and the entry is untouched.
        assert!(acme.get_workflow("private").is_some());
    }

    /// Two packages inside the SAME tenant claiming one name is a loud error,
    /// not a silent overwrite; the incumbent survives. Re-registration by the
    /// same package replaces (the package-reload path depends on it).
    #[test]
    fn same_tenant_cross_package_collision_is_loud() {
        let rt = Runtime::empty().scoped_to_tenant("acme");
        let first = TenantOwner::package("pkg-a");
        let second = TenantOwner::package("pkg-b");

        let a = wf("from-a");
        rt.try_register_workflow(&first, "reports".to_string(), move || a.clone())
            .expect("first claim");

        let b = wf("from-b");
        let err = rt
            .try_register_workflow(&second, "reports".to_string(), move || b.clone())
            .expect_err("second package must not silently replace");
        let msg = err.to_string();
        assert!(msg.contains("pkg-a"), "{msg}");
        assert!(msg.contains("pkg-b"), "{msg}");
        assert!(msg.contains("acme"), "{msg}");
        assert_eq!(rt.get_workflow("reports").unwrap().name(), "from-a");

        // Same owner re-registering replaces.
        let a2 = wf("from-a-v2");
        rt.try_register_workflow(&first, "reports".to_string(), move || a2.clone())
            .expect("same package may replace");
        assert_eq!(rt.get_workflow("reports").unwrap().name(), "from-a-v2");
    }

    /// The same name in two *different* tenants is not a conflict even when
    /// both are owned by named packages.
    #[test]
    fn cross_tenant_same_name_is_not_a_conflict() {
        let shared = Runtime::empty();
        let owner = TenantOwner::package("pkg-a");
        for tenant in ["acme", "globex"] {
            let w = wf(tenant);
            shared
                .scoped_to_tenant(tenant)
                .try_register_workflow(&owner, "reports".to_string(), move || w.clone())
                .expect("distinct tenants never collide");
        }
        assert_eq!(shared.workflow_keys().len(), 2);
    }

    /// EMBEDDED COMPATIBILITY: an untenanted deployment is unchanged — every
    /// entry is untenanted and every lookup, listing and unregister behaves
    /// exactly as it did before tenant keying.
    #[test]
    fn untenanted_path_is_unchanged() {
        let rt = Runtime::empty();
        assert_eq!(rt.tenant_id(), None);
        let w = wf("embedded");
        rt.register_workflow("embedded".to_string(), move || w.clone());
        assert!(rt.get_workflow("embedded").is_some());
        assert_eq!(rt.workflow_names(), vec!["embedded".to_string()]);
        assert!(rt.unregister_workflow("embedded"));
        assert!(rt.get_workflow("embedded").is_none());
        assert!(rt.workflow_names().is_empty());
    }

    /// Untenanted (inventory / macro-seeded) entries stay reachable from every
    /// tenant view — that fallback is what keeps `seed_from_inventory` working
    /// on a per-tenant runner.
    #[test]
    fn untenanted_entries_are_visible_from_a_tenant_view() {
        let shared = Runtime::empty();
        let w = wf("inventory");
        shared.register_workflow("inventory".to_string(), move || w.clone());

        let acme = shared.scoped_to_tenant("acme");
        assert!(acme.get_workflow("inventory").is_some());
        assert_eq!(acme.workflow_names(), vec!["inventory".to_string()]);
    }

    /// The tenant's own entry shadows a same-named untenanted one.
    #[test]
    fn own_tenant_shadows_untenanted() {
        let shared = Runtime::empty();
        let global = wf("global");
        shared.register_workflow("dup".to_string(), move || global.clone());
        let acme = shared.scoped_to_tenant("acme");
        let own = wf("own");
        acme.register_workflow("dup".to_string(), move || own.clone());

        assert_eq!(acme.get_workflow("dup").unwrap().name(), "own");
        assert_eq!(shared.get_workflow("dup").unwrap().name(), "global");
        // Deduplicated in listings even though two keys carry the name.
        assert_eq!(acme.workflow_names(), vec!["dup".to_string()]);
    }

    /// An admin view resolves a unique cross-tenant name and refuses an
    /// ambiguous one rather than guessing.
    #[test]
    fn admin_view_resolves_unique_and_refuses_ambiguous() {
        let shared = Runtime::empty();
        let a = wf("a");
        shared
            .scoped_to_tenant("acme")
            .register_workflow("only-acme".to_string(), move || a.clone());

        let admin = shared.admin_view();
        assert!(admin.is_admin());
        assert!(admin.get_workflow("only-acme").is_some());

        for tenant in ["acme", "globex"] {
            let w = wf(tenant);
            shared
                .scoped_to_tenant(tenant)
                .register_workflow("shared-name".to_string(), move || w.clone());
        }
        assert!(
            admin.get_workflow("shared-name").is_none(),
            "an ambiguous name must not resolve to a guess"
        );
        // But the admin view can still enumerate everything.
        let mut names = admin.workflow_names();
        names.sort();
        assert_eq!(
            names,
            vec!["only-acme".to_string(), "shared-name".to_string()]
        );
    }

    /// The same isolation holds for the reactor registry, which is what the
    /// reconciler's unload bookkeeping walks.
    #[test]
    fn reactors_and_graphs_are_tenant_scoped_too() {
        let shared = Runtime::empty();
        let acme = shared.scoped_to_tenant("acme");
        let globex = shared.scoped_to_tenant("globex");

        for rt in [&acme, &globex] {
            rt.register_triggerless_graph("g".to_string(), || unreachable!());
            rt.register_computation_graph("cg".to_string(), || unreachable!());
        }
        assert_eq!(shared.triggerless_graph_keys().len(), 2);
        assert_eq!(shared.computation_graph_keys().len(), 2);

        assert!(acme.unregister_triggerless_graph("g"));
        assert!(!acme.unregister_triggerless_graph("g"));
        assert_eq!(globex.triggerless_graph_names(), vec!["g".to_string()]);

        assert!(acme.unregister_computation_graph("cg"));
        assert_eq!(globex.computation_graph_names(), vec!["cg".to_string()]);
    }

    /// `seed_from_inventory` always writes untenanted entries, whatever scope
    /// the handle carries — otherwise the reconciler's post-dlopen re-seed
    /// would stamp one tenant's cdylib entries onto the next tenant to load.
    #[test]
    fn inventory_seeding_is_always_untenanted() {
        let shared = Runtime::empty();
        shared.scoped_to_tenant("acme").seed_from_inventory();
        assert!(
            shared.workflow_keys().iter().all(|k| k.tenant_id.is_none()),
            "inventory entries must never be stamped with a tenant"
        );
        assert!(shared.reactor_keys().iter().all(|k| k.tenant_id.is_none()));
        assert!(shared.trigger_keys().iter().all(|k| k.tenant_id.is_none()));
    }
}
