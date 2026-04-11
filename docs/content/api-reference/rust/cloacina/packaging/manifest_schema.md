# cloacina::packaging::manifest_schema <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified package manifest (v2) supporting both Rust and Python workflows.

The v2 manifest extends the original Rust-only format to support Python
workflow packages. It uses a language discriminator to determine which
runtime configuration applies.

## Structs

### `cloacina::packaging::manifest_schema::PythonRuntime`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Python runtime configuration.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `requires_python` | `String` | PEP 440 version specifier (e.g., ">=3.10"). |
| `entry_module` | `String` | Entry module for task discovery (e.g., "workflow.tasks"). |



### `cloacina::packaging::manifest_schema::RustRuntime`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Rust runtime configuration.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `library_path` | `String` | Relative path to the compiled dynamic library within the package. |



### `cloacina::packaging::manifest_schema::PackageInfo`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Package metadata.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Package name. |
| `version` | `String` | Semantic version. |
| `description` | `Option < String >` | Optional description. |
| `fingerprint` | `String` | SHA-256 fingerprint of package contents. |
| `targets` | `Vec < String >` | Target platforms this package supports. |



### `cloacina::packaging::manifest_schema::TaskDefinition`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Task definition within a package.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `String` | Task identifier (unique within the package). |
| `function` | `String` | Callable function path.

For Python: `"module.path:function_name"`
For Rust: `"symbol_name"` (FFI symbol in the compiled library) |
| `dependencies` | `Vec < String >` | IDs of tasks that must complete before this one. |
| `description` | `Option < String >` | Human-readable description. |
| `retries` | `u32` | Number of automatic retries on failure. |
| `timeout_seconds` | `Option < u64 >` | Maximum execution time in seconds. |



### `cloacina::packaging::manifest_schema::TriggerDefinition`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Trigger definition within a package.

Declares a trigger that should be auto-registered when the package is loaded.
Any type implementing the `Trigger` trait can be packaged this way.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Unique name for this trigger (within the package). |
| `trigger_type` | `String` | Trigger type discriminator (e.g. `"rust"`, `"python"`, `"webhook"`,
`"http_poll"`, `"file_watch"`, or any user-defined string). |
| `workflow` | `String` | Name of the workflow to fire when this trigger activates. |
| `poll_interval` | `String` | How often to poll (e.g. `"30s"`, `"5m"`, `"100ms"`). |
| `allow_concurrent` | `bool` | Whether to allow concurrent executions with the same context. |
| `config` | `Option < serde_json :: Value >` | Trigger-specific configuration (e.g. URL, file path, custom params). |



### `cloacina::packaging::manifest_schema::Manifest`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Unified package manifest (v2).

Supports both Rust (dynamic library) and Python workflow packages.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `format_version` | `String` | Format version, always "2" for this schema. |
| `package` | `PackageInfo` | Package metadata. |
| `language` | `PackageLanguage` | Package language. |
| `python` | `Option < PythonRuntime >` | Python runtime config (required when `language == Python`). |
| `rust` | `Option < RustRuntime >` | Rust runtime config (required when `language == Rust`). |
| `tasks` | `Vec < TaskDefinition >` | Task definitions. |
| `triggers` | `Vec < TriggerDefinition >` | Trigger definitions (optional — packages without triggers omit this). |
| `created_at` | `DateTime < Utc >` | When the manifest was created. |
| `signature` | `Option < String >` | Package signature (optional). |

#### Methods

##### `validate` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn validate (& self) -> Result < () , ManifestValidationError >
```

Validate the manifest for structural correctness.

<details>
<summary>Source</summary>

```rust
    pub fn validate(&self) -> Result<(), ManifestValidationError> {
        if self.format_version != "2" {
            return Err(ManifestValidationError::InvalidFormatVersion {
                version: self.format_version.clone(),
            });
        }

        match self.language {
            PackageLanguage::Python if self.python.is_none() => {
                return Err(ManifestValidationError::MissingRuntime {
                    language: "python".to_string(),
                });
            }
            PackageLanguage::Rust if self.rust.is_none() => {
                return Err(ManifestValidationError::MissingRuntime {
                    language: "rust".to_string(),
                });
            }
            _ => {}
        }

        for target in &self.package.targets {
            if !SUPPORTED_TARGETS.contains(&target.as_str()) {
                return Err(ManifestValidationError::UnsupportedTarget {
                    target: target.clone(),
                });
            }
        }

        if self.tasks.is_empty() {
            return Err(ManifestValidationError::NoTasks);
        }

        let mut seen_ids = std::collections::HashSet::new();
        for task in &self.tasks {
            if !seen_ids.insert(&task.id) {
                return Err(ManifestValidationError::DuplicateTaskId {
                    id: task.id.clone(),
                });
            }
        }

        for task in &self.tasks {
            for dep in &task.dependencies {
                if !seen_ids.contains(dep) {
                    return Err(ManifestValidationError::InvalidDependency {
                        task_id: task.id.clone(),
                        dep_id: dep.clone(),
                    });
                }
            }
        }

        if self.language == PackageLanguage::Python {
            for task in &self.tasks {
                if !task.function.contains(':') {
                    return Err(ManifestValidationError::InvalidFunctionPath {
                        path: task.function.clone(),
                    });
                }
            }
        }

        // Validate triggers (if any)
        let mut seen_trigger_names = std::collections::HashSet::new();
        // Workflow names that triggers can reference: use the package name as
        // the workflow identifier (matching how packaged workflows are registered).
        let valid_workflow_names: std::collections::HashSet<&str> =
            std::iter::once(self.package.name.as_str())
                .chain(self.tasks.iter().map(|t| t.id.as_str()))
                .collect();

        for trigger in &self.triggers {
            if !seen_trigger_names.insert(&trigger.name) {
                return Err(ManifestValidationError::DuplicateTriggerName {
                    name: trigger.name.clone(),
                });
            }

            if !valid_workflow_names.contains(trigger.workflow.as_str()) {
                return Err(ManifestValidationError::InvalidTriggerWorkflow {
                    trigger_name: trigger.name.clone(),
                    workflow: trigger.workflow.clone(),
                });
            }

            parse_duration_str(&trigger.poll_interval).map_err(|reason| {
                ManifestValidationError::InvalidTriggerPollInterval {
                    trigger_name: trigger.name.clone(),
                    interval: trigger.poll_interval.clone(),
                    reason,
                }
            })?;
        }

        Ok(())
    }
```

</details>



##### `is_compatible_with_platform` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_compatible_with_platform (& self , platform_str : & str) -> bool
```

Check if this package is compatible with a specific platform.

<details>
<summary>Source</summary>

```rust
    pub fn is_compatible_with_platform(&self, platform_str: &str) -> bool {
        self.package.targets.contains(&platform_str.to_string())
    }
```

</details>





## Enums

### `cloacina::packaging::manifest_schema::ManifestValidationError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors from manifest validation.

#### Variants

- **`MissingRuntime`**
- **`UnsupportedTarget`**
- **`NoTasks`**
- **`DuplicateTaskId`**
- **`InvalidDependency`**
- **`InvalidFunctionPath`**
- **`InvalidFormatVersion`**
- **`DuplicateTriggerName`**
- **`InvalidTriggerWorkflow`**
- **`InvalidTriggerPollInterval`**



### `cloacina::packaging::manifest_schema::PackageLanguage` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Package language discriminator.

#### Variants

- **`Python`**
- **`Rust`**



## Functions

### `cloacina::packaging::manifest_schema::parse_duration_str`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn parse_duration_str (s : & str) -> Result < std :: time :: Duration , String >
```

Parse a duration string like "30s", "5m", "2h", "100ms" into a [`std::time::Duration`].

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
