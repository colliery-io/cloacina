# cloacina::registry::loader::validator <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Package validator for ensuring workflow package safety and compatibility.

This module provides comprehensive validation of workflow packages before
they are registered and loaded, including security checks, symbol validation,
metadata verification, and compatibility testing.

## Structs

### `cloacina::registry::loader::validator::PackageValidator`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Comprehensive package validator

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `temp_dir` | `TempDir` | Temporary directory for validation operations |
| `strict_mode` | `bool` | Strict validation mode (fails on warnings) |
| `max_package_size` | `u64` | Maximum allowed package size in bytes |
| `required_symbols` | `HashSet < String >` | Required symbols for cloacina packages |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new () -> Result < Self , LoaderError >
```

Create a new package validator with default settings.

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Result<Self, LoaderError> {
        let temp_dir = TempDir::new().map_err(|e| LoaderError::TempDirectory {
            error: e.to_string(),
        })?;

        let mut required_symbols = HashSet::new();
        required_symbols.insert("fidius_get_registry".to_string());

        Ok(Self {
            temp_dir,
            strict_mode: false,
            max_package_size: 100 * 1024 * 1024, // 100MB default limit
            required_symbols,
        })
    }
```

</details>



##### `strict` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn strict () -> Result < Self , LoaderError >
```

Create a validator with strict validation mode enabled.

<details>
<summary>Source</summary>

```rust
    pub fn strict() -> Result<Self, LoaderError> {
        let mut validator = Self::new()?;
        validator.strict_mode = true;
        Ok(validator)
    }
```

</details>



##### `with_max_size` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_max_size (mut self , max_bytes : u64) -> Self
```

Set the maximum allowed package size.

<details>
<summary>Source</summary>

```rust
    pub fn with_max_size(mut self, max_bytes: u64) -> Self {
        self.max_package_size = max_bytes;
        self
    }
```

</details>



##### `with_required_symbols` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_required_symbols < I , S > (mut self , symbols : I) -> Self where I : IntoIterator < Item = S > , S : Into < String > ,
```

Add additional required symbols for validation.

<details>
<summary>Source</summary>

```rust
    pub fn with_required_symbols<I, S>(mut self, symbols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for symbol in symbols {
            self.required_symbols.insert(symbol.into());
        }
        self
    }
```

</details>



##### `validate_package` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn validate_package (& self , package_data : & [u8] , metadata : Option < & PackageMetadata > ,) -> Result < ValidationResult , LoaderError >
```

Validate a package comprehensively.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_data` | `-` | Binary data of the library package |
| `metadata` | `-` | Package metadata (if available) |


**Returns:**

* `Ok(ValidationResult)` - Validation completed (check is_valid field) * `Err(LoaderError)` - Validation process failed

<details>
<summary>Source</summary>

```rust
    pub async fn validate_package(
        &self,
        package_data: &[u8],
        metadata: Option<&PackageMetadata>,
    ) -> Result<ValidationResult, LoaderError> {
        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            security_level: SecurityLevel::Safe,
            compatibility: CompatibilityInfo {
                architecture: "unknown".to_string(),
                required_symbols: Vec::new(),
                missing_symbols: Vec::new(),
                cloacina_version: None,
            },
        };

        // Basic size validation
        self.validate_package_size(package_data, &mut result);

        // Write package to temporary file for analysis with correct extension
        let library_extension = get_library_extension();
        let temp_path = self
            .temp_dir
            .path()
            .join(format!("validation_package.{}", library_extension));
        fs::write(&temp_path, package_data)
            .await
            .map_err(|e| LoaderError::FileSystem {
                path: temp_path.to_string_lossy().to_string(),
                error: e.to_string(),
            })?;

        // File format validation
        self.validate_file_format(&temp_path, &mut result).await;

        // Symbol validation
        self.validate_symbols(&temp_path, &mut result).await;

        // Metadata validation
        if let Some(metadata) = metadata {
            self.validate_metadata(metadata, &mut result);
        }

        // Security assessment
        self.assess_security(&temp_path, &mut result).await;

        // Final validation decision
        if !result.errors.is_empty() || (self.strict_mode && !result.warnings.is_empty()) {
            result.is_valid = false;
        }

        Ok(result)
    }
```

</details>



##### `temp_dir` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn temp_dir (& self) -> & Path
```

Get the temporary directory path.

<details>
<summary>Source</summary>

```rust
    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }
```

</details>



##### `is_strict_mode` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_strict_mode (& self) -> bool
```

Check if strict mode is enabled.

<details>
<summary>Source</summary>

```rust
    pub fn is_strict_mode(&self) -> bool {
        self.strict_mode
    }
```

</details>



##### `max_package_size` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn max_package_size (& self) -> u64
```

Get the maximum package size limit.

<details>
<summary>Source</summary>

```rust
    pub fn max_package_size(&self) -> u64 {
        self.max_package_size
    }
```

</details>
