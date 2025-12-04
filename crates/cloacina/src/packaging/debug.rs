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

//! Debug functionality for workflow packages.
//!
//! This module provides functions for debugging packaged workflows, including
//! extracting package contents, listing tasks, and executing individual tasks
//! for testing and development purposes.

use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use libloading::{Library, Symbol};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use tar::Archive;

use super::types::PackageManifest;

const MANIFEST_FILENAME: &str = "manifest.json";
const EXECUTE_TASK_SYMBOL: &str = "cloacina_execute_task";

/// Extract the manifest from a package archive.
pub fn extract_manifest_from_package(package_path: &PathBuf) -> Result<PackageManifest> {
    let file = File::open(package_path)
        .with_context(|| format!("Failed to open package file: {:?}", package_path))?;

    let gz_decoder = GzDecoder::new(file);
    let mut archive = Archive::new(gz_decoder);

    for entry in archive.entries()? {
        let mut entry = entry.context("Failed to read archive entry")?;
        let path = entry.path().context("Failed to get entry path")?;

        if path == std::path::Path::new(MANIFEST_FILENAME) {
            let mut manifest_content = String::new();
            entry
                .read_to_string(&mut manifest_content)
                .context("Failed to read manifest.json content")?;

            let manifest: PackageManifest =
                serde_json::from_str(&manifest_content).context("Failed to parse manifest.json")?;

            return Ok(manifest);
        }
    }

    bail!("manifest.json not found in package archive")
}

/// Extract the dynamic library from a package archive to a temporary location.
pub fn extract_library_from_package(
    package_path: &PathBuf,
    manifest: &PackageManifest,
    temp_dir: &tempfile::TempDir,
) -> Result<PathBuf> {
    let file = File::open(package_path)
        .with_context(|| format!("Failed to open package file: {:?}", package_path))?;

    let gz_decoder = GzDecoder::new(file);
    let mut archive = Archive::new(gz_decoder);

    for entry in archive.entries()? {
        let mut entry = entry.context("Failed to read archive entry")?;
        let path = entry.path().context("Failed to get entry path")?;

        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        let manifest_filename = std::path::Path::new(&manifest.library.filename)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        if filename == manifest_filename || path.to_str() == Some(&manifest.library.filename) {
            let extract_path = temp_dir.path().join(filename);
            let mut output_file = File::create(&extract_path).with_context(|| {
                format!(
                    "Failed to create extracted library file: {:?}",
                    extract_path
                )
            })?;

            std::io::copy(&mut entry, &mut output_file)
                .context("Failed to extract library file")?;

            return Ok(extract_path);
        }
    }

    bail!(
        "Library file '{}' not found in package archive",
        manifest.library.filename
    );
}

/// Execute a task from a dynamic library.
pub fn execute_task_from_library(
    library_path: &PathBuf,
    task_name: &str,
    context_json: &str,
) -> Result<String> {
    // Load the dynamic library
    let lib = unsafe {
        Library::new(library_path)
            .with_context(|| format!("Failed to load library: {:?}", library_path))?
    };

    // Get the cloacina_execute_task symbol
    let execute_task: Symbol<
        unsafe extern "C" fn(
            task_name: *const u8,
            task_name_len: u32,
            context_json: *const u8,
            context_len: u32,
            result_buffer: *mut u8,
            result_capacity: u32,
            result_len: *mut u32,
        ) -> i32,
    > = unsafe {
        lib.get(EXECUTE_TASK_SYMBOL.as_bytes())
            .context("Symbol 'cloacina_execute_task' not found in library")?
    };

    // Prepare input parameters
    let task_name_bytes = task_name.as_bytes();
    let context_bytes = context_json.as_bytes();
    const RESULT_BUFFER_SIZE: usize = 10 * 1024 * 1024; // 10MB buffer for result
    let mut result_buffer = vec![0u8; RESULT_BUFFER_SIZE];
    let mut result_len: u32 = 0;

    // Call the function
    let return_code = unsafe {
        execute_task(
            task_name_bytes.as_ptr(),
            task_name_bytes.len() as u32,
            context_bytes.as_ptr(),
            context_bytes.len() as u32,
            result_buffer.as_mut_ptr(),
            result_buffer.len() as u32,
            &mut result_len,
        )
    };

    // Handle result
    if return_code == 0 {
        // Success
        if result_len > 0 && result_len <= result_buffer.len() as u32 {
            let result_json = String::from_utf8_lossy(&result_buffer[..result_len as usize]);
            Ok(result_json.to_string())
        } else if result_len > result_buffer.len() as u32 {
            bail!(
                "Task execution result too large: {} bytes exceeds maximum buffer size of {} bytes",
                result_len,
                result_buffer.len()
            );
        } else {
            Ok(String::new()) // No result data
        }
    } else {
        // Error
        let error_msg = if result_len > 0 && result_len <= result_buffer.len() as u32 {
            String::from_utf8_lossy(&result_buffer[..result_len as usize]).to_string()
        } else if result_len > result_buffer.len() as u32 {
            format!(
                "Task execution failed (code: {}) with oversized error message: {} bytes exceeds buffer size of {} bytes",
                return_code, result_len, result_buffer.len()
            )
        } else {
            format!("Unknown error (code: {})", return_code)
        };

        bail!("Task execution failed: {}", error_msg);
    }
}

/// Resolve a task identifier (index or name) to a task name.
pub fn resolve_task_name(manifest: &PackageManifest, task_identifier: &str) -> Result<String> {
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
                    description: task.description.clone(),
                    dependencies: task.dependencies.clone(),
                    source_location: task.source_location.clone(),
                })
                .collect();

            Ok(DebugResult::TaskList { tasks })
        }
        Some(task_id) => {
            // Execute task
            let task_name = resolve_task_name(&manifest, task_id)?;
            let context = context_json.unwrap_or("{}");

            // Create temporary directory for library extraction
            let temp_dir =
                tempfile::TempDir::new().context("Failed to create temporary directory")?;

            // Extract library from package
            let library_path = extract_library_from_package(package_path, &manifest, &temp_dir)?;

            // Execute task
            let output = execute_task_from_library(&library_path, &task_name, context)?;

            Ok(DebugResult::TaskExecution { output })
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
    pub source_location: String,
}
