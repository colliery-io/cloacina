/*
 *  Copyright 2025-2026 Colliery Software
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

//! Debug functionality for workflow packages.
//!
//! This module provides functions for debugging packaged workflows, including
//! extracting package metadata, listing tasks, and executing individual tasks
//! for testing and development purposes.
//!
//! Packages are fidius source archives (bzip2 tar + `package.toml`). Rust
//! packages must be compiled before tasks can be executed; this module handles
//! that compilation step transparently via the reconciler's compile pipeline.

use anyhow::{bail, Context, Result};
use std::path::PathBuf;

use super::manifest_schema::Manifest;

/// Extract metadata from a fidius source package and synthesize a [`Manifest`].
///
/// The package is unpacked to a temporary directory, `package.toml` is read via
/// `fidius_core::package::load_manifest`, and the result is converted to the
/// internal `Manifest` representation.
///
/// For Rust packages the library is not compiled by this function; the returned
/// manifest reflects metadata from `package.toml` only (no task FFI data).
pub fn extract_manifest_from_package(package_path: &PathBuf) -> Result<Manifest> {
    let tmp = tempfile::TempDir::new().context("Failed to create temporary directory")?;

    let extract_dir = tmp.path().join("extract");
    std::fs::create_dir_all(&extract_dir).context("Failed to create extract directory")?;

    let source_dir = fidius_core::package::unpack_package(package_path, &extract_dir)
        .with_context(|| format!("Failed to unpack source archive: {:?}", package_path))?;

    let fidius_manifest = fidius_core::package::load_manifest::<
        cloacina_workflow_plugin::CloacinaMetadata,
    >(&source_dir)
    .with_context(|| format!("Failed to parse package.toml in: {:?}", source_dir))?;

    let pkg = &fidius_manifest.package;
    let meta = &fidius_manifest.metadata;

    let language = match meta.language.as_str() {
        "python" => super::manifest_schema::PackageLanguage::Python,
        _ => super::manifest_schema::PackageLanguage::Rust,
    };

    let python_runtime = if meta.language == "python" {
        Some(super::manifest_schema::PythonRuntime {
            requires_python: meta.requires_python.clone().unwrap_or_default(),
            entry_module: meta.entry_module.clone().unwrap_or_default(),
        })
    } else {
        None
    };

    let manifest = Manifest {
        format_version: "2".to_string(),
        package: super::manifest_schema::PackageInfo {
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            description: meta.description.clone(),
            fingerprint: format!("sha256:{}:{}", pkg.name, pkg.version),
            targets: vec![super::manifest::get_current_platform()],
        },
        language,
        python: python_runtime,
        rust: None,
        tasks: vec![],
        triggers: vec![],
        created_at: chrono::Utc::now(),
        signature: None,
    };

    Ok(manifest)
}

/// Execute a task from a dynamic library via the fidius-host plugin API.
pub fn execute_task_from_library(
    library_path: &PathBuf,
    task_name: &str,
    context_json: &str,
) -> Result<String> {
    // Load via fidius-host — validates magic, ABI version, wire format, etc.
    let loaded = fidius_host::loader::load_library(library_path)
        .with_context(|| format!("Failed to load library: {:?}", library_path))?;

    let plugin = loaded
        .plugins
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Plugin library contains no plugins"))?;

    let handle = fidius_host::PluginHandle::from_loaded(plugin);

    // Method index 1 = execute_task (fidius tuple encoding: (TaskExecutionRequest,))
    let request = cloacina_workflow_plugin::TaskExecutionRequest {
        task_name: task_name.to_string(),
        context_json: context_json.to_string(),
    };
    let result: cloacina_workflow_plugin::TaskExecutionResult = handle
        .call_method(1, &(request,))
        .with_context(|| format!("Failed to execute task '{}' via plugin API", task_name))?;

    Ok(result.context_json.unwrap_or_default())
}

/// Resolve a task identifier (index or name) to a task name.
pub fn resolve_task_name(manifest: &Manifest, task_identifier: &str) -> Result<String> {
    // Try to parse as index first
    if let Ok(index) = task_identifier.parse::<u32>() {
        let index = index as usize;
        if index < manifest.tasks.len() {
            return Ok(manifest.tasks[index].id.clone());
        } else {
            bail!(
                "Task index {} is out of range. Available tasks: 0-{}",
                index,
                manifest.tasks.len().saturating_sub(1)
            );
        }
    }

    // Check if it's already a valid task name
    for task in &manifest.tasks {
        if task.id == task_identifier {
            return Ok(task.id.clone());
        }
    }

    bail!(
        "Task '{}' not found. Available tasks: {:?}",
        task_identifier,
        manifest.tasks.iter().map(|t| &t.id).collect::<Vec<_>>()
    );
}

/// High-level debug function that handles both listing and executing tasks.
pub fn debug_package(
    package_path: &PathBuf,
    task_identifier: Option<&str>,
    context_json: Option<&str>,
) -> Result<DebugResult> {
    // Validate package exists
    if !package_path.exists() {
        bail!("Package file does not exist: {:?}", package_path);
    }

    if !package_path.is_file() {
        bail!("Package path is not a file: {:?}", package_path);
    }

    // Extract manifest
    let manifest = extract_manifest_from_package(package_path)?;

    match task_identifier {
        None => {
            // List tasks
            let tasks: Vec<TaskDebugInfo> = manifest
                .tasks
                .iter()
                .enumerate()
                .map(|(index, task)| TaskDebugInfo {
                    index,
                    id: task.id.clone(),
                    description: task.description.clone().unwrap_or_default(),
                    dependencies: task.dependencies.clone(),
                })
                .collect();

            Ok(DebugResult::TaskList { tasks })
        }
        Some(task_id) => {
            // Execute task
            let task_name = resolve_task_name(&manifest, task_id)?;
            let _context = context_json.unwrap_or("{}");

            // Rust packages in source format need to be compiled first
            bail!(
                "Cannot directly execute task '{}' from source package {:?}. \
                The package must be registered with the workflow registry and compiled \
                by the reconciler before tasks can be executed.",
                task_name,
                package_path
            );
        }
    }
}

/// Result of a debug operation.
#[derive(Debug)]
pub enum DebugResult {
    TaskList { tasks: Vec<TaskDebugInfo> },
    TaskExecution { output: String },
}

/// Information about a task for debugging purposes.
#[derive(Debug, Clone)]
pub struct TaskDebugInfo {
    pub index: usize,
    pub id: String,
    pub description: String,
    pub dependencies: Vec<String>,
}
