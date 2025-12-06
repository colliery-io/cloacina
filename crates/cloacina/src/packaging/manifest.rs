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
use once_cell::sync::Lazy;
use regex::Regex;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::Path;
use thiserror::Error;

use super::types::{
    CargoToml, LibraryInfo, PackageInfo, PackageManifest, TaskInfo, CLOACINA_VERSION,
    EXECUTE_TASK_SYMBOL,
};

/// Maximum number of tasks allowed in a single package.
/// This limit prevents resource exhaustion from malformed packages.
const MAX_TASKS: usize = 10_000;

/// Statically compiled regex for matching packaged_workflow attributes.
/// Compiled once at first use, avoiding runtime compilation overhead.
static PACKAGED_WORKFLOW_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"#\[packaged_workflow\s*\(\s*[^)]*package\s*=\s*"([^"]+)"[^)]*\)\s*\]"#)
        .expect("Invalid packaged_workflow regex pattern - this is a compile-time bug")
});

/// Errors that can occur during manifest extraction from FFI.
///
/// These errors represent various failure modes when reading metadata
/// from compiled workflow libraries via FFI.
#[derive(Debug, Error)]
pub enum ManifestError {
    /// A required pointer was null
    #[error("Null pointer encountered for field: {field}")]
    NullPointer { field: &'static str },

    /// A pointer had incorrect alignment for its type
    #[error("Misaligned pointer for field: {field}")]
    MisalignedPointer { field: &'static str },

    /// A C string pointer was null when a string was expected
    #[error("Null string pointer for field: {field}")]
    NullString { field: String },

    /// A C string contained invalid UTF-8 data
    #[error("Invalid UTF-8 in field '{field}': {source}")]
    InvalidUtf8 {
        field: String,
        #[source]
        source: std::str::Utf8Error,
    },

    /// Failed to parse dependencies JSON for a task
    #[error("Invalid dependencies JSON for task '{task_id}': {source}")]
    InvalidDependencies {
        task_id: String,
        #[source]
        source: serde_json::Error,
    },

    /// The task slice pointer was null but count was non-zero
    #[error("Null task slice with non-zero count ({count})")]
    NullTaskSlice { count: usize },

    /// The task count exceeded the maximum allowed limit
    #[error("Task count {count} exceeds maximum allowed ({max})")]
    TooManyTasks { count: usize, max: usize },

    /// Failed to parse graph data JSON
    #[error("Invalid graph data JSON: {source}")]
    InvalidGraphData {
        #[source]
        source: serde_json::Error,
    },

    /// Library loading or symbol resolution failed
    #[error("Library error: {message}")]
    LibraryError { message: String },
}

/// Safely converts a C string pointer to a Rust String.
///
/// # Arguments
/// * `ptr` - Pointer to a null-terminated C string
/// * `field_name` - Name of the field for error reporting
///
/// # Returns
/// * `Ok(String)` - The converted string
/// * `Err(ManifestError)` - If the pointer is null or contains invalid UTF-8
///
/// # Safety
/// The caller must ensure that if the pointer is non-null, it points to
/// a valid null-terminated C string that remains valid for the duration
/// of this call.
pub(crate) fn safe_cstr_to_string(
    ptr: *const c_char,
    field_name: &str,
) -> Result<String, ManifestError> {
    if ptr.is_null() {
        return Err(ManifestError::NullString {
            field: field_name.to_string(),
        });
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(|s| s.to_string())
        .map_err(|e| ManifestError::InvalidUtf8 {
            field: field_name.to_string(),
            source: e,
        })
}

/// Safely converts a C string pointer to an optional Rust String.
///
/// Returns `Ok(None)` if the pointer is null, `Ok(Some(String))` if valid,
/// or an error if the string contains invalid UTF-8.
///
/// # Safety
/// The caller must ensure that if the pointer is non-null, it points to
/// a valid null-terminated C string that remains valid for the duration
/// of this call.
pub(crate) fn safe_cstr_to_option_string(
    ptr: *const c_char,
    field_name: &str,
) -> Result<Option<String>, ManifestError> {
    if ptr.is_null() {
        return Ok(None);
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(|s| Some(s.to_string()))
        .map_err(|e| ManifestError::InvalidUtf8 {
            field: field_name.to_string(),
            source: e,
        })
}

/// Validates and dereferences a pointer to a type T.
///
/// # Safety
/// The caller must ensure the pointer, if non-null, points to a valid
/// instance of T that remains valid for the lifetime of the returned reference.
pub(crate) unsafe fn validate_ptr<'a, T>(
    ptr: *const T,
    field_name: &'static str,
) -> Result<&'a T, ManifestError> {
    if ptr.is_null() {
        return Err(ManifestError::NullPointer { field: field_name });
    }
    // Validate alignment
    if (ptr as usize) % std::mem::align_of::<T>() != 0 {
        return Err(ManifestError::MisalignedPointer { field: field_name });
    }
    Ok(&*ptr)
}

/// Validates and creates a slice from a pointer and count.
///
/// # Safety
/// The caller must ensure that if `ptr` is non-null, it points to
/// `count` consecutive valid instances of T.
pub(crate) unsafe fn validate_slice<'a, T>(
    ptr: *const T,
    count: usize,
    field_name: &'static str,
) -> Result<&'a [T], ManifestError> {
    if count > MAX_TASKS {
        return Err(ManifestError::TooManyTasks {
            count,
            max: MAX_TASKS,
        });
    }
    if ptr.is_null() && count > 0 {
        return Err(ManifestError::NullTaskSlice { count });
    }
    if ptr.is_null() {
        // count == 0, return empty slice
        return Ok(&[]);
    }
    // Validate alignment
    if (ptr as usize) % std::mem::align_of::<T>() != 0 {
        return Err(ManifestError::MisalignedPointer { field: field_name });
    }
    Ok(std::slice::from_raw_parts(ptr, count))
}

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
        extract_task_info_and_graph_from_library(so_path, project_path)?;

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

    let package_tasks = unsafe { validate_ptr(package_tasks_ptr, "package_tasks") }
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Extract graph data JSON if available
    let graph_data = if !package_tasks.graph_data_json.is_null() {
        let graph_json_str = safe_cstr_to_string(package_tasks.graph_data_json, "graph_data_json")
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Parse the JSON string into WorkflowGraphData
        let graph = serde_json::from_str::<crate::WorkflowGraphData>(&graph_json_str)
            .map_err(|e| ManifestError::InvalidGraphData { source: e })
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Some(graph)
    } else {
        None
    };

    // Convert C strings and data to Rust structures
    let mut tasks = Vec::new();

    let task_count = package_tasks.task_count as usize;
    let tasks_slice = unsafe { validate_slice(package_tasks.tasks, task_count, "tasks") }
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    for (index, task_metadata) in tasks_slice.iter().enumerate() {
        let local_id = safe_cstr_to_string(task_metadata.local_id, "local_id")
            .map_err(|e| anyhow::anyhow!("Task {}: {}", index, e))?;

        let description = safe_cstr_to_string(task_metadata.description, "description")
            .map_err(|e| anyhow::anyhow!("Task {}: {}", index, e))?;

        let source_location = safe_cstr_to_string(task_metadata.source_location, "source_location")
            .map_err(|e| anyhow::anyhow!("Task {}: {}", index, e))?;

        let dependencies_json =
            safe_cstr_to_string(task_metadata.dependencies_json, "dependencies_json")
                .map_err(|e| anyhow::anyhow!("Task {}: {}", index, e))?;

        // Parse dependencies JSON
        let dependencies: Vec<String> = serde_json::from_str(&dependencies_json)
            .map_err(|e| ManifestError::InvalidDependencies {
                task_id: local_id.clone(),
                source: e,
            })
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        tasks.push(TaskInfo {
            index: index as u32,
            id: local_id,
            dependencies,
            description,
            source_location,
        });
    }

    // Extract package metadata
    let package_description =
        safe_cstr_to_option_string(package_tasks.package_description, "package_description")
            .map_err(|e| anyhow::anyhow!("{}", e))?;

    let package_author = safe_cstr_to_option_string(package_tasks.package_author, "package_author")
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let workflow_fingerprint =
        safe_cstr_to_option_string(package_tasks.workflow_fingerprint, "workflow_fingerprint")
            .map_err(|e| anyhow::anyhow!("{}", e))?;

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

    // Walk through .rs files in src directory
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

pub(crate) fn get_current_architecture() -> String {
    // Use the current host target
    std::env::consts::ARCH.to_string()
}
