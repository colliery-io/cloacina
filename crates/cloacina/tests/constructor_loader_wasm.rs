/*
 *  Copyright 2025-2026 Colliery Software
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

//! CLOACI-T-0823 — end-to-end: a WASM **task constructor** runs as a cloacina
//! [`Task`].
//!
//! Builds the proven task-constructor fixture (a wasm32-wasip2 component), stages
//! it as an constructor package (`.wasm` + `package.toml` + sidecar
//! `constructor.json`), loads it through the cloacina `constructors-wasm` loader to
//! get a `Arc<dyn Task>`, runs `execute` with a `Context { name: "world" }`, and
//! asserts the resulting context has `result == "hello, world"`. A second load
//! with a different prefix proves config-binding (the two constructors differ).
//!
//! Feature-gated: only built/run with `--features constructors-wasm` (which pulls
//! wasmtime). The default cloacina build excludes this entirely.
#![cfg(feature = "constructors-wasm")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use cloacina::registry::loader::constructor_loader::load_task_constructor;
use cloacina::Context;
use cloacina_constructor_contract::{ConstructorManifest, InputSlot, PrimitiveKind};
use serde::Serialize;

/// Per-instance config the loader binds once at load (mirrors the fixture's
/// `Config { prefix }`). bincode-compatible with the guest struct.
#[derive(Serialize)]
struct Config {
    prefix: String,
}

/// The manifest the `#[constructor]` macro (CLOACI-T-0826) WOULD emit for the
/// fixture; constructed by hand here. Uses the REAL contract crate's types
/// (canonical `InputSlot` reused, not vendored).
fn fixture_manifest() -> ConstructorManifest {
    ConstructorManifest {
        name: "greet".into(),
        version: "0.1.0".into(),
        primitive_kind: PrimitiveKind::Task,
        interface: "task-constructor".into(),
        interface_version: 1,
        params: vec![InputSlot::required(
            "name",
            serde_json::json!({"type": "string"}),
        )],
        config_fields: vec![],
        dependencies: vec![],
        description: Some("Prefixes the context `name` into `result`.".into()),
        author: Some("CLOACI-T-0823".into()),
    }
}

/// Build the constructor fixture to a wasm component once; return its bytes. Reuses
/// the proven fixture under `examples/constructor-contract/task-constructor-fixture`.
fn component() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/constructor-contract/task-constructor-fixture");
        let status = Command::new("cargo")
            .args(["build", "--target", "wasm32-wasip2", "--release"])
            .current_dir(&fixture)
            .status()
            .expect("spawn cargo build --target wasm32-wasip2");
        assert!(
            status.success(),
            "task-constructor-fixture wasm build failed"
        );
        std::fs::read(fixture.join("target/wasm32-wasip2/release/task_constructor_fixture.wasm"))
            .expect("read built wasm component")
    })
}

/// Stage the component + package.toml + the constructor manifest sidecar.
fn stage(root: &Path) {
    let dir = root.join("task-constructor-pkg");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("task_constructor_fixture.wasm"), component()).unwrap();
    std::fs::write(
        dir.join("package.toml"),
        "[package]\nname = \"task-constructor-pkg\"\nversion = \"0.1.0\"\n\
         interface = \"task-constructor\"\ninterface_version = 1\nruntime = \"wasm\"\n\n\
         [metadata]\ncategory = \"constructor\"\n\n\
         [wasm]\ncomponent = \"task_constructor_fixture.wasm\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("constructor.json"),
        fixture_manifest().to_json().unwrap(),
    )
    .unwrap();
}

#[tokio::test]
async fn wasm_task_constructor_runs_as_cloacina_task() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());

    // Load the constructor through the cloacina loader → a runnable Task.
    let task = load_task_constructor(
        tmp.path(),
        "task-constructor-pkg",
        &Config {
            prefix: "hello, ".into(),
        },
        &cloacina::registry::loader::grants::ResolvedGrants::deny_all(),
    )
    .expect("load_task_constructor");

    assert_eq!(task.id(), "greet");
    assert!(task.dependencies().is_empty());

    // Run it as a Task with a real Context.
    let mut ctx = Context::new();
    ctx.insert("name", serde_json::json!("world")).unwrap();

    let out = task.execute(ctx).await.expect("constructor task execute");

    assert_eq!(
        out.get("result"),
        Some(&serde_json::json!("hello, world")),
        "constructor should write result = prefix + name"
    );
    // Original context key is preserved across the boundary.
    assert_eq!(out.get("name"), Some(&serde_json::json!("world")));
}

#[tokio::test]
async fn config_binds_at_load_so_instances_differ() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());

    let hello = load_task_constructor(
        tmp.path(),
        "task-constructor-pkg",
        &Config {
            prefix: "hello, ".into(),
        },
        &cloacina::registry::loader::grants::ResolvedGrants::deny_all(),
    )
    .expect("load hello constructor");

    let goodbye = load_task_constructor(
        tmp.path(),
        "task-constructor-pkg",
        &Config {
            prefix: "goodbye, ".into(),
        },
        &cloacina::registry::loader::grants::ResolvedGrants::deny_all(),
    )
    .expect("load goodbye constructor");

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
async fn non_task_primitive_fails_closed() {
    // A manifest that claims a non-Task primitive must be rejected by the
    // Task loader (trigger/accumulator/reactor loading is CLOACI-T-0824).
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());
    // Overwrite the sidecar with a non-Task primitive_kind.
    let mut manifest = fixture_manifest();
    manifest.primitive_kind = PrimitiveKind::Trigger;
    std::fs::write(
        tmp.path()
            .join("task-constructor-pkg")
            .join("constructor.json"),
        manifest.to_json().unwrap(),
    )
    .unwrap();

    let result = load_task_constructor(
        tmp.path(),
        "task-constructor-pkg",
        &Config { prefix: "x".into() },
        &cloacina::registry::loader::grants::ResolvedGrants::deny_all(),
    );
    match result {
        Ok(_) => panic!("non-Task primitive should fail closed"),
        Err(err) => assert!(format!("{err}").contains("not Task"), "got: {err}"),
    }
}
