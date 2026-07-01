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

//! CLOACI-T-0828 — end-to-end: a `#[constructor(kind = reactor)]`-AUTHORED WASM
//! reactor constructor drives a [`Reactor`]'s firing decision.
//!
//! Proves:
//!   1. build the macro fixture to a wasm32-wasip2 component + materialize its
//!      `constructor.json` from `__constructor_manifest()`;
//!   2. load through the cloacina `constructors-wasm` loader → `WasmReactorConstructor`;
//!   3. the bridge: `WasmReactorConstructor::evaluate(boundaries_json)` returns
//!      the config-bound fire/hold decision across the sandbox;
//!   4. the scheduler seam: a live `Reactor::with_evaluator(<loaded>)` consults the
//!      WASM guest for its firing decision and fires the graph only when the guest
//!      says so.
//!
//! Feature-gated (`constructors-wasm`, which pulls wasmtime). Excluded from the
//! default build.
#![cfg(feature = "constructors-wasm")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use tokio::sync::{mpsc, watch};

use cloacina::computation_graph::reactor::{
    CompiledGraphFn, InputStrategy, ManualCommand, ReactionCriteria, Reactor, ReactorFireDecider,
};
use cloacina::computation_graph::types::{GraphResult, InputCache, SourceName};
use cloacina::registry::loader::constructor_loader::load_reactor_constructor;
use cloacina_constructor_contract::{PrimitiveKind, ProviderManifest};
use serde::Serialize;

/// Per-instance config the loader binds once at load (mirrors the fixture's
/// generated `__GateConfig { gate }`).
#[derive(Serialize)]
struct Config {
    gate: f64,
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/constructor-contract/reactor-constructor-fixture")
}

fn component() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| {
        let fixture = fixture_dir();
        let status = Command::new("cargo")
            .args(["build", "--lib", "--target", "wasm32-wasip2", "--release"])
            .current_dir(&fixture)
            .status()
            .expect("spawn cargo build --target wasm32-wasip2");
        assert!(
            status.success(),
            "reactor-constructor-fixture wasm build failed"
        );
        std::fs::read(fixture.join("target/wasm32-wasip2/release/reactor_constructor_fixture.wasm"))
            .expect("read built wasm component")
    })
}

fn macro_manifest_json() -> &'static str {
    static JSON: OnceLock<String> = OnceLock::new();
    JSON.get_or_init(|| {
        let out = Command::new("cargo")
            .args(["run", "--quiet", "--bin", "emit_manifest"])
            .current_dir(fixture_dir())
            .output()
            .expect("spawn cargo run --bin emit_manifest");
        assert!(
            out.status.success(),
            "emit_manifest failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).expect("emit_manifest stdout is UTF-8")
    })
}

fn stage(root: &Path) {
    let dir = root.join("reactor-constructor-pkg");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("reactor_constructor_fixture.wasm"), component()).unwrap();
    std::fs::write(
        dir.join("package.toml"),
        "[package]\nname = \"reactor-constructor-pkg\"\nversion = \"0.1.0\"\n\
         interface = \"reactor-constructor\"\ninterface_version = 1\nruntime = \"wasm\"\n\n\
         [metadata]\ncategory = \"constructor\"\n\n\
         [wasm]\ncomponent = \"reactor_constructor_fixture.wasm\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("provider.json"), macro_manifest_json()).unwrap();
}

#[tokio::test]
async fn macro_authored_manifest_is_a_reactor() {
    let provider = ProviderManifest::from_json(macro_manifest_json())
        .expect("macro provider manifest parses against the real contract crate");
    let manifest = provider
        .constructor("gate")
        .expect("suite carries the  member");
    assert_eq!(manifest.name, "gate");
    assert_eq!(manifest.primitive_kind, PrimitiveKind::Reactor);
    assert_eq!(manifest.interface, "reactor-constructor");
    assert_eq!(manifest.interface_version, 1);
    assert!(manifest.params.is_empty());
}

#[tokio::test]
async fn wasm_reactor_evaluate_bridge_is_config_bound() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());

    // gate = 5.0 → a held boundary value of 10.0 crosses → fire.
    let firing = load_reactor_constructor(
        tmp.path(),
        "reactor-constructor-pkg",
        "gate",
        &Config { gate: 5.0 },
        &cloacina::registry::loader::grants::ResolvedGrants::deny_all(),
    )
    .expect("load_reactor_constructor (firing)");
    let boundaries = serde_json::json!({ "x": { "value": 10.0 } }).to_string();
    let outcome = firing.evaluate(boundaries.clone()).await.expect("evaluate");
    assert!(outcome.fire, "10.0 >= gate 5.0 → fire");

    // gate = 50.0 → same boundary holds.
    let holding = load_reactor_constructor(
        tmp.path(),
        "reactor-constructor-pkg",
        "gate",
        &Config { gate: 50.0 },
        &cloacina::registry::loader::grants::ResolvedGrants::deny_all(),
    )
    .expect("load_reactor_constructor (holding)");
    let outcome = holding.evaluate(boundaries).await.expect("evaluate");
    assert!(!outcome.fire, "10.0 < gate 50.0 → hold");
}

#[tokio::test]
async fn non_reactor_primitive_fails_closed() {
    let tmp = tempfile::TempDir::new().unwrap();
    let err = load_reactor_constructor(
        tmp.path(),
        "missing-reactor-pkg",
        "gate",
        &Config { gate: 1.0 },
        &cloacina::registry::loader::grants::ResolvedGrants::deny_all(),
    )
    .expect_err("loading a missing package must fail closed");
    assert!(
        format!("{err}").contains("missing-reactor-pkg"),
        "error should name the missing package, got: {err}"
    );
}

#[tokio::test]
async fn reactor_with_wasm_evaluator_fires_when_guest_says_so() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());

    // gate = 5.0: the WASM guest fires when a held boundary crosses 5.0.
    let evaluator = load_reactor_constructor(
        tmp.path(),
        "reactor-constructor-pkg",
        "gate",
        &Config { gate: 5.0 },
        &cloacina::registry::loader::grants::ResolvedGrants::deny_all(),
    )
    .expect("load_reactor_constructor");

    let (acc_tx, acc_rx) = mpsc::channel::<(SourceName, Vec<u8>)>(10);
    let (_manual_tx, manual_rx) = mpsc::channel::<ManualCommand>(10);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let fire_count = Arc::new(AtomicU32::new(0));
    let fire_count_inner = fire_count.clone();
    let graph: CompiledGraphFn = Arc::new(move |_cache: InputCache| {
        let fc = fire_count_inner.clone();
        Box::pin(async move {
            fc.fetch_add(1, Ordering::SeqCst);
            GraphResult::completed(vec![])
        })
    });

    let reactor = Reactor::new(
        graph,
        ReactionCriteria::WhenAny, // ignored — the evaluator replaces the criteria.
        InputStrategy::Latest,
        acc_rx,
        manual_rx,
        shutdown_rx,
    )
    .with_evaluator(Arc::new(evaluator) as Arc<dyn ReactorFireDecider>);

    let handle = tokio::spawn(reactor.run());

    // Below the gate (1.0) → the guest holds → no fire.
    acc_tx
        .send((
            SourceName::new("x"),
            serde_json::to_vec(&serde_json::json!({ "value": 1.0 })).unwrap(),
        ))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        fire_count.load(Ordering::SeqCst),
        0,
        "below-gate boundary must not fire the graph"
    );

    // Above the gate (9.0) → the guest fires → graph runs.
    acc_tx
        .send((
            SourceName::new("x"),
            serde_json::to_vec(&serde_json::json!({ "value": 9.0 })).unwrap(),
        ))
        .await
        .unwrap();

    let mut fired = false;
    for _ in 0..50 {
        if fire_count.load(Ordering::SeqCst) >= 1 {
            fired = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        fired,
        "above-gate boundary should fire the graph via the WASM evaluator"
    );

    shutdown_tx.send(true).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
}
