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

    #[error("Duplicate trigger name: '{name}'")]
    DuplicateTriggerName { name: String },

    #[error("Trigger '{trigger_name}' references unknown workflow '{workflow}'")]
    InvalidTriggerWorkflow {
        trigger_name: String,
        workflow: String,
    },

    #[error("Trigger '{trigger_name}' has invalid poll_interval '{interval}': {reason}")]
    InvalidTriggerPollInterval {
        trigger_name: String,
        interval: String,
        reason: String,
    },
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
pub struct PackageInfo {
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
pub struct TaskDefinition {
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

/// Trigger definition within a package.
///
/// Declares a trigger that should be auto-registered when the package is loaded.
/// Any type implementing the `Trigger` trait can be packaged this way.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDefinition {
    /// Unique name for this trigger (within the package).
    pub name: String,
    /// Trigger type discriminator (e.g. `"rust"`, `"python"`, `"webhook"`,
    /// `"http_poll"`, `"file_watch"`, or any user-defined string).
    pub trigger_type: String,
    /// Name of the workflow to fire when this trigger activates.
    pub workflow: String,
    /// How often to poll (e.g. `"30s"`, `"5m"`, `"100ms"`).
    pub poll_interval: String,
    /// Whether to allow concurrent executions with the same context.
    #[serde(default)]
    pub allow_concurrent: bool,
    /// Trigger-specific configuration (e.g. URL, file path, custom params).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

/// Unified package manifest (v2).
///
/// Supports both Rust (dynamic library) and Python workflow packages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Format version, always "2" for this schema.
    pub format_version: String,
    /// Package metadata.
    pub package: PackageInfo,
    /// Package language.
    pub language: PackageLanguage,
    /// Python runtime config (required when `language == Python`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub python: Option<PythonRuntime>,
    /// Rust runtime config (required when `language == Rust`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust: Option<RustRuntime>,
    /// Task definitions.
    pub tasks: Vec<TaskDefinition>,
    /// Trigger definitions (optional — packages without triggers omit this).
    #[serde(default)]
    pub triggers: Vec<TriggerDefinition>,
    /// When the manifest was created.
    pub created_at: DateTime<Utc>,
    /// Package signature (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl Manifest {
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

        // Validate triggers (if any)
        let mut seen_trigger_names = std::collections::HashSet::new();
        // Workflow names that triggers can reference: use the package name as
        // the workflow identifier (matching how packaged workflows are registered).
        let valid_workflow_names: std::collections::HashSet<&str> =
            std::iter::once(self.package.name.as_str())
                .chain(self.tasks.iter().map(|t| t.id.as_str()))
                .collect();

        for trigger in &self.triggers {
            if !seen_trigger_names.insert(&trigger.name) {
                return Err(ManifestValidationError::DuplicateTriggerName {
                    name: trigger.name.clone(),
                });
            }

            if !valid_workflow_names.contains(trigger.workflow.as_str()) {
                return Err(ManifestValidationError::InvalidTriggerWorkflow {
                    trigger_name: trigger.name.clone(),
                    workflow: trigger.workflow.clone(),
                });
            }

            parse_duration_str(&trigger.poll_interval).map_err(|reason| {
                ManifestValidationError::InvalidTriggerPollInterval {
                    trigger_name: trigger.name.clone(),
                    interval: trigger.poll_interval.clone(),
                    reason,
                }
            })?;
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

    fn make_python_manifest() -> Manifest {
        Manifest {
            format_version: "2".to_string(),
            package: PackageInfo {
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
                TaskDefinition {
                    id: "extract".to_string(),
                    function: "workflow.tasks:extract_data".to_string(),
                    dependencies: vec![],
                    description: Some("Extract data".to_string()),
                    retries: 3,
                    timeout_seconds: Some(300),
                },
                TaskDefinition {
                    id: "transform".to_string(),
                    function: "workflow.tasks:transform_data".to_string(),
                    dependencies: vec!["extract".to_string()],
                    description: None,
                    retries: 0,
                    timeout_seconds: None,
                },
            ],
            triggers: vec![],
            created_at: Utc::now(),
            signature: None,
        }
    }

    fn make_rust_manifest() -> Manifest {
        Manifest {
            format_version: "2".to_string(),
            package: PackageInfo {
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
            tasks: vec![TaskDefinition {
                id: "process".to_string(),
                function: "cloacina_execute_task".to_string(),
                dependencies: vec![],
                description: Some("Process data".to_string()),
                retries: 0,
                timeout_seconds: None,
            }],
            triggers: vec![],
            created_at: Utc::now(),
            signature: None,
        }
    }

    fn make_manifest_with_triggers() -> Manifest {
        let mut m = make_rust_manifest();
        m.triggers = vec![
            TriggerDefinition {
                name: "file_watcher".to_string(),
                trigger_type: "file_watch".to_string(),
                workflow: "rust-workflow".to_string(),
                poll_interval: "5s".to_string(),
                allow_concurrent: false,
                config: Some(serde_json::json!({"path": "/inbox/"})),
            },
            TriggerDefinition {
                name: "api_poller".to_string(),
                trigger_type: "http_poll".to_string(),
                workflow: "rust-workflow".to_string(),
                poll_interval: "1m".to_string(),
                allow_concurrent: true,
                config: Some(serde_json::json!({"url": "https://example.com/status"})),
            },
        ];
        m
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
        let parsed: Manifest = serde_json::from_str(&json).unwrap();

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

    // --- Trigger tests ---

    #[test]
    fn test_manifest_with_triggers_validates() {
        assert!(make_manifest_with_triggers().validate().is_ok());
    }

    #[test]
    fn test_manifest_no_triggers_still_validates() {
        // Existing manifests without triggers should pass unchanged
        assert!(make_python_manifest().validate().is_ok());
        assert!(make_rust_manifest().validate().is_ok());
    }

    #[test]
    fn test_duplicate_trigger_name() {
        let mut m = make_manifest_with_triggers();
        m.triggers[1].name = "file_watcher".to_string();
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::DuplicateTriggerName { .. })
        ));
    }

    #[test]
    fn test_trigger_invalid_workflow_reference() {
        let mut m = make_manifest_with_triggers();
        m.triggers[0].workflow = "nonexistent-workflow".to_string();
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::InvalidTriggerWorkflow { .. })
        ));
    }

    #[test]
    fn test_trigger_references_task_id() {
        // Triggers can reference a task ID as the workflow
        let mut m = make_manifest_with_triggers();
        m.triggers[0].workflow = "process".to_string(); // task ID from rust manifest
        assert!(m.validate().is_ok());
    }

    #[test]
    fn test_trigger_invalid_poll_interval() {
        let mut m = make_manifest_with_triggers();
        m.triggers[0].poll_interval = "invalid".to_string();
        assert!(matches!(
            m.validate(),
            Err(ManifestValidationError::InvalidTriggerPollInterval { .. })
        ));
    }

    #[test]
    fn test_trigger_poll_interval_variants() {
        let mut m = make_manifest_with_triggers();
        // All valid duration formats
        for interval in &["100ms", "5s", "2m", "1h"] {
            m.triggers[0].poll_interval = interval.to_string();
            assert!(m.validate().is_ok(), "Should accept interval: {}", interval);
        }
    }

    #[test]
    fn test_trigger_serialization_roundtrip() {
        let original = make_manifest_with_triggers();
        let json = serde_json::to_string_pretty(&original).unwrap();
        let parsed: Manifest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.triggers.len(), 2);
        assert_eq!(parsed.triggers[0].name, "file_watcher");
        assert_eq!(parsed.triggers[0].trigger_type, "file_watch");
        assert_eq!(parsed.triggers[0].workflow, "rust-workflow");
        assert_eq!(parsed.triggers[0].poll_interval, "5s");
        assert!(!parsed.triggers[0].allow_concurrent);
        assert!(parsed.triggers[0].config.is_some());

        assert_eq!(parsed.triggers[1].name, "api_poller");
        assert!(parsed.triggers[1].allow_concurrent);
    }

    #[test]
    fn test_trigger_no_config() {
        let mut m = make_manifest_with_triggers();
        m.triggers[0].config = None;
        assert!(m.validate().is_ok());

        // Roundtrip preserves None config
        let json = serde_json::to_string(&m).unwrap();
        let parsed: Manifest = serde_json::from_str(&json).unwrap();
        assert!(parsed.triggers[0].config.is_none());
    }

    #[test]
    fn test_deserialization_without_triggers_field() {
        // JSON without "triggers" key should deserialize with empty vec (serde default)
        let json = r#"{
            "format_version": "2",
            "package": {"name": "test", "version": "1.0.0", "fingerprint": "sha256:abc", "targets": ["linux-x86_64"]},
            "language": "rust",
            "rust": {"library_path": "lib/libtest.so"},
            "tasks": [{"id": "t1", "function": "sym"}],
            "created_at": "2026-01-01T00:00:00Z"
        }"#;
        let parsed: Manifest = serde_json::from_str(json).unwrap();
        assert!(parsed.triggers.is_empty());
        assert!(parsed.validate().is_ok());
    }
}

/// Parse a duration string like "30s", "5m", "2h", "100ms" into a [`std::time::Duration`].
pub fn parse_duration_str(s: &str) -> Result<std::time::Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty string".to_string());
    }

    let (num_str, suffix) = if s.ends_with("ms") {
        (&s[..s.len() - 2], "ms")
    } else {
        let split = s.len() - 1;
        if split == 0 || !s.as_bytes()[split].is_ascii_alphabetic() {
            return Err(format!(
                "expected number followed by unit (s, m, h, ms), got '{s}'"
            ));
        }
        (&s[..split], &s[split..])
    };

    let value: u64 = num_str
        .parse()
        .map_err(|_| format!("'{num_str}' is not a valid number"))?;

    match suffix {
        "ms" => Ok(std::time::Duration::from_millis(value)),
        "s" => Ok(std::time::Duration::from_secs(value)),
        "m" => Ok(std::time::Duration::from_secs(value * 60)),
        "h" => Ok(std::time::Duration::from_secs(value * 3600)),
        other => Err(format!("unknown unit '{other}', expected s, m, h, or ms")),
    }
}
