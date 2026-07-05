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

//! CLOACI-T-0829 — end-to-end: the `constructor!(…)` CONSUMER form runs as a DAG
//! node inside a `#[workflow]`, mixed with hand-authored `#[task]`s.
//!
//! Proves the whole consumer surface:
//!   1. package the macro-authored constructor fixtures (the T-0827 path) into
//!      fidius provider archives and unpack them into ONE shared provider dir;
//!   2. point the process provider search path at that dir;
//!   3. run `#[workflow]`s that wire the packaged constructors in as nodes.
//!
//! Three cases:
//!   * `constructor_node_runs_in_workflow_with_deps_and_output` — the baseline:
//!     a single-`#[config]` constructor (`prefix`), deps honored, output visible.
//!   * `reordered_config_binds_by_name` — the NAME-KEYED proof: a TWO-`#[config]`
//!     constructor (`affix`, declared `prefix` THEN `suffix`) is wired with the
//!     kwargs written in the OPPOSITE order (`suffix` first). True kwarg semantics
//!     mean it must still produce `"hello, world!"`, NOT the positional mis-bind.
//!   * `config_kwarg_errors_are_clear` — the negative: an UNKNOWN key and a MISSING
//!     required key each fail closed with a clear, key-named error.
//!
//! Feature-gated (`constructors-wasm`, which pulls wasmtime; the embedded
//! `DefaultRunner` uses the default `sqlite` lane). Excluded from the default build.
#![cfg(feature = "constructors-wasm")]
// The `#[workflow]` macro emits `#[cfg(feature = "packaged")]` arms (resolved
// against this destination test crate, which has no `packaged` feature); benign.
#![allow(unexpected_cfgs)]

use std::path::PathBuf;
use std::sync::OnceLock;

use cloacina::executor::WorkflowExecutor;
use cloacina::packaging::constructor_provider::{
    package_constructor_provider, ProviderPackageOptions,
};
use cloacina::registry::loader::{
    load_constructor_node, set_provider_search_path, unpack_provider_archive,
};
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError};
use serde_json::json;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/constructor-contract")
}

// ===========================================================================
// Baseline workflow: a single-#[config] constructor (`prefix`).
// ===========================================================================
#[workflow(name = "onboard", description = "constructor! consumer-surface e2e")]
pub mod onboard {
    use super::*;

    // A hand-authored upstream task: seeds the context the constructor reads.
    #[task(id = "load_user", dependencies = [])]
    pub async fn load_user(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("name", json!("world"))?;
        Ok(())
    }

    // The packaged constructor wired in as a primitive DAG node — no body written.
    // `prefix` is bound once at load; it depends on `load_user`, and its `result`
    // output flows to `notify`.
    constructor!(
        id = "greet",
        from = "prefix@0.1.0",
        constructor = "prefix",
        config = { prefix = "hello, " },
        dependencies = ["load_user"],
    );

    // A hand-authored downstream task that consumes the constructor's output.
    #[task(id = "notify", dependencies = ["greet"])]
    pub async fn notify(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let greeting = context
            .get("result")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        context.insert("notified", json!(greeting))?;
        Ok(())
    }
}

// ===========================================================================
// Reordered-config workflow: a TWO-#[config] constructor (`affix`).
//
// The guest declares `#[config] prefix` THEN `#[config] suffix`. Here the kwargs
// are written suffix-FIRST — the opposite order — to prove the bind is by NAME.
// ===========================================================================
#[workflow(name = "affixed", description = "reordered-config constructor! e2e")]
pub mod affixed {
    use super::*;

    // Distinct task ids/fn names from the `onboard` module: the `#[task]`/`#[workflow]`
    // macros track ids in a crate-global registry, so two workflows in one test binary
    // can't reuse `load_user`/`notify`.
    #[task(id = "seed", dependencies = [])]
    pub async fn seed(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        context.insert("name", json!("world"))?;
        Ok(())
    }

    // Kwargs intentionally in REVERSE of the guest's #[config] declaration order
    // (guest: prefix, suffix). Name-keyed binding must still yield "hello, world!".
    constructor!(
        id = "wrap",
        from = "affix@0.1.0",
        constructor = "affix",
        config = { suffix = "!", prefix = "hello, " },
        dependencies = ["seed"],
    );

    #[task(id = "announce", dependencies = ["wrap"])]
    pub async fn announce(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let wrapped = context
            .get("result")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        context.insert("notified", json!(wrapped))?;
        Ok(())
    }
}

/// Package a macro fixture under `examples/constructor-contract/<dir>` into an
/// (unsigned) provider archive and unpack it into `providers`.
fn stage_into(work: &tempfile::TempDir, providers: &PathBuf, fixture_dir: PathBuf) {
    let archive = work.path().join(format!(
        "{}.cloacina",
        fixture_dir.file_name().unwrap().to_string_lossy()
    ));
    let opts = ProviderPackageOptions {
        crate_dir: fixture_dir,
        output: Some(archive.clone()),
        sign_key: None,
        manifest_bin: "emit_manifest".to_string(),
        release: true,
    };
    package_constructor_provider(&opts).expect("package_constructor_provider");
    unpack_provider_archive(&archive, providers, &[]).expect("unpack provider archive");
}

/// Build + stage BOTH constructor fixtures into one shared provider dir, ONCE for
/// the whole test binary. Returns the provider search-path dir. Sharing one dir (and
/// one path value) keeps the global provider search path race-free across the
/// parallel `#[tokio::test]`s, and builds each wasm component only once.
fn providers_dir() -> &'static PathBuf {
    static PROVIDERS: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();
    &PROVIDERS
        .get_or_init(|| {
            let work = tempfile::TempDir::new().unwrap();
            let providers = work.path().join("providers");
            stage_into(
                &work,
                &providers,
                examples_dir().join("task-constructor-macro-fixture"),
            );
            stage_into(
                &work,
                &providers,
                examples_dir().join("task-constructor-twocfg-fixture"),
            );
            (work, providers)
        })
        .1
}

#[tokio::test]
async fn constructor_node_runs_in_workflow_with_deps_and_output() {
    set_provider_search_path(providers_dir());

    // Embedded runner against in-memory SQLite (background reconciler off).
    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .build()
        .unwrap();
    let runner = DefaultRunner::with_config(":memory:", config)
        .await
        .expect("create DefaultRunner");

    let result = runner
        .execute("onboard", Context::new())
        .await
        .expect("workflow execution");

    // The constructor executed as its primitive: config-bound prefix + the upstream
    // task's `name`, with the output visible to the dependent task.
    assert_eq!(
        result.final_context.get("name"),
        Some(&json!("world")),
        "load_user ran first (dependency honored)"
    );
    assert_eq!(
        result.final_context.get("result"),
        Some(&json!("hello, world")),
        "the packaged constructor ran as a node: config prefix + load_user's name"
    );
    assert_eq!(
        result.final_context.get("notified"),
        Some(&json!("hello, world")),
        "the dependent #[task] saw the constructor node's output"
    );

    runner.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn reordered_config_binds_by_name() {
    set_provider_search_path(providers_dir());

    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .build()
        .unwrap();
    let runner = DefaultRunner::with_config(":memory:", config)
        .await
        .expect("create DefaultRunner");

    let result = runner
        .execute("affixed", Context::new())
        .await
        .expect("workflow execution");

    // Guest declared prefix THEN suffix; the workflow wrote the kwargs suffix-FIRST.
    // Name-keyed binding => prefix="hello, ", suffix="!" => "hello, world!".
    // A positional (written-order) bind would have produced "!world hello, ".
    assert_eq!(
        result.final_context.get("result"),
        Some(&json!("hello, world!")),
        "config kwargs bound by NAME despite reversed written order"
    );
    assert_eq!(
        result.final_context.get("notified"),
        Some(&json!("hello, world!")),
        "the dependent #[task] saw the name-keyed constructor output"
    );

    runner.shutdown().await.expect("shutdown");
}

#[test]
fn config_kwarg_errors_are_clear() {
    set_provider_search_path(providers_dir());

    // `Arc<dyn Task>` is not `Debug`, so `expect_err` won't compile — match the Ok
    // arm to a panic instead.
    let unwrap_err = |r: Result<_, cloacina::registry::error::LoaderError>, what: &str| match r {
        Ok(_) => panic!("{what}"),
        Err(e) => e.to_string(),
    };

    // UNKNOWN key: `bogus` is not a #[config] field of `affix`.
    let msg = unwrap_err(
        load_constructor_node(
            "wrap",
            "affix@0.1.0",
            "affix",
            vec![
                ("prefix".to_string(), json!("hello, ")),
                ("suffix".to_string(), json!("!")),
                ("bogus".to_string(), json!("x")),
            ],
            vec![],
            cloacina::registry::loader::grants::GrantSpec::default(),
        ),
        "unknown config key must fail closed",
    );
    assert!(
        msg.contains("bogus") && msg.contains("not a #[config] field"),
        "unknown-key error must name the offending key: {msg}"
    );

    // MISSING required key: `suffix` is declared but not supplied.
    let msg = unwrap_err(
        load_constructor_node(
            "wrap",
            "affix@0.1.0",
            "affix",
            vec![("prefix".to_string(), json!("hello, "))],
            vec![],
            cloacina::registry::loader::grants::GrantSpec::default(),
        ),
        "missing required config field must fail closed",
    );
    assert!(
        msg.contains("missing required config field") && msg.contains("suffix"),
        "missing-field error must name the missing field: {msg}"
    );
}

/// T-0833: the `@version` pin is ENFORCED against the resolved provider's
/// `provider.json` version — a matching pin loads, a mismatched pin fails
/// closed naming BOTH versions, and a segment-prefix pin ("0.1") matches
/// the 0.1.x series.
#[test]
fn version_pin_is_enforced_at_load() {
    set_provider_search_path(providers_dir());

    let unwrap_err = |r: Result<_, cloacina::registry::error::LoaderError>, what: &str| match r {
        Ok(_) => panic!("{what}"),
        Err(e) => e.to_string(),
    };
    let node = |from: &str| {
        load_constructor_node(
            "wrap",
            from,
            "affix",
            vec![
                ("prefix".to_string(), json!("hello, ")),
                ("suffix".to_string(), json!("!")),
            ],
            vec![],
            cloacina::registry::loader::grants::GrantSpec::default(),
        )
    };

    // Exact pin and segment-prefix pin both load (the fixture is 0.1.0).
    assert!(node("affix@0.1.0").is_ok(), "exact pin must load");
    assert!(node("affix@0.1").is_ok(), "segment-prefix pin must load");
    assert!(node("affix").is_ok(), "unpinned ref must load");

    // A mismatched pin fails closed, naming both versions.
    let msg = unwrap_err(node("affix@9.9.9"), "mismatched pin must fail closed");
    assert!(
        msg.contains("9.9.9") && msg.contains("0.1.0") && msg.contains("pins"),
        "pin-mismatch error must name the pin and the resolved version: {msg}"
    );

    // "0.1" is a SEGMENT prefix — it must not be satisfied by e.g. 0.10.x, and
    // conversely a "0.10" pin must not match the 0.1.x fixture.
    let msg = unwrap_err(node("affix@0.10"), "0.10 pin must not match 0.1.x");
    assert!(msg.contains("0.10"), "boundary error names the pin: {msg}");
}
