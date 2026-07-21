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

//! CLOACI-T-0824 / T-0837 — the **Runtime-registration** path for loaded
//! constructors: [`load_constructor`] lands a loaded provider member in the correct
//! [`Runtime`] registry by `primitive_kind` (Trigger → `get_trigger`, Task →
//! `get_task`), and a mismatched [`ConstructorBinding`] fails closed.
//!
//! The trigger's fire/skip poll behavior itself is covered by
//! `constructor_trigger_macro_wasm`; this file focuses on the registration seam.
//! Both providers are the macro-authored suite fixtures (`trigger-constructor-macro-fixture`
//! → member `heartbeat`, `task-constructor-macro-fixture` → member `prefix`), packaged
//! via the real `package_constructor_provider` path and resolved by provider name.
//!
//! Feature-gated: only built/run with `--features constructors-wasm`.
#![cfg(feature = "constructors-wasm")]

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use cloacina::packaging::constructor_provider::{
    package_constructor_provider, ProviderPackageOptions,
};
use cloacina::registry::loader::constructor_loader::{
    load_constructor, load_trigger_constructor, ConstructorBinding, TriggerBinding,
};
use cloacina::registry::loader::{grants::ResolvedGrants, unpack_provider_archive};
use cloacina::{Context, Runtime, TaskNamespace};
use serde::Serialize;

/// Per-instance config for the `heartbeat` trigger member (mirrors its generated
/// `__HeartbeatConfig { should_fire, message }`).
#[derive(Serialize)]
struct TriggerConfig {
    should_fire: bool,
    message: String,
}

/// Per-instance config for the `prefix` task member (mirrors `__PrefixConfig { prefix }`).
#[derive(Serialize)]
struct TaskConfig {
    prefix: String,
}

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/constructor-contract")
}

/// Package a macro suite fixture into an (unsigned) provider archive and unpack it
/// into the shared providers dir.
fn stage_into(work: &tempfile::TempDir, providers: &std::path::Path, fixture: &str) {
    let archive = work.path().join(format!("{fixture}.cloacina"));
    let opts = ProviderPackageOptions {
        crate_dir: examples_dir().join(fixture),
        output: Some(archive.clone()),
        sign_key: None,
        manifest_bin: "emit_manifest".to_string(),
        runtime: cloacina_constructor_contract::ProviderRuntime::Wasm,
        release: true,
    };
    package_constructor_provider(&opts).expect("package_constructor_provider");
    unpack_provider_archive(&archive, providers, &[]).expect("unpack provider archive");
}

/// Build + stage BOTH provider fixtures into one shared dir ONCE for the whole test
/// binary (each wasm component builds only once; the dir outlives every test).
fn providers_dir() -> &'static PathBuf {
    static PROVIDERS: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();
    &PROVIDERS
        .get_or_init(|| {
            let work = tempfile::TempDir::new().unwrap();
            let providers = work.path().join("providers");
            std::fs::create_dir_all(&providers).unwrap();
            stage_into(&work, &providers, "trigger-constructor-macro-fixture");
            stage_into(&work, &providers, "task-constructor-macro-fixture");
            (work, providers)
        })
        .1
}

#[tokio::test]
async fn non_trigger_primitive_fails_closed() {
    // The `prefix` provider's member is a Task; the trigger loader must reject it.
    let result = load_trigger_constructor(
        providers_dir(),
        "prefix",
        "prefix",
        &TaskConfig { prefix: "x".into() },
        TriggerBinding::default(),
        &ResolvedGrants::deny_all(),
    );
    match result {
        Ok(_) => panic!("a Task member must not load as a Trigger"),
        Err(err) => assert!(format!("{err}").contains("not Trigger"), "got: {err}"),
    }
}

#[tokio::test]
async fn load_constructor_registers_trigger_into_runtime() {
    let runtime = Runtime::empty();
    assert!(runtime.get_trigger("heartbeat").is_none());

    load_constructor(
        &runtime,
        providers_dir(),
        "heartbeat",
        "heartbeat",
        &TriggerConfig {
            should_fire: true,
            message: "registered fire".into(),
        },
        ConstructorBinding::Trigger(TriggerBinding {
            poll_interval: Duration::from_secs(30),
            allow_concurrent: false,
            workflow_name: "wf".into(),
            cron_expression: None,
        }),
        &ResolvedGrants::deny_all(),
    )
    .expect("load_constructor (trigger)");

    // Registered in the trigger registry, NOT the task registry.
    assert_eq!(runtime.trigger_names(), vec!["heartbeat".to_string()]);
    let trigger = runtime
        .get_trigger("heartbeat")
        .expect("registered trigger");
    assert_eq!(trigger.poll_interval(), Duration::from_secs(30));

    // And it actually dispatches into the configured sandbox.
    let result = trigger.poll().await.expect("poll registered trigger");
    let ctx = result.into_context().expect("fire");
    assert_eq!(
        ctx.get("reason"),
        Some(&serde_json::json!("registered fire"))
    );
}

#[tokio::test]
async fn load_constructor_registers_task_into_runtime() {
    let runtime = Runtime::empty();
    let ns = TaskNamespace::new("public", "prefix", "ops", "prefix");
    assert!(runtime.get_task(&ns).is_none());

    load_constructor(
        &runtime,
        providers_dir(),
        "prefix",
        "prefix",
        &TaskConfig {
            prefix: "hello, ".into(),
        },
        ConstructorBinding::Task {
            namespace: ns.clone(),
        },
        &ResolvedGrants::deny_all(),
    )
    .expect("load_constructor (task)");

    // Registered in the task registry, NOT the trigger registry.
    assert!(runtime.trigger_names().is_empty());
    let task = runtime.get_task(&ns).expect("registered task");

    let mut ctx = Context::new();
    ctx.insert("name", serde_json::json!("world")).unwrap();
    let out = task.execute(ctx).await.expect("execute registered task");
    assert_eq!(out.get("result"), Some(&serde_json::json!("hello, world")));
}

#[tokio::test]
async fn load_constructor_rejects_mismatched_binding() {
    // Task member, but caller hands a Trigger binding → fail closed.
    let runtime = Runtime::empty();
    let result = load_constructor(
        &runtime,
        providers_dir(),
        "prefix",
        "prefix",
        &TaskConfig { prefix: "x".into() },
        ConstructorBinding::Trigger(TriggerBinding::default()),
        &ResolvedGrants::deny_all(),
    );
    match result {
        Ok(_) => panic!("mismatched binding must fail closed"),
        Err(err) => assert!(format!("{err}").contains("does not match"), "got: {err}"),
    }
    assert!(runtime.trigger_names().is_empty());
}
