# cloacina::packaging <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Workflow packaging functionality for creating distributable workflow packages.

This module provides the core library functions for packaging workflow projects
into distributable fidius source archives. These functions can be used by CLI
tools, tests, or other applications that need to package workflows.

## Functions

### `cloacina::packaging::package_workflow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn package_workflow (project_path : PathBuf , output_path : PathBuf) -> Result < () >
```

High-level function to package a workflow project using fidius source packaging.

This function performs the packaging pipeline:
1. Validates the project structure (Cargo.toml, src/, cdylib crate type)
2. Verifies that a `package.toml` exists in the project directory
3. Calls `fidius_core::package::pack_package` to create the bzip2 tar archive

<details>
<summary>Source</summary>

```rust
pub fn package_workflow(project_path: PathBuf, output_path: PathBuf) -> Result<()> {
    // Step 1: Validate the project structure
    validation::validate_rust_crate_structure(&project_path)?;
    let cargo_toml = validation::validate_cargo_toml(&project_path)?;
    validation::validate_cloacina_compatibility(&cargo_toml)?;
    validation::validate_packaged_workflow_presence(&project_path)?;

    // Step 2: Verify package.toml exists
    let package_toml_path = project_path.join("package.toml");
    if !package_toml_path.exists() {
        bail!(
            "package.toml not found in project directory: {:?}. \
            Create a package.toml with [package] name, version, interface, interface_version, \
            and extension = \"cloacina\" fields.",
            project_path
        );
    }

    // Step 3: Pack the source package using fidius
    fidius_core::package::pack_package(&project_path, Some(&output_path))
        .map_err(|e| anyhow::anyhow!("Failed to pack package: {}", e))?;

    Ok(())
}
```

</details>
