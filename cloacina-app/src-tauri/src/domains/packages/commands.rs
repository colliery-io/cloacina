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

use cloacina::packaging::types::{PackageManifest, MANIFEST_FILENAME};
use cloacina::packaging::{package_workflow, CompileOptions};
use flate2::read::GzDecoder;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use tar::Archive;
use tauri::command;

use super::types::{
    BuildPackageRequest, BuildPackageResponse, DebugPackageRequest, DebugPackageResponse,
    InspectPackageRequest, InspectPackageResponse, VisualizePackageRequest,
    VisualizePackageResponse,
};

/// Build a workflow package from a Rust project
#[command]
pub async fn build_package(request: BuildPackageRequest) -> Result<BuildPackageResponse, String> {
    let project_path = PathBuf::from(&request.project_path);
    let output_path = PathBuf::from(&request.output_path);

    // Validate project path exists
    if !project_path.exists() {
        return Ok(BuildPackageResponse {
            success: false,
            output: String::new(),
            error: Some(format!(
                "Project path does not exist: {}",
                request.project_path
            )),
            package_path: None,
        });
    }

    // Create compile options from request
    let options = CompileOptions {
        target: request.target.clone(),
        profile: request.profile.clone(),
        cargo_flags: request.cargo_flags.clone(),
        jobs: None,
    };

    // Attempt to build the package
    match package_workflow(project_path, output_path.clone(), options) {
        Ok(()) => Ok(BuildPackageResponse {
            success: true,
            output: format!("Successfully built package: {}", output_path.display()),
            error: None,
            package_path: Some(output_path.to_string_lossy().to_string()),
        }),
        Err(e) => Ok(BuildPackageResponse {
            success: false,
            output: String::new(),
            error: Some(format!("Build failed: {}", e)),
            package_path: None,
        }),
    }
}

/// Extract manifest from a package archive
fn extract_manifest_from_package(package_path: &PathBuf) -> Result<PackageManifest, String> {
    let file =
        File::open(package_path).map_err(|e| format!("Failed to open package file: {}", e))?;

    let gz_decoder = GzDecoder::new(file);
    let mut archive = Archive::new(gz_decoder);

    for entry in archive
        .entries()
        .map_err(|e| format!("Failed to read archive entries: {}", e))?
    {
        let mut entry = entry.map_err(|e| format!("Failed to read archive entry: {}", e))?;

        if let Ok(path) = entry.path() {
            if path.to_string_lossy() == MANIFEST_FILENAME {
                let mut contents = String::new();
                entry
                    .read_to_string(&mut contents)
                    .map_err(|e| format!("Failed to read manifest contents: {}", e))?;

                let manifest: PackageManifest = serde_json::from_str(&contents)
                    .map_err(|e| format!("Failed to parse manifest JSON: {}", e))?;

                return Ok(manifest);
            }
        }
    }

    Err("Manifest file not found in package".to_string())
}

/// Inspect a workflow package
#[command]
pub async fn inspect_package(
    request: InspectPackageRequest,
) -> Result<InspectPackageResponse, String> {
    let package_path = PathBuf::from(&request.package_path);

    // Validate package path exists
    if !package_path.exists() {
        return Ok(InspectPackageResponse {
            success: false,
            manifest: None,
            info: None,
            error: Some(format!(
                "Package path does not exist: {}",
                request.package_path
            )),
        });
    }

    // Extract manifest from package
    match extract_manifest_from_package(&package_path) {
        Ok(manifest) => {
            let info = if request.format == "human" {
                Some(format!(
                    "Package: {} v{}\n\
                     Description: {}\n\
                     Authors: {}\n\
                     Library: {} ({} bytes)\n\
                     Architecture: {}\n\
                     Tasks: {}\n\
                     Cloacina Version: {}",
                    manifest.package.name,
                    manifest.package.version,
                    manifest
                        .package
                        .description
                        .as_ref()
                        .unwrap_or(&"N/A".to_string()),
                    manifest
                        .package
                        .authors
                        .as_ref()
                        .map(|a| a.join(", "))
                        .unwrap_or_else(|| "N/A".to_string()),
                    manifest.library.filename,
                    manifest.library.size,
                    manifest.library.architecture,
                    manifest.tasks.len(),
                    manifest.cloacina_version
                ))
            } else {
                None
            };

            Ok(InspectPackageResponse {
                success: true,
                manifest: Some(
                    serde_json::to_value(&manifest)
                        .map_err(|e| format!("Failed to serialize manifest: {}", e))?,
                ),
                info,
                error: None,
            })
        }
        Err(e) => Ok(InspectPackageResponse {
            success: false,
            manifest: None,
            info: None,
            error: Some(e),
        }),
    }
}

/// Debug a workflow package (list tasks or execute a task)
#[command]
pub async fn debug_package(request: DebugPackageRequest) -> Result<DebugPackageResponse, String> {
    let package_path = PathBuf::from(&request.package_path);

    // Validate package path exists
    if !package_path.exists() {
        return Ok(DebugPackageResponse {
            success: false,
            tasks: None,
            output: None,
            error: Some(format!(
                "Package path does not exist: {}",
                request.package_path
            )),
        });
    }

    // Extract manifest to get task information
    match extract_manifest_from_package(&package_path) {
        Ok(manifest) => {
            if request.task_identifier.is_none() {
                // List tasks
                let tasks: Vec<super::types::TaskInfo> = manifest
                    .tasks
                    .iter()
                    .enumerate()
                    .map(|(index, task)| super::types::TaskInfo {
                        index,
                        id: task.name.clone(),
                        description: task
                            .description
                            .as_ref()
                            .unwrap_or(&"No description".to_string())
                            .clone(),
                        dependencies: Vec::new(), // TODO: Extract dependency info from workflow
                        source_location: format!("Symbol: {}", task.symbol),
                    })
                    .collect();

                Ok(DebugPackageResponse {
                    success: true,
                    tasks: Some(tasks),
                    output: None,
                    error: None,
                })
            } else {
                // Execute specific task - this would require dynamic loading and execution
                // For now, return a message indicating this feature needs more work
                Ok(DebugPackageResponse {
                    success: false,
                    tasks: None,
                    output: None,
                    error: Some("Task execution debugging not yet implemented - requires dynamic library loading".to_string()),
                })
            }
        }
        Err(e) => Ok(DebugPackageResponse {
            success: false,
            tasks: None,
            output: None,
            error: Some(e),
        }),
    }
}

/// Visualize a workflow package dependencies
#[command]
pub async fn visualize_package(
    request: VisualizePackageRequest,
) -> Result<VisualizePackageResponse, String> {
    let package_path = PathBuf::from(&request.package_path);

    // Validate package path exists
    if !package_path.exists() {
        return Ok(VisualizePackageResponse {
            success: false,
            visualization: None,
            graph_data: None,
            error: Some(format!(
                "Package path does not exist: {}",
                request.package_path
            )),
        });
    }

    // Extract manifest to get task information
    match extract_manifest_from_package(&package_path) {
        Ok(manifest) => {
            // Create nodes for each task
            let nodes: Vec<super::types::GraphNode> = manifest
                .tasks
                .iter()
                .map(|task| super::types::GraphNode {
                    id: task.name.clone(),
                    label: task.name.clone(),
                    description: task
                        .description
                        .as_ref()
                        .unwrap_or(&"No description".to_string())
                        .clone(),
                    node_type: "task".to_string(),
                    x: None, // Let D3.js calculate positions
                    y: None,
                })
                .collect();

            // For now, we don't have dependency information in the manifest
            // In a full implementation, we'd need to extract this from the workflow definition
            let edges: Vec<super::types::GraphEdge> = Vec::new();

            let graph_data = super::types::GraphData { nodes, edges };

            // Generate ASCII visualization if requested
            let visualization = if request.format == "ascii" {
                let task_list = manifest
                    .tasks
                    .iter()
                    .map(|task| format!("  ┌─ {}", task.name))
                    .collect::<Vec<_>>()
                    .join("\n");

                Some(format!(
                    "Package: {}\n\
                     Tasks:\n\
                     {}",
                    manifest.package.name, task_list
                ))
            } else {
                None
            };

            Ok(VisualizePackageResponse {
                success: true,
                visualization,
                graph_data: Some(graph_data),
                error: None,
            })
        }
        Err(e) => Ok(VisualizePackageResponse {
            success: false,
            visualization: None,
            graph_data: None,
            error: Some(e),
        }),
    }
}
