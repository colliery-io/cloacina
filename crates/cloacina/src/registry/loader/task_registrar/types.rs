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

//! FFI types for task metadata exchange with dynamic libraries.

/// C-compatible task metadata structure for FFI (from packaged_workflow macro)
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TaskMetadata {
    /// Local task ID (e.g., "collect_data")
    pub local_id: *const std::os::raw::c_char,
    /// Template for namespaced ID (e.g., "{tenant}::simple_demo::data_processing::collect_data")
    pub namespaced_id_template: *const std::os::raw::c_char,
    /// JSON string of task dependencies
    pub dependencies_json: *const std::os::raw::c_char,
    /// Name of the task constructor function in the library
    pub constructor_fn_name: *const std::os::raw::c_char,
    /// Task description
    pub description: *const std::os::raw::c_char,
}

/// C-compatible collection of task metadata for FFI (from packaged_workflow macro)
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TaskMetadataCollection {
    /// Number of tasks in this package
    pub task_count: u32,
    /// Array of task metadata
    pub tasks: *const TaskMetadata,
    /// Name of the workflow (e.g., "data_processing")
    pub workflow_name: *const std::os::raw::c_char,
    /// Name of the package (e.g., "simple_demo")
    pub package_name: *const std::os::raw::c_char,
}

/// Owned version of task metadata - safe to use after library is unloaded.
///
/// This struct contains owned `String` copies of all data from the FFI structs,
/// ensuring no dangling pointers after the dynamic library is dropped.
#[derive(Debug, Clone)]
pub struct OwnedTaskMetadata {
    /// Local task ID (e.g., "collect_data")
    pub local_id: String,
    /// JSON string of task dependencies
    pub dependencies_json: String,
    /// Name of the task constructor function in the library
    pub constructor_fn_name: String,
}

/// Owned version of task metadata collection - safe to use after library is unloaded.
///
/// This struct contains owned `String` copies of all data from the FFI structs,
/// ensuring no dangling pointers after the dynamic library is dropped.
#[derive(Debug, Clone)]
pub struct OwnedTaskMetadataCollection {
    /// Name of the workflow (e.g., "data_processing")
    pub workflow_name: String,
    /// Name of the package (e.g., "simple_demo")
    pub package_name: String,
    /// Owned task metadata for each task in the package
    pub tasks: Vec<OwnedTaskMetadata>,
}
