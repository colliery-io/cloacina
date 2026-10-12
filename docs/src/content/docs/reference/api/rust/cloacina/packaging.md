---
title: "cloacina::packaging"
description: "Workflow packaging functionality for creating distributable workflow packages."
sidebar:
  order: 165
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Workflow packaging functionality for creating distributable workflow packages.

This module provides the core library functions for packaging workflow projects
into distributable fidius source archives. These functions can be used by CLI
tools, tests, or other applications that need to package workflows.

## Functions

### <span id="fn.package_workflow"></span>`fn package_workflow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn package_workflow (project_path : PathBuf , output_path : PathBuf) -> Result < () >
```

High-level function to package a workflow project using fidius source packaging.

This function performs the packaging workflow:
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



### <span id="fn.parse_duration_str"></span>`fn parse_duration_str`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn parse_duration_str (s : & str) -> Result < std :: time :: Duration , String >
```

Parse a duration string like "30s", "5m", "2h", "100ms" into a [`std::time::Duration`].

(Moved here from the removed test-only `manifest_schema` module —
CLOACI-T-0918. Shipped packages carry `package.toml`, not the
`manifest.json` format that module described; this parser was its one
production-used piece.)

<details>
<summary>Source</summary>

```rust
pub fn parse_duration_str(s: &str) -> Result<std::time::Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty string".to_string());
    }

    let (num_str, suffix) = if let Some(stripped) = s.strip_suffix("ms") {
        (stripped, "ms")
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
```

</details>



