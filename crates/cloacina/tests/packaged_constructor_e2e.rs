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

//! CLOACI-T-0836 — the **build-side chain end-to-end** at the library level:
//! discover + resolve a provider from a consumer's Cargo graph → build it to wasm →
//! bundle it into a `providers/` tree → point the loader's `provider_search_path` at
//! the bundle → resolve a `constructor!` node BY NAME against it → run it, grant-gated.
//!
//! This is the packaged-consumer path minus the server/registry storage: it proves
//! that a provider resolved as an ordinary Cargo dependency
//! (`provider-consumer-fixture` depends on `cloacina-provider-fs`) can be built,
//! bundled, and run hermetically — the same `providers/` layout the server load path
//! will unpack once the storage plumbing (T-0836 increment c) lands.
//!
//! Feature-gated `constructors-wasm` (wasmtime + implies `constructor-packaging`, so
//! both `bundle_providers` and `load_constructor_node` are available). It shells a
//! `cargo build --target wasm32-wasip2` for the provider.
#![cfg(feature = "constructors-wasm")]

use std::path::PathBuf;
use std::sync::OnceLock;

use cloacina::packaging::provider_bundle::{bundle_providers, ProviderRef};
use cloacina::registry::loader::grants::GrantSpec;
use cloacina::registry::loader::{load_constructor_node, set_provider_search_path};
use cloacina::Context;

fn consumer_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/constructor-contract/provider-consumer-fixture")
}

/// Bundle `cloacina-provider-fs` (resolved from the consumer's Cargo graph) into a
/// `providers/` tree ONCE, point the process-wide provider search path at it, and
/// return a data dir the tests read/write through the sandbox. Kept in a `OnceLock`
/// so the tempdir + the search path outlive every `#[tokio::test]` in this binary.
fn provider_search_root() -> &'static PathBuf {
    static ROOT: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();
    &ROOT
        .get_or_init(|| {
            let work = tempfile::TempDir::new().unwrap();
            let bundled = bundle_providers(
                &consumer_dir(),
                &[ProviderRef::parse("cloacina-provider-fs")],
                work.path(),
                false,
            )
            .expect("bundle cloacina-provider-fs from the consumer's Cargo graph");
            assert_eq!(bundled.len(), 1);

            let providers = work.path().join("providers");
            set_provider_search_path(&providers);

            // A data dir for the constructors to touch, with a seed file to read.
            let data = work.path().join("data");
            std::fs::create_dir_all(&data).unwrap();
            std::fs::write(data.join("secret.txt"), "packaged secret").unwrap();

            (work, data)
        })
        .1
}

fn fs_ro_grant(dir: &std::path::Path) -> GrantSpec {
    GrantSpec::from_lists(
        vec![],
        vec![],
        vec![format!("ro:{}", dir.display())],
        vec![],
    )
}

#[tokio::test]
async fn bundled_provider_node_resolves_by_name_and_runs_with_a_grant() {
    let data = provider_search_root();
    let secret = data.join("secret.txt");

    // Resolve the `read_file` member of the BUNDLED provider as a workflow node —
    // exactly what a `constructor!(from = "cloacina-provider-fs", constructor =
    // "read_file", ...)` lowers to — and run it with a read grant.
    let task = load_constructor_node(
        "reader",
        "cloacina-provider-fs@0.1.0",
        "read_file",
        vec![(
            "path".to_string(),
            serde_json::json!(secret.display().to_string()),
        )],
        vec![],
        fs_ro_grant(data),
    )
    .expect("resolve read_file from the bundled provider");

    assert_eq!(task.id(), "reader", "the node id is the author-chosen id");

    let out = task
        .execute(Context::new())
        .await
        .expect("the bundled constructor node runs");
    assert_eq!(
        out.get("contents"),
        Some(&serde_json::json!("packaged secret")),
        "read_file read the granted file through the bundled+sandboxed component"
    );
}

#[tokio::test]
async fn bundled_provider_node_fails_closed_without_a_grant() {
    let data = provider_search_root();
    let secret = data.join("secret.txt");

    // Same bundled member, but NO grant → default-closed sandbox denies the read.
    let task = load_constructor_node(
        "reader_denied",
        "cloacina-provider-fs@0.1.0",
        "read_file",
        vec![(
            "path".to_string(),
            serde_json::json!(secret.display().to_string()),
        )],
        vec![],
        GrantSpec::from_lists(vec![], vec![], vec![], vec![]),
    )
    .expect("resolve read_file (load succeeds; the DENIAL is at execute)");

    let err = task
        .execute(Context::new())
        .await
        .expect_err("a read with no fs grant must fail closed");
    let msg = format!("{err}");
    assert!(
        msg.contains("read") || msg.to_lowercase().contains("pre-opened") || msg.contains("secret"),
        "error should reflect the denied sandbox read, got: {msg}"
    );
}

#[tokio::test]
async fn unknown_member_of_bundled_provider_fails_closed() {
    let _ = provider_search_root();
    let err = load_constructor_node(
        "nope",
        "cloacina-provider-fs@0.1.0",
        "delete_file", // not a member of this suite
        vec![("path".to_string(), serde_json::json!("/x"))],
        vec![],
        GrantSpec::from_lists(vec![], vec![], vec![], vec![]),
    )
    .err()
    .expect("an unknown member must fail closed");
    let msg = format!("{err}");
    assert!(
        msg.contains("delete_file") && msg.contains("read_file"),
        "error should name the missing member + the available ones, got: {msg}"
    );
}
