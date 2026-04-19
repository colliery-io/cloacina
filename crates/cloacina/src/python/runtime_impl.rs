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

//! Wires the existing `crate::python::*` helpers into the
//! [`crate::python_runtime::PythonRuntime`] trait so the reconciler can
//! dispatch without a direct `crate::python` reference. Phase A of
//! CLOACI-T-0529 — the impl still lives in the `cloacina` crate; Phase B
//! lifts it into `cloacina-python` and drops pyo3 from `cloacina`
//! entirely.

use std::path::Path;
use std::sync::Arc;

use crate::computation_graph::scheduler::ComputationGraphDeclaration;
use crate::python_runtime::{register_python_runtime, LoadedPythonWorkflow, PythonRuntime};
use crate::task::TaskNamespace;

/// The bundled-with-core implementation of [`PythonRuntime`]. Zero state —
/// every method thin-wraps the module-level helpers in `crate::python` and
/// `crate::registry::loader::python_loader`.
pub struct CoreCloacinaPythonRuntime;

impl PythonRuntime for CoreCloacinaPythonRuntime {
    fn load_workflow_package(
        &self,
        archive_data: &[u8],
        staging_dir: &Path,
        tenant_id: &str,
    ) -> Result<LoadedPythonWorkflow, String> {
        std::fs::create_dir_all(staging_dir)
            .map_err(|e| format!("Failed to create Python staging dir: {}", e))?;

        let extracted = crate::registry::loader::python_loader::extract_python_package(
            archive_data,
            staging_dir,
        )
        .map_err(|e| format!("Failed to extract Python package: {}", e))?;

        pyo3::prepare_freethreaded_python();
        let task_namespaces: Vec<TaskNamespace> =
            crate::python::loader::import_and_register_python_workflow_named(
                &extracted.workflow_dir,
                &extracted.vendor_dir,
                &extracted.entry_module,
                &extracted.package_name,
                &extracted.workflow_name,
                tenant_id,
            )
            .map_err(|e| format!("Python workflow import failed: {}", e))?;

        Ok(LoadedPythonWorkflow {
            task_namespaces,
            workflow_name: extracted.workflow_name,
        })
    }

    fn load_cg_package(
        &self,
        archive_data: &[u8],
        staging_dir: &Path,
        tenant_id: &str,
        graph_name: &str,
        entry_module: &str,
        accumulator_overrides: &[cloacina_workflow_plugin::types::AccumulatorConfig],
    ) -> Result<Option<ComputationGraphDeclaration>, String> {
        std::fs::create_dir_all(staging_dir)
            .map_err(|e| format!("Failed to create Python CG staging dir: {}", e))?;

        let extracted = crate::registry::loader::python_loader::extract_python_package(
            archive_data,
            staging_dir,
        )
        .map_err(|e| format!("Failed to extract Python CG package: {}", e))?;

        pyo3::prepare_freethreaded_python();
        crate::python::loader::import_python_computation_graph(
            &extracted.workflow_dir,
            &extracted.vendor_dir,
            entry_module,
            graph_name,
        )
        .map_err(|e| format!("Python CG import failed: {}", e))?;

        Ok(
            crate::python::computation_graph::build_python_graph_declaration(
                graph_name,
                Some(tenant_id.to_string()),
                accumulator_overrides,
            ),
        )
    }
}

/// Register [`CoreCloacinaPythonRuntime`] as this process's
/// [`PythonRuntime`]. Idempotent — subsequent calls are no-ops. Intended
/// to be called from a process that intentionally wants Python support
/// bundled into its own binary (e.g. `cloacina-server` today).
pub fn install_in_process() {
    register_python_runtime(Arc::new(CoreCloacinaPythonRuntime));
}

/// Phase A of CLOACI-T-0529 keeps behavior identical: since the impl
/// still lives inside the `cloacina` crate (pyo3 still linked), install
/// it eagerly so every process that depends on `cloacina` behaves like
/// it did before the trait was introduced. Phase B deletes this `ctor`
/// and the server registers the impl explicitly at startup.
#[ctor::ctor]
fn auto_install_core_runtime() {
    install_in_process();
}
