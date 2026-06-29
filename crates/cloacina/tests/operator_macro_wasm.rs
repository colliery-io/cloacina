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

//! CLOACI-T-0826 — end-to-end: a `#[operator]`-AUTHORED WASM task operator runs
//! as a cloacina [`Task`].
//!
//! The macro counterpart to `operator_loader_wasm.rs` (T-0823, which loads the
//! HAND-WRITTEN fixture). Here the operator is authored with the `#[operator(
//! kind = task, ...)]` macro (`examples/operator-contract/task-operator-macro-fixture`):
//! the author wrote only a struct (with `#[config]` / `#[param]` fields) and an
//! `execute` body; the macro generated the fidius `TaskOperator` trait + impl +
//! `configure` + the JSON wire, plus `__operator_manifest()`.
//!
//! The proof exercises the WHOLE generated surface:
//!   1. build the fixture to a wasm32-wasip2 component (the macro's guest glue);
//!   2. materialize `operator.json` from the macro-generated `__operator_manifest()`
//!      (via the fixture's `emit_manifest` host bin — the stand-in for packaging
//!      T-0827, which the macro itself cannot do);
//!   3. load through the cloacina `operators-wasm` loader → `Arc<dyn Task>`;
//!   4. run `execute` with `Context { name: "world" }` and assert
//!      `result == "<prefix>world"`.
//!
//! Feature-gated (`operators-wasm`, which pulls wasmtime). Excluded from the
//! default build.
#![cfg(feature = "operators-wasm")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use cloacina::registry::loader::operator_loader::load_task_operator;
use cloacina::Context;
use cloacina_operator_contract::{OperatorManifest, PrimitiveKind};
use serde::Serialize;

/// Per-instance config the loader binds once at load (mirrors the fixture's
/// generated `__PrefixConfig { prefix }`). serde-compatible with the guest.
#[derive(Serialize)]
struct Config {
    prefix: String,
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/operator-contract/task-operator-macro-fixture")
}

/// Build the macro-authored fixture to a wasm component once; return its bytes.
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
            "task-operator-macro-fixture wasm build failed"
        );
        std::fs::read(fixture.join("target/wasm32-wasip2/release/task_operator_macro_fixture.wasm"))
            .expect("read built wasm component")
    })
}

/// Materialize the manifest from the macro-generated `__operator_manifest()` by
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

/// Stage the component + package.toml + the macro-generated operator manifest.
fn stage(root: &Path) {
    let dir = root.join("task-operator-macro-pkg");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("task_operator_macro_fixture.wasm"), component()).unwrap();
    std::fs::write(
        dir.join("package.toml"),
        "[package]\nname = \"task-operator-macro-pkg\"\nversion = \"0.1.0\"\n\
         interface = \"task-operator\"\ninterface_version = 1\nruntime = \"wasm\"\n\n\
         [metadata]\ncategory = \"operator\"\n\n\
         [wasm]\ncomponent = \"task_operator_macro_fixture.wasm\"\n",
    )
    .unwrap();
    // The sidecar is the macro-generated manifest verbatim.
    std::fs::write(dir.join("operator.json"), macro_manifest_json()).unwrap();
}

#[tokio::test]
async fn macro_authored_manifest_carries_the_declared_surface() {
    // The macro-generated `__operator_manifest()` describes the operator: a Task
    // named "prefix", interface v1, with the single required `name` param the
    // author declared via `#[param(required)]`.
    let manifest = OperatorManifest::from_json(macro_manifest_json())
        .expect("macro manifest parses against the real contract crate");

    assert_eq!(manifest.name, "prefix");
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!(manifest.primitive_kind, PrimitiveKind::Task);
    assert_eq!(manifest.interface, "task-operator");
    assert_eq!(manifest.interface_version, 1);
    assert_eq!(manifest.params.len(), 1);
    assert_eq!(manifest.params[0].name, "name");
    assert!(manifest.params[0].required);
    assert_eq!(
        manifest.params[0].schema,
        serde_json::json!({"type": "string"}),
        "the #[param] String field derives a JSON-Schema string slot"
    );
}

#[tokio::test]
async fn macro_authored_wasm_task_operator_runs_as_cloacina_task() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());

    // Load the macro-authored operator through the cloacina loader → a Task.
    let task = load_task_operator(
        tmp.path(),
        "task-operator-macro-pkg",
        &Config {
            prefix: "hello, ".into(),
        },
    )
    .expect("load_task_operator");

    assert_eq!(task.id(), "prefix");
    assert!(task.dependencies().is_empty());

    // Run it as a Task with a real Context.
    let mut ctx = Context::new();
    ctx.insert("name", serde_json::json!("world")).unwrap();

    let out = task.execute(ctx).await.expect("operator task execute");

    assert_eq!(
        out.get("result"),
        Some(&serde_json::json!("hello, world")),
        "the macro-generated execute writes result = prefix + name"
    );
    // Original context key survives the boundary.
    assert_eq!(out.get("name"), Some(&serde_json::json!("world")));
}

#[tokio::test]
async fn macro_config_binds_at_load_so_instances_differ() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());

    let hello = load_task_operator(
        tmp.path(),
        "task-operator-macro-pkg",
        &Config {
            prefix: "hello, ".into(),
        },
    )
    .expect("load hello operator");

    let goodbye = load_task_operator(
        tmp.path(),
        "task-operator-macro-pkg",
        &Config {
            prefix: "goodbye, ".into(),
        },
    )
    .expect("load goodbye operator");

    let mk = || {
        let mut c = Context::new();
        c.insert("name", serde_json::json!("world")).unwrap();
        c
    };

    let hello_out = hello.execute(mk()).await.unwrap();
    let goodbye_out = goodbye.execute(mk()).await.unwrap();

    assert_eq!(
        hello_out.get("result"),
        Some(&serde_json::json!("hello, world"))
    );
    assert_eq!(
        goodbye_out.get("result"),
        Some(&serde_json::json!("goodbye, world")),
    );
    assert_ne!(hello_out.get("result"), goodbye_out.get("result"));
}

#[tokio::test]
async fn macro_missing_required_param_fails_closed() {
    // The generated glue pulls `#[param(required)] name` from the context; a
    // context without it must surface as a task error, not a panic.
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());

    let task = load_task_operator(
        tmp.path(),
        "task-operator-macro-pkg",
        &Config { prefix: "x".into() },
    )
    .expect("load operator");

    let ctx = Context::new(); // no `name`
    let err = task
        .execute(ctx)
        .await
        .expect_err("missing required param should fail");
    assert!(
        format!("{err}").contains("name"),
        "error should name the missing required param, got: {err}"
    );
}
