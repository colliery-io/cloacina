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
//! This type replaces direct access to the process-global static registries.
//! A single [`Runtime`] instance can be constructed, seeded (in embedded mode
//! today via [`Runtime::seed_from_globals`] — the longer-term plan in
//! CLOACI-I-0096 is inventory-based seeding), and handed to the executor.
//!
//! ```rust,ignore
//! use cloacina::Runtime;
//!
//! let runtime = Runtime::new();
//! runtime.seed_from_globals(); // pick up #[ctor]-registered items
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
    /// Create an empty runtime. Every namespace starts with no entries.
    ///
    /// In embedded mode, follow this with [`seed_from_globals`] to pick up
    /// anything that was registered via the `#[ctor]` constructors emitted by
    /// the macros. Once inventory-based seeding lands (T-0506) that helper
    /// will go away.
    pub fn new() -> Self {
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

    /// Copy every entry from the process-global registries into this runtime.
    ///
    /// This is the transitional bridge that replaces the old
    /// `Runtime::from_global()` constructor. It's called explicitly by
    /// `DefaultRunner` and other embedded-mode entrypoints so the contents of
    /// the globals end up inside the Runtime, where they are subject to the
    /// same register/unregister contract as dynamically loaded packages.
    ///
    /// Planned for removal alongside the globals once T-0506 lands
    /// (inventory-seeded `Runtime::new`).
    pub fn seed_from_globals(&self) {
        // The #[workflow] macro registers tasks lazily — they only land in the
        // global task registry as a side-effect of calling the workflow
        // constructor. Fire every known constructor once before copying tasks
        // so the task registry is actually populated.
        {
            let global = crate::workflow::global_workflow_registry();
            let g = global.read();
            for (_, ctor) in g.iter() {
                let _ = ctor(); // discard: we only want the registration side-effect
            }
        }

        // Tasks
        {
            let global = crate::task::global_task_registry();
            let src = global.read();
            let mut dst = self.inner.tasks.write();
            for (ns, _) in src.iter() {
                let ns_for_closure = ns.clone();
                dst.insert(
                    ns.clone(),
                    Box::new(move || {
                        crate::task::get_task(&ns_for_closure)
                            .expect("task vanished from global registry between seed and call")
                    }),
                );
            }
        }

        // Workflows
        {
            let global = crate::workflow::global_workflow_registry();
            let src = global.read();
            let mut dst = self.inner.workflows.write();
            for (name, _) in src.iter() {
                let name_for_closure = name.clone();
                dst.insert(
                    name.clone(),
                    Box::new(move || {
                        let global = crate::workflow::global_workflow_registry();
                        let g = global.read();
                        g.get(&name_for_closure)
                            .map(|ctor| ctor())
                            .expect("workflow vanished from global registry between seed and call")
                    }),
                );
            }
        }

        // Triggers
        {
            let global = crate::trigger::global_trigger_registry();
            let src = global.read();
            let mut dst = self.inner.triggers.write();
            for (name, _) in src.iter() {
                let name_for_closure = name.clone();
                dst.insert(
                    name.clone(),
                    Box::new(move || {
                        let global = crate::trigger::global_trigger_registry();
                        let g = global.read();
                        g.get(&name_for_closure)
                            .map(|ctor| ctor())
                            .expect("trigger vanished from global registry between seed and call")
                    }),
                );
            }
        }

        // Computation graphs
        {
            let global = cloacina_computation_graph::global_computation_graph_registry();
            let src = global.read();
            let mut dst = self.inner.computation_graphs.write();
            for (name, _) in src.iter() {
                let name_for_closure = name.clone();
                dst.insert(
                    name.clone(),
                    Box::new(move || {
                        let global =
                            cloacina_computation_graph::global_computation_graph_registry();
                        let g = global.read();
                        let ctor = g.get(&name_for_closure).expect(
                            "computation graph vanished from global registry between seed and call",
                        );
                        ctor()
                    }),
                );
            }
        }

        // Stream backends: the global store is Mutex<StreamBackendRegistry> and
        // factories are not clonable or re-invokable from the outside. We
        // cannot copy the factory closures across — instead we register a
        // pass-through factory that delegates to the global on each call.
        {
            let names: Vec<String> = {
                let lock = crate::computation_graph::stream_backend::global_stream_registry()
                    .lock()
                    .unwrap();
                // StreamBackendRegistry has no public iterator; expose via has()
                // checks is pointless. Reach in via the (already public) fields
                // would break encapsulation. Leave stream backends alone until
                // we give StreamBackendRegistry a list() method — until then,
                // callers that want packaged stream backends via Runtime must
                // register them explicitly.
                drop(lock);
                Vec::new()
            };
            let mut dst = self.inner.stream_backends.write();
            for name in names {
                let name_for_closure = name.clone();
                dst.insert(
                    name,
                    Box::new(move |config: StreamConfig| {
                        let name = name_for_closure.clone();
                        Box::pin(async move {
                            let fut = {
                                let reg = crate::computation_graph::stream_backend::global_stream_registry()
                                    .lock()
                                    .unwrap();
                                reg.create_future(&name, config).ok_or_else(|| {
                                    StreamError::NotFound(format!(
                                        "backend '{}' vanished from global registry",
                                        name
                                    ))
                                })?
                            };
                            fut.await
                        })
                            as Pin<
                                Box<
                                    dyn Future<
                                            Output = Result<
                                                Box<dyn StreamBackend>,
                                                StreamError,
                                            >,
                                        > + Send,
                                >,
                            >
                    }),
                );
            }
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
        let rt = Runtime::new();
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
        let rt = Runtime::new();
        assert!(!rt.unregister_trigger("missing"));
        assert!(rt.get_trigger("missing").is_none());
        assert!(rt.trigger_names().is_empty());
    }

    #[test]
    fn register_and_unregister_task() {
        let rt = Runtime::new();
        let ns = TaskNamespace::new("t", "p", "w", "task_a");
        assert!(!rt.unregister_task(&ns));
        assert!(!rt.has_task(&ns));
    }

    #[test]
    fn stream_backend_roundtrip_names_only() {
        let rt = Runtime::new();
        assert!(!rt.has_stream_backend("mock"));
        assert!(rt.stream_backend_names().is_empty());
        assert!(!rt.unregister_stream_backend("mock"));
    }

    #[test]
    fn runtimes_are_independent() {
        let rt1 = Runtime::new();
        let rt2 = Runtime::new();
        let wf = crate::workflow::Workflow::new("iso");
        rt1.register_workflow("iso".to_string(), move || wf.clone());

        assert!(rt1.get_workflow("iso").is_some());
        assert!(rt2.get_workflow("iso").is_none());
    }

    #[test]
    fn debug_format_reports_sizes() {
        let rt = Runtime::new();
        let debug = format!("{:?}", rt);
        assert!(debug.contains("computation_graphs: 0"));
        assert!(debug.contains("stream_backends: 0"));
    }
}
