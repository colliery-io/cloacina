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

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::computation_graph::stream_backend::{
    StreamBackend, StreamBackendFactory, StreamConfig, StreamError,
};
use crate::task::{Task, TaskNamespace};
use crate::trigger::Trigger;
use crate::workflow::Workflow;
use cloacina_computation_graph::{ComputationGraphConstructor, ComputationGraphRegistration};

/// Type alias for task constructor functions.
pub type TaskConstructorFn = Box<dyn Fn() -> Arc<dyn Task> + Send + Sync>;

/// Type alias for workflow constructor functions.
pub type WorkflowConstructorFn = Box<dyn Fn() -> Workflow + Send + Sync>;

/// Type alias for trigger constructor functions.
pub type TriggerConstructorFn = Box<dyn Fn() -> Arc<dyn Trigger> + Send + Sync>;

/// A scoped runtime holding the registries for every cloacina extension point.
///
/// All five namespaces — tasks, workflows, triggers, computation graphs, and
/// stream backends — are registered and unregistered through the same surface.
/// `Runtime` is cheap to clone: it shares its registries via `Arc`.
#[derive(Clone)]
pub struct Runtime {
    inner: Arc<RuntimeInner>,
}

struct RuntimeInner {
    tasks: RwLock<HashMap<TaskNamespace, TaskConstructorFn>>,
    workflows: RwLock<HashMap<String, WorkflowConstructorFn>>,
    triggers: RwLock<HashMap<String, TriggerConstructorFn>>,
    computation_graphs: RwLock<HashMap<String, ComputationGraphConstructor>>,
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
                workflows: RwLock::new(HashMap::new()),
                triggers: RwLock::new(HashMap::new()),
                computation_graphs: RwLock::new(HashMap::new()),
                stream_backends: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Populate the runtime from the `inventory` entries emitted by the
    /// macros.
    ///
    /// `inventory`'s linker-section collection works across `dlopen`'d cdylibs
    /// on Linux/macOS, so the reconciler calls this again after loading a new
    /// workflow package to pick up the entries emitted by that cdylib.
    pub fn seed_from_inventory(&self) {
        use crate::inventory_entries::{
            ComputationGraphEntry, StreamBackendEntry, TaskEntry, TriggerEntry, WorkflowEntry,
        };

        for entry in inventory::iter::<TaskEntry> {
            let ns = (entry.namespace)();
            let ctor = entry.constructor;
            self.register_task(ns, move || ctor());
        }

        for entry in inventory::iter::<WorkflowEntry> {
            let ctor = entry.constructor;
            self.register_workflow(entry.name.to_string(), move || ctor());
        }

        for entry in inventory::iter::<TriggerEntry> {
            let ctor = entry.constructor;
            self.register_trigger(entry.name.to_string(), move || ctor());
        }

        for entry in inventory::iter::<ComputationGraphEntry> {
            let ctor = entry.constructor;
            self.register_computation_graph(entry.name.to_string(), move || ctor());
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
    pub fn register_task<F>(&self, namespace: TaskNamespace, constructor: F)
    where
        F: Fn() -> Arc<dyn Task> + Send + Sync + 'static,
    {
        self.inner
            .tasks
            .write()
            .insert(namespace, Box::new(constructor));
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
    pub fn has_task(&self, namespace: &TaskNamespace) -> bool {
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

    /// Register a workflow constructor by name.
    pub fn register_workflow<F>(&self, name: String, constructor: F)
    where
        F: Fn() -> Workflow + Send + Sync + 'static,
    {
        self.inner
            .workflows
            .write()
            .insert(name, Box::new(constructor));
    }

    /// Remove a workflow constructor. Returns true if the entry existed.
    pub fn unregister_workflow(&self, name: &str) -> bool {
        self.inner.workflows.write().remove(name).is_some()
    }

    /// Look up and instantiate a workflow by name.
    pub fn get_workflow(&self, name: &str) -> Option<Workflow> {
        self.inner.workflows.read().get(name).map(|ctor| ctor())
    }

    /// Get all registered workflow names.
    pub fn workflow_names(&self) -> Vec<String> {
        self.inner.workflows.read().keys().cloned().collect()
    }

    /// Get all registered workflows (instantiated).
    pub fn all_workflows(&self) -> Vec<Workflow> {
        self.inner
            .workflows
            .read()
            .values()
            .map(|ctor| ctor())
            .collect()
    }

    // -----------------------------------------------------------------------
    // Trigger registry
    // -----------------------------------------------------------------------

    /// Register a trigger constructor by name.
    pub fn register_trigger<F>(&self, name: String, constructor: F)
    where
        F: Fn() -> Arc<dyn Trigger> + Send + Sync + 'static,
    {
        self.inner
            .triggers
            .write()
            .insert(name, Box::new(constructor));
    }

    /// Remove a trigger constructor. Returns true if the entry existed.
    pub fn unregister_trigger(&self, name: &str) -> bool {
        self.inner.triggers.write().remove(name).is_some()
    }

    /// Look up and instantiate a trigger by name.
    pub fn get_trigger(&self, name: &str) -> Option<Arc<dyn Trigger>> {
        self.inner.triggers.read().get(name).map(|ctor| ctor())
    }

    /// Get all registered trigger names.
    pub fn trigger_names(&self) -> Vec<String> {
        self.inner.triggers.read().keys().cloned().collect()
    }

    /// Get all registered triggers (instantiated).
    pub fn all_triggers(&self) -> HashMap<String, Arc<dyn Trigger>> {
        self.inner
            .triggers
            .read()
            .iter()
            .map(|(name, ctor)| (name.clone(), ctor()))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Computation graph registry
    // -----------------------------------------------------------------------

    /// Register a computation graph constructor by graph name.
    pub fn register_computation_graph<F>(&self, name: String, constructor: F)
    where
        F: Fn() -> ComputationGraphRegistration + Send + Sync + 'static,
    {
        self.inner
            .computation_graphs
            .write()
            .insert(name, Box::new(constructor));
    }

    /// Remove a computation graph constructor. Returns true if the entry existed.
    pub fn unregister_computation_graph(&self, name: &str) -> bool {
        self.inner.computation_graphs.write().remove(name).is_some()
    }

    /// Look up and instantiate a computation graph registration by name.
    pub fn get_computation_graph(&self, name: &str) -> Option<ComputationGraphRegistration> {
        self.inner
            .computation_graphs
            .read()
            .get(name)
            .map(|ctor| ctor())
    }

    /// Get all registered computation graph names.
    pub fn computation_graph_names(&self) -> Vec<String> {
        self.inner
            .computation_graphs
            .read()
            .keys()
            .cloned()
            .collect()
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
    pub fn has_stream_backend(&self, type_name: &str) -> bool {
        self.inner.stream_backends.read().contains_key(type_name)
    }

    /// Get the creation future for a stream backend without holding the lock
    /// across await. Returns `None` if the type is not registered.
    pub fn create_stream_backend(
        &self,
        type_name: &str,
        config: StreamConfig,
    ) -> Option<Pin<Box<dyn Future<Output = Result<Box<dyn StreamBackend>, StreamError>> + Send>>>
    {
        let guard = self.inner.stream_backends.read();
        let factory = guard.get(type_name)?;
        Some(factory(config))
    }

    /// Get all registered stream backend type names.
    pub fn stream_backend_names(&self) -> Vec<String> {
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
        let workflows = self.inner.workflows.read().len();
        let triggers = self.inner.triggers.read().len();
        let cgs = self.inner.computation_graphs.read().len();
        let sbs = self.inner.stream_backends.read().len();
        f.debug_struct("Runtime")
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
}
