# cloacina::registry::loader::validator::types <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Types for package validation results and assessments.

## Structs

### `cloacina::registry::loader::validator::types::ValidationResult`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Package validation results

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `is_valid` | `bool` | Whether the package passed all validations |
| `errors` | `Vec < String >` | List of validation errors (if any) |
| `warnings` | `Vec < String >` | List of validation warnings (non-fatal issues) |
| `security_level` | `SecurityLevel` | Security assessment |
| `compatibility` | `CompatibilityInfo` | Compatibility assessment |



### `cloacina::registry::loader::validator::types::CompatibilityInfo`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Compatibility information for packages

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `architecture` | `String` | Target architecture of the package |
| `required_symbols` | `Vec < String >` | Required symbols present |
| `missing_symbols` | `Vec < String >` | Missing required symbols |
| `cloacina_version` | `Option < String >` | cloacina version compatibility (if detectable) |



## Enums

### `cloacina::registry::loader::validator::types::SecurityLevel` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Security assessment levels for packages

#### Variants

- **`Safe`** - Package appears safe for production use
- **`Warning`** - Package has minor security concerns but is likely safe
- **`Dangerous`** - Package has significant security risks
- **`Unknown`** - Package cannot be assessed (insufficient information)
