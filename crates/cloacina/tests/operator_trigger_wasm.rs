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

//! CLOACI-T-0824 — end-to-end: a WASM **trigger operator** runs as a cloacina
//! [`Trigger`], and a loaded operator registers into the correct [`Runtime`]
//! registry by `primitive_kind`.
//!
//! Builds the trigger-operator fixture (a wasm32-wasip2 component), stages it as
//! an operator package (`.wasm` + `package.toml` + sidecar `operator.json`),
//! loads it through the cloacina `operators-wasm` loader to get an
//! `Arc<dyn Trigger>`, and asserts `poll()` returns `Fire`/`Skip` per the
//! config bound at load. Then proves the Runtime-registration path: loading a
//! trigger operator lands it in `Runtime::get_trigger`, and loading the task
//! operator lands it in `Runtime::get_task` — each keyed by `primitive_kind`.
//!
//! Feature-gated: only built/run with `--features operators-wasm`.
#![cfg(feature = "operators-wasm")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use cloacina::registry::loader::operator_loader::{
    load_operator, load_trigger_operator, OperatorBinding, TriggerBinding,
};
use cloacina::trigger::TriggerResult;
use cloacina::{Context, Runtime, TaskNamespace};
use cloacina_operator_contract::{InputSlot, OperatorManifest, PrimitiveKind};
use serde::Serialize;

/// Per-instance config the loader binds once at load (mirrors the trigger
/// fixture's `Config { should_fire, message }`).
#[derive(Serialize)]
struct TriggerConfig {
    should_fire: bool,
    message: String,
}

/// Config for the task fixture (mirrors `task-operator-fixture::Config`).
#[derive(Serialize)]
struct TaskConfig {
    prefix: String,
}

fn trigger_manifest() -> OperatorManifest {
    OperatorManifest {
        name: "tick".into(),
        version: "0.1.0".into(),
        primitive_kind: PrimitiveKind::Trigger,
        interface: "trigger-operator".into(),
        interface_version: 1,
        params: vec![InputSlot::optional(
            "should_fire",
            serde_json::json!({"type": "boolean"}),
            Some(serde_json::json!(false)),
        )],
        dependencies: vec![],
        description: Some("Fires (or skips) on a config flag.".into()),
        author: Some("CLOACI-T-0824".into()),
    }
}

fn task_manifest() -> OperatorManifest {
    OperatorManifest {
        name: "greet".into(),
        version: "0.1.0".into(),
        primitive_kind: PrimitiveKind::Task,
        interface: "task-operator".into(),
        interface_version: 1,
        params: vec![InputSlot::required(
            "name",
            serde_json::json!({"type": "string"}),
        )],
        dependencies: vec![],
        description: Some("Prefixes the context `name` into `result`.".into()),
        author: Some("CLOACI-T-0824".into()),
    }
}

/// Build a fixture crate under `examples/operator-contract/<dir>` to a wasm
/// component once; return its bytes.
fn build_component(dir_name: &'static str, wasm_file: &'static str) -> Vec<u8> {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/operator-contract")
        .join(dir_name);
    let status = Command::new("cargo")
        .args(["build", "--target", "wasm32-wasip2", "--release"])
        .current_dir(&fixture)
        .status()
        .expect("spawn cargo build --target wasm32-wasip2");
    assert!(status.success(), "{dir_name} wasm build failed");
    std::fs::read(
        fixture
            .join("target/wasm32-wasip2/release")
            .join(wasm_file),
    )
    .expect("read built wasm component")
}

fn trigger_component() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| {
        build_component("trigger-operator-fixture", "trigger_operator_fixture.wasm")
    })
}

fn task_component() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| build_component("task-operator-fixture", "task_operator_fixture.wasm"))
}

/// Stage a wasm operator package: component + package.toml + operator.json.
fn stage(
    root: &Path,
    pkg: &str,
    wasm_file: &str,
    bytes: &[u8],
    interface: &str,
    manifest: &OperatorManifest,
) {
    let dir = root.join(pkg);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(wasm_file), bytes).unwrap();
    std::fs::write(
        dir.join("package.toml"),
        format!(
            "[package]\nname = \"{pkg}\"\nversion = \"0.1.0\"\n\
             interface = \"{interface}\"\ninterface_version = 1\nruntime = \"wasm\"\n\n\
             [metadata]\ncategory = \"operator\"\n\n\
             [wasm]\ncomponent = \"{wasm_file}\"\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("operator.json"), manifest.to_json().unwrap()).unwrap();
}

fn stage_trigger(root: &Path) {
    stage(
        root,
        "trigger-operator-pkg",
        "trigger_operator_fixture.wasm",
        trigger_component(),
        "trigger-operator",
        &trigger_manifest(),
    );
}

fn stage_task(root: &Path) {
    stage(
        root,
        "task-operator-pkg",
        "task_operator_fixture.wasm",
        task_component(),
        "task-operator",
        &task_manifest(),
    );
}

#[tokio::test]
async fn wasm_trigger_operator_fires_when_configured() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage_trigger(tmp.path());

    let trigger = load_trigger_operator(
        tmp.path(),
        "trigger-operator-pkg",
        &TriggerConfig {
            should_fire: true,
            message: "boundary crossed".into(),
        },
        TriggerBinding {
            poll_interval: Duration::from_secs(5),
            allow_concurrent: true,
            workflow_name: "my_workflow".into(),
            cron_expression: None,
        },
    )
    .expect("load_trigger_operator");

    assert_eq!(trigger.name(), "tick");
    assert_eq!(trigger.poll_interval(), Duration::from_secs(5));
    assert!(trigger.allow_concurrent());
    assert_eq!(trigger.workflow_name(), "my_workflow");
    assert!(trigger.cron_expression().is_none());

    let result = trigger.poll().await.expect("poll");
    assert!(result.should_fire(), "config should_fire=true → Fire");
    let ctx = result.into_context().expect("fire carries a context");
    assert_eq!(ctx.get("reason"), Some(&serde_json::json!("boundary crossed")));
}

#[tokio::test]
async fn wasm_trigger_operator_skips_when_configured_off() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage_trigger(tmp.path());

    let trigger = load_trigger_operator(
        tmp.path(),
        "trigger-operator-pkg",
        &TriggerConfig {
            should_fire: false,
            message: "unused".into(),
        },
        TriggerBinding::default(),
    )
    .expect("load_trigger_operator");

    let result = trigger.poll().await.expect("poll");
    assert!(
        !result.should_fire(),
        "config should_fire=false → Skip, got {result:?}"
    );
    assert!(matches!(result, TriggerResult::Skip));
}

#[tokio::test]
async fn non_trigger_primitive_fails_closed() {
    // The task package's manifest claims primitive Task; the trigger loader
    // must reject it.
    let tmp = tempfile::TempDir::new().unwrap();
    stage_task(tmp.path());

    let result = load_trigger_operator(
        tmp.path(),
        "task-operator-pkg",
        &TaskConfig {
            prefix: "x".into(),
        },
        TriggerBinding::default(),
    );
    match result {
        Ok(_) => panic!("a Task package must not load as a Trigger"),
        Err(err) => assert!(format!("{err}").contains("not Trigger"), "got: {err}"),
    }
}

#[tokio::test]
async fn load_operator_registers_trigger_into_runtime() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage_trigger(tmp.path());

    let runtime = Runtime::empty();
    assert!(runtime.get_trigger("tick").is_none());

    load_operator(
        &runtime,
        tmp.path(),
        "trigger-operator-pkg",
        &TriggerConfig {
            should_fire: true,
            message: "registered fire".into(),
        },
        OperatorBinding::Trigger(TriggerBinding {
            poll_interval: Duration::from_secs(30),
            allow_concurrent: false,
            workflow_name: "wf".into(),
            cron_expression: None,
        }),
    )
    .expect("load_operator (trigger)");

    // Registered in the trigger registry, NOT the task registry.
    assert_eq!(runtime.trigger_names(), vec!["tick".to_string()]);
    let trigger = runtime.get_trigger("tick").expect("registered trigger");
    assert_eq!(trigger.poll_interval(), Duration::from_secs(30));

    // And it actually dispatches into the configured sandbox.
    let result = trigger.poll().await.expect("poll registered trigger");
    let ctx = result.into_context().expect("fire");
    assert_eq!(ctx.get("reason"), Some(&serde_json::json!("registered fire")));
}

#[tokio::test]
async fn load_operator_registers_task_into_runtime() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage_task(tmp.path());

    let runtime = Runtime::empty();
    let ns = TaskNamespace::new("public", "task-operator-pkg", "ops", "greet");
    assert!(runtime.get_task(&ns).is_none());

    load_operator(
        &runtime,
        tmp.path(),
        "task-operator-pkg",
        &TaskConfig {
            prefix: "hello, ".into(),
        },
        OperatorBinding::Task {
            namespace: ns.clone(),
        },
    )
    .expect("load_operator (task)");

    // Registered in the task registry, NOT the trigger registry.
    assert!(runtime.trigger_names().is_empty());
    let task = runtime.get_task(&ns).expect("registered task");

    let mut ctx = Context::new();
    ctx.insert("name", serde_json::json!("world")).unwrap();
    let out = task.execute(ctx).await.expect("execute registered task");
    assert_eq!(out.get("result"), Some(&serde_json::json!("hello, world")));
}

#[tokio::test]
async fn load_operator_rejects_mismatched_binding() {
    // Task package, but caller hands a Trigger binding → fail closed.
    let tmp = tempfile::TempDir::new().unwrap();
    stage_task(tmp.path());

    let runtime = Runtime::empty();
    let result = load_operator(
        &runtime,
        tmp.path(),
        "task-operator-pkg",
        &TaskConfig {
            prefix: "x".into(),
        },
        OperatorBinding::Trigger(TriggerBinding::default()),
    );
    match result {
        Ok(_) => panic!("mismatched binding must fail closed"),
        Err(err) => assert!(
            format!("{err}").contains("does not match"),
            "got: {err}"
        ),
    }
    assert!(runtime.trigger_names().is_empty());
}
