---
id: python-package-manifest-schema-and
level: task
title: "Python package manifest schema and validation"
short_code: "CLOACI-T-0066"
created_at: 2026-01-28T14:29:01.695062+00:00
updated_at: 2026-01-28T14:29:01.695062+00:00
parent: CLOACI-I-0020
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0020
---

# Python package manifest schema and validation

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0020]]

## Objective

Extend the `.cloacina` package manifest schema to support Python packages, including runtime requirements, task definitions, and platform targeting. Implement validation on both build (cloaca) and load (cloacina server) sides.

## Acceptance Criteria

- [ ] Manifest schema extended with Python-specific fields
- [ ] Schema supports both Rust and Python packages (unified format)
- [ ] Pydantic models for Python-side validation (in cloaca)
- [ ] Serde structs for Rust-side validation (in cloacina)
- [ ] Platform targeting in manifest (`targets` array)
- [ ] Python version requirements (`requires_python`)
- [ ] Task definitions with function paths
- [ ] Unit tests for schema validation

## Manifest Schema (v2)

```json
{
  "format_version": "2",
  "package": {
    "name": "my-workflow",
    "version": "1.2.0",
    "description": "ETL workflow for daily processing",
    "fingerprint": "sha256:abc123...",
    "targets": ["linux-x86_64", "macos-arm64"]
  },
  "language": "python",
  "rust": null,
  "python": {
    "requires_python": ">=3.10",
    "entry_module": "workflow.tasks"
  },
  "tasks": [
    {
      "id": "extract",
      "function": "workflow.tasks:extract_data",
      "dependencies": [],
      "description": "Extract data from source",
      "retries": 3,
      "timeout_seconds": 300
    },
    {
      "id": "transform",
      "function": "workflow.tasks:transform_data",
      "dependencies": ["extract"],
      "description": "Transform extracted data"
    }
  ],
  "created_at": "2026-01-28T12:00:00Z",
  "signature": null
}
```

## Python Models (cloaca side)

```python
# cloaca/manifest.py
from pydantic import BaseModel, Field
from typing import Literal
from datetime import datetime

class TaskDefinition(BaseModel):
    id: str
    function: str  # "module.path:function_name"
    dependencies: list[str] = []
    description: str | None = None
    retries: int = 0
    timeout_seconds: int | None = None

class PythonRuntime(BaseModel):
    requires_python: str  # PEP 440 version specifier
    entry_module: str

class RustRuntime(BaseModel):
    library_path: str  # Relative path to .so/.dylib

class PackageInfo(BaseModel):
    name: str
    version: str
    description: str | None = None
    fingerprint: str
    targets: list[str]  # ["linux-x86_64", "macos-arm64"]

class Manifest(BaseModel):
    format_version: Literal["2"] = "2"
    package: PackageInfo
    language: Literal["python", "rust"]
    python: PythonRuntime | None = None
    rust: RustRuntime | None = None
    tasks: list[TaskDefinition]
    created_at: datetime
    signature: str | None = None

    def validate_language_runtime(self) -> None:
        if self.language == "python" and self.python is None:
            raise ValueError("Python package requires 'python' runtime config")
        if self.language == "rust" and self.rust is None:
            raise ValueError("Rust package requires 'rust' runtime config")
```

## Rust Structs (cloacina side)

```rust
// crates/cloacina/src/packaging/manifest.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub id: String,
    pub function: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub retries: u32,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonRuntime {
    pub requires_python: String,
    pub entry_module: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustRuntime {
    pub library_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub fingerprint: String,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageLanguage {
    Python,
    Rust,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: String,
    pub package: PackageInfo,
    pub language: PackageLanguage,
    pub python: Option<PythonRuntime>,
    pub rust: Option<RustRuntime>,
    pub tasks: Vec<TaskDefinition>,
    pub created_at: DateTime<Utc>,
    pub signature: Option<String>,
}

impl Manifest {
    pub fn validate(&self) -> Result<(), ManifestError> {
        // Check language/runtime match
        match self.language {
            PackageLanguage::Python if self.python.is_none() => {
                return Err(ManifestError::MissingRuntime("python"));
            }
            PackageLanguage::Rust if self.rust.is_none() => {
                return Err(ManifestError::MissingRuntime("rust"));
            }
            _ => {}
        }

        // Validate targets
        for target in &self.package.targets {
            if !SUPPORTED_TARGETS.contains(&target.as_str()) {
                return Err(ManifestError::UnsupportedTarget(target.clone()));
            }
        }

        Ok(())
    }

    pub fn is_compatible_with_platform(&self, platform: &str) -> bool {
        self.package.targets.contains(&platform.to_string())
    }
}

const SUPPORTED_TARGETS: &[&str] = &[
    "linux-x86_64",
    "linux-arm64",
    "macos-x86_64",
    "macos-arm64",
];
```

## Platform Detection

```rust
pub fn detect_current_platform() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    { "linux-x86_64" }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    { "linux-arm64" }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    { "macos-x86_64" }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    { "macos-arm64" }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
    )))]
    { "unknown" }
}
```

## File Locations

- Python models: `crates/cloaca/cloaca/manifest.py`
- Rust structs: `crates/cloacina/src/packaging/manifest.rs` (extend existing)
- Platform detection: `crates/cloacina/src/packaging/platform.rs`

## Status Updates

*To be added during implementation*
