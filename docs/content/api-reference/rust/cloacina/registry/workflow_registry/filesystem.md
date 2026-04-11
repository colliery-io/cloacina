# cloacina::registry::workflow_registry::filesystem <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Filesystem-backed workflow registry for daemon mode.

Implements `WorkflowRegistry` by scanning directories for `.cloacina` package
files. Packages live on disk — the filesystem IS the package store. SQLite
handles operational state (schedules, executions) separately.

## Structs

### `cloacina::registry::workflow_registry::filesystem::FilesystemWorkflowRegistry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


A `WorkflowRegistry` implementation backed by directories of `.cloacina` files.

The daemon uses this instead of the database-backed registry. Packages are
discovered by scanning watch directories for `.cloacina` files. Package data
is read from disk on demand — no blobs stored in the database.
Supports multiple watch directories so users can organize packages across
different locations (e.g., `~/.cloacina/packages/`, `/opt/workflows/`,
`~/my-project/packages/`).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `watch_dirs` | `Vec < PathBuf >` | Directories to scan for `.cloacina` package files. |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (watch_dirs : Vec < PathBuf >) -> Self
```

Create a new filesystem registry watching the given directories.

Directories that don't exist are logged as warnings but not rejected —
they may be created later (e.g., on first package drop).

<details>
<summary>Source</summary>

```rust
    pub fn new(watch_dirs: Vec<PathBuf>) -> Self {
        for dir in &watch_dirs {
            if !dir.exists() {
                warn!(
                    "Watch directory does not exist (will be watched if created later): {}",
                    dir.display()
                );
            }
        }
        Self { watch_dirs }
    }
```

</details>



##### `scan_packages` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn scan_packages (& self) -> HashMap < (String , String) , (PathBuf , WorkflowMetadata) >
```

Scan all watch directories for `.cloacina` files.

Returns a map of `(package_name, version)` -> `(path, archive_data, metadata)`.
Corrupt or unreadable files are logged and skipped.

<details>
<summary>Source</summary>

```rust
    fn scan_packages(&self) -> HashMap<(String, String), (PathBuf, WorkflowMetadata)> {
        let mut packages = HashMap::new();

        for dir in &self.watch_dirs {
            if !dir.exists() {
                debug!("Skipping non-existent watch directory: {}", dir.display());
                continue;
            }

            let entries = match std::fs::read_dir(dir) {
                Ok(entries) => entries,
                Err(e) => {
                    warn!("Failed to read watch directory {}: {}", dir.display(), e);
                    continue;
                }
            };

            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        warn!("Failed to read directory entry: {}", e);
                        continue;
                    }
                };

                let path = entry.path();

                // Only process .cloacina files
                if path.extension().and_then(|e| e.to_str()) != Some("cloacina") {
                    continue;
                }

                // Unpack archive to a temp dir and read package.toml
                let tmp = match tempfile::TempDir::new() {
                    Ok(t) => t,
                    Err(e) => {
                        warn!("Failed to create temp dir for {}: {}", path.display(), e);
                        continue;
                    }
                };

                let source_dir = match fidius_core::package::unpack_package(&path, tmp.path()) {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Skipping unreadable package {}: {}", path.display(), e);
                        continue;
                    }
                };

                match fidius_core::package::load_manifest::<
                    cloacina_workflow_plugin::CloacinaMetadata,
                >(&source_dir)
                {
                    Ok(manifest) => {
                        let package_name = manifest.package.name.clone();
                        let version = manifest.package.version.clone();

                        // Derive a stable package ID from name+version
                        let fingerprint = format!("{}:{}", package_name, version);
                        let id = uuid_from_fingerprint(&fingerprint);

                        let now = chrono::Utc::now();
                        let metadata = WorkflowMetadata {
                            id,
                            registry_id: id, // Same as id for filesystem registry
                            package_name: package_name.clone(),
                            version: version.clone(),
                            description: manifest.metadata.description.clone(),
                            author: manifest.metadata.author.clone(),
                            tasks: vec![],
                            schedules: Vec::new(),
                            created_at: now,
                            updated_at: now,
                        };

                        debug!(
                            "Found package: {} v{} at {}",
                            package_name,
                            version,
                            path.display()
                        );

                        // If duplicate (same name+version in different dirs), first one wins
                        packages
                            .entry((package_name, version))
                            .or_insert((path.clone(), metadata));
                    }
                    Err(e) => {
                        warn!("Skipping unreadable package {}: {}", path.display(), e);
                    }
                }
            }
        }

        packages
    }
```

</details>



##### `find_package_path` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn find_package_path (& self , package_name : & str , version : & str) -> Option < PathBuf >
```

Find the file path for a package by name and version.

<details>
<summary>Source</summary>

```rust
    fn find_package_path(&self, package_name: &str, version: &str) -> Option<PathBuf> {
        let packages = self.scan_packages();
        packages
            .get(&(package_name.to_string(), version.to_string()))
            .map(|(path, _)| path.clone())
    }
```

</details>





## Functions

### `cloacina::registry::workflow_registry::filesystem::uuid_from_fingerprint`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn uuid_from_fingerprint (fingerprint : & str) -> Uuid
```

Derive a deterministic UUID from a string fingerprint.

Uses UUID v5 (SHA-1 based) with a fixed namespace so the same
fingerprint always produces the same UUID.

<details>
<summary>Source</summary>

```rust
fn uuid_from_fingerprint(fingerprint: &str) -> Uuid {
    // Use the URL namespace as a base — the fingerprint is our "name"
    Uuid::new_v5(&Uuid::NAMESPACE_URL, fingerprint.as_bytes())
}
```

</details>
