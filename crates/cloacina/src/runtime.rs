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

//! Scoped runtime for isolated task, workflow, and trigger registries.
//!
//! [`Runtime`] replaces direct access to process-global static registries,
//! enabling multiple isolated workflow environments in the same process and
//! parallel test execution without `#[serial]`.
//!
//! # Usage
//!
//! ```rust,ignore
//! use cloacina::Runtime;
//!
//! // Snapshot global registries (backward-compatible with #[ctor] registration)
//! let runtime = Runtime::from_global();
//!
//! // Or create an empty runtime for isolation
//! let runtime = Runtime::new();
//! runtime.register_task(namespace, || Arc::new(my_task()));
//! runtime.register_workflow("my_workflow".to_string(), || workflow.clone());
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::task::{Task, TaskNamespace};
use crate::trigger::Trigger;
use crate::workflow::Workflow;

/// Type alias for task constructor functions.
pub type TaskConstructorFn = Box<dyn Fn() -> Arc<dyn Task> + Send + Sync>;

/// Type alias for workflow constructor functions.
pub type WorkflowConstructorFn = Box<dyn Fn() -> Workflow + Send + Sync>;

/// Type alias for trigger constructor functions.
pub type TriggerConstructorFn = Box<dyn Fn() -> Arc<dyn Trigger> + Send + Sync>;

/// A scoped runtime holding isolated registries for tasks, workflows, and triggers.
///
/// `Runtime` enables multiple independent workflow environments in the same process.
/// Each runtime has its own set of registered tasks, workflows, and triggers that
/// do not interfere with other runtimes or the process-global registries.
///
/// Two modes:
/// - [`Runtime::new()`] — isolated, no fallback (for tests)
/// - [`Runtime::from_global()`] — delegates to global registries for dynamic
///   package loading (for the server)
#[derive(Clone)]
pub struct Runtime {
    inner: Arc<RuntimeInner>,
}

struct RuntimeInner {
    tasks: RwLock<HashMap<TaskNamespace, TaskConstructorFn>>,
    workflows: RwLock<HashMap<String, WorkflowConstructorFn>>,
    triggers: RwLock<HashMap<String, TriggerConstructorFn>>,
    /// When true, `get_*()` falls back to the process-global registries
    /// if the local map doesn't contain the entry. This enables dynamic
    /// package loading (reconciler registers in globals after startup).
    use_globals: bool,
}

impl Runtime {
    /// Create an empty runtime with no registered tasks, workflows, or triggers.
    ///
    /// Lookups only check the local maps — no fallback to globals.
    /// Use this for test isolation.
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

    /// Create a runtime that delegates to the process-global registries.
    ///
    /// Lookups check the local maps first, then fall back to the global
    /// task/workflow/trigger registries. This supports dynamic package
    /// loading — packages registered after startup are visible immediately.
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
    ///
    /// Checks local registry first, then falls back to the global registry
    /// if `use_globals` is enabled (i.e., created via `from_global()`).
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

    /// Check if a task is registered for the given namespace.
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
    ///
    /// Checks local registry first, then falls back to the global registry
    /// if `use_globals` is enabled.
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
    ///
    /// Checks local registry first, then falls back to the global registry
    /// if `use_globals` is enabled.
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
        f.debug_struct("Runtime")
            .field("tasks", &tasks)
            .field("workflows", &workflows)
            .field("triggers", &triggers)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskNamespace;

    #[test]
    fn test_empty_runtime() {
        let runtime = Runtime::new();
        let ns = TaskNamespace::new("t", "p", "w", "task1");
        assert!(runtime.get_task(&ns).is_none());
        assert!(runtime.get_workflow("test").is_none());
        assert!(runtime.get_trigger("test").is_none());
        assert!(!runtime.has_task(&ns));
    }

    #[test]
    fn test_register_and_get_workflow() {
        let runtime = Runtime::new();
        let wf = Workflow::new("test_workflow");
        runtime.register_workflow("test_workflow".to_string(), move || wf.clone());

        let result = runtime.get_workflow("test_workflow");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "test_workflow");
    }

    #[test]
    fn test_scoped_mutations_dont_affect_other_runtimes() {
        let rt1 = Runtime::new();
        let rt2 = Runtime::new();

        let wf = Workflow::new("only_in_rt1");
        rt1.register_workflow("only_in_rt1".to_string(), move || wf.clone());

        assert!(rt1.get_workflow("only_in_rt1").is_some());
        assert!(rt2.get_workflow("only_in_rt1").is_none());
    }

    #[test]
    fn test_clone_is_shared() {
        let rt1 = Runtime::new();
        let rt2 = rt1.clone();

        let wf = Workflow::new("shared");
        rt1.register_workflow("shared".to_string(), move || wf.clone());

        // Clone shares the same Arc — both see the registration
        assert!(rt2.get_workflow("shared").is_some());
    }

    #[test]
    fn test_from_global_captures_workflows() {
        // Register a workflow in globals
        let wf = Workflow::new("global_test_wf");
        crate::workflow::register_workflow_constructor("global_test_wf".to_string(), move || {
            wf.clone()
        });

        let runtime = Runtime::from_global();
        assert!(runtime.get_workflow("global_test_wf").is_some());
    }

    #[test]
    fn test_workflow_names() {
        let runtime = Runtime::new();
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
        let runtime = Runtime::new();
        let debug = format!("{:?}", runtime);
        assert!(debug.contains("Runtime"));
        assert!(debug.contains("tasks: 0"));
    }
}
