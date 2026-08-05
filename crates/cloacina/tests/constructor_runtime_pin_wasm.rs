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

//! CLOACI-T-0920 — the NATIVE TRUST CLIFF is surfaced at the consumption site.
//!
//! `runtime` is an emission target, not part of a provider's identity: the same
//! `name@version` range can ship as a sandboxed wasm component in one release and
//! an in-process native cdylib in the next. Capability `grants` are ENFORCED on
//! wasm (fidius `WasiCtx` + `EgressPolicy`) and merely ADVISORY on native
//! (`load_native_member` takes no grants at all, by design — I-0139 (e)), so that
//! flip used to silently turn a consumer's `grants = { .. }` from a security
//! control into decoration, signalled only by a `tracing::info!` at load.
//!
//! Two rules close that, both enforced in `enforce_runtime_pin` at the consumer
//! resolution site (`load_constructor_node[_pinned]`), BEFORE any load work:
//!
//!   1. PIN — `constructor!(.., runtime = "wasm"|"native")`. If pinned, the
//!      RESOLVED provider's runtime must match or the load fails, naming both.
//!   2. DEFAULT HARDENING — grants present + resolved provider NATIVE + no explicit
//!      `runtime = "native"` acknowledgement ⇒ the load fails. Grants absent is
//!      unchanged (the pre-existing load-time log + CLI banner stay).
//!
//! The four cases below are exactly the matrix:
//!
//! | pin      | resolved | grants | expected                                  |
//! |----------|----------|--------|-------------------------------------------|
//! | `wasm`   | native   | —      | ERROR naming both runtimes                |
//! | none     | native   | yes    | ERROR: "grants are not enforced on native"|
//! | `native` | native   | yes    | LOADS (grants advisory; warn! emitted)    |
//! | `wasm`   | wasm     | yes    | LOADS, grants enforced exactly as before  |
//!
//! Feature-gated (`constructors-wasm`, which compiles the constructor loader +
//! fidius-host) and requires the `wasm32-wasip2` target for the wasm half.
#![cfg(feature = "constructors-wasm")]
// The `#[workflow]`/`constructor!` expansion emits a `cfg(packaged)`-style gate the
// workspace check-cfg lint flags as unknown here; benign (mirrors the sibling
// constructor tests).
#![allow(unexpected_cfgs)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde_json::json;

use cloacina::packaging::constructor_provider::{
    package_constructor_provider, ProviderPackageOptions,
};
use cloacina::registry::error::LoaderError;
use cloacina::registry::loader::grants::GrantSpec;
use cloacina::registry::loader::{
    load_constructor_node, load_constructor_node_pinned, set_provider_search_path,
    unpack_provider_archive, ProviderRuntime,
};

/// The NATIVE provider's `[package].name` (the suite its cdylib carries).
const NATIVE_PROVIDER: &str = "native-task-provider-fixture";
/// The WASM provider's `[package].name`.
const WASM_PROVIDER: &str = "prefix";
/// Both fixtures expose a `prefix` task member with a single `#[config] prefix`.
const MEMBER: &str = "prefix";

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/constructor-contract")
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
/// `provider.json` from its `emit_manifest` bin).
fn build_native_fixture() -> (PathBuf, String) {
    let dir = examples_dir().join(NATIVE_PROVIDER);

    let status = std::process::Command::new(env!("CARGO"))
        .arg("build")
        .current_dir(&dir)
        .status()
        .expect("spawn cargo build (native fixture)");
    assert!(status.success(), "native fixture build failed");

    let out = std::process::Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--bin", "emit_manifest"])
        .current_dir(&dir)
        .output()
        .expect("run emit_manifest");
    assert!(out.status.success(), "emit_manifest failed");
    let manifest_json = String::from_utf8(out.stdout).expect("manifest utf8");

    let cdylib = dir.join("target/debug").join(format!(
        "lib{}.{}",
        NATIVE_PROVIDER.replace('-', "_"),
        dylib_ext()
    ));
    assert!(
        cdylib.exists(),
        "built cdylib missing at {}",
        cdylib.display()
    );
    (cdylib, manifest_json)
}

/// Stage the native provider under `providers/<NATIVE_PROVIDER>/` with its
/// `provider.json` patched to `runtime = "native"` + the built dylib as
/// `component` (mirroring `constructor_provider_native.rs`).
fn stage_native(providers: &Path) {
    let (cdylib, base_manifest) = build_native_fixture();
    let pkg_dir = providers.join(NATIVE_PROVIDER);
    std::fs::create_dir_all(&pkg_dir).unwrap();

    let component = format!("lib{}.{}", NATIVE_PROVIDER.replace('-', "_"), dylib_ext());
    std::fs::copy(&cdylib, pkg_dir.join(&component)).expect("copy cdylib into provider dir");

    let mut manifest: serde_json::Value =
        serde_json::from_str(&base_manifest).expect("parse base manifest");
    manifest["runtime"] = json!("native");
    manifest["component"] = json!(component);
    std::fs::write(
        pkg_dir.join("provider.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
}

/// Package the wasm task fixture into an (unsigned) provider archive and unpack it
/// into `providers` — the same path `constructor_workflow_node_wasm.rs` uses.
fn stage_wasm(work: &Path, providers: &Path) {
    let archive = work.join("task-constructor-macro-fixture.cloacina");
    let opts = ProviderPackageOptions {
        crate_dir: examples_dir().join("task-constructor-macro-fixture"),
        output: Some(archive.clone()),
        sign_key: None,
        manifest_bin: "emit_manifest".to_string(),
        runtime: ProviderRuntime::Wasm,
        release: true,
    };
    package_constructor_provider(&opts).expect("package_constructor_provider");
    unpack_provider_archive(&archive, &providers.to_path_buf(), &[]).expect("unpack provider");
}

/// Build + stage BOTH providers (one native, one wasm) into a single shared
/// provider dir, ONCE for the whole test binary. Sharing one dir (and one path
/// value) keeps the process-global provider search path race-free across the
/// parallel tests and builds each artifact only once.
fn providers_dir() -> &'static PathBuf {
    static PROVIDERS: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();
    &PROVIDERS
        .get_or_init(|| {
            let work = tempfile::TempDir::new().unwrap();
            let providers = work.path().join("providers");
            std::fs::create_dir_all(&providers).unwrap();
            stage_native(&providers);
            stage_wasm(work.path(), &providers);
            (work, providers)
        })
        .1
}

/// `Arc<dyn Task>` is not `Debug`, so `expect_err` won't compile — match instead.
fn err_of<T>(r: Result<T, LoaderError>, what: &str) -> String {
    match r {
        Ok(_) => panic!("{what}"),
        Err(e) => e.to_string(),
    }
}

/// A `prefix`-member load against `from`, with the given grants and pin.
fn load(
    from: &str,
    grants: GrantSpec,
    pin: Option<ProviderRuntime>,
) -> Result<std::sync::Arc<dyn cloacina::task::Task>, LoaderError> {
    load_constructor_node_pinned(
        "greet",
        from,
        MEMBER,
        vec![("prefix".to_string(), json!("hello, "))],
        vec![],
        grants,
        pin,
    )
}

/// A non-empty grant set — the thing whose enforcement must never silently lapse.
fn some_grants() -> GrantSpec {
    GrantSpec::from_pairs(vec![("fs".to_string(), vec!["ro:/tmp".to_string()])])
}

// ===========================================================================
// Case 1 — pinned wasm, provider resolves NATIVE ⇒ hard error naming BOTH.
// ===========================================================================
#[test]
fn pinned_wasm_against_native_provider_fails_naming_both_runtimes() {
    set_provider_search_path(providers_dir());

    let msg = err_of(
        load(
            NATIVE_PROVIDER,
            GrantSpec::default(),
            Some(ProviderRuntime::Wasm),
        ),
        "a wasm pin against a native provider must fail closed",
    );
    assert!(
        msg.contains("runtime = \"wasm\"") && msg.contains("runtime = \"native\""),
        "the error must name BOTH the pin and the resolved runtime: {msg}"
    );
    assert!(
        msg.contains(NATIVE_PROVIDER),
        "the error must name the provider: {msg}"
    );
}

// ===========================================================================
// Case 2 — grants present, provider resolves NATIVE, no acknowledgement ⇒ error.
// This is the silent-flip scenario: before T-0920 this loaded, and the grants
// quietly became decoration.
// ===========================================================================
#[test]
fn grants_against_native_provider_without_acknowledgement_fails() {
    set_provider_search_path(providers_dir());

    let msg = err_of(
        load(NATIVE_PROVIDER, some_grants(), None),
        "grants against an unacknowledged native provider must fail closed",
    );
    assert!(
        msg.contains("grants are not enforced on native providers"),
        "the error must state the trust cliff plainly: {msg}"
    );
    assert!(
        msg.contains("runtime = \"wasm\"")
            && msg.contains("runtime = \"native\"")
            && msg.contains("remove grants"),
        "the error must offer all three remedies (pin wasm / ack native / drop grants): {msg}"
    );
}

/// The hardening keys off GRANTS, not merely on nativeness: a native provider with
/// NO grants keeps loading exactly as before (existing load-time log + CLI banner
/// remain the signal). This is what keeps the rule from breaking today's native
/// consumers.
#[test]
fn native_provider_without_grants_is_unchanged() {
    set_provider_search_path(providers_dir());

    assert!(
        load(NATIVE_PROVIDER, GrantSpec::default(), None).is_ok(),
        "a grant-free native load must not be affected by the T-0920 hardening"
    );
}

// ===========================================================================
// Case 3 — grants present, NATIVE, explicitly acknowledged ⇒ loads (with warn!).
// ===========================================================================
#[test]
fn grants_against_acknowledged_native_provider_loads() {
    set_provider_search_path(providers_dir());

    let task = load(
        NATIVE_PROVIDER,
        some_grants(),
        Some(ProviderRuntime::Native),
    )
    .expect("an acknowledged native load must succeed");
    assert_eq!(
        task.id(),
        "greet",
        "the acknowledged load yields the DAG node under its authored id"
    );
}

// ===========================================================================
// Case 4 — grants present against a WASM provider ⇒ loads and stays enforced,
// with or without an explicit `runtime = "wasm"` pin. T-0920 must not perturb
// the enforced path at all.
// ===========================================================================
#[test]
fn grants_against_wasm_provider_are_unchanged() {
    set_provider_search_path(providers_dir());

    assert!(
        load(WASM_PROVIDER, some_grants(), Some(ProviderRuntime::Wasm)).is_ok(),
        "a matching wasm pin must load"
    );
    assert!(
        load(WASM_PROVIDER, some_grants(), None).is_ok(),
        "an unpinned wasm load with grants is unchanged (grants stay enforced)"
    );
    // And the unpinned public entry point (what the packaged/server path calls,
    // since the pin cannot cross the bincode FFI declaration) behaves identically.
    assert!(
        load_constructor_node(
            "greet",
            WASM_PROVIDER,
            MEMBER,
            vec![("prefix".to_string(), json!("hello, "))],
            vec![],
            some_grants(),
        )
        .is_ok(),
        "the unpinned entry point is unchanged for wasm providers"
    );
}

// ===========================================================================
// The MACRO GRAMMAR half: `constructor!(.., runtime = "wasm")` must parse, lower,
// and thread the pin into the loader. A broken lowering either fails to compile or
// fails the load, so a green end-to-end run is the proof.
// ===========================================================================
#[cfg(feature = "macros")]
#[cloacina::workflow(name = "pinned_greet", description = "constructor! runtime-pin e2e")]
pub mod pinned_greet {
    use cloacina::{task, Context, TaskError};
    use serde_json::json;

    #[task(id = "seed_name", dependencies = [])]
    pub async fn seed_name(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("name", json!("world"))?;
        Ok(())
    }

    // CLOACI-T-0920: the trust tier is PINNED at the call site. The staged provider
    // is a wasm component, so this matches and the grants stay enforced.
    constructor!(
        id = "greet",
        from = "prefix@0.1.0",
        constructor = "prefix",
        config = { prefix = "hello, " },
        dependencies = ["seed_name"],
        runtime = "wasm",
    );
}

#[cfg(feature = "macros")]
#[tokio::test]
async fn constructor_macro_runtime_pin_threads_through_to_the_loader() {
    use cloacina::executor::WorkflowExecutor;
    use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
    use cloacina::Context;

    set_provider_search_path(providers_dir());

    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .build()
        .unwrap();
    let runner = DefaultRunner::with_config(":memory:", config)
        .await
        .expect("create DefaultRunner");

    let result = runner
        .execute("pinned_greet", Context::new())
        .await
        .expect("workflow execution");

    assert_eq!(
        result.final_context.get("result"),
        Some(&json!("hello, world")),
        "a `runtime = \"wasm\"` pinned constructor node runs exactly like an unpinned one"
    );

    runner.shutdown().await.expect("shutdown");
}

/// The inverse pin also fails: `runtime = "native"` against a provider that
/// resolves as wasm. A pin is exact in BOTH directions, so a native→wasm flip is
/// caught just as loudly as wasm→native.
#[test]
fn pinned_native_against_wasm_provider_fails() {
    set_provider_search_path(providers_dir());

    let msg = err_of(
        load(
            WASM_PROVIDER,
            GrantSpec::default(),
            Some(ProviderRuntime::Native),
        ),
        "a native pin against a wasm provider must fail closed",
    );
    assert!(
        msg.contains("runtime = \"native\"") && msg.contains("runtime = \"wasm\""),
        "the error must name BOTH the pin and the resolved runtime: {msg}"
    );
}
