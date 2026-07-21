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

//! CLOACI-I-0139 / T-0903 — end-to-end: a `#[constructor(kind = task)]` crate is
//! **packaged as a NATIVE provider** via the exact code path
//! `cloacinactl constructor package --native` drives
//! ([`package_constructor_provider`] with `ProviderRuntime::Native`), then loaded
//! back out of the packed `.cloacina` archive through the T-0902 native load path
//! and executed.
//!
//! This is T-0903's acceptance proof for the packaging half: it exercises the
//! WHOLE round-trip that the loose-directory T-0902 test (`constructor_provider_native`)
//! stubbed by hand-patching `provider.json`:
//!   1. `package_constructor_provider(new_native)` → `cargo build --lib --features
//!      native` (host cdylib, NO wasm target) + emit `provider.json`
//!      (`runtime = "native"`, `component = <dylib>`) + a fidius `package.toml`
//!      (`runtime = "rust"`, no `[wasm]` section) → fidius `pack_package` →
//!      `<name>-<version>.cloacina`;
//!   2. `load_task_constructor_from_package` → fidius `unpack_package` → the
//!      loader's native fast-path (`resolve_native_provider` sees `runtime =
//!      "native"`) → `load_library` + `configure_from_loaded`;
//!   3. execute `{ name: "world" }` with bound `prefix = "native-"` → `result ==
//!      "native-world"`.
//!
//! Requires BOTH `constructor-packaging` (the packager) and `constructors-wasm`
//! (the loader). Excluded from the default build.
#![cfg(all(feature = "constructor-packaging", feature = "constructors-wasm"))]

use std::path::PathBuf;

use serde::Serialize;

use cloacina::packaging::constructor_provider::{
    package_constructor_provider, ProviderPackageOptions,
};
use cloacina::registry::loader::constructor_loader::load_task_constructor_from_package;
use cloacina::registry::loader::grants::ResolvedGrants;
use cloacina::Context;

const PROVIDER: &str = "native-task-provider-fixture";

#[derive(Serialize)]
struct PrefixConfig {
    prefix: String,
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/constructor-contract/native-task-provider-fixture")
}

#[tokio::test]
async fn native_provider_packages_and_loads_from_archive() {
    let out = tempfile::TempDir::new().unwrap();
    let archive_path = out.path().join("native-provider.cloacina");

    // (1) Package as NATIVE — same options `cloacinactl constructor package
    // --native` builds. Debug profile keeps the test fast.
    let opts = ProviderPackageOptions {
        output: Some(archive_path.clone()),
        release: false,
        ..ProviderPackageOptions::new_native(fixture_dir())
    };
    let result = package_constructor_provider(&opts).expect("package native provider");
    assert_eq!(result.provider_name, PROVIDER);
    assert!(
        result.archive.exists(),
        "packed archive should exist at {}",
        result.archive.display()
    );
    assert!(
        result.constructors.iter().any(|c| c == "prefix"),
        "suite carries the `prefix` member: {:?}",
        result.constructors
    );

    // (2) Load the `prefix` task back OUT of the packed archive via the native
    // path (unpack → resolve runtime=native → load_library → configure).
    let dest = tempfile::TempDir::new().unwrap();
    let task = load_task_constructor_from_package(
        &result.archive,
        dest.path(),
        PROVIDER,
        "prefix",
        &PrefixConfig {
            prefix: "native-".to_string(),
        },
        &[], // unsigned package → no verifying keys
        &ResolvedGrants::default(),
    )
    .expect("load native task constructor from packed archive");
    assert_eq!(task.id(), "prefix");

    // (3) Execute → the configure-bound prefix + context param round-trip through
    // the in-process cdylib that was just unpacked from the archive.
    let mut ctx = Context::new();
    ctx.insert("name", serde_json::json!("world")).unwrap();
    let out_ctx = task.execute(ctx).await.expect("native task execute");
    assert_eq!(
        out_ctx.get("result"),
        Some(&serde_json::json!("native-world")),
        "packaged native constructor bound `prefix` + read context `name` in-process"
    );
}
