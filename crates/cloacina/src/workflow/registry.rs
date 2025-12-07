/*
 *  Copyright 2025 Colliery Software
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

//! Global workflow registry for automatic workflow registration.
//!
//! This module provides the global registry used by the `workflow!` macro
//! to automatically register workflows at startup.

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use super::Workflow;

/// Type alias for the workflow constructor function stored in the global registry
pub type WorkflowConstructor = Box<dyn Fn() -> Workflow + Send + Sync>;

/// Type alias for the global workflow registry containing workflow constructors
pub type GlobalWorkflowRegistry = Arc<RwLock<HashMap<String, WorkflowConstructor>>>;

/// Global registry for automatically registering workflows created with the `workflow!` macro
pub static GLOBAL_WORKFLOW_REGISTRY: Lazy<GlobalWorkflowRegistry> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Register a workflow constructor function globally
///
/// This is used internally by the `workflow!` macro to automatically register workflows.
/// Most users won't call this directly.
pub fn register_workflow_constructor<F>(workflow_name: String, constructor: F)
where
    F: Fn() -> Workflow + Send + Sync + 'static,
{
    let mut registry = GLOBAL_WORKFLOW_REGISTRY.write();
    registry.insert(workflow_name, Box::new(constructor));
    tracing::debug!("Successfully registered workflow constructor");
}

/// Get the global workflow registry
///
/// This provides access to the global workflow registry used by the macro system.
/// Most users won't need to call this directly.
pub fn global_workflow_registry() -> GlobalWorkflowRegistry {
    GLOBAL_WORKFLOW_REGISTRY.clone()
}

/// Get all workflows from the global registry
///
/// Returns instances of all workflows registered with the `workflow!` macro.
///
/// # Examples
///
/// ```rust
/// use cloacina::*;
///
/// let all_workflows = get_all_workflows();
/// for workflow in all_workflows {
///     println!("Found workflow: {}", workflow.name());
/// }
/// ```
pub fn get_all_workflows() -> Vec<Workflow> {
    let registry = GLOBAL_WORKFLOW_REGISTRY.read();
    registry.values().map(|constructor| constructor()).collect()
}
