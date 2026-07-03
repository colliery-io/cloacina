/*
 *  Copyright 2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Constructor **provider package** assembly + packing (CLOACI-T-0827 / A-0011).
//!
//! Turns a built `#[constructor]` **provider crate** (a *suite* of constructor
//! members, CLOACI-A-0011) into a distributable, signable **fidius provider
//! package** — the same machinery cloacina already uses to pack a workflow into a
//! `.cloacina` archive ([`super::package_workflow`] →
//! [`fidius_core::package::pack_package`]), reused for constructor providers rather
//! than a parallel format.
//!
//! ## What a provider package is
//!
//! A provider package is an ordinary fidius `runtime = "wasm"` source package — a
//! directory with a `package.toml` header that fidius understands — carrying:
//!
//!   * the provider's single `.wasm` **component** (built to `wasm32-wasip2`),
//!     which exposes EVERY member constructor behind one per-kind fidius interface
//!     (the member is selected at load by name-in-configure), and
//!   * `provider.json`, the [`ProviderManifest`] (`List[Constructor]`) the loader
//!     reads to select a member by `constructor = "<name>"` — written here from the
//!     macro-generated `__provider_manifest()` (the step the macro cannot do).
//!
//! It is packed (tar + bzip2) into a `<name>-<version>.cloacina` archive and may be
//! **Ed25519-signed** (a `package.sig` over the package digest) reusing fidius's
//! signing scheme verbatim, so `fidius_host::verify_package` /
//! `fidius_core::package::package_digest` verify it unchanged.
//!
//! ## Providers carry N constructors (Airflow "provider" = package of operators)
//!
//! One provider crate compiles to ONE component exposing N members; the
//! `provider.json` is the `List[Constructor]` index over them. A homogeneous suite
//! (every member the same kind — the common case) exports one fidius interface, so
//! the `package.toml` header's `interface`/`interface_version` come from the
//! members (they share one). A mixed-kind suite (multiple interfaces in one
//! component) is a documented follow-on.
//!
//! ## Gating
//!
//! Behind the default-OFF `constructor-packaging` feature, which pulls only the
//! serde-only `cloacina-constructor-contract` crate. It deliberately does NOT
//! enable `fidius-host/wasm`, so building the *packaging* tool does not drag in
//! wasmtime — only the *loader* (`constructors-wasm`) does. Resolving a packed
//! provider back to a runnable primitive lives in the loader
//! (`registry::loader::constructor_loader::load_task_constructor_from_package`).

use std::path::{Path, PathBuf};
use std::process::Command;

use ed25519_dalek::{Signer, SigningKey};

use cloacina_constructor_contract::{PrimitiveKind, ProviderManifest};

/// The provider-index filename inside a provider package (the file the loader
/// reads). Mirrors `constructor_loader::PROVIDER_MANIFEST_FILE`.
pub const PROVIDER_MANIFEST_FILE: &str = "provider.json";

/// The archive extension cloacina packages use (vs fidius's default `fid`).
pub const PROVIDER_EXTENSION: &str = "cloacina";

/// Inputs to [`package_constructor_provider`].
#[derive(Debug, Clone)]
pub struct ProviderPackageOptions {
    /// The `#[constructor]` provider crate directory to package.
    pub crate_dir: PathBuf,
    /// Output archive path. `None` → `<name>-<version>.cloacina` in the CWD.
    pub output: Option<PathBuf>,
    /// Ed25519 secret-key file (32 raw bytes) to sign the package with. `None`
    /// produces an unsigned provider.
    pub sign_key: Option<PathBuf>,
    /// Host binary in the crate that prints the provider manifest JSON to stdout
    /// (the `__provider_manifest()` emitter). Defaults to `emit_manifest`.
    pub manifest_bin: String,
    /// Build the `wasm32-wasip2` component in release profile (default `true`).
    pub release: bool,
}

impl ProviderPackageOptions {
    /// Options for `crate_dir` with the conventional defaults
    /// (`emit_manifest` bin, release build, unsigned, CWD output).
    pub fn new(crate_dir: impl Into<PathBuf>) -> Self {
        Self {
            crate_dir: crate_dir.into(),
            output: None,
            sign_key: None,
            manifest_bin: "emit_manifest".to_string(),
            release: true,
        }
    }
}

/// What [`package_constructor_provider`] produced.
#[derive(Debug, Clone)]
pub struct ProviderPackageResult {
    /// Path to the packed `.cloacina` provider archive.
    pub archive: PathBuf,
    /// Whether the archive carries a `package.sig` (was signed).
    pub signed: bool,
    /// The provider name (fidius package name).
    pub provider_name: String,
    /// The provider version (from its `provider.json`).
    pub provider_version: String,
    /// Names of the member constructors the provider carries.
    pub constructors: Vec<String>,
}

/// Errors assembling/packing a constructor provider package.
#[derive(Debug, thiserror::Error)]
pub enum ProviderPackageError {
    /// The crate directory or an expected build artifact was missing/unreadable.
    #[error("{0}")]
    Io(String),
    /// `cargo build`/`cargo run` failed.
    #[error("build step failed: {0}")]
    Build(String),
    /// The emitted manifest JSON did not parse as a `ProviderManifest`, or the
    /// provider declared no members.
    #[error("provider manifest parse failed: {0}")]
    Manifest(String),
    /// The Ed25519 secret key was missing or not exactly 32 bytes.
    #[error("signing key error: {0}")]
    SigningKey(String),
    /// The underlying fidius pack step failed.
    #[error("pack failed: {0}")]
    Pack(String),
}

/// Build, assemble, (optionally sign,) and pack a `#[constructor]` provider crate
/// into a distributable provider package.
///
/// Steps, mirroring `package_workflow` but for a constructor suite component:
/// 1. `cargo build --lib --target wasm32-wasip2` → the `.wasm` component.
/// 2. `cargo run --bin <manifest_bin>` → the provider's manifest JSON, parsed into
///    a [`ProviderManifest`] (this is the packaging step writing `provider.json`,
///    which the macro cannot do itself).
/// 3. Stage `package.toml` (`runtime = "wasm"`), the component, and `provider.json`
///    (its `component` corrected to the actual built artifact name) into a temp dir.
/// 4. If `sign_key` is set, write a `package.sig` (Ed25519 over the package
///    digest) reusing fidius's signing scheme.
/// 5. [`fidius_core::package::pack_package`] → the `.cloacina` archive.
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

    // 1. Build the wasm component.
    let wasm = build_wasm_component(crate_dir, opts.release)?;

    // 2. Emit + parse the provider manifest.
    let manifest_json = emit_manifest_json(crate_dir, &opts.manifest_bin)?;
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
    let component_file = wasm
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "provider.wasm".to_string());

    std::fs::copy(&wasm, pkg_dir.join(&component_file))
        .map_err(|e| ProviderPackageError::Io(format!("copy wasm component: {e}")))?;

    provider.component = component_file.clone();

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

/// Render the fidius `package.toml` header for a constructor provider.
#[allow(clippy::too_many_arguments)]
fn render_package_toml(
    name: &str,
    version: &str,
    interface: &str,
    interface_version: u32,
    primitive: PrimitiveKind,
    component_file: &str,
) -> String {
    format!(
        "# Generated by cloacina constructor packaging (CLOACI-T-0827).\n\
         [package]\n\
         name = \"{name}\"\n\
         version = \"{version}\"\n\
         interface = \"{interface}\"\n\
         interface_version = {iface_version}\n\
         extension = \"{ext}\"\n\
         runtime = \"wasm\"\n\n\
         [metadata]\n\
         category = \"constructor\"\n\
         primitive_kind = \"{primitive}\"\n\n\
         [wasm]\n\
         component = \"{component}\"\n",
        name = name,
        version = version,
        interface = interface,
        iface_version = interface_version,
        ext = PROVIDER_EXTENSION,
        primitive = primitive_kind_str(primitive),
        component = component_file,
    )
}

fn primitive_kind_str(kind: PrimitiveKind) -> &'static str {
    match kind {
        PrimitiveKind::Task => "task",
        PrimitiveKind::Trigger => "trigger",
        PrimitiveKind::Accumulator => "accumulator",
        PrimitiveKind::Reactor => "reactor",
    }
}

/// `cargo build --lib --target wasm32-wasip2 [--release]` in `crate_dir`, then
/// locate the produced `.wasm` component.
fn build_wasm_component(crate_dir: &Path, release: bool) -> Result<PathBuf, ProviderPackageError> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--lib")
        .arg("--target")
        .arg("wasm32-wasip2")
        .current_dir(crate_dir);
    let profile = if release {
        cmd.arg("--release");
        "release"
    } else {
        "debug"
    };

    let status = cmd
        .status()
        .map_err(|e| ProviderPackageError::Build(format!("spawn cargo build: {e}")))?;
    if !status.success() {
        return Err(ProviderPackageError::Build(format!(
            "cargo build --target wasm32-wasip2 failed with status {status}"
        )));
    }

    let out_dir = crate_dir.join("target").join("wasm32-wasip2").join(profile);

    // Prefer the artifact named after the crate; fall back to the sole `.wasm`.
    let preferred = crate_name(crate_dir).map(|n| out_dir.join(format!("{n}.wasm")));
    if let Some(p) = &preferred {
        if p.exists() {
            return Ok(p.clone());
        }
    }

    let mut wasms: Vec<PathBuf> = std::fs::read_dir(&out_dir)
        .map_err(|e| ProviderPackageError::Io(format!("read {}: {e}", out_dir.display())))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("wasm"))
        .collect();
    wasms.sort();
    match wasms.len() {
        0 => Err(ProviderPackageError::Build(format!(
            "build succeeded but no .wasm component found in {}",
            out_dir.display()
        ))),
        1 => Ok(wasms.pop().unwrap()),
        _ => Err(ProviderPackageError::Build(format!(
            "multiple .wasm components in {} ({:?}); name the lib to match the crate",
            out_dir.display(),
            wasms
        ))),
    }
}

/// Best-effort crate name (`[package].name`, `-`→`_`) for artifact matching.
fn crate_name(crate_dir: &Path) -> Option<String> {
    let toml = std::fs::read_to_string(crate_dir.join("Cargo.toml")).ok()?;
    let value: toml::Value = toml.parse().ok()?;
    let name = value.get("package")?.get("name")?.as_str()?;
    Some(name.replace('-', "_"))
}

/// Run the crate's manifest-emitter host binary and capture its stdout JSON.
fn emit_manifest_json(
    crate_dir: &Path,
    manifest_bin: &str,
) -> Result<String, ProviderPackageError> {
    let out = Command::new("cargo")
        .args(["run", "--quiet", "--bin", manifest_bin])
        .current_dir(crate_dir)
        .output()
        .map_err(|e| {
            ProviderPackageError::Build(format!("spawn cargo run --bin {manifest_bin}: {e}"))
        })?;
    if !out.status.success() {
        return Err(ProviderPackageError::Build(format!(
            "`cargo run --bin {manifest_bin}` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    String::from_utf8(out.stdout)
        .map_err(|e| ProviderPackageError::Manifest(format!("manifest stdout not UTF-8: {e}")))
}

/// Sign a staged package directory in place, reusing fidius's scheme: an Ed25519
/// signature over [`fidius_core::package::package_digest`], written to
/// `package.sig`. This is byte-compatible with `fidius_host::verify_package`, so
/// we are reusing fidius's signing/verification rather than rolling our own.
fn sign_package_dir(pkg_dir: &Path, key_path: &Path) -> Result<(), ProviderPackageError> {
    let key_bytes: [u8; 32] = std::fs::read(key_path)
        .map_err(|e| ProviderPackageError::SigningKey(format!("read {}: {e}", key_path.display())))?
        .try_into()
        .map_err(|_| {
            ProviderPackageError::SigningKey("secret key must be exactly 32 bytes".to_string())
        })?;
    let signing_key = SigningKey::from_bytes(&key_bytes);

    let digest = fidius_core::package::package_digest(pkg_dir)
        .map_err(|e| ProviderPackageError::Pack(format!("compute package digest: {e}")))?;
    let signature = signing_key.sign(&digest);

    std::fs::write(pkg_dir.join("package.sig"), signature.to_bytes())
        .map_err(|e| ProviderPackageError::Io(format!("write package.sig: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina_constructor_contract::ConstructorManifest;

    fn member(name: &str) -> ConstructorManifest {
        ConstructorManifest {
            name: name.into(),
            version: "0.1.0".into(),
            primitive_kind: PrimitiveKind::Task,
            interface: "task-constructor".into(),
            interface_version: 1,
            params: vec![],
            config_fields: vec![],
            dependencies: vec![],
            description: None,
            author: None,
        }
    }

    #[test]
    fn provider_manifest_round_trips() {
        let p = ProviderManifest {
            name: "cloacina-provider-fs".into(),
            version: "0.1.0".into(),
            component: "cloacina_provider_fs.wasm".into(),
            constructors: vec![member("read_file"), member("write_file")],
        };
        let s = p.to_json().unwrap();
        let back = ProviderManifest::from_json(&s).unwrap();
        assert_eq!(p, back);
        assert_eq!(back.constructors.len(), 2);
        assert_eq!(back.constructor("write_file").unwrap().name, "write_file");
    }

    #[test]
    fn package_toml_has_wasm_runtime_and_component() {
        let toml = render_package_toml(
            "cloacina-provider-fs",
            "0.1.0",
            "task-constructor",
            1,
            PrimitiveKind::Task,
            "cloacina_provider_fs.wasm",
        );
        assert!(toml.contains("runtime = \"wasm\""));
        assert!(toml.contains("component = \"cloacina_provider_fs.wasm\""));
        assert!(toml.contains("interface = \"task-constructor\""));
        assert!(toml.contains("name = \"cloacina-provider-fs\""));
        assert!(toml.contains("extension = \"cloacina\""));
        // Parses as valid TOML.
        let _: toml::Value = toml.parse().expect("rendered package.toml is valid TOML");
    }
}
