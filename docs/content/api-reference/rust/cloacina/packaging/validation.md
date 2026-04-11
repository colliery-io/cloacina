# cloacina::packaging::validation <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


## Functions

### `cloacina::packaging::validation::validate_rust_crate_structure`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn validate_rust_crate_structure (project_path : & PathBuf) -> Result < () >
```

Validate that the project has a valid Rust crate structure

<details>
<summary>Source</summary>

```rust
pub fn validate_rust_crate_structure(project_path: &PathBuf) -> Result<()> {
    let cargo_toml_path = project_path.join("Cargo.toml");

    if !cargo_toml_path.exists() {
        bail!(
            "Cargo.toml not found in project directory: {:?}",
            project_path
        );
    }

    let src_dir = project_path.join("src");
    if !src_dir.exists() {
        bail!(
            "src directory not found in project directory: {:?}",
            project_path
        );
    }

    Ok(())
}
```

</details>



### `cloacina::packaging::validation::validate_cargo_toml`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn validate_cargo_toml (project_path : & Path) -> Result < CargoToml >
```

Parse and validate Cargo.toml

<details>
<summary>Source</summary>

```rust
pub fn validate_cargo_toml(project_path: &Path) -> Result<CargoToml> {
    let cargo_toml_path = project_path.join("Cargo.toml");
    let cargo_toml_content = fs::read_to_string(&cargo_toml_path)
        .with_context(|| format!("Failed to read Cargo.toml: {:?}", cargo_toml_path))?;

    let cargo_toml: CargoToml =
        toml::from_str(&cargo_toml_content).context("Failed to parse Cargo.toml")?;

    // Validate that it's configured as a cdylib
    if let Some(lib) = &cargo_toml.lib {
        if let Some(crate_types) = &lib.crate_type {
            if !crate_types.contains(&"cdylib".to_string()) {
                bail!(
                    "Cargo.toml must specify crate-type = [\"cdylib\"] in [lib] section for workflow compilation"
                );
            }
        } else {
            bail!("Cargo.toml must specify crate-type = [\"cdylib\"] in [lib] section");
        }
    } else {
        bail!("Cargo.toml must have a [lib] section with crate-type = [\"cdylib\"]");
    }

    Ok(cargo_toml)
}
```

</details>



### `cloacina::packaging::validation::validate_cloacina_compatibility`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn validate_cloacina_compatibility (cargo_toml : & CargoToml) -> Result < () >
```

Validate cloacina dependency compatibility.

Packaged workflows need either `cloacina` (full engine) or the lightweight
pair of `cloacina-macros` + `cloacina-workflow` as dependencies.

<details>
<summary>Source</summary>

```rust
pub fn validate_cloacina_compatibility(cargo_toml: &CargoToml) -> Result<()> {
    let dependencies = cargo_toml
        .dependencies
        .as_ref()
        .ok_or_else(|| anyhow!("No dependencies section found in Cargo.toml"))?;

    let has_cloacina = dependencies.get("cloacina").is_some();
    let has_workflow = dependencies.get("cloacina-workflow").is_some();
    let has_macros = dependencies.get("cloacina-macros").is_some();

    if has_cloacina || (has_workflow && has_macros) {
        Ok(())
    } else {
        bail!(
            "Cargo.toml must depend on either `cloacina` or both `cloacina-macros` + `cloacina-workflow`"
        )
    }
}
```

</details>



### `cloacina::packaging::validation::validate_packaged_workflow_presence`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn validate_packaged_workflow_presence (project_path : & Path) -> Result < () >
```

Check for workflow macros in the source code.

Accepts both `#[workflow]` (new unified macro) and `#[packaged_workflow]` (legacy).

<details>
<summary>Source</summary>

```rust
pub fn validate_packaged_workflow_presence(project_path: &Path) -> Result<()> {
    let src_dir = project_path.join("src");
    let lib_rs = src_dir.join("lib.rs");
    let main_rs = src_dir.join("main.rs");

    let source_file = if lib_rs.exists() {
        lib_rs
    } else if main_rs.exists() {
        main_rs
    } else {
        return Err(anyhow!("Neither src/lib.rs nor src/main.rs found"));
    };

    let source_content = fs::read_to_string(&source_file)
        .with_context(|| format!("Failed to read source file: {:?}", source_file))?;

    // Match #[workflow(...)] or #[packaged_workflow(...)]
    let workflow_regex = Regex::new(r"#\[\s*(?:packaged_)?workflow\s*[\]\(]")
        .context("Failed to compile regex for workflow detection")?;

    if !workflow_regex.is_match(&source_content) {
        bail!(
            "No #[workflow] macro found in {:?}. \
            Workflows must use the #[workflow] macro to be packageable.",
            source_file
        );
    }

    Ok(())
}
```

</details>



### `cloacina::packaging::validation::validate_rust_version_compatibility`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn validate_rust_version_compatibility (cargo_toml : & CargoToml) -> Result < () >
```

Validate Rust version compatibility

<details>
<summary>Source</summary>

```rust
pub fn validate_rust_version_compatibility(cargo_toml: &CargoToml) -> Result<()> {
    if let Some(package) = &cargo_toml.package {
        if let Some(rust_version) = &package.rust_version {
            // Parse the required Rust version
            let required_version = semver::Version::parse(rust_version)
                .with_context(|| format!("Invalid rust-version format: {}", rust_version))?;

            // For now, just validate that it's a reasonable version (1.70+)
            let min_version =
                semver::Version::parse("1.70.0").context("Failed to parse minimum Rust version")?;

            if required_version < min_version {
                bail!(
                    "Rust version {} is too old. Minimum supported version is {}",
                    required_version,
                    min_version
                );
            }
        }
    }

    Ok(())
}
```

</details>
