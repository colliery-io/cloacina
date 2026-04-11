# cloacina::registry::reconciler::extraction <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Source package compilation — unpacks a bzip2 tar source archive and compiles it to a cdylib using `cargo build`.

## Functions

### `cloacina::registry::reconciler::extraction::host_workspace_root`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn host_workspace_root () -> PathBuf
```

Returns the host workspace root, derived from `CARGO_MANIFEST_DIR` at compile time. `CARGO_MANIFEST_DIR` for the cloacina crate is `<root>/crates/cloacina`.

<details>
<summary>Source</summary>

```rust
fn host_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR should have parent (crates/)")
        .parent()
        .expect("crates/ should have parent (workspace root)")
        .to_path_buf()
}
```

</details>



### `cloacina::registry::reconciler::extraction::rewrite_host_dependencies`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn rewrite_host_dependencies (source_dir : & Path) -> Result < () , RegistryError >
```

Rewrite path dependencies in an extracted source package's Cargo.toml to point to the host's workspace crates. Debug builds only.

This solves the chicken-and-egg problem: source packages need cloacina
crates to compile, but we can't publish them to crates.io before testing.
In debug mode, we inject the host's local workspace paths so everything
resolves without requiring published crates.

<details>
<summary>Source</summary>

```rust
fn rewrite_host_dependencies(source_dir: &Path) -> Result<(), RegistryError> {
    let cargo_toml_path = source_dir.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml_path).map_err(|e| {
        RegistryError::RegistrationFailed {
            message: format!(
                "Failed to read Cargo.toml at {}: {}",
                cargo_toml_path.display(),
                e
            ),
        }
    })?;

    let mut doc: toml::Value =
        content
            .parse::<toml::Value>()
            .map_err(|e| RegistryError::RegistrationFailed {
                message: format!("Failed to parse Cargo.toml: {}", e),
            })?;

    let workspace_root = host_workspace_root();
    let dep_tables = ["dependencies", "dev-dependencies", "build-dependencies"];
    let mut modified = false;

    for table_name in &dep_tables {
        if let Some(table) = doc.get_mut(table_name).and_then(|v| v.as_table_mut()) {
            for &(crate_name, crate_subpath) in HOST_CRATES {
                if let Some(dep_value) = table.get_mut(crate_name) {
                    let abs_path = workspace_root.join(crate_subpath);
                    let abs_path_str = abs_path.to_string_lossy().to_string();

                    match dep_value {
                        toml::Value::Table(dep_table) => {
                            dep_table.insert("path".to_string(), toml::Value::String(abs_path_str));
                            modified = true;
                        }
                        toml::Value::String(_version) => {
                            let mut dep_table = toml::map::Map::new();
                            dep_table.insert("version".to_string(), dep_value.clone());
                            dep_table.insert("path".to_string(), toml::Value::String(abs_path_str));
                            *dep_value = toml::Value::Table(dep_table);
                            modified = true;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Ensure bare [workspace] exists to prevent parent workspace lookup
    if doc.get("workspace").is_none() {
        if let Some(table) = doc.as_table_mut() {
            table.insert(
                "workspace".to_string(),
                toml::Value::Table(toml::map::Map::new()),
            );
            modified = true;
        }
    }

    if modified {
        let new_content =
            toml::to_string_pretty(&doc).map_err(|e| RegistryError::RegistrationFailed {
                message: format!("Failed to serialize modified Cargo.toml: {}", e),
            })?;

        std::fs::write(&cargo_toml_path, new_content).map_err(|e| {
            RegistryError::RegistrationFailed {
                message: format!("Failed to write modified Cargo.toml: {}", e),
            }
        })?;

        debug!(
            "Rewrote host dependencies in {} (workspace root: {})",
            cargo_toml_path.display(),
            workspace_root.display()
        );
    }

    Ok(())
}
```

</details>
