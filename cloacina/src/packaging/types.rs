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

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Result of compiling a workflow project
#[derive(Debug, Clone)]
pub struct CompileResult {
    /// Path to the compiled dynamic library
    pub so_path: PathBuf,
    /// Generated package manifest
    pub manifest: PackageManifest,
}

/// Options for compiling a workflow
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Target triple for cross-compilation
    pub target: Option<String>,
    /// Build profile (debug/release)
    pub profile: String,
    /// Additional cargo flags
    pub cargo_flags: Vec<String>,
    /// Number of parallel jobs
    pub jobs: Option<u32>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            target: None,
            profile: "debug".to_string(),
            cargo_flags: Vec::new(),
            jobs: None,
        }
    }
}

/// Package manifest containing workflow metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    /// Manifest format version
    pub version: String,
    /// Package information
    pub package: PackageInfo,
    /// Library information
    pub library: LibraryInfo,
    /// Task information
    pub tasks: Vec<TaskInfo>,
    /// Cloacina compatibility version
    pub cloacina_version: String,
}

/// Package information from Cargo.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Package description
    pub description: Option<String>,
    /// Package authors
    pub authors: Option<Vec<String>>,
    /// Package keywords
    pub keywords: Option<Vec<String>>,
}

/// Dynamic library information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryInfo {
    /// Library filename
    pub filename: String,
    /// Target architecture
    pub architecture: String,
    /// File size in bytes
    pub size: u64,
    /// SHA256 checksum
    pub checksum: String,
}

/// Task information extracted from the workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    /// Task name
    pub name: String,
    /// Task description
    pub description: Option<String>,
    /// Task symbol name in the library
    pub symbol: String,
}

/// Parsed Cargo.toml structure
#[derive(Debug, Clone, Deserialize)]
pub struct CargoToml {
    pub package: Option<CargoPackage>,
    pub lib: Option<CargoLib>,
    pub dependencies: Option<toml::Value>,
}

/// Package section from Cargo.toml
#[derive(Debug, Clone, Deserialize)]
pub struct CargoPackage {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
    #[serde(rename = "rust-version")]
    pub rust_version: Option<String>,
}

/// Library section from Cargo.toml
#[derive(Debug, Clone, Deserialize)]
pub struct CargoLib {
    #[serde(rename = "crate-type")]
    pub crate_type: Option<Vec<String>>,
}

/// Constants
pub const MANIFEST_FILENAME: &str = "manifest.json";
pub const EXECUTE_TASK_SYMBOL: &str = "execute_task";
pub const CLOACINA_VERSION: &str = env!("CARGO_PKG_VERSION");
