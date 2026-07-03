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

//! Provider **discovery + bundling** for the packaged-constructor build side
//! (CLOACI-T-0836 / S-0015 / A-0010).
//!
//! A constructor provider is an ordinary **Cargo dependency** of the consumer
//! workflow crate (`from = "<exact package name>"`). To make a packaged workflow
//! HERMETIC — so a server can load + run a `constructor!`-using workflow with no
//! provider directory and no network — the consumer's build resolves each provider
//! dep, builds it to a wasm component, and **bundles** it inside the package under
//! `providers/<crate>-<version>/`. The loader then resolves `constructor!` `from`
//! references against that bundled directory (the same on-disk layout
//! [`crate::registry::loader::provider_search_path`] already expects).
//!
//! This module is the reusable core the compiler orchestrates:
//!   * [`resolve_provider_crate`] — locate a provider crate in the consumer's
//!     resolved dependency graph via `cargo metadata` (crates.io / path / git
//!     uniformly);
//!   * [`bundle_providers`] — resolve + build + unpack every referenced provider
//!     into a `providers/` tree, returning the `from`→bundled-dir map.
//!
//! Gated behind `constructor-packaging` (the serde-only contract path) — it builds
//! wasm via [`super::constructor_provider::package_constructor_provider`] and
//! unpacks with [`fidius_core::package::unpack_package`], neither of which pulls
//! wasmtime (only the *loader* does).

use std::path::{Path, PathBuf};
use std::process::Command;

use cloacina_constructor_contract::ProviderManifest;

use super::constructor_provider::{
    package_constructor_provider, ProviderPackageError, ProviderPackageOptions,
    PROVIDER_MANIFEST_FILE,
};

/// The subdirectory (inside a package / bundle) that holds unpacked provider
/// packages, one per `providers/<crate>-<version>/`.
pub const PROVIDERS_DIR: &str = "providers";

/// A provider reference discovered on a consumer's `constructor!` / `#[reactor]`
/// declaration: the `from = "<name>[@version]"` string, split into parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRef {
    /// The exact Cargo package name the consumer depends on.
    pub name: String,
    /// Optional `@version` suffix (advisory pin; must be satisfiable by the
    /// resolved dep). `None` if the consumer wrote a bare `from = "<name>"`.
    pub version: Option<String>,
}

impl ProviderRef {
    /// Parse a `from = "name[@version]"` reference.
    pub fn parse(from: &str) -> Self {
        match from.split_once('@') {
            Some((name, ver)) => Self {
                name: name.to_string(),
                version: Some(ver.to_string()),
            },
            None => Self {
                name: from.to_string(),
                version: None,
            },
        }
    }
}

/// One provider that was resolved, built, and unpacked into the bundle.
#[derive(Debug, Clone)]
pub struct BundledProvider {
    /// The `from` name the consumer referenced (the exact Cargo package name).
    pub from: String,
    /// The provider crate's resolved source directory (the dir holding its `Cargo.toml`).
    pub crate_dir: PathBuf,
    /// The provider's own name from its `provider.json` (usually == `from`).
    pub provider_name: String,
    /// The provider version (from `provider.json`).
    pub version: String,
    /// The bundled directory `providers/<crate>-<version>/` under `dest`.
    pub bundled_dir: PathBuf,
    /// The member constructors the provider carries.
    pub constructors: Vec<String>,
}

/// Errors resolving / building / bundling a provider.
#[derive(Debug, thiserror::Error)]
pub enum ProviderBundleError {
    /// `cargo metadata` failed or its output was unparsable.
    #[error("cargo metadata failed: {0}")]
    Metadata(String),
    /// No dependency with the requested `from` name (+ version) was found in the
    /// consumer's resolved dependency graph.
    #[error("{0}")]
    NotFound(String),
    /// Building / packing the provider failed.
    #[error(transparent)]
    Package(#[from] ProviderPackageError),
    /// An IO / unpack error while bundling.
    #[error("{0}")]
    Io(String),
}

/// Locate a provider crate in the consumer's resolved dependency graph.
///
/// Runs `cargo metadata --format-version 1` in `consumer_dir` and finds the package
/// whose `name` equals `provider.name` (and, when `provider.version` is set, whose
/// resolved version satisfies it — a plain-equality / prefix check for v1; full
/// semver-req matching is a noted follow-on). Returns the crate's source directory
/// (the parent of its `Cargo.toml`). Path, git, and crates.io deps resolve
/// uniformly because `cargo metadata` reports a `manifest_path` for each.
pub fn resolve_provider_crate(
    consumer_dir: &Path,
    provider: &ProviderRef,
) -> Result<PathBuf, ProviderBundleError> {
    let out = Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .current_dir(consumer_dir)
        .output()
        .map_err(|e| ProviderBundleError::Metadata(format!("spawn cargo metadata: {e}")))?;
    if !out.status.success() {
        return Err(ProviderBundleError::Metadata(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }

    let meta: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| ProviderBundleError::Metadata(format!("parse cargo metadata JSON: {e}")))?;
    let packages = meta
        .get("packages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| ProviderBundleError::Metadata("cargo metadata has no `packages`".into()))?;

    // Every package matching the name, with its (version, manifest_path).
    let mut matches: Vec<(String, PathBuf)> = Vec::new();
    for pkg in packages {
        let name = pkg.get("name").and_then(|v| v.as_str()).unwrap_or_default();
        if name != provider.name {
            continue;
        }
        let version = pkg
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let manifest_path = pkg
            .get("manifest_path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);
        if let Some(mp) = manifest_path {
            if let Some(dir) = mp.parent() {
                matches.push((version, dir.to_path_buf()));
            }
        }
    }

    if matches.is_empty() {
        return Err(ProviderBundleError::NotFound(format!(
            "provider crate '{}' is not a dependency in the consumer's graph ({}). \
             Add it to the workflow crate's [dependencies].",
            provider.name,
            consumer_dir.display()
        )));
    }

    // Version filter (advisory pin): keep exact-equal or prefix matches when a
    // version was requested. If nothing matches the pin but the name exists, that
    // is a hard error (the author asked for a version the graph does not provide).
    if let Some(want) = &provider.version {
        let filtered: Vec<&(String, PathBuf)> = matches
            .iter()
            .filter(|(v, _)| v == want || v.starts_with(want))
            .collect();
        return match filtered.first() {
            Some((_, dir)) => Ok((*dir).clone()),
            None => Err(ProviderBundleError::NotFound(format!(
                "provider '{}@{}' — the resolved graph has '{}' at version(s) [{}], not {}",
                provider.name,
                want,
                provider.name,
                matches
                    .iter()
                    .map(|(v, _)| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                want
            ))),
        };
    }

    // No pin: take the single (or first) match.
    Ok(matches.into_iter().next().unwrap().1)
}

/// Resolve, build, and bundle every referenced provider into `dest/providers/`.
///
/// For each unique [`ProviderRef`]: resolve the crate ([`resolve_provider_crate`]),
/// build+pack it ([`package_constructor_provider`]) to a temp archive, and unpack it
/// into `dest/providers/` (fidius names the unpacked dir `<name>-<version>/`). The
/// resulting `providers/` tree is exactly what
/// [`crate::registry::loader::set_provider_search_path`] points the loader at, so
/// bundled constructors resolve with no external provider directory.
///
/// `release` selects the wasm build profile. Duplicate `from` names are built once.
pub fn bundle_providers(
    consumer_dir: &Path,
    provider_refs: &[ProviderRef],
    dest: &Path,
    release: bool,
) -> Result<Vec<BundledProvider>, ProviderBundleError> {
    let providers_dir = dest.join(PROVIDERS_DIR);
    std::fs::create_dir_all(&providers_dir).map_err(|e| {
        ProviderBundleError::Io(format!(
            "create providers dir {}: {e}",
            providers_dir.display()
        ))
    })?;

    // De-duplicate by name (a provider referenced by multiple nodes is built once).
    let mut seen: Vec<String> = Vec::new();
    let mut bundled: Vec<BundledProvider> = Vec::new();

    for provider in provider_refs {
        if seen.contains(&provider.name) {
            continue;
        }
        seen.push(provider.name.clone());

        let crate_dir = resolve_provider_crate(consumer_dir, provider)?;

        // Build + pack the provider to a temp archive.
        let staging = tempfile::TempDir::new()
            .map_err(|e| ProviderBundleError::Io(format!("create staging dir: {e}")))?;
        let archive = staging.path().join(format!("{}.cloacina", provider.name));
        let opts = ProviderPackageOptions {
            crate_dir: crate_dir.clone(),
            output: Some(archive.clone()),
            sign_key: None,
            manifest_bin: "emit_manifest".to_string(),
            release,
        };
        let result = package_constructor_provider(&opts)?;

        // Unpack it into the bundle's providers/ tree (fidius makes `<name>-<ver>/`).
        let bundled_dir =
            fidius_core::package::unpack_package(&archive, &providers_dir).map_err(|e| {
                ProviderBundleError::Io(format!(
                    "unpack provider '{}' into bundle: {e}",
                    provider.name
                ))
            })?;

        // Read the bundled provider.json back for the authoritative name/version.
        let manifest_path = bundled_dir.join(PROVIDER_MANIFEST_FILE);
        let manifest_raw = std::fs::read_to_string(&manifest_path).map_err(|e| {
            ProviderBundleError::Io(format!("read bundled {}: {e}", manifest_path.display()))
        })?;
        let manifest = ProviderManifest::from_json(&manifest_raw)
            .map_err(|e| ProviderBundleError::Io(format!("parse bundled provider.json: {e}")))?;

        bundled.push(BundledProvider {
            from: provider.name.clone(),
            crate_dir,
            provider_name: manifest.name.clone(),
            version: manifest.version.clone(),
            bundled_dir,
            constructors: result.constructors,
        });
    }

    Ok(bundled)
}

/// One provider resolved + built + PACKED (not unpacked) — the storage form the
/// compiler persists into `package_providers` (the reconciler unpacks at load).
#[derive(Debug, Clone)]
pub struct PackedProvider {
    /// The `from` name the consumer referenced (the exact Cargo package name).
    pub from: String,
    /// The provider's own name from its `provider.json`.
    pub provider_name: String,
    /// The provider version (from `provider.json`).
    pub version: String,
    /// The member constructors the provider carries.
    pub constructors: Vec<String>,
    /// The packed provider `.cloacina` archive bytes.
    pub archive: Vec<u8>,
}

/// Resolve + build + PACK every referenced provider, returning the archives
/// (the compiler-side variant of [`bundle_providers`]: same resolve/build, but the
/// output is bytes for the `package_providers` store rather than an unpacked
/// `providers/` tree). Duplicate `from` names are built once.
pub fn pack_providers(
    consumer_dir: &Path,
    provider_refs: &[ProviderRef],
    release: bool,
) -> Result<Vec<PackedProvider>, ProviderBundleError> {
    let mut seen: Vec<String> = Vec::new();
    let mut packed: Vec<PackedProvider> = Vec::new();

    for provider in provider_refs {
        if seen.contains(&provider.name) {
            continue;
        }
        seen.push(provider.name.clone());

        let crate_dir = resolve_provider_crate(consumer_dir, provider)?;

        let staging = tempfile::TempDir::new()
            .map_err(|e| ProviderBundleError::Io(format!("create staging dir: {e}")))?;
        let archive_path = staging.path().join(format!("{}.cloacina", provider.name));
        let opts = ProviderPackageOptions {
            crate_dir,
            output: Some(archive_path.clone()),
            sign_key: None,
            manifest_bin: "emit_manifest".to_string(),
            release,
        };
        let result = package_constructor_provider(&opts)?;

        let archive = std::fs::read(&archive_path).map_err(|e| {
            ProviderBundleError::Io(format!(
                "read packed provider archive for '{}': {e}",
                provider.name
            ))
        })?;

        packed.push(PackedProvider {
            from: provider.name.clone(),
            provider_name: result.provider_name,
            version: result.provider_version,
            constructors: result.constructors,
            archive,
        });
    }

    Ok(packed)
}

/// Discover the provider references a consumer's SOURCE declares: scan `.rs` files
/// for `constructor!( ... from = "<ref>" ... )` and `#[reactor( ... from = "<ref>"
/// ... )]` occurrences (the S-0015 discovery rule — build + bundle ONLY what the
/// package references). Anchored on the macro tokens so stray `from = "..."`
/// strings elsewhere don't false-positive; a wrong ref fails loudly at resolve.
pub fn discover_provider_refs(source_dir: &Path) -> Vec<ProviderRef> {
    let mut refs: Vec<ProviderRef> = Vec::new();
    let mut stack = vec![source_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip build output; everything else is fair game.
                if path.file_name().and_then(|n| n.to_str()) != Some("target") {
                    stack.push(path);
                }
            } else if path.extension().and_then(|x| x.to_str()) == Some("rs") {
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                for anchor in ["constructor!", "#[reactor("] {
                    let mut rest = text.as_str();
                    while let Some(pos) = rest.find(anchor) {
                        // Search a bounded window after the anchor for `from = "..."`.
                        let window = &rest[pos..rest.len().min(pos + 2048)];
                        if let Some(from) = extract_from_literal(window) {
                            let parsed = ProviderRef::parse(&from);
                            if !refs.iter().any(|r| r == &parsed) {
                                refs.push(parsed);
                            }
                        }
                        rest = &rest[pos + anchor.len()..];
                    }
                }
            }
        }
    }
    refs
}

/// Pull the first `from = "<value>"` string literal out of a macro-body window.
fn extract_from_literal(window: &str) -> Option<String> {
    let idx = window.find("from")?;
    let after = window[idx + 4..].trim_start();
    let after = after.strip_prefix('=')?.trim_start();
    let after = after.strip_prefix('"')?;
    let end = after.find('"')?;
    Some(after[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_constructor_from_refs_in_source() {
        let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/constructor-contract/packaged-consumer-fixture");
        let refs = discover_provider_refs(&src);
        assert_eq!(
            refs,
            vec![ProviderRef {
                name: "cloacina-provider-fs".into(),
                version: Some("0.1.0".into())
            }],
            "the packaged consumer fixture declares exactly one provider ref"
        );
    }

    #[test]
    fn discovery_ignores_unanchored_from_strings() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src/lib.rs"),
            r#"// from = "not-a-provider" (comment, no macro anchor)
               fn f() { let _x = ("from", "also-not"); }"#,
        )
        .unwrap();
        assert!(discover_provider_refs(dir.path()).is_empty());
    }

    #[test]
    fn provider_ref_parses_name_and_optional_version() {
        assert_eq!(
            ProviderRef::parse("cloacina-provider-fs"),
            ProviderRef {
                name: "cloacina-provider-fs".into(),
                version: None
            }
        );
        assert_eq!(
            ProviderRef::parse("cloacina-provider-fs@0.1.0"),
            ProviderRef {
                name: "cloacina-provider-fs".into(),
                version: Some("0.1.0".into())
            }
        );
    }
}
