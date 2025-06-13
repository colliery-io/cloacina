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

use cloacina::WorkflowGraphData;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub const CLOACINA_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MANIFEST_FILENAME: &str = "manifest.json";
pub const EXECUTE_TASK_SYMBOL: &str = "cloacina_execute_task";

// Standard FFI interface that packaged workflows must implement:
//
// #[no_mangle]
// extern "C" fn cloacina_execute_task(
//     task_name: *const u8,        // Task name as UTF-8 bytes
//     task_name_len: u32,          // Length of task name
//     context_json: *const u8,     // Input context as JSON bytes
//     context_len: u32,            // Length of context JSON
//     result_buffer: *mut u8,      // Buffer for result JSON
//     result_capacity: u32,        // Size of result buffer
//     result_len: *mut u32,        // Actual length of result written
// ) -> i32;                       // 0 = success, negative = error

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageManifest {
    pub package: PackageInfo,
    pub library: LibraryInfo,
    pub tasks: Vec<TaskInfo>,
    pub execution_order: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<WorkflowGraphData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub cloacina_version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LibraryInfo {
    pub filename: String,
    pub symbols: Vec<String>,
    pub architecture: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskInfo {
    pub index: u32,
    pub id: String,
    pub dependencies: Vec<String>,
    pub description: String,
    pub source_location: String,
}

#[derive(Debug)]
pub struct CompileResult {
    pub so_path: PathBuf,
    pub manifest: PackageManifest,
}

#[derive(Deserialize, Debug)]
pub struct CargoToml {
    pub package: Option<PackageSection>,
    pub lib: Option<LibSection>,
    pub dependencies: Option<HashMap<String, DependencySpec>>,
}

#[derive(Deserialize, Debug)]
pub struct PackageSection {
    pub name: String,
    pub version: String,
    #[serde(rename = "rust-version")]
    pub rust_version: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct LibSection {
    #[serde(rename = "crate-type")]
    pub crate_type: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum DependencySpec {
    Simple(String),
    Detailed {
        version: Option<String>,
        #[allow(dead_code)]
        path: Option<String>,
        #[allow(dead_code)]
        git: Option<String>,
        #[allow(dead_code)]
        branch: Option<String>,
        #[allow(dead_code)]
        tag: Option<String>,
        #[allow(dead_code)]
        rev: Option<String>,
        #[allow(dead_code)]
        features: Option<Vec<String>>,
        #[allow(dead_code)]
        default_features: Option<bool>,
    },
}
