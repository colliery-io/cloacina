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
use std::time::Duration;

use serde::Serialize;
use tokio::sync::mpsc;

use cloacina::computation_graph::accumulator::{
    accumulator_runtime, accumulator_runtime_with_source, shutdown_signal, Accumulator,
    AccumulatorContext, AccumulatorError, AccumulatorRuntimeConfig, BoundarySender,
};
use cloacina::computation_graph::types::{deserialize, SourceName};
use cloacina::registry::loader::constructor_loader::{
    load_accumulator_constructor, load_stream_accumulator_source,
};
use cloacina::registry::loader::grants::ResolvedGrants;
use cloacina::registry::loader::load_task_constructor;
use cloacina::Context;

/// The stream accumulator member's `#[config]` (base + count), bound once at load.
#[derive(Serialize)]
struct CounterConfig {
    base: u64,
    count: u64,
}

/// A trivial passthrough [`Accumulator`]: each raw event byte-vector IS the
/// boundary (the provider stream already yields boundary JSON). This is what a
/// provider-supplied stream source pairs with — the runtime just forwards.
struct Passthrough;

#[async_trait::async_trait]
impl Accumulator for Passthrough {
    type Output = Vec<u8>;
    fn process(&mut self, event: Vec<u8>) -> Option<Vec<u8>> {
        Some(event)
    }
    async fn init(&mut self, _ctx: &AccumulatorContext) -> Result<(), AccumulatorError> {
        Ok(())
    }
}

/// The provider's `[package].name` — the suite the native cdylib carries.
const PROVIDER: &str = "native-task-provider-fixture";

/// The single `#[config] prefix` the `prefix` member binds once at load.
#[derive(Serialize)]
struct PrefixConfig {
    prefix: String,
}

/// The accumulator member's `#[config] threshold`, bound once at load.
#[derive(Serialize)]
struct ThresholdConfig {
    threshold: f64,
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

/// A SECOND kind through the same generic native path: the `threshold`
/// accumulator member of the same cdylib loads via the `__ProviderAccumulator`
/// holder and, driven by `accumulator_runtime`, emits a boundary only when an
/// event crosses the load-bound threshold. This proves `load_native_member` is
/// genuinely kind-generic (not task-special-cased) and is the exact shape
/// T-0904's stream accumulator builds on.
#[tokio::test]
async fn native_provider_accumulator_loads_and_ingests_in_process() {
    let (cdylib, base_manifest) = build_fixture();
    let work = tempfile::TempDir::new().unwrap();
    let search_path = stage_native_provider(work.path(), &cdylib, &base_manifest);

    let acc = load_accumulator_constructor(
        &search_path,
        PROVIDER,
        "threshold",
        &ThresholdConfig { threshold: 5.0 },
        &ResolvedGrants::default(),
    )
    .expect("load native accumulator constructor");
    assert_eq!(acc.name(), "threshold");

    let (boundary_tx, mut boundary_rx) = mpsc::channel::<(SourceName, Vec<u8>)>(10);
    let (socket_tx, socket_rx) = mpsc::channel::<Vec<u8>>(10);
    let (shutdown_tx, shutdown_rx) = shutdown_signal();

    let ctx = AccumulatorContext {
        output: BoundarySender::new(boundary_tx, SourceName::new("threshold")),
        name: "threshold".to_string(),
        shutdown: shutdown_rx,
        checkpoint: None,
        health: None,
    };
    let handle = tokio::spawn(accumulator_runtime(
        acc,
        ctx,
        socket_rx,
        AccumulatorRuntimeConfig::default(),
    ));

    // 7.0 >= threshold 5.0 → the config-bound native `ingest` emits a boundary.
    socket_tx
        .send(serde_json::to_vec(&serde_json::json!({ "value": 7.0 })).unwrap())
        .await
        .unwrap();

    let (name, bytes) = tokio::time::timeout(Duration::from_secs(5), boundary_rx.recv())
        .await
        .expect("boundary within 5s")
        .expect("boundary channel open");
    assert_eq!(name, SourceName::new("threshold"));
    let json_bytes: Vec<u8> = deserialize(&bytes).expect("decode boundary frame");
    let boundary: serde_json::Value = serde_json::from_slice(&json_bytes).expect("boundary json");
    assert_eq!(
        boundary.get("crossed"),
        Some(&serde_json::json!(7.0)),
        "native accumulator emits {{crossed: value}} above the load-bound threshold"
    );

    shutdown_tx.send(true).unwrap();
    let _ = handle.await;
}

/// The same native accumulator BUFFERS below threshold — no boundary emitted.
#[tokio::test]
async fn native_provider_accumulator_buffers_below_threshold() {
    let (cdylib, base_manifest) = build_fixture();
    let work = tempfile::TempDir::new().unwrap();
    let search_path = stage_native_provider(work.path(), &cdylib, &base_manifest);

    let acc = load_accumulator_constructor(
        &search_path,
        PROVIDER,
        "threshold",
        &ThresholdConfig { threshold: 5.0 },
        &ResolvedGrants::default(),
    )
    .expect("load native accumulator constructor");

    let (boundary_tx, mut boundary_rx) = mpsc::channel::<(SourceName, Vec<u8>)>(10);
    let (socket_tx, socket_rx) = mpsc::channel::<Vec<u8>>(10);
    let (shutdown_tx, shutdown_rx) = shutdown_signal();

    let ctx = AccumulatorContext {
        output: BoundarySender::new(boundary_tx, SourceName::new("threshold")),
        name: "threshold".to_string(),
        shutdown: shutdown_rx,
        checkpoint: None,
        health: None,
    };
    let handle = tokio::spawn(accumulator_runtime(
        acc,
        ctx,
        socket_rx,
        AccumulatorRuntimeConfig::default(),
    ));

    // 1.0 < threshold 5.0 → native `ingest` buffers (Ok(None)); no boundary.
    socket_tx
        .send(serde_json::to_vec(&serde_json::json!({ "value": 1.0 })).unwrap())
        .await
        .unwrap();

    let got = tokio::time::timeout(Duration::from_millis(500), boundary_rx.recv()).await;
    assert!(
        got.is_err(),
        "below-threshold event must not emit a boundary, got {got:?}"
    );

    shutdown_tx.send(true).unwrap();
    let _ = handle.await;
}

/// CLOACI-T-0904 — the flagship proof: a native provider's `mode = stream`
/// accumulator (`counter`) drives boundaries in-process via fidius server-
/// streaming. `load_stream_accumulator_source` loads it native, `call_streaming`
/// starts its `source`, and the resulting `ProviderStreamSource` (an
/// `EventSource`) pushes each boundary-JSON item onto the accumulator merge
/// channel — items flow all the way to the reactor boundary channel. The finite
/// source then exhausts, tearing the producer down and ending the runtime task
/// cleanly (no leak).
#[tokio::test]
async fn native_stream_accumulator_drives_boundaries_in_process() {
    let (cdylib, base_manifest) = build_fixture();
    let work = tempfile::TempDir::new().unwrap();
    let search_path = stage_native_provider(work.path(), &cdylib, &base_manifest);

    // counter: base=100, count=3 → boundaries {"tick":100},{"tick":101},{"tick":102}.
    let source = load_stream_accumulator_source(
        &search_path,
        PROVIDER,
        "counter",
        &CounterConfig {
            base: 100,
            count: 3,
        },
    )
    .await
    .expect("load native stream accumulator source");

    let (boundary_tx, mut boundary_rx) = mpsc::channel::<(SourceName, Vec<u8>)>(10);
    let (_socket_tx, socket_rx) = mpsc::channel::<Vec<u8>>(10);
    let (shutdown_tx, shutdown_rx) = shutdown_signal();

    let ctx = AccumulatorContext {
        output: BoundarySender::new(boundary_tx, SourceName::new("counter")),
        name: "counter".to_string(),
        shutdown: shutdown_rx,
        checkpoint: None,
        health: None,
    };
    // A passthrough accumulator forwards each streamed boundary verbatim — the
    // provider-supplied source drops straight into `accumulator_runtime_with_source`
    // exactly where the hardcoded Kafka source used to sit.
    let handle = tokio::spawn(accumulator_runtime_with_source(
        Passthrough,
        ctx,
        socket_rx,
        AccumulatorRuntimeConfig::default(),
        source,
    ));

    let mut ticks = Vec::new();
    for _ in 0..3 {
        let (name, bytes) = tokio::time::timeout(Duration::from_secs(5), boundary_rx.recv())
            .await
            .expect("boundary within 5s")
            .expect("boundary channel open");
        assert_eq!(name, SourceName::new("counter"));
        // Boundary frame is `bincode(json_bytes)` (the canonical reactor shape).
        let json_bytes: Vec<u8> = deserialize(&bytes).expect("decode boundary frame");
        let b: serde_json::Value = serde_json::from_slice(&json_bytes).expect("boundary json");
        ticks.push(b.get("tick").and_then(|t| t.as_u64()).expect("tick field"));
    }
    ticks.sort();
    assert_eq!(
        ticks,
        vec![100, 101, 102],
        "native stream source drove all three boundaries in-process (config-bound base + count)"
    );

    // Teardown: the finite source has exhausted (drop → producer torn down); an
    // explicit shutdown is idempotent. The runtime task must join cleanly (no leak).
    let _ = shutdown_tx.send(true);
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("runtime task joins after the stream ends (no leaked task)")
        .expect("runtime task did not panic");
}
