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

//! Scoped runtime for isolated task, workflow, trigger, computation graph,
//! and stream backend registries.
//!
//! [`Runtime`] is the single source of truth for all registries. There is
//! one lookup path — local maps only, no delegation to globals.
//!
//! # Usage
//!
//! ```rust,ignore
//! use cloacina::Runtime;
//!
//! // Snapshot global registries (picks up #[ctor] registrations)
//! let runtime = Runtime::new();
//!
//! // Empty runtime for test isolation
//! let runtime = Runtime::empty();
//! runtime.register_task(namespace, || Arc::new(my_task()));
//! runtime.register_workflow("my_workflow".to_string(), || workflow.clone());
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::computation_graph::stream_backend::{StreamBackendFactory, StreamBackendRegistry};
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

/// A scoped runtime holding isolated registries for tasks, workflows, triggers,
/// computation graphs, and stream backends.
///
/// `Runtime` is the single source of truth for all registries. Every lookup
/// checks local maps only — there is no delegation to globals. This ensures
/// tests and production exercise the same code path.
///
/// - [`Runtime::new()`] — snapshots all global registries (populated by `#[ctor]`)
/// - [`Runtime::empty()`] — completely empty, for test isolation
#[derive(Clone)]
pub struct Runtime {
    inner: Arc<RuntimeInner>,
}

struct RuntimeInner {
    tasks: RwLock<HashMap<TaskNamespace, TaskConstructorFn>>,
    workflows: RwLock<HashMap<String, WorkflowConstructorFn>>,
    triggers: RwLock<HashMap<String, TriggerConstructorFn>>,
    computation_graphs: RwLock<HashMap<String, ComputationGraphConstructor>>,
    stream_backends: RwLock<StreamBackendRegistry>,
}

impl Runtime {
    /// Create a runtime pre-populated from all global registries.
    ///
    /// Snapshots the current state of global task, workflow, trigger,
    /// computation graph, and stream backend registries into local maps.
    /// After creation, lookups only check local maps — no fallback.
    pub fn new() -> Self {
        // Snapshot tasks
        let tasks = {
            let global = crate::task::global_task_registry();
            let g = global.read();
            g.iter()
                .map(|(ns, ctor)| {
                    let task_instance = ctor();
                    let ns = ns.clone();
                    (
                        ns,
                        Box::new(move || task_instance.clone()) as TaskConstructorFn,
                    )
                })
                .collect::<HashMap<_, _>>()
        };

        // Snapshot workflows
        let workflows = {
            let global = crate::workflow::global_workflow_registry();
            let g = global.read();
            g.iter()
                .map(|(name, ctor)| {
                    let wf = ctor();
                    (
                        name.clone(),
                        Box::new(move || wf.clone()) as WorkflowConstructorFn,
                    )
                })
                .collect::<HashMap<_, _>>()
        };

        // Snapshot triggers
        let triggers = {
            let global = crate::trigger::global_trigger_registry();
            let g = global.read();
            g.iter()
                .map(|(name, ctor)| {
                    let trigger = ctor();
                    (
                        name.clone(),
                        Box::new(move || trigger.clone()) as TriggerConstructorFn,
                    )
                })
                .collect::<HashMap<_, _>>()
        };

        // Snapshot computation graphs
        let computation_graphs = {
            let global = cloacina_computation_graph::global_computation_graph_registry();
            let g = global.read();
            let mut map = HashMap::new();
            for (name, _ctor) in g.iter() {
                // We can't clone the constructor (Box<dyn Fn>), so we re-read
                // from the global on each call. This is fine — the snapshot
                // captures which graphs exist, and the constructor is stateless.
                let global_ref = cloacina_computation_graph::global_computation_graph_registry();
                let graph_name = name.clone();
                map.insert(
                    name.clone(),
                    Box::new(move || {
                        let reg = global_ref.read();
                        reg.get(&graph_name)
                            .expect("computation graph disappeared from global registry")(
                        )
                    }) as ComputationGraphConstructor,
                );
            }
            map
        };

        // Snapshot stream backends
        let stream_backends = {
            let global = crate::computation_graph::stream_backend::global_stream_registry();
            let g = global.lock().unwrap();
            g.snapshot()
        };

        Self {
            inner: Arc::new(RuntimeInner {
                tasks: RwLock::new(tasks),
                workflows: RwLock::new(workflows),
                triggers: RwLock::new(triggers),
                computation_graphs: RwLock::new(computation_graphs),
                stream_backends: RwLock::new(stream_backends),
            }),
        }
    }

    /// Create a completely empty runtime with no registrations.
    ///
    /// Use this for test isolation — register only what the test needs.
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(RuntimeInner {
                tasks: RwLock::new(HashMap::new()),
                workflows: RwLock::new(HashMap::new()),
                triggers: RwLock::new(HashMap::new()),
                computation_graphs: RwLock::new(HashMap::new()),
                stream_backends: RwLock::new(StreamBackendRegistry::new()),
            }),
        }
    }

    /// Backward-compatible alias for [`Runtime::new()`].
    #[deprecated(note = "use Runtime::new() instead — same behavior, single code path")]
    pub fn from_global() -> Self {
        Self::new()
    }

    // -----------------------------------------------------------------------
    // Task registry
    // -----------------------------------------------------------------------

    /// Register a task constructor for the given namespace.
    pub fn register_task<F>(&self, namespace: TaskNamespace, constructor: F)
    where
        F: Fn() -> Arc<dyn Task> + Send + Sync + 'static,
    {
        let mut guard = self.inner.tasks.write();
        guard.insert(namespace, Box::new(constructor));
    }

    /// Look up and instantiate a task by namespace.
    pub fn get_task(&self, namespace: &TaskNamespace) -> Option<Arc<dyn Task>> {
        let guard = self.inner.tasks.read();
        guard.get(namespace).map(|ctor| ctor())
    }

    /// Check if a task is registered for the given namespace.
    pub fn has_task(&self, namespace: &TaskNamespace) -> bool {
        let guard = self.inner.tasks.read();
        guard.contains_key(namespace)
    }

    // -----------------------------------------------------------------------
    // Workflow registry
    // -----------------------------------------------------------------------

    /// Register a workflow constructor by name.
    pub fn register_workflow<F>(&self, name: String, constructor: F)
    where
        F: Fn() -> Workflow + Send + Sync + 'static,
    {
        let mut guard = self.inner.workflows.write();
        guard.insert(name, Box::new(constructor));
    }

    /// Look up and instantiate a workflow by name.
    pub fn get_workflow(&self, name: &str) -> Option<Workflow> {
        let guard = self.inner.workflows.read();
        guard.get(name).map(|ctor| ctor())
    }

    /// Get all registered workflow names.
    pub fn workflow_names(&self) -> Vec<String> {
        let guard = self.inner.workflows.read();
        guard.keys().cloned().collect()
    }

    /// Get all registered workflows (instantiated).
    pub fn all_workflows(&self) -> Vec<Workflow> {
        let guard = self.inner.workflows.read();
        guard.values().map(|ctor| ctor()).collect()
    }

    // -----------------------------------------------------------------------
    // Trigger registry
    // -----------------------------------------------------------------------

    /// Register a trigger constructor by name.
    pub fn register_trigger<F>(&self, name: String, constructor: F)
    where
        F: Fn() -> Arc<dyn Trigger> + Send + Sync + 'static,
    {
        let mut guard = self.inner.triggers.write();
        guard.insert(name, Box::new(constructor));
    }

    /// Look up and instantiate a trigger by name.
    pub fn get_trigger(&self, name: &str) -> Option<Arc<dyn Trigger>> {
        let guard = self.inner.triggers.read();
        guard.get(name).map(|ctor| ctor())
    }

    /// Get all registered trigger names.
    pub fn trigger_names(&self) -> Vec<String> {
        let guard = self.inner.triggers.read();
        guard.keys().cloned().collect()
    }

    /// Get all registered triggers (instantiated).
    pub fn all_triggers(&self) -> HashMap<String, Arc<dyn Trigger>> {
        let guard = self.inner.triggers.read();
        guard
            .iter()
            .map(|(name, ctor)| (name.clone(), ctor()))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Computation graph registry
    // -----------------------------------------------------------------------

    /// Register a computation graph constructor by name.
    pub fn register_computation_graph<F>(&self, name: String, constructor: F)
    where
        F: Fn() -> ComputationGraphRegistration + Send + Sync + 'static,
    {
        let mut guard = self.inner.computation_graphs.write();
        guard.insert(name, Box::new(constructor));
    }

    /// Look up and invoke a computation graph constructor by name.
    pub fn get_computation_graph(&self, name: &str) -> Option<ComputationGraphRegistration> {
        let guard = self.inner.computation_graphs.read();
        guard.get(name).map(|ctor| ctor())
    }

    /// Get all registered computation graph names.
    pub fn computation_graph_names(&self) -> Vec<String> {
        let guard = self.inner.computation_graphs.read();
        guard.keys().cloned().collect()
    }

    // -----------------------------------------------------------------------
    // Stream backend registry
    // -----------------------------------------------------------------------

    /// Register a stream backend factory by type name.
    pub fn register_stream_backend(&self, type_name: &str, factory: StreamBackendFactory) {
        let mut guard = self.inner.stream_backends.write();
        guard.register(type_name, factory);
    }

    /// Get a read-locked reference to the stream backend registry.
    pub fn stream_backends(&self) -> parking_lot::RwLockReadGuard<'_, StreamBackendRegistry> {
        self.inner.stream_backends.read()
    }

    /// Sync new entries from global registries into this runtime.
    ///
    /// Call this after dynamically loading packages (e.g., via `dlopen`)
    /// whose `#[ctor]` registered into globals. Only adds entries that
    /// don't already exist in the local maps.
    pub fn sync_from_global(&self) {
        // Sync tasks
        {
            let global = crate::task::global_task_registry();
            let g = global.read();
            let mut local = self.inner.tasks.write();
            for (ns, ctor) in g.iter() {
                if !local.contains_key(ns) {
                    let task_instance = ctor();
                    let ns = ns.clone();
                    local.insert(
                        ns,
                        Box::new(move || task_instance.clone()) as TaskConstructorFn,
                    );
                }
            }
        }
        // Sync workflows
        {
            let global = crate::workflow::global_workflow_registry();
            let g = global.read();
            let mut local = self.inner.workflows.write();
            for (name, ctor) in g.iter() {
                if !local.contains_key(name) {
                    let wf = ctor();
                    local.insert(
                        name.clone(),
                        Box::new(move || wf.clone()) as WorkflowConstructorFn,
                    );
                }
            }
        }
        // Sync triggers
        {
            let global = crate::trigger::global_trigger_registry();
            let g = global.read();
            let mut local = self.inner.triggers.write();
            for (name, ctor) in g.iter() {
                if !local.contains_key(name) {
                    let trigger = ctor();
                    local.insert(
                        name.clone(),
                        Box::new(move || trigger.clone()) as TriggerConstructorFn,
                    );
                }
            }
        }
        // Sync computation graphs
        {
            let global_ref = cloacina_computation_graph::global_computation_graph_registry();
            let g = global_ref.read();
            let mut local = self.inner.computation_graphs.write();
            for (name, _) in g.iter() {
                if !local.contains_key(name) {
                    let global_clone = global_ref.clone();
                    let graph_name = name.clone();
                    local.insert(
                        name.clone(),
                        Box::new(move || {
                            let reg = global_clone.read();
                            reg.get(&graph_name)
                                .expect("computation graph disappeared from global registry")(
                            )
                        }) as ComputationGraphConstructor,
                    );
                }
            }
        }
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
        f.debug_struct("Runtime")
            .field("tasks", &tasks)
            .field("workflows", &workflows)
            .field("triggers", &triggers)
            .field("computation_graphs", &cgs)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskNamespace;
    use std::sync::Arc;

    #[test]
    fn test_empty_runtime() {
        let runtime = Runtime::empty();
        let ns = TaskNamespace::new("t", "p", "w", "task1");
        assert!(runtime.get_task(&ns).is_none());
        assert!(runtime.get_workflow("test").is_none());
        assert!(runtime.get_trigger("test").is_none());
        assert!(!runtime.has_task(&ns));
        assert!(runtime.get_computation_graph("test").is_none());
    }

    #[test]
    fn test_register_and_get_workflow() {
        let runtime = Runtime::empty();
        let wf = Workflow::new("test_workflow");
        runtime.register_workflow("test_workflow".to_string(), move || wf.clone());

        let result = runtime.get_workflow("test_workflow");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "test_workflow");
    }

    #[test]
    fn test_scoped_mutations_dont_affect_other_runtimes() {
        let rt1 = Runtime::empty();
        let rt2 = Runtime::empty();

        let wf = Workflow::new("only_in_rt1");
        rt1.register_workflow("only_in_rt1".to_string(), move || wf.clone());

        assert!(rt1.get_workflow("only_in_rt1").is_some());
        assert!(rt2.get_workflow("only_in_rt1").is_none());
    }

    #[test]
    fn test_clone_is_shared() {
        let rt1 = Runtime::empty();
        let rt2 = rt1.clone();

        let wf = Workflow::new("shared");
        rt1.register_workflow("shared".to_string(), move || wf.clone());

        // Clone shares the same Arc — both see the registration
        assert!(rt2.get_workflow("shared").is_some());
    }

    #[test]
    fn test_new_snapshots_globals() {
        // Register a workflow in globals
        let wf = Workflow::new("global_test_wf");
        crate::workflow::register_workflow_constructor("global_test_wf".to_string(), move || {
            wf.clone()
        });

        // new() snapshots globals — should see the registration
        let runtime = Runtime::new();
        assert!(runtime.get_workflow("global_test_wf").is_some());
    }

    #[test]
    fn test_workflow_names() {
        let runtime = Runtime::empty();
        let wf1 = Workflow::new("alpha");
        let wf2 = Workflow::new("beta");
        runtime.register_workflow("alpha".to_string(), move || wf1.clone());
        runtime.register_workflow("beta".to_string(), move || wf2.clone());

        let mut names = runtime.workflow_names();
        names.sort();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_debug_format() {
        let runtime = Runtime::empty();
        let debug = format!("{:?}", runtime);
        assert!(debug.contains("Runtime"));
        assert!(debug.contains("tasks: 0"));
    }

    #[test]
    fn test_new_does_not_see_late_registrations() {
        // Snapshot BEFORE registering
        let runtime = Runtime::new();

        // Register AFTER snapshot
        let unique_name = format!(
            "late_reg_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let wf = Workflow::new(&unique_name);
        crate::workflow::register_workflow_constructor(unique_name.clone(), move || wf.clone());

        // Snapshot is frozen — should NOT see late registration
        assert!(
            runtime.get_workflow(&unique_name).is_none(),
            "Runtime::new() snapshot should not see workflows registered after creation"
        );
    }

    #[test]
    fn test_empty_does_not_see_global_registrations() {
        // Register a workflow in globals
        let unique_name = format!(
            "isolated_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let wf = Workflow::new(&unique_name);
        crate::workflow::register_workflow_constructor(unique_name.clone(), move || wf.clone());

        // Runtime::empty() is isolated
        let runtime = Runtime::empty();
        assert!(
            runtime.get_workflow(&unique_name).is_none(),
            "Runtime::empty() should NOT see globally registered workflows"
        );
    }

    #[test]
    fn test_local_registration_takes_precedence() {
        let runtime = Runtime::new();

        // Register a workflow in globals
        let unique_name = format!(
            "precedence_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let global_wf = Workflow::new(&unique_name);
        crate::workflow::register_workflow_constructor(unique_name.clone(), move || {
            global_wf.clone()
        });

        // Register a DIFFERENT workflow with the same name locally
        let mut local_wf = Workflow::new(&unique_name);
        local_wf.set_tenant("local_tenant");
        runtime.register_workflow(unique_name.clone(), move || local_wf.clone());

        // Local should win (overwrites the snapshot)
        let result = runtime.get_workflow(&unique_name).unwrap();
        assert_eq!(
            result.tenant(),
            "local_tenant",
            "Local registration should take precedence over snapshot"
        );
    }

    #[test]
    fn test_sync_from_global() {
        let runtime = Runtime::empty();

        // Register in globals
        let unique_name = format!(
            "sync_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let wf = Workflow::new(&unique_name);
        crate::workflow::register_workflow_constructor(unique_name.clone(), move || wf.clone());

        // Not visible yet
        assert!(runtime.get_workflow(&unique_name).is_none());

        // Sync picks it up
        runtime.sync_from_global();
        assert!(runtime.get_workflow(&unique_name).is_some());
    }

    #[test]
    fn test_computation_graph_registry() {
        let runtime = Runtime::empty();
        assert!(runtime.computation_graph_names().is_empty());

        runtime.register_computation_graph("test_graph".to_string(), || {
            ComputationGraphRegistration {
                graph_fn: Arc::new(|_| {
                    Box::pin(async { cloacina_computation_graph::GraphResult::completed_empty() })
                }),
                accumulator_names: vec!["acc1".to_string()],
                reaction_mode: "when_any".to_string(),
            }
        });

        assert!(runtime.get_computation_graph("test_graph").is_some());
        assert_eq!(runtime.computation_graph_names(), vec!["test_graph"]);
    }
}
