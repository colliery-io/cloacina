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

//! CLOACI-T-0837 — end-to-end: a `#[constructor]`-authored **provider SUITE** is
//! packaged into a **signed fidius provider package**, and the loader resolves
//! **two different members by name** from the ONE packed component (unpack → verify
//! signature → `load_wasm_configured` with name-in-configure) to run as cloacina
//! [`Task`](cloacina::task::Task)s.
//!
//! The fixture is `cloacina-provider-fs`, now a 2-member suite (CLOACI-A-0011):
//! provider `cloacina-provider-fs` = { `read_file`, `write_file` }, one component.
//! The proof:
//!   1. `package_constructor_provider` builds the suite to a `wasm32-wasip2`
//!      component, emits `provider.json` from `__provider_manifest()` (BOTH
//!      members), assembles a `runtime = "wasm"` package, Ed25519-signs it, packs
//!      a `.cloacina` archive;
//!   2. `load_task_constructor_from_package(.., "read_file", ..)` and `.., "write_file", ..)`
//!      each select their member by name from the SAME archive/component and load
//!      it → `Arc<dyn Task>`;
//!   3. with an `fs` grant, `read_file` reads a file and `write_file` writes one —
//!      proving the two members coexist and run independently from one package.
//! Plus fail-closed paths: an unknown member, a wrong verifying key, a tampered
//! package, and a missing `provider.json` are all rejected.
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
use cloacina::registry::loader::grants::{translate, GrantSpec, ResolvedGrants};
use cloacina::registry::loader::{
    load_task_constructor, load_task_constructor_from_package, unpack_provider_archive,
};
use cloacina::Context;

/// The provider's fidius `[package].name` — the suite the archive carries.
const PROVIDER: &str = "cloacina-provider-fs";

/// Per-instance config the loader binds once at load: both `read_file` and
/// `write_file` take a single `#[config] path` (serde-compatible with the
/// macro-generated guest config tuple).
#[derive(Serialize)]
struct PathConfig {
    path: String,
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/constructor-contract/cloacina-provider-fs")
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

/// Package the fs suite into a signed `.cloacina` provider archive once.
fn signed_archive(work: &std::path::Path) -> (PathBuf, ed25519_dalek::VerifyingKey) {
    let (key_path, vk) = write_test_key(work);
    let out = work.join("fs-provider.cloacina");

    let opts = ProviderPackageOptions {
        crate_dir: fixture_dir(),
        output: Some(out.clone()),
        sign_key: Some(key_path),
        manifest_bin: "emit_manifest".to_string(),
        // Debug build keeps the test fast (reuses the debug wasm artifact).
        release: false,
    };
    let result = package_constructor_provider(&opts).expect("package_constructor_provider");

    assert!(result.signed, "archive should be signed");
    assert_eq!(result.provider_name, PROVIDER);
    // The provider carries BOTH members (a real suite), in declaration order.
    assert_eq!(
        result.constructors,
        vec!["read_file".to_string(), "write_file".to_string()],
        "provider.json should list both suite members"
    );
    assert!(out.exists(), "archive written to {}", out.display());
    (out, vk)
}

/// Build a granting [`ResolvedGrants`] for the given `ro:`/`rw:`-prefixed fs entries.
fn fs_grant(entries: Vec<String>) -> ResolvedGrants {
    translate(&GrantSpec::from_lists(vec![], vec![], entries, vec![])).expect("translate fs grant")
}

#[tokio::test]
async fn suite_members_coexist_and_run_from_one_signed_package() {
    let work = tempfile::TempDir::new().unwrap();
    let (archive, vk) = signed_archive(work.path());

    // A data dir to grant, holding a file for `read_file` to read.
    let data = work.path().join("data");
    std::fs::create_dir_all(&data).unwrap();
    let in_file = data.join("input.txt");
    std::fs::write(&in_file, "hello from disk").unwrap();

    // --- Member 1: read_file (selected by name; granted read of `data`). ---
    let dest_read = work.path().join("unpacked-read");
    let read = load_task_constructor_from_package(
        &archive,
        &dest_read,
        PROVIDER,
        "read_file",
        &PathConfig {
            path: in_file.display().to_string(),
        },
        &[vk],
        &fs_grant(vec![format!("ro:{}", data.display())]),
    )
    .expect("load read_file from signed package");
    assert_eq!(read.id(), "read_file");

    let out = read
        .execute(Context::new())
        .await
        .expect("read_file execute");
    assert_eq!(
        out.get("contents"),
        Some(&serde_json::json!("hello from disk")),
        "read_file (member 1) reads the granted file from inside the sandbox"
    );

    // --- Member 2: write_file (SAME component, different member, granted rw). ---
    let out_file = data.join("output.txt");
    let dest_write = work.path().join("unpacked-write");
    let write = load_task_constructor_from_package(
        &archive,
        &dest_write,
        PROVIDER,
        "write_file",
        &PathConfig {
            path: out_file.display().to_string(),
        },
        &[vk],
        &fs_grant(vec![format!("rw:{}", data.display())]),
    )
    .expect("load write_file from signed package");
    assert_eq!(write.id(), "write_file");

    let msg = "written by write_file";
    let mut ctx = Context::new();
    ctx.insert("contents", serde_json::json!(msg)).unwrap();
    let out2 = write.execute(ctx).await.expect("write_file execute");
    assert_eq!(
        out2.get("written_bytes"),
        Some(&serde_json::json!(msg.len() as i64)),
        "write_file (member 2) reports the bytes written"
    );
    assert_eq!(
        std::fs::read_to_string(&out_file).unwrap(),
        msg,
        "write_file actually wrote the granted file"
    );
}

#[tokio::test]
async fn unknown_member_fails_closed_naming_the_suite() {
    let work = tempfile::TempDir::new().unwrap();
    let (archive, vk) = signed_archive(work.path());

    let dest = work.path().join("unpacked-unknown");
    let err = load_task_constructor_from_package(
        &archive,
        &dest,
        PROVIDER,
        "delete_file", // not a member of this suite
        &PathConfig { path: "x".into() },
        &[vk],
        &ResolvedGrants::deny_all(),
    )
    .err()
    .expect("an unknown member must fail closed");
    let msg = format!("{err}");
    assert!(
        msg.contains("delete_file") && msg.contains("read_file") && msg.contains("write_file"),
        "error should name the missing member + list the available ones, got: {msg}"
    );
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
        PROVIDER,
        "read_file",
        &PathConfig { path: "x".into() },
        &[wrong],
        &ResolvedGrants::deny_all(),
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

    // Unpack WITHOUT verifying, tamper the provider manifest, then REPACK it
    // (carrying the original, now-stale `package.sig`).
    let dest = work.path().join("unpacked-tamper");
    let pkg_dir = unpack_provider_archive(&archive, &dest, &[]).expect("unpack (no verify)");

    let manifest = pkg_dir.join("provider.json");
    let mut content = std::fs::read_to_string(&manifest).unwrap();
    content.push('\n');
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
        PROVIDER,
        "read_file",
        &PathConfig { path: "x".into() },
        &[vk],
        &ResolvedGrants::deny_all(),
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
async fn missing_provider_manifest_fails_closed() {
    let work = tempfile::TempDir::new().unwrap();
    let (archive, _vk) = signed_archive(work.path());

    // Unpack, delete the provider index, then load through the loose-dir path the
    // package loader delegates to: a provider without its `provider.json` must not
    // load.
    let dest = work.path().join("unpacked-nomanifest");
    let pkg_dir = unpack_provider_archive(&archive, &dest, &[]).expect("unpack");
    std::fs::remove_file(pkg_dir.join("provider.json")).unwrap();

    let err = load_task_constructor(
        &dest,
        PROVIDER,
        "read_file",
        &PathConfig { path: "x".into() },
        &ResolvedGrants::deny_all(),
    )
    .err()
    .expect("missing provider.json must fail closed");
    let msg = format!("{err}");
    assert!(
        msg.contains("provider.json") || msg.to_lowercase().contains("manifest"),
        "error should mention the missing manifest, got: {msg}"
    );
}
