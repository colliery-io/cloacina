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

//! Indirection layer between the reconciler and the Python runtime.
//!
//! Python-language workflow packages need a pyo3-backed runtime to import
//! user code and register tasks. That runtime lives behind the
//! [`PythonRuntime`] trait so it can move into a separate crate
//! (`cloacina-python`, CLOACI-T-0529) — binaries that don't execute
//! Python (e.g. `cloacina-compiler`) simply don't link the impl and
//! therefore don't drag in pyo3 / `Python3.framework`.
//!
//! A process that needs Python support calls [`register_python_runtime`]
//! once at startup. The reconciler looks the registration up via
//! [`python_runtime`]; if nothing is registered, Python packages fail
//! with a clear `not attached` error at reconcile time.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use crate::computation_graph::scheduler::ComputationGraphDeclaration;
use crate::task::TaskNamespace;

/// Result of loading a Python workflow package.
pub struct LoadedPythonWorkflow {
    /// Tasks registered in the global task registry under their fully-qualified
    /// namespace. The reconciler tracks these so it can unregister on unload.
    pub task_namespaces: Vec<TaskNamespace>,
    /// Name of the workflow registered in the global workflow registry.
    pub workflow_name: String,
}

/// Runtime backing Python-language package loading.
///
/// Implementations run on the calling thread (typically inside a
/// `spawn_blocking`) — they're responsible for initializing the Python
/// interpreter as needed. Errors are flattened to `String` because the
/// reconciler wraps them in `RegistryError::RegistrationFailed` anyway.
pub trait PythonRuntime: Send + Sync {
    /// Extract a `.cloacina` archive, import the entry module, and register
    /// its tasks + triggers in the global registries for the given tenant.
    fn load_workflow_package(
        &self,
        archive_data: &[u8],
        staging_dir: &Path,
        tenant_id: &str,
    ) -> Result<LoadedPythonWorkflow, String>;

    /// Extract + import a Python computation graph package, then build the
    /// [`ComputationGraphDeclaration`] the `ReactiveScheduler` loads. Returns
    /// `None` if the imported module registered no executor for
    /// `graph_name` — matches the prior behavior where the caller silently
    /// moves on.
    fn load_cg_package(
        &self,
        archive_data: &[u8],
        staging_dir: &Path,
        tenant_id: &str,
        graph_name: &str,
        entry_module: &str,
        accumulator_overrides: &[cloacina_workflow_plugin::types::AccumulatorConfig],
    ) -> Result<Option<ComputationGraphDeclaration>, String>;
}

static PYTHON_RUNTIME: OnceLock<Arc<dyn PythonRuntime>> = OnceLock::new();

/// Install a [`PythonRuntime`] implementation for this process. Only the
/// first call wins — subsequent calls are silently ignored. Processes with
/// no Python responsibility (e.g. `cloacina-compiler`) simply never call
/// this and Python packages fail at reconcile time with a clear error.
pub fn register_python_runtime(runtime: Arc<dyn PythonRuntime>) {
    let _ = PYTHON_RUNTIME.set(runtime);
}

/// Fetch the registered [`PythonRuntime`], if any. Returns `None` when no
/// runtime is attached to this process.
pub fn python_runtime() -> Option<Arc<dyn PythonRuntime>> {
    PYTHON_RUNTIME.get().cloned()
}
