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

//! CLOACI-I-0139 / T-0902 — end-to-end: a **native** (`runtime = "native"`)
//! constructor provider loads through the cloacina loader and runs as a
//! [`Task`](cloacina::task::Task).
//!
//! This is T-0902's acceptance proof (the spine was only compile-verified
//! before). The fixture `native-task-provider-fixture` is the SAME
//! `#[constructor(kind = task)]` + `constructor_provider!` author surface as
//! the wasm task fixture, but built to a HOST cdylib — so
//! `constructor_provider!` emits its native shell (`crate = fidius_core`,
//! `#[cfg(not(wasm32))]`, plugin `__ProviderTask`) and the loader `dlopen`s it
//! via `load_library` + `configure_from_loaded` (fidius 0.5.6) instead of
//! `load_wasm_configured`.
//!
//! Flow:
//!   1. `cargo build` the fixture cdylib + emit its base `provider.json`;
//!   2. stage a provider dir: `provider.json` (patched `runtime = "native"` +
//!      `component = <the built dylib>`) alongside a copy of the dylib;
//!   3. `load_task_constructor(search_path, provider, "prefix", cfg, grants)`
//!      takes the native fast-path → `Arc<dyn Task>`;
//!   4. execute with `{ name: "world" }` and a bound `prefix = "native-"` →
//!      output `result == "native-world"`, proving the native member's
//!      `configure`-bound config + context param round-trip in-process.
//!
//! Grants are ADVISORY for native (I-0139 (e)); the constructor loads with an
//! empty grant set (no sandbox to gate).
//!
//! Feature-gated (`constructors-wasm`, which compiles the constructor loader +
//! fidius-host). Excluded from the default build.
#![cfg(feature = "constructors-wasm")]

use std::path::{Path, PathBuf};

use serde::Serialize;

use cloacina::registry::loader::grants::ResolvedGrants;
use cloacina::registry::loader::load_task_constructor;
use cloacina::Context;

/// The provider's `[package].name` — the suite the native cdylib carries.
const PROVIDER: &str = "native-task-provider-fixture";

/// The single `#[config] prefix` the `prefix` member binds once at load.
#[derive(Serialize)]
struct PrefixConfig {
    prefix: String,
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/constructor-contract/native-task-provider-fixture")
}

/// The host dynamic-library extension for this target.
fn dylib_ext() -> &'static str {
    if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    }
}

/// `cargo build` the native fixture and return (built cdylib path, base
/// provider.json string from its `emit_manifest`).
fn build_fixture() -> (PathBuf, String) {
    let dir = fixture_dir();

    let status = std::process::Command::new(env!("CARGO"))
        .arg("build")
        .current_dir(&dir)
        .status()
        .expect("spawn cargo build (native fixture)");
    assert!(status.success(), "native fixture build failed");

    // rlib is linked by the emit_manifest bin; run it for the base manifest JSON.
    let out = std::process::Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--bin", "emit_manifest"])
        .current_dir(&dir)
        .output()
        .expect("run emit_manifest");
    assert!(out.status.success(), "emit_manifest failed");
    let manifest_json = String::from_utf8(out.stdout).expect("manifest utf8");

    let cdylib =
        dir.join("target/debug")
            .join(format!("lib{}.{}", PROVIDER.replace('-', "_"), dylib_ext()));
    assert!(
        cdylib.exists(),
        "built cdylib missing at {}",
        cdylib.display()
    );
    (cdylib, manifest_json)
}

/// Stage a native provider dir under `root`: `<root>/<PROVIDER>/{provider.json,
/// <component-dylib>}`, with the manifest patched to `runtime = "native"` and
/// `component = <dylib filename>`. Returns the search path (`root`) the loader
/// scans.
fn stage_native_provider(root: &Path, cdylib: &Path, base_manifest: &str) -> PathBuf {
    let pkg_dir = root.join(PROVIDER);
    std::fs::create_dir_all(&pkg_dir).unwrap();

    let component = format!("lib{}.{}", PROVIDER.replace('-', "_"), dylib_ext());
    std::fs::copy(cdylib, pkg_dir.join(&component)).expect("copy cdylib into provider dir");

    // Patch runtime + component into the emitted manifest (the macro defaults
    // them to wasm; the native BUILD/packaging path — T-0903 remainder — will
    // do this stamping, here we do it directly to exercise the LOADER).
    let mut manifest: serde_json::Value =
        serde_json::from_str(base_manifest).expect("parse base manifest");
    manifest["runtime"] = serde_json::json!("native");
    manifest["component"] = serde_json::json!(component);
    std::fs::write(
        pkg_dir.join("provider.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    root.to_path_buf()
}

#[tokio::test]
async fn native_provider_task_loads_and_runs_in_process() {
    let (cdylib, base_manifest) = build_fixture();

    let work = tempfile::TempDir::new().unwrap();
    let search_path = stage_native_provider(work.path(), &cdylib, &base_manifest);

    // Native grants are advisory — load with an empty grant set (no sandbox).
    let task = load_task_constructor(
        &search_path,
        PROVIDER,
        "prefix",
        &PrefixConfig {
            prefix: "native-".to_string(),
        },
        &ResolvedGrants::default(),
    )
    .expect("load native task constructor");
    assert_eq!(task.id(), "prefix");

    // Execute with the declared `name` param → the configure-bound prefix +
    // context param round-trip through the in-process cdylib.
    let mut ctx = Context::new();
    ctx.insert("name", serde_json::json!("world")).unwrap();
    let out = task.execute(ctx).await.expect("native task execute");
    assert_eq!(
        out.get("result"),
        Some(&serde_json::json!("native-world")),
        "native constructor bound `prefix` + read context `name` in-process"
    );
}

/// Fail-closed: asking for a member the native provider does not expose is a
/// clear error, not a silent wrong-load.
#[tokio::test]
async fn native_provider_unknown_member_rejected() {
    let (cdylib, base_manifest) = build_fixture();
    let work = tempfile::TempDir::new().unwrap();
    let search_path = stage_native_provider(work.path(), &cdylib, &base_manifest);

    // `Arc<dyn Task>` isn't `Debug`, so don't use `expect_err` — match instead.
    let msg = match load_task_constructor(
        &search_path,
        PROVIDER,
        "does-not-exist",
        &PrefixConfig {
            prefix: "x".to_string(),
        },
        &ResolvedGrants::default(),
    ) {
        Ok(_) => panic!("unknown member must be rejected, but it loaded"),
        Err(e) => format!("{e}"),
    };
    assert!(
        msg.contains("does-not-exist") || msg.to_lowercase().contains("no constructor"),
        "error should name the missing member: {msg}"
    );
}
