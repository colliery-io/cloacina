# cloacina::registry::workflow_registry <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Complete implementation of the workflow registry.

This module provides the `WorkflowRegistryImpl` that combines all registry
components - storage, loading, validation, and task registration - into a
cohesive system for managing packaged workflows.

## Structs

### `cloacina::registry::workflow_registry::WorkflowRegistryImpl`<S: RegistryStorage>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Complete implementation of the workflow registry.

This registry implementation combines storage backends, package loading,
validation, and task registration to provide a full-featured system for
managing packaged workflows with proper lifecycle management.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `storage` | `S` | Storage backend for binary data |
| `database` | `Database` | Database for metadata storage |
| `loader` | `PackageLoader` | Package loader for metadata extraction (FFI-driven; the
reconciler reads metadata via fidius directly). |
| `registrar` | `TaskRegistrar` | Task registrar for global registry integration |
| `loaded_packages` | `HashMap < Uuid , Vec < TaskNamespace > >` | Map of package IDs to registered task namespaces for cleanup tracking |



## Functions

### `cloacina::registry::workflow_registry::extract_source_files`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn extract_source_files (archive_bytes : & [u8]) -> Result < Vec < WorkflowSourceFile > , RegistryError >
```

Unpack a `.cloacina` source archive in a temp dir and collect its UTF-8 text files for display (CLOACI-T-0750). Binary, oversized, and unreadable files are skipped; the result is sorted by path. The temp dir is removed when the returned `TempDir` guard drops at end of scope.

<details>
<summary>Source</summary>

```rust
fn extract_source_files(archive_bytes: &[u8]) -> Result<Vec<WorkflowSourceFile>, RegistryError> {
    let work_dir = tempfile::TempDir::new()
        .map_err(|e| RegistryError::Internal(format!("Failed to create temp dir: {}", e)))?;
    let archive_path = work_dir.path().join("pkg.cloacina");
    std::fs::write(&archive_path, archive_bytes)
        .map_err(|e| RegistryError::Internal(format!("Failed to write archive: {}", e)))?;
    let extract_dir = work_dir.path().join("source");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| RegistryError::Internal(format!("Failed to create extract dir: {}", e)))?;

    let source_dir =
        fidius_core::package::unpack_package(&archive_path, &extract_dir).map_err(|e| {
            RegistryError::ValidationError {
                reason: format!("Failed to unpack source archive: {}", e),
            }
        })?;

    let mut files = Vec::new();
    collect_source_files(&source_dir, &source_dir, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}
```

</details>



### `cloacina::registry::workflow_registry::collect_source_files`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn collect_source_files (root : & Path , dir : & Path , out : & mut Vec < WorkflowSourceFile > ,) -> Result < () , RegistryError >
```

Recursively walk `dir`, pushing each UTF-8 text file (path relative to `root`) into `out`. Binary, oversized, and unreadable files are silently skipped so a single odd file never fails the whole request.

<details>
<summary>Source</summary>

```rust
fn collect_source_files(
    root: &Path,
    dir: &Path,
    out: &mut Vec<WorkflowSourceFile>,
) -> Result<(), RegistryError> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| RegistryError::Internal(format!("Failed to read source dir: {}", e)))?;
    for entry in entries {
        let entry = entry
            .map_err(|e| RegistryError::Internal(format!("Failed to read dir entry: {}", e)))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| RegistryError::Internal(format!("Failed to stat dir entry: {}", e)))?;

        if file_type.is_dir() {
            collect_source_files(root, &path, out)?;
        } else if file_type.is_file() {
            // Skip oversized files without reading them into memory.
            if let Ok(meta) = entry.metadata() {
                if meta.len() > MAX_SOURCE_FILE_BYTES {
                    continue;
                }
            }
            // Only surface valid UTF-8; binary files are skipped.
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let Ok(contents) = String::from_utf8(bytes) else {
                continue;
            };
            let rel = path.strip_prefix(root).unwrap_or(&path);
            out.push(WorkflowSourceFile {
                path: rel.to_string_lossy().replace('\\', "/"),
                contents,
            });
        }
    }
    Ok(())
}
```

</details>
