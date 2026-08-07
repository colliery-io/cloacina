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

//! CLOACI-T-0925 — constructor/provider resolution respects the TENANT boundary.
//!
//! The bug: `constructor!(from = "provider@version")` resolved against ONE
//! process-wide directory. A shared server process runs one reconciler per tenant,
//! each on its own task, so tenant A's staged providers were visible to (and
//! racily overwritten by) tenant B's resolution — a doctrine breach, since tenant
//! is THE isolation boundary.
//!
//! What these tests pin:
//!   * a provider staged for tenant A is NOT resolvable by tenant B's constructor
//!     node in the SAME process — with both the explicit-directory entry point and
//!     an installed per-load scope;
//!   * a leftover process-wide override (what another tenant's load used to leave
//!     behind) cannot leak into a scoped load;
//!   * the embedded/untenanted path — `set_provider_search_path` + a plain
//!     `load_constructor_node` — resolves exactly as it always did.
//!
//! Feature-gated (`constructors-wasm`); needs the `wasm32-wasip2` target to build
//! the fixture provider.
#![cfg(feature = "constructors-wasm")]

use std::path::PathBuf;
use std::sync::OnceLock;

use cloacina::packaging::constructor_provider::{
    package_constructor_provider, ProviderPackageOptions,
};
use cloacina::registry::loader::grants::GrantSpec;
use cloacina::registry::loader::{
    clear_provider_search_path, load_constructor_node, load_constructor_node_in,
    set_provider_search_path, unpack_provider_archive, ProviderScope, ScopedProviderSearch,
};
use serde_json::json;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/constructor-contract/task-constructor-macro-fixture")
}

/// Build the `prefix` provider ONCE and unpack it into a directory that stands in
/// for ONE tenant's staged bundle. Tenant B deliberately gets nothing.
struct Tenants {
    _work: tempfile::TempDir,
    /// Tenant A's staged providers — holds the `prefix` provider.
    a: PathBuf,
    /// Tenant B's staged providers — exists, but stages nothing.
    b: PathBuf,
}

fn tenants() -> &'static Tenants {
    static TENANTS: OnceLock<Tenants> = OnceLock::new();
    TENANTS.get_or_init(|| {
        let work = tempfile::TempDir::new().unwrap();
        let a = work.path().join("tenant-a-providers");
        let b = work.path().join("tenant-b-providers");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();

        let archive = work.path().join("prefix.cloacina");
        let opts = ProviderPackageOptions {
            crate_dir: fixture_dir(),
            output: Some(archive.clone()),
            sign_key: None,
            manifest_bin: "emit_manifest".to_string(),
            runtime: cloacina_constructor_contract::ProviderRuntime::Wasm,
            release: true,
        };
        package_constructor_provider(&opts).expect("package_constructor_provider");
        unpack_provider_archive(&archive, &a, &[]).expect("unpack provider archive");

        Tenants { _work: work, a, b }
    })
}

fn node_config() -> Vec<(String, serde_json::Value)> {
    vec![("prefix".to_string(), json!("hello, "))]
}

/// The heart of the ticket: same process, same `from`, two tenants — only the
/// tenant that staged the provider resolves it.
#[test]
fn provider_staged_for_tenant_a_is_not_resolvable_by_tenant_b() {
    let t = tenants();

    load_constructor_node_in(
        &t.a,
        "greet",
        "prefix@0.1.0",
        "prefix",
        node_config(),
        vec![],
        GrantSpec::default(),
    )
    .expect("tenant A staged this provider — its own node must resolve");

    let err = load_constructor_node_in(
        &t.b,
        "greet",
        "prefix@0.1.0",
        "prefix",
        node_config(),
        vec![],
        GrantSpec::default(),
    )
    .err()
    .expect("tenant B staged NO providers — it must not see tenant A's tree");
    let msg = err.to_string();
    assert!(
        msg.contains(&t.b.display().to_string()),
        "the failure must name the tenant's OWN search path, got: {msg}"
    );
    assert!(
        !msg.contains(&t.a.display().to_string()),
        "tenant A's directory must never appear in tenant B's resolution: {msg}"
    );
}

/// A scoped load ignores a process-wide override — which is exactly what another
/// tenant's load used to leave behind when it pointed the global at its own tree.
#[test]
#[serial_test::serial(provider_search_path)]
fn a_leftover_process_override_cannot_leak_into_a_scoped_load() {
    let t = tenants();
    // Stand in for "some other tenant's load set the global to its staged tree".
    set_provider_search_path(&t.a);

    {
        let _scope = ScopedProviderSearch::enter(ProviderScope::Staged(t.b.clone()));
        let scoped = load_constructor_node(
            "greet",
            "prefix@0.1.0",
            "prefix",
            node_config(),
            vec![],
            GrantSpec::default(),
        );
        assert!(
            scoped.is_err(),
            "a tenant-B-scoped load must not resolve tenant A's provider"
        );
    }

    {
        // A package that bundles NOTHING must not inherit the leftover either.
        let _scope = ScopedProviderSearch::enter(ProviderScope::Unbundled);
        let unbundled = load_constructor_node(
            "greet",
            "prefix@0.1.0",
            "prefix",
            node_config(),
            vec![],
            GrantSpec::default(),
        );
        assert!(
            unbundled.is_err(),
            "an unbundled package must not inherit the process-wide override"
        );
    }

    clear_provider_search_path();
}

/// Embedded/untenanted parity: no scope in play, `set_provider_search_path` +
/// `load_constructor_node` behave exactly as before T-0925.
#[test]
#[serial_test::serial(provider_search_path)]
fn embedded_untenanted_path_is_unchanged() {
    let t = tenants();
    set_provider_search_path(&t.a);

    load_constructor_node(
        "greet",
        "prefix@0.1.0",
        "prefix",
        node_config(),
        vec![],
        GrantSpec::default(),
    )
    .expect("with no scope installed, the process-wide override still resolves");

    clear_provider_search_path();
}
