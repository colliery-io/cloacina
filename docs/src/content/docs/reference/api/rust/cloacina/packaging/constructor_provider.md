---
title: "cloacina::packaging::constructor_provider"
description: "Constructor **provider package** assembly + packing (CLOACI-T-0827 / A-0011)."
editUrl: false
sidebar:
  order: 149
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Constructor **provider package** assembly + packing (CLOACI-T-0827 / A-0011).

Turns a built `#[constructor]` **provider crate** (a *suite* of constructor
members, CLOACI-A-0011) into a distributable, signable **fidius provider
package** — the same machinery cloacina already uses to pack a workflow into a
`.cloacina` archive ([`super::package_workflow`](../../../../rust/cloacina/packaging/#fn.package_workflow) →
`fidius_core::package::pack_package`), reused for constructor providers rather
than a parallel format.

**What a provider package is**

A provider package is an ordinary fidius source package — a directory with a
`package.toml` header that fidius understands — carrying:

  * the provider's single **component** — a `.wasm` component built to
    `wasm32-wasip2` (fidius `runtime = "wasm"`), OR a host **cdylib** for a
    NATIVE provider (fidius `runtime = "rust"`, CLOACI-T-0903) — which exposes
    EVERY member constructor behind one per-kind fidius interface (the member is
    selected at load by name-in-configure), and
  * `provider.json`, the `ProviderManifest` (`List[Constructor]`) the loader
    reads to select a member by `constructor = "<name>"` — written here from the
    macro-generated `__provider_manifest()` (the step the macro cannot do).

It is packed (tar + bzip2) into a `<name>-<version>.cloacina` archive and may be
**Ed25519-signed** (a `package.sig` over the package digest) reusing fidius's
signing scheme verbatim, so `fidius_host::verify_package` /
`fidius_core::package::package_digest` verify it unchanged.

**Providers carry N constructors (Airflow "provider" = package of operators)**

One provider crate compiles to ONE component exposing N members; the
`provider.json` is the `List[Constructor]` index over them. A homogeneous suite
(every member the same kind — the common case) exports one fidius interface, so
the `package.toml` header's `interface`/`interface_version` come from the
members (they share one). A mixed-kind suite (multiple interfaces in one
component) is a documented follow-on.

**Gating**

Behind the default-OFF `constructor-packaging` feature, which pulls only the
serde-only `cloacina-constructor-contract` crate. It deliberately does NOT
enable `fidius-host/wasm`, so building the *packaging* tool does not drag in
wasmtime — only the *loader* (`constructors-wasm`) does. Resolving a packed
provider back to a runnable primitive lives in the loader
(`registry::loader::constructor_loader::load_task_constructor_from_package`).

## Re-exports

- `ProviderRuntime` (`cloacina_constructor_contract::ProviderRuntime`)

## Structs

### <span id="struct.ProviderPackageOptions"></span><code style="font-size: 0.85em">struct ProviderPackageOptions</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Inputs to [`package_constructor_provider`](../../../../rust/cloacina/packaging/constructor_provider/#fn.package_constructor_provider).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `crate_dir` | `PathBuf` | The `#[constructor]` provider crate directory to package. |
| `output` | `Option<PathBuf>` | Output archive path. `None` → `<name>-<version>.cloacina` in the CWD. |
| `sign_key` | `Option<PathBuf>` | Ed25519 secret-key file (32 raw bytes) to sign the package with. `None` produces an unsigned provider. |
| `manifest_bin` | `String` | Host binary in the crate that prints the provider manifest JSON to stdout (the `__provider_manifest()` emitter). Defaults to `emit_manifest`. |
| `release` | `bool` | Build in release profile (default `true`). |
| `runtime` | `ProviderRuntime` | Runtime to build + package for (CLOACI-T-0903). `Wasm` (default) builds a `wasm32-wasip2` component; `Native` builds a host cdylib (loaded in-process via the T-0902 native path). |

#### Methods

##### <span id="ProviderPackageOptions.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(crate_dir: impl Into<PathBuf>) -> Self
```

Options for `crate_dir` with the conventional defaults (`emit_manifest` bin, release build, unsigned, CWD output, WASM runtime).

<details>
<summary>Source</summary>

```rust
    pub fn new(crate_dir: impl Into<PathBuf>) -> Self {
        Self {
            crate_dir: crate_dir.into(),
            output: None,
            sign_key: None,
            manifest_bin: "emit_manifest".to_string(),
            release: true,
            runtime: ProviderRuntime::Wasm,
        }
    }
```

</details>

##### <span id="ProviderPackageOptions.new_native"></span><code>new_​native</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new_native(crate_dir: impl Into<PathBuf>) -> Self
```

As `new` but building + packaging a NATIVE host cdylib (CLOACI-T-0903).

<details>
<summary>Source</summary>

```rust
    pub fn new_native(crate_dir: impl Into<PathBuf>) -> Self {
        Self {
            runtime: ProviderRuntime::Native,
            ..Self::new(crate_dir)
        }
    }
```

</details>

### <span id="struct.ProviderPackageResult"></span><code style="font-size: 0.85em">struct ProviderPackageResult</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

What [`package_constructor_provider`](../../../../rust/cloacina/packaging/constructor_provider/#fn.package_constructor_provider) produced.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `archive` | `PathBuf` | Path to the packed `.cloacina` provider archive. |
| `signed` | `bool` | Whether the archive carries a `package.sig` (was signed). |
| `provider_name` | `String` | The provider name (fidius package name). |
| `provider_version` | `String` | The provider version (from its `provider.json`). |
| `constructors` | `Vec<String>` | Names of the member constructors the provider carries. |

## Enums

### <span id="enum.ProviderPackageError"></span><code style="font-size: 0.85em">enum ProviderPackageError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors assembling/packing a constructor provider package.

#### Variants

- **`Io`** - The crate directory or an expected build artifact was missing/unreadable.
- **`Build`** - `cargo build`/`cargo run` failed.
- **`Manifest`** - The emitted manifest JSON did not parse as a `ProviderManifest`, or the
  provider declared no members.
- **`SigningKey`** - The Ed25519 secret key was missing or not exactly 32 bytes.
- **`Pack`** - The underlying fidius pack step failed.

## Functions

### <span id="fn.package_constructor_provider"></span><code style="font-size: 0.85em">fn package_​constructor_​provider</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn package_constructor_provider(
    opts: &ProviderPackageOptions,
) -> Result<ProviderPackageResult, ProviderPackageError>
```

Build, assemble, (optionally sign,) and pack a `#[constructor]` provider crate into a distributable provider package.

Steps, mirroring `package_workflow` but for a constructor suite component:
1. Build the component per `opts.runtime`: WASM → `cargo build --lib --target
   wasm32-wasip2` (a `.wasm` component); NATIVE → `cargo build --lib --features
   native` (a host cdylib, CLOACI-T-0903).
2. `cargo run --bin <manifest_bin>` → the provider's manifest JSON, parsed into
   a `ProviderManifest` (this is the packaging step writing `provider.json`,
   which the macro cannot do itself).
3. Stage `package.toml`, the component, and `provider.json` (its `component`
   corrected to the actual built artifact, its `runtime` stamped to
   `opts.runtime`) into a temp dir. The fidius `package.toml` `runtime` is
   `"wasm"` for WASM and `"rust"` for NATIVE (fidius's cdylib runtime — it has
   no `"native"` value; cloacina's `native` discriminator lives in
   `provider.json`).
4. If `sign_key` is set, write a `package.sig` (Ed25519 over the package
   digest) reusing fidius's signing scheme.
5. `fidius_core::package::pack_package` → the `.cloacina` archive.

<details>
<summary>Source</summary>

```rust
pub fn package_constructor_provider(
    opts: &ProviderPackageOptions,
) -> Result<ProviderPackageResult, ProviderPackageError> {
    let crate_dir = &opts.crate_dir;
    if !crate_dir.join("Cargo.toml").exists() {
        return Err(ProviderPackageError::Io(format!(
            "no Cargo.toml in constructor provider crate dir {}",
            crate_dir.display()
        )));
    }

    // 1. Build the component: a wasm32-wasip2 component (WASM) or a host cdylib
    //    (NATIVE, CLOACI-T-0903). Both yield the file staged into the package +
    //    named in the manifest's `component`.
    let component = match opts.runtime {
        ProviderRuntime::Wasm => build_wasm_component(crate_dir, opts.release)?,
        ProviderRuntime::Native => build_native_cdylib(crate_dir, opts.release)?,
    };

    // 2. Emit + parse the provider manifest.
    let manifest_json = emit_manifest_json(crate_dir, &opts.manifest_bin, opts.runtime)?;
    let mut provider = ProviderManifest::from_json(&manifest_json)
        .map_err(|e| ProviderPackageError::Manifest(e.to_string()))?;

    let head = provider
        .constructors
        .first()
        .ok_or_else(|| {
            ProviderPackageError::Manifest(format!(
                "provider '{}' declares no constructors",
                provider.name
            ))
        })?
        .clone();

    // 3. Stage the provider package directory.
    let staging = tempfile::TempDir::new()
        .map_err(|e| ProviderPackageError::Io(format!("create staging dir: {e}")))?;
    let pkg_dir = staging.path();

    // Component filename: keep the built artifact's own name so it is stable +
    // recognizable inside the archive, and make the manifest authoritative about it.
    let component_file = component
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "provider.wasm".to_string());

    std::fs::copy(&component, pkg_dir.join(&component_file))
        .map_err(|e| ProviderPackageError::Io(format!("copy provider component: {e}")))?;

    provider.component = component_file.clone();
    // Stamp the cloacina-side runtime discriminator the loader dispatches on
    // (the macro emits Wasm by default; native packaging flips it here).
    provider.runtime = opts.runtime;

    // The provider index the loader reads (`List[Constructor]`).
    let provider_name = provider.name.clone();
    let constructor_names: Vec<String> = provider
        .constructors
        .iter()
        .map(|c| c.name.clone())
        .collect();
    let provider_json = provider
        .to_json()
        .map_err(|e| ProviderPackageError::Manifest(format!("serialize provider.json: {e}")))?;
    std::fs::write(pkg_dir.join(PROVIDER_MANIFEST_FILE), provider_json)
        .map_err(|e| ProviderPackageError::Io(format!("write provider.json: {e}")))?;

    // The fidius wasm package header. `interface` / `interface_version` come from the
    // members (a homogeneous suite shares one) so the loader's descriptor + version
    // gate line up.
    let package_toml = render_package_toml(
        &provider_name,
        &provider.version,
        &head.interface,
        head.interface_version,
        head.primitive_kind,
        &component_file,
        opts.runtime,
    );
    std::fs::write(pkg_dir.join("package.toml"), package_toml)
        .map_err(|e| ProviderPackageError::Io(format!("write package.toml: {e}")))?;

    // 4. Optional signing — must happen before packing (the .sig is archived).
    let signed = if let Some(key_path) = &opts.sign_key {
        sign_package_dir(pkg_dir, key_path)?;
        true
    } else {
        false
    };

    // 5. Pack via fidius (same path as workflow packaging).
    let result = fidius_core::package::pack_package(pkg_dir, opts.output.as_deref())
        .map_err(|e| ProviderPackageError::Pack(e.to_string()))?;

    Ok(ProviderPackageResult {
        archive: result.path,
        signed,
        provider_name,
        provider_version: provider.version.clone(),
        constructors: constructor_names,
    })
}
```

</details>

## Constants

| Name | Type | Value | Description |
|------|------|-------|-------------|
| <span id="const.PROVIDER_MANIFEST_FILE"></span>`PROVIDER_MANIFEST_FILE` | `&str` | `"provider.json"` | The provider-index filename inside a provider package (the file the loader reads). Mirrors `constructor_loader::PROVIDER_MANIFEST_FILE`. |
| <span id="const.PROVIDER_EXTENSION"></span>`PROVIDER_EXTENSION` | `&str` | `"cloacina"` | The archive extension cloacina packages use (vs fidius's default `fid`). |
