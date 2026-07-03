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

//! CLOACI-T-0836 — build-side provider discovery + bundling against a REAL Cargo
//! dependency graph.
//!
//! `provider-consumer-fixture` declares `cloacina-provider-fs` as an ordinary path
//! dependency (exactly as a consumer's `from = "cloacina-provider-fs"` would). This
//! proves [`resolve_provider_crate`] finds it via `cargo metadata`, and
//! [`bundle_providers`] resolves → builds it to a wasm component → unpacks it into a
//! `providers/<name>-<version>/` tree — the hermetic layout a packaged workflow
//! carries and the loader's `provider_search_path` resolves against.
//!
//! Gated on `constructor-packaging` (no wasmtime); it does shell a `cargo build
//! --target wasm32-wasip2` for the provider, so it needs that target installed.
#![cfg(feature = "constructor-packaging")]

use std::path::PathBuf;

use cloacina::packaging::provider_bundle::{bundle_providers, resolve_provider_crate, ProviderRef};

fn consumer_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/constructor-contract/provider-consumer-fixture")
}

#[test]
fn resolves_provider_from_the_consumer_dependency_graph() {
    let dir = resolve_provider_crate(&consumer_dir(), &ProviderRef::parse("cloacina-provider-fs"))
        .expect("resolve the provider from the consumer's cargo metadata");
    assert!(
        dir.ends_with("cloacina-provider-fs"),
        "resolved to the provider crate dir, got {}",
        dir.display()
    );
    assert!(dir.join("Cargo.toml").exists());
}

#[test]
fn unknown_provider_fails_closed() {
    let err = resolve_provider_crate(
        &consumer_dir(),
        &ProviderRef::parse("cloacina-provider-nonexistent"),
    )
    .err()
    .expect("a provider not in the graph must fail closed");
    assert!(
        format!("{err}").contains("cloacina-provider-nonexistent"),
        "error should name the missing provider, got: {err}"
    );
}

#[test]
fn version_pin_mismatch_fails_closed() {
    // The graph has cloacina-provider-fs @ 0.1.0; asking for @9.9.9 must fail.
    let err = resolve_provider_crate(
        &consumer_dir(),
        &ProviderRef::parse("cloacina-provider-fs@9.9.9"),
    )
    .err()
    .expect("an unsatisfiable version pin must fail closed");
    let msg = format!("{err}");
    assert!(
        msg.contains("9.9.9") && msg.contains("0.1.0"),
        "error should name the requested + available versions, got: {msg}"
    );
}

#[test]
fn bundles_the_provider_into_a_providers_tree() {
    let dest = tempfile::TempDir::new().unwrap();
    let bundled = bundle_providers(
        &consumer_dir(),
        &[ProviderRef::parse("cloacina-provider-fs")],
        dest.path(),
        false, // debug wasm build keeps the test faster
    )
    .expect("bundle the provider");

    assert_eq!(bundled.len(), 1, "one provider bundled");
    let p = &bundled[0];
    assert_eq!(p.from, "cloacina-provider-fs");
    assert_eq!(p.provider_name, "cloacina-provider-fs");
    assert_eq!(p.version, "0.1.0");
    assert_eq!(
        p.constructors,
        vec!["read_file".to_string(), "write_file".to_string()],
        "both suite members carried through the bundle"
    );

    // The on-disk layout is the provider_search_path shape: providers/<name>-<ver>/.
    let provider_json = p.bundled_dir.join("provider.json");
    assert!(
        provider_json.exists(),
        "bundled provider.json at {}",
        provider_json.display()
    );
    assert!(p.bundled_dir.starts_with(dest.path().join("providers")));
    // The wasm component is bundled alongside its manifest.
    let has_wasm = std::fs::read_dir(&p.bundled_dir)
        .unwrap()
        .flatten()
        .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("wasm"));
    assert!(has_wasm, "the wasm component is bundled with the manifest");
}

#[test]
fn duplicate_refs_bundle_once() {
    let dest = tempfile::TempDir::new().unwrap();
    let bundled = bundle_providers(
        &consumer_dir(),
        &[
            ProviderRef::parse("cloacina-provider-fs"),
            ProviderRef::parse("cloacina-provider-fs@0.1.0"),
        ],
        dest.path(),
        false,
    )
    .expect("bundle with duplicate refs");
    assert_eq!(
        bundled.len(),
        1,
        "a provider referenced twice is built once"
    );
}

/// CLOACI-T-0831: the PYTHON-consumer path — no Cargo.toml, providers declared as
/// manifest dep specs. A scratch Cargo project is synthesized around the specs and
/// the provider resolves+packs exactly like the Rust path (here via a `path` spec;
/// crates.io/git specs ride the same synthesized manifest).
#[test]
fn packs_providers_from_manifest_specs() {
    use cloacina::packaging::provider_bundle::pack_providers_from_specs;

    let provider_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/constructor-contract/cloacina-provider-fs")
        .canonicalize()
        .unwrap();
    let specs = vec![(
        "cloacina-provider-fs".to_string(),
        format!("{{ path = {:?} }}", provider_dir.display().to_string()),
    )];

    let packed = pack_providers_from_specs(&specs, false).expect("pack from specs");
    assert_eq!(packed.len(), 1);
    let p = &packed[0];
    assert_eq!(p.provider_name, "cloacina-provider-fs");
    assert_eq!(p.version, "0.1.0");
    assert_eq!(
        p.constructors,
        vec!["read_file".to_string(), "write_file".to_string()]
    );
    assert!(!p.archive.is_empty(), "the packed archive travels as bytes");
}

/// An unknown provider spec fails closed at resolve (cargo can't find it).
#[test]
fn unknown_manifest_spec_fails_closed() {
    use cloacina::packaging::provider_bundle::pack_providers_from_specs;
    let specs = vec![(
        "cloacina-provider-nonexistent".to_string(),
        "{ path = \"/definitely/not/a/crate\" }".to_string(),
    )];
    let err = pack_providers_from_specs(&specs, false)
        .err()
        .expect("bad spec must fail closed");
    assert!(
        format!("{err}").contains("cargo metadata") || format!("{err}").contains("nonexistent"),
        "error should surface the resolution failure, got: {err}"
    );
}
