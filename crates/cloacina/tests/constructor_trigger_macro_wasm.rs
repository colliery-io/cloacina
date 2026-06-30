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

//! CLOACI-T-0829 — end-to-end: a `#[constructor(kind = trigger)]`-AUTHORED WASM
//! trigger constructor compiles, loads, and polls as a cloacina [`Trigger`].
//!
//! The macro counterpart to `constructor_trigger_wasm.rs` (T-0824, which loads the
//! HAND-WRITTEN trigger fixture). Here the trigger is authored with the
//! `#[constructor(kind = trigger, ...)]` macro
//! (`examples/constructor-contract/trigger-constructor-macro-fixture`): the author
//! wrote only a struct (with `#[config]` fields) and a `poll` body returning a fire
//! decision; the macro generated the fidius `TriggerConstructor` trait + impl +
//! `configure` + the JSON wire, plus `__constructor_manifest()`.
//!
//! The proof exercises the WHOLE generated trigger surface:
//!   1. build the fixture to a wasm32-wasip2 component (the macro's guest glue);
//!   2. materialize `constructor.json` from the macro-generated `__constructor_manifest()`
//!      (via the fixture's `emit_manifest` host bin — the T-0827 packaging stand-in);
//!   3. load through the cloacina `constructors-wasm` loader → `Arc<dyn Trigger>`;
//!   4. `poll()` and assert Fire/Skip per the config bound at load.
//!
//! Feature-gated (`constructors-wasm`, which pulls wasmtime). Excluded from the
//! default build.
#![cfg(feature = "constructors-wasm")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use cloacina::registry::loader::constructor_loader::{load_trigger_constructor, TriggerBinding};
use cloacina::trigger::TriggerResult;
use cloacina_constructor_contract::{ConstructorManifest, PrimitiveKind};
use serde::Serialize;

/// Per-instance config the loader binds once at load (mirrors the fixture's
/// generated `__HeartbeatConfig { should_fire, message }`).
#[derive(Serialize)]
struct Config {
    should_fire: bool,
    message: String,
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/constructor-contract/trigger-constructor-macro-fixture")
}

/// Build the macro-authored trigger fixture to a wasm component once; return bytes.
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
            "trigger-constructor-macro-fixture wasm build failed"
        );
        std::fs::read(
            fixture.join("target/wasm32-wasip2/release/trigger_constructor_macro_fixture.wasm"),
        )
        .expect("read built wasm component")
    })
}

/// Materialize the manifest from the macro-generated `__constructor_manifest()` by
/// running the fixture's host `emit_manifest` bin (the T-0827 packaging stand-in).
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

/// Stage the component + package.toml + the macro-generated constructor manifest.
fn stage(root: &Path) {
    let dir = root.join("trigger-constructor-macro-pkg");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("trigger_constructor_macro_fixture.wasm"),
        component(),
    )
    .unwrap();
    std::fs::write(
        dir.join("package.toml"),
        "[package]\nname = \"trigger-constructor-macro-pkg\"\nversion = \"0.1.0\"\n\
         interface = \"trigger-constructor\"\ninterface_version = 1\nruntime = \"wasm\"\n\n\
         [metadata]\ncategory = \"constructor\"\n\n\
         [wasm]\ncomponent = \"trigger_constructor_macro_fixture.wasm\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("constructor.json"), macro_manifest_json()).unwrap();
}

#[tokio::test]
async fn macro_authored_trigger_manifest_carries_the_declared_surface() {
    // The macro-generated `__constructor_manifest()` describes the trigger: a
    // Trigger named "heartbeat", interface v1, no params (a trigger's poll has no
    // task-context inputs).
    let manifest = ConstructorManifest::from_json(macro_manifest_json())
        .expect("macro manifest parses against the real contract crate");

    assert_eq!(manifest.name, "heartbeat");
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!(manifest.primitive_kind, PrimitiveKind::Trigger);
    assert_eq!(manifest.interface, "trigger-constructor");
    assert_eq!(manifest.interface_version, 1);
    assert!(manifest.params.is_empty());
}

#[tokio::test]
async fn macro_authored_wasm_trigger_constructor_fires_when_configured() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());

    let trigger = load_trigger_constructor(
        tmp.path(),
        "trigger-constructor-macro-pkg",
        &Config {
            should_fire: true,
            message: "boundary crossed".into(),
        },
        TriggerBinding {
            poll_interval: Duration::from_secs(5),
            allow_concurrent: true,
            workflow_name: "my_workflow".into(),
            cron_expression: None,
        },
        &cloacina::registry::loader::grants::ResolvedGrants::deny_all(),
    )
    .expect("load_trigger_constructor");

    assert_eq!(trigger.name(), "heartbeat");
    assert_eq!(trigger.poll_interval(), Duration::from_secs(5));

    let result = trigger.poll().await.expect("poll");
    assert!(result.should_fire(), "config should_fire=true → Fire");
    let ctx = result.into_context().expect("fire carries a context");
    assert_eq!(
        ctx.get("reason"),
        Some(&serde_json::json!("boundary crossed")),
        "the macro-generated poll `set`s the fire context's `reason`"
    );
}

#[tokio::test]
async fn macro_authored_wasm_trigger_constructor_skips_when_configured_off() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());

    let trigger = load_trigger_constructor(
        tmp.path(),
        "trigger-constructor-macro-pkg",
        &Config {
            should_fire: false,
            message: "unused".into(),
        },
        TriggerBinding::default(),
        &cloacina::registry::loader::grants::ResolvedGrants::deny_all(),
    )
    .expect("load_trigger_constructor");

    let result = trigger.poll().await.expect("poll");
    assert!(
        matches!(result, TriggerResult::Skip),
        "config should_fire=false → Skip, got {result:?}"
    );
}
