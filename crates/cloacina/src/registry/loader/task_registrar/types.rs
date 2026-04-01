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

//! Types for task metadata exchange with dynamic libraries.
//!
//! The raw FFI structs have been removed — fidius-host handles all serialization
//! and FFI safety transparently. Only the owned (post-extraction) types remain.

/// Owned task metadata — safe to use after library is unloaded.
///
/// All fields are owned `String` values; no raw pointers are involved.
#[derive(Debug, Clone)]
pub struct OwnedTaskMetadata {
    /// Local task ID (e.g., "collect_data")
    pub local_id: String,
    /// JSON string of task dependencies
    pub dependencies_json: String,
}

/// Owned collection of task metadata — safe to use after library is unloaded.
///
/// All fields are owned `String` values; no raw pointers are involved.
#[derive(Debug, Clone)]
pub struct OwnedTaskMetadataCollection {
    /// Name of the workflow (e.g., "data_processing")
    pub workflow_name: String,
    /// Name of the package (e.g., "simple_demo")
    pub package_name: String,
    /// Owned task metadata for each task in the package
    pub tasks: Vec<OwnedTaskMetadata>,
}
