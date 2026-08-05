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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::computation_graph::scheduler::ComputationGraphDeclaration;
use crate::runtime::Runtime;
use crate::task::TaskNamespace;

/// One task in a loaded Python workflow, with its dependency edges — captured
/// host-side (from the scoped Runtime, before it is dropped) so the reconciler
/// can persist the task DAG into package metadata and the UI can render it like
/// Rust workflows. (CLOACI-T-0672)
pub struct PythonTaskNode {
    /// Local task id (e.g. `"finish"`).
    pub id: String,
    /// Local ids of the tasks this task depends on (e.g. `["prepare"]`).
    pub dependencies: Vec<String>,
}

/// Result of loading a Python workflow package.
pub struct LoadedPythonWorkflow {
    /// Tasks registered in the global task registry under their fully-qualified
    /// namespace. The reconciler tracks these so it can unregister on unload.
    pub task_namespaces: Vec<TaskNamespace>,
    /// Per-task dependency edges (local ids), for persisting/rendering the task
    /// DAG. Mirrors the Rust path's `PackageMetadata.tasks`. (CLOACI-T-0672)
    pub tasks: Vec<PythonTaskNode>,
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
        runtime: &Arc<Runtime>,
    ) -> Result<LoadedPythonWorkflow, String>;

    /// Extract + import a Python computation graph package, then build the
    /// [`ComputationGraphDeclaration`] the `ComputationGraphScheduler` loads. Returns
    /// `None` if the imported module registered no executor for
    /// `graph_name` — matches the prior behavior where the caller silently
    /// moves on.
    #[allow(clippy::too_many_arguments)]
    fn load_cg_package(
        &self,
        archive_data: &[u8],
        staging_dir: &Path,
        tenant_id: &str,
        graph_name: &str,
        entry_module: &str,
        accumulator_overrides: &[cloacina_workflow_plugin::types::AccumulatorConfig],
        runtime: &Arc<Runtime>,
    ) -> Result<Option<ComputationGraphDeclaration>, String>;
}

static PYTHON_RUNTIME: OnceLock<Arc<dyn PythonRuntime>> = OnceLock::new();

/// Install a [`PythonRuntime`] implementation for this process. Only the
/// first call wins — subsequent calls are silently ignored. Processes with
/// no Python responsibility (e.g. `cloacina-compiler`) simply never call
/// this and Python packages fail at reconcile time with a clear error.
pub fn register_python_runtime(runtime: Arc<dyn PythonRuntime>) {
    let _ = PYTHON_RUNTIME.set(runtime);
    init_python_runtime_health_metrics();
}

/// Fetch the registered [`PythonRuntime`], if any. Returns `None` when no
/// runtime is attached to this process.
pub fn python_runtime() -> Option<Arc<dyn PythonRuntime>> {
    PYTHON_RUNTIME.get().cloned()
}

// ---------------------------------------------------------------------------
// Wedged-runtime health surface (CLOACI-T-0919)
// ---------------------------------------------------------------------------
//
// CPython is a single embedded interpreter per process. If user code hangs at
// module scope during a package import AND cannot be interrupted (a C-level
// hold that never releases the GIL), the Python subsystem is disabled
// process-wide until restart — every later import blocks on a GIL that will
// never be handed back. There is no in-process recovery for that state, so the
// floor requirement is that it is LOUD: a sticky process-global flag with a
// human-readable reason, a gauge, and a readiness failure. Operators see the
// replica as unhealthy instead of watching Python packages silently stop
// loading.
//
// The flag is deliberately one-way (latching). A wedged interpreter does not
// un-wedge; anything that "succeeds" afterwards succeeded despite the wedge,
// not because it cleared.

static PYTHON_RUNTIME_WEDGED: AtomicBool = AtomicBool::new(false);
static PYTHON_RUNTIME_WEDGED_REASON: Mutex<Option<String>> = Mutex::new(None);

/// Latch the process-global "Python runtime is wedged" flag.
///
/// Called by `cloacina-python` when an import thread could not be interrupted
/// and had to be abandoned while (presumably) holding the GIL. Emits the
/// `cloacina_python_runtime_wedged` gauge and an `error!` naming the cause.
/// Repeat calls keep the FIRST reason — that's the one that actually wedged
/// the interpreter; later ones are collateral.
pub fn mark_python_runtime_wedged(reason: impl Into<String>) {
    let reason = reason.into();
    let first = !PYTHON_RUNTIME_WEDGED.swap(true, Ordering::SeqCst);
    if first {
        if let Ok(mut slot) = PYTHON_RUNTIME_WEDGED_REASON.lock() {
            *slot = Some(reason.clone());
        }
    }
    metrics::gauge!("cloacina_python_runtime_wedged").set(1.0);
    tracing::error!(
        reason = %reason,
        first_wedge = first,
        "PYTHON RUNTIME WEDGED — the embedded interpreter is holding the GIL and \
         cannot be recovered in-process. All further Python package loads in this \
         process will hang or fail; /ready now reports not-ready. Restart the \
         process to recover, and remove/fix the offending package first."
    );
}

/// Whether the embedded Python interpreter is known-wedged in this process.
pub fn is_python_runtime_wedged() -> bool {
    PYTHON_RUNTIME_WEDGED.load(Ordering::SeqCst)
}

/// Reason the Python runtime is wedged, or `None` when it is healthy (or when
/// Python is not used in this process at all).
pub fn python_runtime_wedged_reason() -> Option<String> {
    if !is_python_runtime_wedged() {
        return None;
    }
    PYTHON_RUNTIME_WEDGED_REASON
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .or_else(|| Some("python runtime wedged (reason unavailable)".to_string()))
}

/// Record that a hung Python import thread WAS successfully interrupted
/// (`PyThreadState_SetAsyncExc` landed and the thread joined). The package load
/// failed but the interpreter stayed healthy — the good outcome of the ladder.
pub fn record_python_import_interrupted() {
    metrics::counter!("cloacina_python_import_interrupted_total").increment(1);
}

/// Clear the wedged flag. **Test support only.**
///
/// The flag is latching in production for a reason (a wedged interpreter does
/// not heal), but it is process-global, so a test that sets it would poison
/// every later test in the same binary. Downstream crates (`cloacina-server`)
/// need this to assert the `/ready` effect and then put the process back.
#[doc(hidden)]
pub fn reset_python_runtime_wedged_for_tests() {
    PYTHON_RUNTIME_WEDGED.store(false, Ordering::SeqCst);
    if let Ok(mut slot) = PYTHON_RUNTIME_WEDGED_REASON.lock() {
        *slot = None;
    }
    metrics::gauge!("cloacina_python_runtime_wedged").set(0.0);
}

/// Publish the wedged gauge as `0` so the series exists on healthy processes
/// (a gauge that only appears when broken is invisible to alert rules that
/// need a baseline). Called once when a Python runtime is registered.
pub fn init_python_runtime_health_metrics() {
    if !is_python_runtime_wedged() {
        metrics::gauge!("cloacina_python_runtime_wedged").set(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The flag plumbing: set → read → reason, latching on first reason.
    /// (Process-global by design; this is the only test that touches it.)
    #[test]
    fn wedged_flag_latches_first_reason() {
        reset_python_runtime_wedged_for_tests();
        assert!(!is_python_runtime_wedged());
        assert_eq!(python_runtime_wedged_reason(), None);

        mark_python_runtime_wedged("python runtime wedged by package alpha import hang");
        assert!(is_python_runtime_wedged());
        assert_eq!(
            python_runtime_wedged_reason().as_deref(),
            Some("python runtime wedged by package alpha import hang")
        );

        // A second wedge does not overwrite the original cause.
        mark_python_runtime_wedged("python runtime wedged by package beta import hang");
        assert_eq!(
            python_runtime_wedged_reason().as_deref(),
            Some("python runtime wedged by package alpha import hang")
        );

        // Leave the process healthy for anything else in this binary.
        reset_python_runtime_wedged_for_tests();
        assert!(!is_python_runtime_wedged());
    }
}
