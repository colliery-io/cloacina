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

//! CLOACI-T-0827 — end-to-end: a `#[constructor]`-authored crate is packaged into
//! a **signed fidius provider package**, and the loader resolves the constructor
//! **from the packed archive** (unpack → verify signature → `load_wasm_configured`)
//! to run as a cloacina [`Task`].
//!
//! This is the packaged counterpart to `constructor_macro_wasm.rs` (which loads
//! the same fixture from a loose dir). The proof:
//!   1. `package_constructor_provider` builds the fixture to a `wasm32-wasip2`
//!      component, emits `constructor.json` from `__constructor_manifest()`,
//!      assembles a `runtime = "wasm"` provider package, Ed25519-signs it, and
//!      packs it into a `.cloacina` archive;
//!   2. `load_task_constructor_from_package` unpacks + verifies the signature and
//!      loads the constructor → `Arc<dyn Task>`;
//!   3. running it with `Context { name: "world" }` yields `result == "hello, world"`.
//! Plus fail-closed paths: a wrong verifying key, a tampered package, and a
//! missing `constructor.json` are all rejected.
//!
//! Feature-gated (`constructors-wasm`, which pulls wasmtime + implies
//! `constructor-packaging`). Excluded from the default build.
#![cfg(feature = "constructors-wasm")]

use std::path::PathBuf;

use ed25519_dalek::SigningKey;
use serde::Serialize;

use cloacina::packaging::constructor_provider::{
    package_constructor_provider, ProviderPackageOptions,
};
use cloacina::registry::loader::{load_task_constructor_from_package, unpack_provider_archive};
use cloacina::Context;

/// Per-instance config the loader binds once at load (serde-compatible with the
/// macro-generated guest `__PrefixConfig { prefix }`).
#[derive(Serialize)]
struct Config {
    prefix: String,
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/constructor-contract/task-constructor-macro-fixture")
}

/// A deterministic Ed25519 keypair for the test (no OsRng needed). Writes the
/// 32-byte secret to `dir/key.secret` and returns (secret_path, verifying_key).
fn write_test_key(dir: &std::path::Path) -> (PathBuf, ed25519_dalek::VerifyingKey) {
    let signing = SigningKey::from_bytes(&[7u8; 32]);
    let verifying = signing.verifying_key();
    let secret_path = dir.join("key.secret");
    std::fs::write(&secret_path, signing.to_bytes()).unwrap();
    (secret_path, verifying)
}

/// Package the fixture into a signed `.cloacina` provider archive once.
fn signed_archive(work: &std::path::Path) -> (PathBuf, ed25519_dalek::VerifyingKey) {
    let (key_path, vk) = write_test_key(work);
    let out = work.join("prefix-provider.cloacina");

    let opts = ProviderPackageOptions {
        crate_dir: fixture_dir(),
        output: Some(out.clone()),
        sign_key: Some(key_path),
        manifest_bin: "emit_manifest".to_string(),
        release: true,
    };
    let result = package_constructor_provider(&opts).expect("package_constructor_provider");

    assert!(result.signed, "archive should be signed");
    assert_eq!(result.provider_name, "prefix");
    assert_eq!(result.constructors, vec!["prefix".to_string()]);
    assert!(out.exists(), "archive written to {}", out.display());
    (out, vk)
}

#[tokio::test]
async fn signed_provider_loads_from_package_and_runs_as_task() {
    let work = tempfile::TempDir::new().unwrap();
    let (archive, vk) = signed_archive(work.path());

    let dest = work.path().join("unpacked");
    let task = load_task_constructor_from_package(
        &archive,
        &dest,
        "prefix",
        &Config {
            prefix: "hello, ".into(),
        },
        &[vk],
        &cloacina::registry::loader::grants::ResolvedGrants::deny_all(),
    )
    .expect("load_task_constructor_from_package (signed, verified)");

    assert_eq!(task.id(), "prefix");

    let mut ctx = Context::new();
    ctx.insert("name", serde_json::json!("world")).unwrap();
    let out = task.execute(ctx).await.expect("constructor task execute");

    assert_eq!(
        out.get("result"),
        Some(&serde_json::json!("hello, world")),
        "constructor loaded FROM THE SIGNED PACKAGE produces result = prefix + name"
    );
    assert_eq!(out.get("name"), Some(&serde_json::json!("world")));
}

#[tokio::test]
async fn wrong_verifying_key_fails_closed() {
    let work = tempfile::TempDir::new().unwrap();
    let (archive, _vk) = signed_archive(work.path());

    // A different key never signed this package.
    let wrong = SigningKey::from_bytes(&[9u8; 32]).verifying_key();
    let dest = work.path().join("unpacked-wrong");

    let err = load_task_constructor_from_package(
        &archive,
        &dest,
        "prefix",
        &Config { prefix: "x".into() },
        &[wrong],
        &cloacina::registry::loader::grants::ResolvedGrants::deny_all(),
    )
    .err()
    .expect("loading with a non-signing key must fail closed");
    let msg = format!("{err}");
    assert!(
        msg.contains("signature") || msg.contains("verify"),
        "error should be a signature-verification failure, got: {msg}"
    );
}

#[tokio::test]
async fn tampered_package_fails_signature_verification() {
    let work = tempfile::TempDir::new().unwrap();
    let (archive, vk) = signed_archive(work.path());

    // Unpack WITHOUT verifying, tamper a packaged file, then REPACK it (carrying
    // the original, now-stale `package.sig`).
    let dest = work.path().join("unpacked-tamper");
    let pkg_dir = unpack_provider_archive(&archive, &dest, &[]).expect("unpack (no verify)");

    let manifest = pkg_dir.join("constructor.json");
    let mut content = std::fs::read_to_string(&manifest).unwrap();
    content.push_str("\n");
    content.push_str("{\"tampered\":true}");
    std::fs::write(&manifest, content).unwrap();

    let tampered = work.path().join("tampered.cloacina");
    fidius_core::package::pack_package(&pkg_dir, Some(&tampered)).expect("repack tampered dir");

    // Loading the tampered archive with the original key must fail closed: the
    // recomputed digest no longer matches the carried signature.
    let dest2 = work.path().join("unpacked-tamper-load");
    let err = load_task_constructor_from_package(
        &tampered,
        &dest2,
        "prefix",
        &Config { prefix: "x".into() },
        &[vk],
        &cloacina::registry::loader::grants::ResolvedGrants::deny_all(),
    )
    .err()
    .expect("tampered package must fail signature verification");
    let msg = format!("{err}");
    assert!(
        msg.contains("signature") || msg.contains("verify"),
        "tamper should surface a signature-verification error, got: {msg}"
    );
}

#[tokio::test]
async fn missing_constructor_manifest_fails_closed() {
    let work = tempfile::TempDir::new().unwrap();
    let (archive, _vk) = signed_archive(work.path());

    // Unpack, delete the sidecar manifest, then load through the loose-dir path
    // the package loader delegates to: a provider without its `constructor.json`
    // must not load.
    let dest = work.path().join("unpacked-nomanifest");
    let pkg_dir = unpack_provider_archive(&archive, &dest, &[]).expect("unpack");
    std::fs::remove_file(pkg_dir.join("constructor.json")).unwrap();

    let err = cloacina::registry::loader::load_task_constructor(
        &dest,
        "prefix",
        &Config { prefix: "x".into() },
        &cloacina::registry::loader::grants::ResolvedGrants::deny_all(),
    )
    .err()
    .expect("missing constructor.json must fail closed");
    let msg = format!("{err}");
    assert!(
        msg.contains("constructor.json") || msg.to_lowercase().contains("manifest"),
        "error should mention the missing manifest, got: {msg}"
    );
}
