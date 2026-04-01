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

use serde::Deserialize;
use std::path::PathBuf;

use super::manifest_schema::Manifest;

/// Result of compiling a workflow project.
///
/// Contains the path to the compiled cdylib and the unified Manifest
/// that describes the package, tasks, triggers, and runtime configuration.
#[derive(Debug, Clone)]
pub struct CompileResult {
    /// Path to the compiled dynamic library
    pub so_path: PathBuf,
    /// Generated package manifest (v2 unified format)
    pub manifest: Manifest,
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
pub const CLOACINA_VERSION: &str = env!("CARGO_PKG_VERSION");
