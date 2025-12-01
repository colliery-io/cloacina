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

use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;

use super::types::{
    CargoToml, LibraryInfo, PackageInfo, PackageManifest, TaskInfo, CLOACINA_VERSION,
    EXECUTE_TASK_SYMBOL,
};

/// Generate a package manifest from Cargo.toml and compiled library
pub fn generate_manifest(
    cargo_toml: &CargoToml,
    so_path: &Path,
    target: &Option<String>,
    project_path: &Path,
) -> Result<PackageManifest> {
    let package = cargo_toml
        .package
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing package section in Cargo.toml"))?;

    // Extract architecture from target or use current platform
    let architecture = if let Some(target_triple) = target {
        target_triple.clone()
    } else {
        get_current_architecture()
    };

    // Get library filename
    let library_filename = so_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid so_path"))?
        .to_string_lossy()
        .to_string();

    let (tasks, graph_data, package_metadata) =
        extract_task_info_and_graph_from_library(&so_path, project_path)?;

    let manifest = PackageManifest {
        package: PackageInfo {
            name: package.name.clone(),
            version: package.version.clone(),
            description: package_metadata
                .description
                .unwrap_or_else(|| format!("Packaged workflow: {}", package.name)),
            author: package_metadata.author,
            workflow_fingerprint: package_metadata.workflow_fingerprint,
            cloacina_version: CLOACINA_VERSION.to_string(),
        },
        library: LibraryInfo {
            filename: library_filename,
            symbols: vec![EXECUTE_TASK_SYMBOL.to_string()],
            architecture,
        },
        tasks,
        execution_order: vec![], // TODO: Generate from task dependencies based on extracted tasks
        graph: graph_data,
    };

    Ok(manifest)
}

/// Package metadata extracted from the FFI
#[derive(Debug, Clone)]
pub(crate) struct PackageMetadata {
    pub description: Option<String>,
    pub author: Option<String>,
    pub workflow_fingerprint: Option<String>,
}

/// Extract task information and graph data from a compiled library using FFI metadata functions
fn extract_task_info_and_graph_from_library(
    so_path: &Path,
    project_path: &Path,
) -> Result<(
    Vec<TaskInfo>,
    Option<crate::WorkflowGraphData>,
    PackageMetadata,
)> {
    // Define the C structures that match the macro-generated ones
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    struct CTaskMetadata {
        index: u32,
        local_id: *const std::os::raw::c_char,
        namespaced_id_template: *const std::os::raw::c_char,
        dependencies_json: *const std::os::raw::c_char,
        description: *const std::os::raw::c_char,
        source_location: *const std::os::raw::c_char,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    struct CPackageTasks {
        task_count: u32,
        tasks: *const CTaskMetadata,
        package_name: *const std::os::raw::c_char,
        package_description: *const std::os::raw::c_char,
        package_author: *const std::os::raw::c_char,
        workflow_fingerprint: *const std::os::raw::c_char,
        graph_data_json: *const std::os::raw::c_char,
    }

    // Load the compiled library
    let lib = unsafe {
        libloading::Library::new(so_path).with_context(|| {
            format!(
                "Failed to load library for metadata extraction: {:?}",
                so_path
            )
        })?
    };

    // Try to find a metadata function - first try the standard name
    let get_metadata = unsafe {
        // Try standard name first
        match lib
            .get::<unsafe extern "C" fn() -> *const CPackageTasks>(b"cloacina_get_task_metadata")
        {
            Ok(func) => func,
            Err(_) => {
                // If that fails, try to find package-specific functions by reading package names from Cargo.toml
                let cargo_toml_path = project_path.join("Cargo.toml");
                let _cargo_content = std::fs::read_to_string(&cargo_toml_path)
                    .context("Failed to read Cargo.toml for package name extraction")?;

                // Look for packaged_workflow attributes in source files to find package names
                let package_names = extract_package_names_from_source(project_path)?;

                let mut found_func = None;
                for package_name in package_names {
                    let normalized_name = package_name
                        .replace("-", "_")
                        .replace(" ", "_")
                        .to_lowercase();
                    let func_name = format!("cloacina_get_task_metadata_{}\0", normalized_name);

                    if let Ok(func) = lib
                        .get::<unsafe extern "C" fn() -> *const CPackageTasks>(func_name.as_bytes())
                    {
                        found_func = Some(func);
                        break;
                    }
                }

                found_func
                    .ok_or_else(|| anyhow::anyhow!("No task metadata function found in library"))?
            }
        }
    };

    // Call the metadata function
    let package_tasks_ptr = unsafe { get_metadata() };

    if package_tasks_ptr.is_null() {
        return Ok((
            vec![],
            None,
            PackageMetadata {
                description: None,
                author: None,
                workflow_fingerprint: None,
            },
        ));
    }

    let package_tasks = unsafe { &*package_tasks_ptr };

    // Extract graph data JSON if available
    let graph_data = if !package_tasks.graph_data_json.is_null() {
        let graph_json_str = unsafe {
            std::ffi::CStr::from_ptr(package_tasks.graph_data_json)
                .to_str()
                .unwrap_or("{}")
        };

        // Parse the JSON string into WorkflowGraphData
        match serde_json::from_str::<crate::WorkflowGraphData>(graph_json_str) {
            Ok(graph) => Some(graph),
            Err(e) => {
                eprintln!("Warning: Failed to parse graph data: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Convert C strings and data to Rust structures
    let mut tasks = Vec::new();

    if package_tasks.task_count > 0 && !package_tasks.tasks.is_null() {
        let tasks_slice = unsafe {
            std::slice::from_raw_parts(package_tasks.tasks, package_tasks.task_count as usize)
        };

        for (index, task_metadata) in tasks_slice.iter().enumerate() {
            let local_id = unsafe {
                std::ffi::CStr::from_ptr(task_metadata.local_id)
                    .to_str()
                    .unwrap_or("unknown")
                    .to_string()
            };

            let description = unsafe {
                std::ffi::CStr::from_ptr(task_metadata.description)
                    .to_str()
                    .unwrap_or("")
                    .to_string()
            };

            let source_location = unsafe {
                std::ffi::CStr::from_ptr(task_metadata.source_location)
                    .to_str()
                    .unwrap_or("")
                    .to_string()
            };

            let dependencies_json = unsafe {
                std::ffi::CStr::from_ptr(task_metadata.dependencies_json)
                    .to_str()
                    .unwrap_or("[]")
            };

            // Parse dependencies JSON
            let dependencies: Vec<String> =
                serde_json::from_str(dependencies_json).unwrap_or_else(|_| vec![]);

            tasks.push(TaskInfo {
                index: index as u32,
                id: local_id,
                dependencies,
                description,
                source_location,
            });
        }
    }

    // Extract package metadata
    let package_description = if !package_tasks.package_description.is_null() {
        unsafe {
            std::ffi::CStr::from_ptr(package_tasks.package_description)
                .to_str()
                .ok()
                .map(|s| s.to_string())
        }
    } else {
        None
    };

    let package_author = if !package_tasks.package_author.is_null() {
        unsafe {
            std::ffi::CStr::from_ptr(package_tasks.package_author)
                .to_str()
                .ok()
                .map(|s| s.to_string())
        }
    } else {
        None
    };

    let workflow_fingerprint = if !package_tasks.workflow_fingerprint.is_null() {
        unsafe {
            std::ffi::CStr::from_ptr(package_tasks.workflow_fingerprint)
                .to_str()
                .ok()
                .map(|s| s.to_string())
        }
    } else {
        None
    };

    let package_metadata = PackageMetadata {
        description: package_description,
        author: package_author,
        workflow_fingerprint,
    };

    Ok((tasks, graph_data, package_metadata))
}

/// Extract package names from source files by looking for #[packaged_workflow] attributes
pub(crate) fn extract_package_names_from_source(project_path: &Path) -> Result<Vec<String>> {
    let src_path = project_path.join("src");
    let mut package_names = Vec::new();

    // Regex to find packaged_workflow attributes and extract package names
    let packaged_workflow_regex =
        Regex::new(r#"#\[packaged_workflow\s*\(\s*[^)]*package\s*=\s*"([^"]+)"[^)]*\)\s*\]"#)
            .expect("Failed to compile regex");

    // Walk through .rs files in src directory
    for entry in std::fs::read_dir(&src_path)
        .with_context(|| format!("Failed to read src directory: {:?}", src_path))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read file: {:?}", path))?;

            for captures in packaged_workflow_regex.captures_iter(&content) {
                if let Some(package_name) = captures.get(1) {
                    package_names.push(package_name.as_str().to_string());
                }
            }
        }
    }

    Ok(package_names)
}

pub(crate) fn get_current_architecture() -> String {
    // Use the current host target
    std::env::consts::ARCH.to_string()
}
