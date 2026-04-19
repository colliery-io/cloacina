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

//! `cloacina-python`'s implementation of `cloacina::python_runtime::PythonRuntime`.
//!
//! A process that wants Python support (the server today, any future
//! Python-enabled service) calls [`install`] at startup to register this
//! runtime into cloacina core's dispatch slot. After that, uploaded
//! Python packages are loaded through the runtime just like Rust ones
//! are loaded through fidius.

use std::path::Path;
use std::sync::Arc;

use cloacina::computation_graph::scheduler::ComputationGraphDeclaration;
use cloacina::python_runtime::{register_python_runtime, LoadedPythonWorkflow, PythonRuntime};
use cloacina::task::TaskNamespace;

/// Zero-state runtime — every method thin-wraps the crate-local helpers.
pub struct CloacinaPythonRuntime;

impl PythonRuntime for CloacinaPythonRuntime {
    fn load_workflow_package(
        &self,
        archive_data: &[u8],
        staging_dir: &Path,
        tenant_id: &str,
    ) -> Result<LoadedPythonWorkflow, String> {
        std::fs::create_dir_all(staging_dir)
            .map_err(|e| format!("Failed to create Python staging dir: {}", e))?;

        let extracted = crate::package_loader::extract_python_package(archive_data, staging_dir)
            .map_err(|e| format!("Failed to extract Python package: {}", e))?;

        pyo3::prepare_freethreaded_python();
        let task_namespaces: Vec<TaskNamespace> =
            crate::loader::import_and_register_python_workflow_named(
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

        let extracted = crate::package_loader::extract_python_package(archive_data, staging_dir)
            .map_err(|e| format!("Failed to extract Python CG package: {}", e))?;

        pyo3::prepare_freethreaded_python();
        crate::loader::import_python_computation_graph(
            &extracted.workflow_dir,
            &extracted.vendor_dir,
            entry_module,
            graph_name,
        )
        .map_err(|e| format!("Python CG import failed: {}", e))?;

        Ok(crate::computation_graph::build_python_graph_declaration(
            graph_name,
            Some(tenant_id.to_string()),
            accumulator_overrides,
        ))
    }
}

/// Install [`CloacinaPythonRuntime`] as this process's Python runtime.
/// Idempotent — subsequent calls are no-ops. Call once at startup from
/// any binary that wants to load Python packages.
pub fn install() {
    register_python_runtime(Arc::new(CloacinaPythonRuntime));
}
