# cloacina::registry::loader::python_loader <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Server-side Python package loader.

Extracts `.cloacina` source archives containing Python workflow packages,
validates `package.toml` with `CloacinaMetadata`, and prepares the package
for task execution via PyO3.

## Structs

### `cloacina::registry::loader::python_loader::ExtractedPythonPackage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

An extracted Python package ready for task execution.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `root_dir` | `PathBuf` | Root directory containing the extracted source. |
| `vendor_dir` | `PathBuf` | Path to the `vendor/` directory (added to `sys.path`). |
| `workflow_dir` | `PathBuf` | Path to the `workflow/` directory (added to `sys.path`). |
| `entry_module` | `String` | Entry module to import tasks from (e.g., `"workflow.tasks"`). |
| `package_name` | `String` | Package name from `package.toml`. |
| `version` | `String` | Package version from `package.toml`. |
| `workflow_name` | `String` | Workflow name from metadata. |



## Enums

### `cloacina::registry::loader::python_loader::PackageKind` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Result of detecting the package language from a source archive.

#### Variants

- **`Python`** - Python workflow package.
- **`Rust`** - Rust dynamic-library package.



## Functions

### `cloacina::registry::loader::python_loader::detect_package_kind`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn detect_package_kind (archive_data : & [u8]) -> Result < PackageKind , LoaderError >
```

Detect the package kind (Python or Rust) from a `.cloacina` source archive.

Unpacks the archive to a temp dir, reads `package.toml`, and checks
the `language` field in `CloacinaMetadata`.

<details>
<summary>Source</summary>

```rust
pub fn detect_package_kind(archive_data: &[u8]) -> Result<PackageKind, LoaderError> {
    let tmp = tempfile::TempDir::new().map_err(|e| LoaderError::FileSystem {
        path: "tempdir".to_string(),
        error: e.to_string(),
    })?;

    let archive_path = tmp.path().join("pkg.cloacina");
    std::fs::write(&archive_path, archive_data).map_err(|e| LoaderError::FileSystem {
        path: archive_path.display().to_string(),
        error: e.to_string(),
    })?;

    let extract_dir = tmp.path().join("extract");
    std::fs::create_dir_all(&extract_dir).map_err(|e| LoaderError::FileSystem {
        path: extract_dir.display().to_string(),
        error: e.to_string(),
    })?;

    let source_dir =
        fidius_core::package::unpack_package(&archive_path, &extract_dir).map_err(|e| {
            LoaderError::MetadataExtraction {
                reason: format!("Failed to unpack source archive: {e}"),
            }
        })?;

    let manifest =
        fidius_core::package::load_manifest::<cloacina_workflow_plugin::CloacinaMetadata>(
            &source_dir,
        )
        .map_err(|e| LoaderError::ManifestParse {
            reason: format!("Failed to parse package.toml: {e}"),
        })?;

    let pkg = &manifest.package;
    let meta = &manifest.metadata;

    let wf_name = meta
        .effective_workflow_name()
        .unwrap_or("unknown")
        .to_string();

    match meta.language.as_str() {
        "python" => Ok(PackageKind::Python {
            workflow_name: wf_name,
            package_name: pkg.name.clone(),
            version: pkg.version.clone(),
        }),
        _ => Ok(PackageKind::Rust {
            workflow_name: wf_name,
            package_name: pkg.name.clone(),
            version: pkg.version.clone(),
        }),
    }
}
```

</details>



### `cloacina::registry::loader::python_loader::extract_python_package`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn extract_python_package (archive_data : & [u8] , staging_dir : & Path ,) -> Result < ExtractedPythonPackage , LoaderError >
```

Extract a Python workflow package from a `.cloacina` source archive.

The archive is unpacked via fidius into a sub-directory of *staging_dir*.
The returned [`ExtractedPythonPackage`] contains paths to the
workflow source and vendored dependencies.

<details>
<summary>Source</summary>

```rust
pub fn extract_python_package(
    archive_data: &[u8],
    staging_dir: &Path,
) -> Result<ExtractedPythonPackage, LoaderError> {
    // Write archive to staging dir
    let archive_path = staging_dir.join(format!("{}.cloacina", uuid::Uuid::new_v4()));
    std::fs::write(&archive_path, archive_data).map_err(|e| LoaderError::FileSystem {
        path: archive_path.display().to_string(),
        error: e.to_string(),
    })?;

    // Unpack via fidius
    let extract_dir = staging_dir.join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&extract_dir).map_err(|e| LoaderError::FileSystem {
        path: extract_dir.display().to_string(),
        error: e.to_string(),
    })?;

    let source_dir =
        fidius_core::package::unpack_package(&archive_path, &extract_dir).map_err(|e| {
            LoaderError::FileSystem {
                path: archive_path.display().to_string(),
                error: format!("Failed to unpack source archive: {e}"),
            }
        })?;

    // Read package.toml
    let manifest =
        fidius_core::package::load_manifest::<cloacina_workflow_plugin::CloacinaMetadata>(
            &source_dir,
        )
        .map_err(|e| LoaderError::ManifestParse {
            reason: format!("Failed to parse package.toml: {e}"),
        })?;

    // Validate language
    if manifest.metadata.language != "python" {
        return Err(LoaderError::WrongLanguage {
            expected: "python".to_string(),
            actual: manifest.metadata.language.clone(),
        });
    }

    let entry_module = manifest
        .metadata
        .entry_module
        .as_ref()
        .ok_or(LoaderError::MissingPythonConfig)?
        .clone();

    let vendor_dir = source_dir.join("vendor");
    let workflow_dir = source_dir.join("workflow");

    // Workflow directory is required
    if !workflow_dir.exists() {
        return Err(LoaderError::MissingSourceDir);
    }

    // Clean up archive file
    let _ = std::fs::remove_file(&archive_path);

    Ok(ExtractedPythonPackage {
        root_dir: source_dir,
        vendor_dir,
        workflow_dir,
        entry_module,
        package_name: manifest.package.name,
        version: manifest.package.version,
        workflow_name: manifest
            .metadata
            .effective_workflow_name()
            .unwrap_or("unknown")
            .to_string(),
    })
}
```

</details>
