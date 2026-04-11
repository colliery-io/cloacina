# cloacina::packaging::manifest <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


## Structs

### `cloacina::packaging::manifest::PackageMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">pub(crate)</span>


**Derives:** `Debug`, `Clone`

Package metadata extracted from the plugin.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `description` | `Option < String >` |  |
| `_author` | `Option < String >` |  |
| `workflow_fingerprint` | `Option < String >` |  |



### `cloacina::packaging::manifest::FfiTaskInfo`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


**Derives:** `Debug`, `Clone`

Task information extracted from a cdylib via the fidius plugin API (internal type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `_index` | `u32` |  |
| `id` | `String` |  |
| `dependencies` | `Vec < String >` |  |
| `description` | `String` |  |
| `_source_location` | `String` |  |



## Enums

### `cloacina::packaging::manifest::ManifestError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during manifest extraction.

#### Variants

- **`InvalidDependencies`** - Failed to parse dependencies JSON for a task
- **`InvalidGraphData`** - Failed to parse graph data JSON
- **`LibraryError`** - Library loading or plugin call failed



## Functions

### `cloacina::packaging::manifest::generate_manifest`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn generate_manifest (cargo_toml : & CargoToml , so_path : & Path , target : & Option < String > , project_path : & Path ,) -> Result < Manifest >
```

Generate a package manifest from Cargo.toml and compiled library.

Returns a `Manifest` — the unified manifest format used by both
Rust and Python packages.

<details>
<summary>Source</summary>

```rust
pub fn generate_manifest(
    cargo_toml: &CargoToml,
    so_path: &Path,
    target: &Option<String>,
    project_path: &Path,
) -> Result<Manifest> {
    let package = cargo_toml
        .package
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing package section in Cargo.toml"))?;

    // Get library filename
    let library_filename = so_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid so_path"))?
        .to_string_lossy()
        .to_string();

    let (ffi_tasks, _graph_data, package_metadata) =
        extract_task_info_and_graph_from_library(so_path, project_path)?;

    // Determine target platform string
    let target_platform = if let Some(target_triple) = target {
        target_triple.clone()
    } else {
        get_current_platform()
    };

    // Build fingerprint from package name + version + workflow fingerprint
    let fingerprint = format!(
        "sha256:{}:{}:{}",
        package.name,
        package.version,
        package_metadata
            .workflow_fingerprint
            .as_deref()
            .unwrap_or("none")
    );

    // Convert FFI task info to TaskDefinition
    let tasks: Vec<TaskDefinition> = ffi_tasks
        .iter()
        .map(|t| TaskDefinition {
            id: t.id.clone(),
            function: "cloacina_execute_task".to_string(),
            dependencies: t.dependencies.clone(),
            description: if t.description.is_empty() {
                None
            } else {
                Some(t.description.clone())
            },
            retries: 0,
            timeout_seconds: None,
        })
        .collect();

    let manifest = Manifest {
        format_version: "2".to_string(),
        package: PackageInfo {
            name: package.name.clone(),
            version: package.version.clone(),
            description: package_metadata
                .description
                .or_else(|| Some(format!("Packaged workflow: {}", package.name))),
            fingerprint,
            targets: vec![target_platform],
        },
        language: PackageLanguage::Rust,
        python: None,
        rust: Some(RustRuntime {
            library_path: library_filename,
        }),
        tasks,
        triggers: vec![],
        created_at: chrono::Utc::now(),
        signature: None,
    };

    Ok(manifest)
}
```

</details>



### `cloacina::packaging::manifest::extract_task_info_and_graph_from_library`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn extract_task_info_and_graph_from_library (so_path : & Path , project_path : & Path ,) -> Result < (Vec < FfiTaskInfo > , Option < crate :: WorkflowGraphData > , PackageMetadata ,) >
```

Extract task information and graph data from a compiled library using the fidius plugin API.

<details>
<summary>Source</summary>

```rust
fn extract_task_info_and_graph_from_library(
    so_path: &Path,
    project_path: &Path,
) -> Result<(
    Vec<FfiTaskInfo>,
    Option<crate::WorkflowGraphData>,
    PackageMetadata,
)> {
    // Load via fidius-host — validates magic, ABI version, wire format, etc.
    let loaded = fidius_host::loader::load_library(so_path).with_context(|| {
        format!(
            "Failed to load plugin library for metadata extraction: {:?}",
            so_path
        )
    })?;

    let plugin = loaded
        .plugins
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Plugin library {:?} contains no plugins", so_path))?;

    let handle = fidius_host::PluginHandle::from_loaded(plugin);

    // Method index 0 = get_task_metadata (zero-arg, encoded as empty tuple)
    let meta: cloacina_workflow_plugin::PackageTasksMetadata = handle
        .call_method(0, &())
        .with_context(|| format!("Failed to call get_task_metadata on library {:?}", so_path))?;

    // Parse graph data if present
    let graph_data = if let Some(ref json) = meta.graph_data_json {
        if json.trim().is_empty() {
            None
        } else {
            Some(
                serde_json::from_str::<crate::WorkflowGraphData>(json)
                    .map_err(|e| ManifestError::InvalidGraphData { source: e })
                    .map_err(|e| anyhow::anyhow!("{}", e))?,
            )
        }
    } else {
        None
    };

    // Convert tasks
    let mut tasks = Vec::new();
    for t in meta.tasks {
        tasks.push(FfiTaskInfo {
            _index: t.index,
            id: t.id,
            dependencies: t.dependencies,
            description: t.description,
            _source_location: t.source_location,
        });
    }

    let package_metadata = PackageMetadata {
        description: meta.package_description,
        _author: meta.package_author,
        workflow_fingerprint: meta.workflow_fingerprint,
    };

    // Suppress unused variable warning if project_path isn't needed further
    let _ = project_path;

    Ok((tasks, graph_data, package_metadata))
}
```

</details>



### `cloacina::packaging::manifest::extract_package_names_from_source`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">pub(crate)</span>


```rust
fn extract_package_names_from_source (project_path : & Path) -> Result < Vec < String > >
```

Extract package names from source files by looking for #[packaged_workflow] attributes.

<details>
<summary>Source</summary>

```rust
pub(crate) fn extract_package_names_from_source(project_path: &Path) -> Result<Vec<String>> {
    let src_path = project_path.join("src");
    let mut package_names = Vec::new();

    for entry in std::fs::read_dir(&src_path)
        .with_context(|| format!("Failed to read src directory: {:?}", src_path))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read file: {:?}", path))?;

            for captures in PACKAGED_WORKFLOW_REGEX.captures_iter(&content) {
                if let Some(package_name) = captures.get(1) {
                    package_names.push(package_name.as_str().to_string());
                }
            }
        }
    }

    Ok(package_names)
}
```

</details>



### `cloacina::packaging::manifest::get_current_platform`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">pub(crate)</span>


```rust
fn get_current_platform () -> String
```

<details>
<summary>Source</summary>

```rust
pub(crate) fn get_current_platform() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let platform = match (os, arch) {
        ("macos", "aarch64") => "macos-arm64",
        ("macos", "x86_64") => "macos-x86_64",
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-arm64",
        _ => return format!("{}-{}", os, arch),
    };
    platform.to_string()
}
```

</details>



### `cloacina::packaging::manifest::get_current_architecture`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">pub(crate)</span>


```rust
fn get_current_architecture () -> String
```

Kept for backward compatibility with external callers.

<details>
<summary>Source</summary>

```rust
pub(crate) fn get_current_architecture() -> String {
    std::env::consts::ARCH.to_string()
}
```

</details>
