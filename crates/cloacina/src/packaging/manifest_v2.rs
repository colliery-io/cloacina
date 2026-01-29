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

//! Unified package manifest (v2) supporting both Rust and Python workflows.
//!
//! The v2 manifest extends the original Rust-only format to support Python
//! workflow packages. It uses a language discriminator to determine which
//! runtime configuration applies.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::platform::SUPPORTED_TARGETS;

/// Errors from manifest validation.
#[derive(Debug, Error)]
pub enum ManifestValidationError {
    #[error("Missing runtime config: {language} package requires '{language}' field")]
    MissingRuntime { language: String },

    #[error("Unsupported target platform: {target}")]
    UnsupportedTarget { target: String },

    #[error("Empty task list: package must define at least one task")]
    NoTasks,

    #[error("Duplicate task ID: '{id}'")]
    DuplicateTaskId { id: String },

    #[error("Invalid task dependency: task '{task_id}' depends on unknown task '{dep_id}'")]
    InvalidDependency { task_id: String, dep_id: String },

    #[error("Invalid Python function path '{path}': expected 'module.path:function_name'")]
    InvalidFunctionPath { path: String },

    #[error("Invalid format version: expected '2', got '{version}'")]
    InvalidFormatVersion { version: String },
}

/// Package language discriminator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageLanguage {
    Python,
    Rust,
}

/// Python runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonRuntime {
    /// PEP 440 version specifier (e.g., ">=3.10").
    pub requires_python: String,
    /// Entry module for task discovery (e.g., "workflow.tasks").
    pub entry_module: String,
}

/// Rust runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustRuntime {
    /// Relative path to the compiled dynamic library within the package.
    pub library_path: String,
}

/// Package metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfoV2 {
    /// Package name.
    pub name: String,
    /// Semantic version.
    pub version: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// SHA-256 fingerprint of package contents.
    pub fingerprint: String,
    /// Target platforms this package supports.
    pub targets: Vec<String>,
}

/// Task definition within a package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinitionV2 {
    /// Task identifier (unique within the package).
    pub id: String,
    /// Callable function path.
    ///
    /// For Python: `"module.path:function_name"`
    /// For Rust: `"symbol_name"` (FFI symbol in the compiled library)
    pub function: String,
    /// IDs of tasks that must complete before this one.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Human-readable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Number of automatic retries on failure.
    #[serde(default)]
    pub retries: u32,
    /// Maximum execution time in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
}

/// Unified package manifest (v2).
///
/// Supports both Rust (dynamic library) and Python workflow packages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestV2 {
    /// Format version, always "2" for this schema.
    pub format_version: String,
    /// Package metadata.
    pub package: PackageInfoV2,
    /// Package language.
    pub language: PackageLanguage,
    /// Python runtime config (required when `language == Python`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub python: Option<PythonRuntime>,
    /// Rust runtime config (required when `language == Rust`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust: Option<RustRuntime>,
    /// Task definitions.
    pub tasks: Vec<TaskDefinitionV2>,
    /// When the manifest was created.
    pub created_at: DateTime<Utc>,
    /// Package signature (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl ManifestV2 {
    /// Validate the manifest for structural correctness.
    pub fn validate(&self) -> Result<(), ManifestValidationError> {
        if self.format_version != "2" {
            return Err(ManifestValidationError::InvalidFormatVersion {
                version: self.format_version.clone(),
            });
        }

        match self.language {
            PackageLanguage::Python if self.python.is_none() => {
                return Err(ManifestValidationError::MissingRuntime {
                    language: "python".to_string(),
                });
            }
            PackageLanguage::Rust if self.rust.is_none() => {
                return Err(ManifestValidationError::MissingRuntime {
                    language: "rust".to_string(),
                });
            }
            _ => {}
        }

        for target in &self.package.targets {
            if !SUPPORTED_TARGETS.contains(&target.as_str()) {
                return Err(ManifestValidationError::UnsupportedTarget {
                    target: target.clone(),
                });
            }
        }

        if self.tasks.is_empty() {
            return Err(ManifestValidationError::NoTasks);
        }

        let mut seen_ids = std::collections::HashSet::new();
        for task in &self.tasks {
            if !seen_ids.insert(&task.id) {
                return Err(ManifestValidationError::DuplicateTaskId {
                    id: task.id.clone(),
                });
            }
        }

        for task in &self.tasks {
            for dep in &task.dependencies {
                if !seen_ids.contains(dep) {
                    return Err(ManifestValidationError::InvalidDependency {
                        task_id: task.id.clone(),
                        dep_id: dep.clone(),
                    });
                }
            }
        }

        if self.language == PackageLanguage::Python {
            for task in &self.tasks {
                if !task.function.contains(':') {
                    return Err(ManifestValidationError::InvalidFunctionPath {
                        path: task.function.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Check if this package is compatible with a specific platform.
    pub fn is_compatible_with_platform(&self, platform_str: &str) -> bool {
        self.package.targets.contains(&platform_str.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_python_manifest() -> ManifestV2 {
        ManifestV2 {
            format_version: "2".to_string(),
            package: PackageInfoV2 {
                name: "my-workflow".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test workflow".to_string()),
                fingerprint: "sha256:abc123".to_string(),
                targets: vec!["linux-x86_64".to_string(), "macos-arm64".to_string()],
            },
            language: PackageLanguage::Python,
            python: Some(PythonRuntime {
                requires_python: ">=3.10".to_string(),
                entry_module: "workflow.tasks".to_string(),
            }),
            rust: None,
            tasks: vec![
                TaskDefinitionV2 {
                    id: "extract".to_string(),
                    function: "workflow.tasks:extract_data".to_string(),
                    dependencies: vec![],
                    description: Some("Extract data".to_string()),
                    retries: 3,
                    timeout_seconds: Some(300),
                },
                TaskDefinitionV2 {
                    id: "transform".to_string(),
                    function: "workflow.tasks:transform_data".to_string(),
                    dependencies: vec!["extract".to_string()],
                    description: None,
                    retries: 0,
                    timeout_seconds: None,
                },
            ],
            created_at: Utc::now(),
            signature: None,
        }
    }

    fn make_rust_manifest() -> ManifestV2 {
        ManifestV2 {
            format_version: "2".to_string(),
            package: PackageInfoV2 {
                name: "rust-workflow".to_string(),
                version: "0.1.0".to_string(),
                description: None,
                fingerprint: "sha256:def456".to_string(),
                targets: vec!["linux-x86_64".to_string()],
            },
            language: PackageLanguage::Rust,
            python: None,
            rust: Some(RustRuntime {
                library_path: "lib/libworkflow.so".to_string(),
            }),
            tasks: vec![TaskDefinitionV2 {
                id: "process".to_string(),
                function: "cloacina_execute_task".to_string(),
                dependencies: vec![],
                description: Some("Process data".to_string()),
                retries: 0,
                timeout_seconds: None,
            }],
            created_at: Utc::now(),
            signature: None,
        }
    }

    #[test]
    fn test_python_manifest_validates() {
        assert!(make_python_manifest().validate().is_ok());
    }

    #[test]
    fn test_rust_manifest_validates() {
        assert!(make_rust_manifest().validate().is_ok());
    }

    #[test]
    fn test_missing_python_runtime() {
        let mut m = make_python_manifest();
        m.python = None;
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::MissingRuntime { .. })
        ));
    }

    #[test]
    fn test_missing_rust_runtime() {
        let mut m = make_rust_manifest();
        m.rust = None;
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::MissingRuntime { .. })
        ));
    }

    #[test]
    fn test_unsupported_target() {
        let mut m = make_python_manifest();
        m.package.targets.push("windows-x86_64".to_string());
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::UnsupportedTarget { .. })
        ));
    }

    #[test]
    fn test_no_tasks() {
        let mut m = make_python_manifest();
        m.tasks.clear();
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::NoTasks)
        ));
    }

    #[test]
    fn test_duplicate_task_id() {
        let mut m = make_python_manifest();
        m.tasks[1].id = "extract".to_string();
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::DuplicateTaskId { .. })
        ));
    }

    #[test]
    fn test_invalid_dependency() {
        let mut m = make_python_manifest();
        m.tasks[1].dependencies = vec!["nonexistent".to_string()];
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::InvalidDependency { .. })
        ));
    }

    #[test]
    fn test_invalid_python_function_path() {
        let mut m = make_python_manifest();
        m.tasks[0].function = "no_colon_separator".to_string();
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::InvalidFunctionPath { .. })
        ));
    }

    #[test]
    fn test_rust_function_path_no_colon_ok() {
        let m = make_rust_manifest();
        assert!(m.validate().is_ok());
    }

    #[test]
    fn test_invalid_format_version() {
        let mut m = make_python_manifest();
        m.format_version = "1".to_string();
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::InvalidFormatVersion { .. })
        ));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let original = make_python_manifest();
        let json = serde_json::to_string_pretty(&original).unwrap();
        let parsed: ManifestV2 = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.format_version, "2");
        assert_eq!(parsed.package.name, "my-workflow");
        assert_eq!(parsed.language, PackageLanguage::Python);
        assert!(parsed.python.is_some());
        assert_eq!(parsed.tasks.len(), 2);
        assert_eq!(parsed.tasks[0].retries, 3);
        assert_eq!(parsed.tasks[1].dependencies, vec!["extract"]);
    }

    #[test]
    fn test_platform_compatibility() {
        let m = make_python_manifest();
        assert!(m.is_compatible_with_platform("linux-x86_64"));
        assert!(m.is_compatible_with_platform("macos-arm64"));
        assert!(!m.is_compatible_with_platform("linux-arm64"));
    }

    #[test]
    fn test_language_serde() {
        let json = serde_json::to_string(&PackageLanguage::Python).unwrap();
        assert_eq!(json, "\"python\"");
        let parsed: PackageLanguage = serde_json::from_str("\"rust\"").unwrap();
        assert_eq!(parsed, PackageLanguage::Rust);
    }
}
