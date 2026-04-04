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

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;
use thiserror::Error;

use super::manifest_schema::{Manifest, PackageInfo, PackageLanguage, RustRuntime, TaskDefinition};
use super::types::CargoToml;

/// Statically compiled regex for matching workflow attributes.
/// Matches both `#[workflow(name = "...")]` (new) and `#[packaged_workflow(package = "...")]` (legacy).
#[allow(dead_code)]
static PACKAGED_WORKFLOW_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"#\[(?:packaged_)?workflow\s*\(\s*[^)]*(?:package|name)\s*=\s*"([^"]+)"[^)]*\)\s*\]"#,
    )
    .expect("Invalid workflow regex pattern - this is a compile-time bug")
});

/// Errors that can occur during manifest extraction.
#[derive(Debug, Error)]
pub enum ManifestError {
    /// Failed to parse dependencies JSON for a task
    #[error("Invalid dependencies JSON for task '{task_id}': {source}")]
    InvalidDependencies {
        task_id: String,
        #[source]
        source: serde_json::Error,
    },

    /// Failed to parse graph data JSON
    #[error("Invalid graph data JSON: {source}")]
    InvalidGraphData {
        #[source]
        source: serde_json::Error,
    },

    /// Library loading or plugin call failed
    #[error("Library error: {message}")]
    LibraryError { message: String },
}

/// Generate a package manifest from Cargo.toml and compiled library.
///
/// Returns a `Manifest` — the unified manifest format used by both
/// Rust and Python packages.
pub fn generate_manifest(
    cargo_toml: &CargoToml,
    so_path: &Path,
    target: &Option<String>,
    project_path: &Path,
) -> Result<Manifest> {
    let package = cargo_toml
        .package
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing package section in Cargo.toml"))?;

    // Get library filename
    let library_filename = so_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid so_path"))?
        .to_string_lossy()
        .to_string();

    let (ffi_tasks, _graph_data, package_metadata) =
        extract_task_info_and_graph_from_library(so_path, project_path)?;

    // Determine target platform string
    let target_platform = if let Some(target_triple) = target {
        target_triple.clone()
    } else {
        get_current_platform()
    };

    // Build fingerprint from package name + version + workflow fingerprint
    let fingerprint = format!(
        "sha256:{}:{}:{}",
        package.name,
        package.version,
        package_metadata
            .workflow_fingerprint
            .as_deref()
            .unwrap_or("none")
    );

    // Convert FFI task info to TaskDefinition
    let tasks: Vec<TaskDefinition> = ffi_tasks
        .iter()
        .map(|t| TaskDefinition {
            id: t.id.clone(),
            function: "cloacina_execute_task".to_string(),
            dependencies: t.dependencies.clone(),
            description: if t.description.is_empty() {
                None
            } else {
                Some(t.description.clone())
            },
            retries: 0,
            timeout_seconds: None,
        })
        .collect();

    let manifest = Manifest {
        format_version: "2".to_string(),
        package: PackageInfo {
            name: package.name.clone(),
            version: package.version.clone(),
            description: package_metadata
                .description
                .or_else(|| Some(format!("Packaged workflow: {}", package.name))),
            fingerprint,
            targets: vec![target_platform],
        },
        language: PackageLanguage::Rust,
        python: None,
        rust: Some(RustRuntime {
            library_path: library_filename,
        }),
        tasks,
        triggers: vec![],
        created_at: chrono::Utc::now(),
        signature: None,
    };

    Ok(manifest)
}

/// Package metadata extracted from the plugin.
#[derive(Debug, Clone)]
pub(crate) struct PackageMetadata {
    pub description: Option<String>,
    pub _author: Option<String>,
    pub workflow_fingerprint: Option<String>,
}

/// Task information extracted from a cdylib via the fidius plugin API (internal type).
#[derive(Debug, Clone)]
struct FfiTaskInfo {
    pub _index: u32,
    pub id: String,
    pub dependencies: Vec<String>,
    pub description: String,
    pub _source_location: String,
}

/// Extract task information and graph data from a compiled library using the fidius plugin API.
fn extract_task_info_and_graph_from_library(
    so_path: &Path,
    project_path: &Path,
) -> Result<(
    Vec<FfiTaskInfo>,
    Option<crate::WorkflowGraphData>,
    PackageMetadata,
)> {
    // Load via fidius-host — validates magic, ABI version, wire format, etc.
    let loaded = fidius_host::loader::load_library(so_path).with_context(|| {
        format!(
            "Failed to load plugin library for metadata extraction: {:?}",
            so_path
        )
    })?;

    let plugin = loaded
        .plugins
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Plugin library {:?} contains no plugins", so_path))?;

    let handle = fidius_host::PluginHandle::from_loaded(plugin);

    // Method index 0 = get_task_metadata (zero-arg, encoded as empty tuple)
    let meta: cloacina_workflow_plugin::PackageTasksMetadata = handle
        .call_method(0, &())
        .with_context(|| format!("Failed to call get_task_metadata on library {:?}", so_path))?;

    // Parse graph data if present
    let graph_data = if let Some(ref json) = meta.graph_data_json {
        if json.trim().is_empty() {
            None
        } else {
            Some(
                serde_json::from_str::<crate::WorkflowGraphData>(json)
                    .map_err(|e| ManifestError::InvalidGraphData { source: e })
                    .map_err(|e| anyhow::anyhow!("{}", e))?,
            )
        }
    } else {
        None
    };

    // Convert tasks
    let mut tasks = Vec::new();
    for t in meta.tasks {
        tasks.push(FfiTaskInfo {
            _index: t.index,
            id: t.id,
            dependencies: t.dependencies,
            description: t.description,
            _source_location: t.source_location,
        });
    }

    let package_metadata = PackageMetadata {
        description: meta.package_description,
        _author: meta.package_author,
        workflow_fingerprint: meta.workflow_fingerprint,
    };

    // Suppress unused variable warning if project_path isn't needed further
    let _ = project_path;

    Ok((tasks, graph_data, package_metadata))
}

/// Extract package names from source files by looking for #[packaged_workflow] attributes.
#[allow(dead_code)]
pub(crate) fn extract_package_names_from_source(project_path: &Path) -> Result<Vec<String>> {
    let src_path = project_path.join("src");
    let mut package_names = Vec::new();

    for entry in std::fs::read_dir(&src_path)
        .with_context(|| format!("Failed to read src directory: {:?}", src_path))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read file: {:?}", path))?;

            for captures in PACKAGED_WORKFLOW_REGEX.captures_iter(&content) {
                if let Some(package_name) = captures.get(1) {
                    package_names.push(package_name.as_str().to_string());
                }
            }
        }
    }

    Ok(package_names)
}

pub(crate) fn get_current_platform() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let platform = match (os, arch) {
        ("macos", "aarch64") => "macos-arm64",
        ("macos", "x86_64") => "macos-x86_64",
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-arm64",
        _ => return format!("{}-{}", os, arch),
    };
    platform.to_string()
}

/// Kept for backward compatibility with external callers.
#[allow(dead_code)]
pub(crate) fn get_current_architecture() -> String {
    std::env::consts::ARCH.to_string()
}
