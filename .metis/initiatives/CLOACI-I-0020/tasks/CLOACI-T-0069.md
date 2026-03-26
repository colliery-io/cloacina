---
id: server-side-python-package-loader
level: task
title: "Server-side Python package loader"
short_code: "CLOACI-T-0069"
created_at: 2026-01-28T14:29:03.277373+00:00
updated_at: 2026-01-28T18:35:20.296059+00:00
parent: CLOACI-I-0020
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0020
---

# Server-side Python package loader

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0020]]

## Objective

Extend the Rust server-side package loading infrastructure to recognize and extract Python workflow packages. This includes detecting package language from manifest, extracting vendor directory and Python source, and preparing the package for task execution via PyO3.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Package loader detects `language: python` in manifest
- [x] Python package archive extracted to staging directory
- [x] Vendor directory preserved with correct structure
- [x] Python source modules extracted and importable
- [x] Manifest v2 format parsed with Python-specific fields
- [x] Integration with existing loader module (new python_loader alongside package_loader)

## Implementation Notes

### Manifest Detection and Parsing

```rust
// crates/cloacina/src/registry/loader/manifest.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageLanguage {
    Rust,
    Python,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestV2 {
    pub format_version: String,
    pub package: PackageInfo,
    pub language: PackageLanguage,

    // Language-specific runtime config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub python: Option<PythonRuntime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust: Option<RustRuntime>,

    pub tasks: Vec<TaskDefinition>,
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonRuntime {
    pub requires_python: String,  // e.g., ">=3.11"
    pub entry_module: String,     // e.g., "my_workflow.tasks"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustRuntime {
    pub target: String,  // e.g., "linux-x86_64"
}

impl ManifestV2 {
    pub fn from_bytes(data: &[u8]) -> Result<Self, ManifestError> {
        serde_json::from_slice(data)
            .map_err(|e| ManifestError::ParseError(e.to_string()))
    }

    pub fn is_python(&self) -> bool {
        matches!(self.language, PackageLanguage::Python)
    }

    pub fn is_rust(&self) -> bool {
        matches!(self.language, PackageLanguage::Rust)
    }
}
```

### Python Package Extraction

```rust
// crates/cloacina/src/registry/loader/python_loader.rs

use std::path::{Path, PathBuf};
use tar::Archive;
use flate2::read::GzDecoder;

/// Extracted Python package ready for execution
pub struct ExtractedPythonPackage {
    /// Root directory containing extracted package
    pub root_dir: PathBuf,
    /// Path to vendor directory (for sys.path)
    pub vendor_dir: PathBuf,
    /// Path to source directory (for sys.path)
    pub source_dir: PathBuf,
    /// Entry module to import tasks from
    pub entry_module: String,
    /// Parsed manifest
    pub manifest: ManifestV2,
}

pub fn extract_python_package(
    archive_data: &[u8],
    staging_dir: &Path,
) -> Result<ExtractedPythonPackage, LoaderError> {
    // Create staging directory
    let package_dir = staging_dir.join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&package_dir)?;

    // Extract tarball
    let decoder = GzDecoder::new(archive_data);
    let mut archive = Archive::new(decoder);
    archive.unpack(&package_dir)?;

    // Read manifest
    let manifest_path = package_dir.join("manifest.json");
    let manifest_data = std::fs::read(&manifest_path)?;
    let manifest = ManifestV2::from_bytes(&manifest_data)?;

    // Validate it's a Python package
    if !manifest.is_python() {
        return Err(LoaderError::WrongLanguage {
            expected: "python".to_string(),
            actual: "rust".to_string(),
        });
    }

    let python_config = manifest.python.as_ref()
        .ok_or(LoaderError::MissingPythonConfig)?;

    // Locate directories
    let vendor_dir = package_dir.join("vendor");
    let source_dir = package_dir.join("src");

    // Validate structure
    if !source_dir.exists() {
        return Err(LoaderError::MissingSourceDir);
    }

    Ok(ExtractedPythonPackage {
        root_dir: package_dir,
        vendor_dir,
        source_dir,
        entry_module: python_config.entry_module.clone(),
        manifest,
    })
}
```

### Package Loader Trait Extension

```rust
// crates/cloacina/src/registry/loader/mod.rs

/// Extended loader that handles both Rust and Python packages
pub trait UnifiedPackageLoader: Send + Sync {
    /// Load a package from binary data
    fn load_package(&self, data: &[u8]) -> Result<LoadedPackage, LoaderError>;
}

pub enum LoadedPackage {
    /// Rust package with dynamic library
    Rust(RustPackage),
    /// Python package with extracted files
    Python(ExtractedPythonPackage),
}

impl LoadedPackage {
    pub fn manifest(&self) -> &ManifestV2 {
        match self {
            LoadedPackage::Rust(p) => &p.manifest,
            LoadedPackage::Python(p) => &p.manifest,
        }
    }

    pub fn task_ids(&self) -> Vec<&str> {
        self.manifest().tasks.iter()
            .map(|t| t.id.as_str())
            .collect()
    }
}
```

### Package Archive Structure (Python)

The Python package archive (.cloaca) has this structure:

```
my-workflow-1.0.0-linux-x86_64.cloaca (tar.gz)
├── manifest.json          # Package metadata and task definitions
├── requirements.lock      # Pinned dependencies with hashes
├── src/                   # Python source code
│   └── my_workflow/
│       ├── __init__.py
│       └── tasks.py       # @task decorated functions
└── vendor/                # Vendored dependencies
    ├── requests/
    ├── requests-2.31.0.dist-info/
    ├── urllib3/
    └── ...
```

### Registration with Task Registry

```rust
// crates/cloacina/src/registry/loader/task_registrar/python_task.rs

use super::ExtractedPythonPackage;
use crate::task::{Task, TaskNamespace};

/// A task that executes Python code via PyO3
pub struct PythonTask {
    /// Extracted package containing the Python code
    package: Arc<ExtractedPythonPackage>,
    /// Task ID within the package
    task_id: String,
    /// Task dependencies
    dependencies: Vec<TaskNamespace>,
}

impl PythonTask {
    pub fn from_package(
        package: Arc<ExtractedPythonPackage>,
        task_def: &TaskDefinition,
    ) -> Self {
        Self {
            package,
            task_id: task_def.id.clone(),
            dependencies: task_def.dependencies.iter()
                .map(|d| TaskNamespace::parse(d))
                .collect(),
        }
    }
}

#[async_trait::async_trait]
impl Task for PythonTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    fn dependencies(&self) -> &[TaskNamespace] {
        &self.dependencies
    }

    async fn execute(
        &self,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        // Delegate to Python executor (T-0070)
        crate::python::execute_python_task(
            &self.package,
            &self.task_id,
            context,
        ).await
    }
}
```

### Loader Factory

```rust
// crates/cloacina/src/registry/loader/factory.rs

pub struct PackageLoaderFactory {
    staging_dir: PathBuf,
}

impl PackageLoaderFactory {
    pub fn new(staging_dir: PathBuf) -> Self {
        Self { staging_dir }
    }

    pub fn load(&self, data: &[u8]) -> Result<LoadedPackage, LoaderError> {
        // Peek at manifest to determine language
        let manifest = self.peek_manifest(data)?;

        match manifest.language {
            PackageLanguage::Rust => {
                let rust_pkg = load_rust_package(data)?;
                Ok(LoadedPackage::Rust(rust_pkg))
            }
            PackageLanguage::Python => {
                let python_pkg = extract_python_package(data, &self.staging_dir)?;
                Ok(LoadedPackage::Python(python_pkg))
            }
        }
    }

    fn peek_manifest(&self, data: &[u8]) -> Result<ManifestV2, LoaderError> {
        // Extract just manifest.json without full extraction
        let decoder = GzDecoder::new(data);
        let mut archive = Archive::new(decoder);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            if path.file_name() == Some("manifest.json".as_ref()) {
                let mut manifest_data = Vec::new();
                entry.read_to_end(&mut manifest_data)?;
                return ManifestV2::from_bytes(&manifest_data);
            }
        }

        Err(LoaderError::MissingManifest)
    }
}
```

### Error Types

```rust
// crates/cloacina/src/registry/loader/error.rs

#[derive(Debug, thiserror::Error)]
pub enum LoaderError {
    #[error("Missing manifest.json in package")]
    MissingManifest,

    #[error("Failed to parse manifest: {0}")]
    ManifestParse(String),

    #[error("Wrong package language: expected {expected}, got {actual}")]
    WrongLanguage { expected: String, actual: String },

    #[error("Missing python configuration in manifest")]
    MissingPythonConfig,

    #[error("Missing src/ directory in Python package")]
    MissingSourceDir,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Archive extraction failed: {0}")]
    ExtractionFailed(String),
}
```

### Technical Dependencies

- **T-0066**: Manifest schema v2 with Python support
- **T-0068**: Archive structure with vendor directory

### Risk Considerations

1. **Staging directory cleanup**: Need lifecycle management for extracted packages
2. **Disk space**: Large packages with many vendored deps could consume space
3. **Concurrent access**: Multiple workflows loading same package need Arc sharing
4. **Python version mismatch**: Server Python version must match package requirements

## Status Updates

### Completed
- Created `crates/cloacina/src/registry/loader/python_loader.rs` — `ExtractedPythonPackage` struct, `peek_manifest()`, `detect_package_kind()`, `extract_python_package()` with full archive extraction, manifest validation, language check
- Extended `crates/cloacina/src/registry/error.rs` — added `WrongLanguage`, `MissingPythonConfig`, `MissingManifest`, `ManifestParse`, `MissingSourceDir` variants to `LoaderError`
- Updated `crates/cloacina/src/registry/loader/mod.rs` — registered `python_loader` module and re-exports
- 6 unit tests: peek manifest, detect kind, extract package, missing workflow dir, missing manifest, wrong language
