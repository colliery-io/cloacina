# cloacina::packaging::types <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


## Structs

### `cloacina::packaging::types::CompileOptions`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Options for compiling a workflow

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `target` | `Option < String >` | Target triple for cross-compilation |
| `profile` | `String` | Build profile (debug/release) |
| `cargo_flags` | `Vec < String >` | Additional cargo flags |
| `jobs` | `Option < u32 >` | Number of parallel jobs |



### `cloacina::packaging::types::CargoToml`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Deserialize`

Parsed Cargo.toml structure

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `package` | `Option < CargoPackage >` |  |
| `lib` | `Option < CargoLib >` |  |
| `dependencies` | `Option < toml :: Value >` |  |



### `cloacina::packaging::types::CargoPackage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Deserialize`

Package section from Cargo.toml

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `version` | `String` |  |
| `description` | `Option < String >` |  |
| `authors` | `Option < Vec < String > >` |  |
| `keywords` | `Option < Vec < String > >` |  |
| `rust_version` | `Option < String >` |  |



### `cloacina::packaging::types::CargoLib`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Deserialize`

Library section from Cargo.toml

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `crate_type` | `Option < Vec < String > >` |  |
