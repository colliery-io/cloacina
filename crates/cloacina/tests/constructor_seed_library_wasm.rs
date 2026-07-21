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

//! CLOACI-T-0825 — the SEED PROVIDER LIBRARY, one constructor per primitive,
//! each packaged via the real `package_constructor_provider` path and loaded +
//! invoked through the real kind loaders:
//!
//! - task: `cloacina-provider-fs` / `read_file`+`write_file` (covered end-to-end
//!   by `constructor_provider_package_wasm` — not repeated here)
//! - trigger: `cloacina-provider-sensor` / `file_present` (grant-gated file sensor)
//! - accumulator: `cloacina-provider-extract` / `extract` (field projection)
//! - reactor: `cloacina-provider-quorum` / `quorum` (N-of-M firing criteria)
//!
//! Feature-gated: only built/run with `--features constructors-wasm`.
#![cfg(feature = "constructors-wasm")]

use std::path::PathBuf;
use std::sync::OnceLock;

use cloacina::computation_graph::accumulator::Accumulator;
use cloacina::packaging::constructor_provider::{
    package_constructor_provider, ProviderPackageOptions,
};
use cloacina::registry::loader::constructor_loader::{
    load_accumulator_constructor, load_reactor_constructor, load_trigger_constructor,
    TriggerBinding,
};
use cloacina::registry::loader::grants::{translate, GrantSpec, ResolvedGrants};
use cloacina::registry::loader::unpack_provider_archive;
use cloacina::trigger::TriggerResult;
use serde::Serialize;

#[derive(Serialize)]
struct SensorConfig {
    path: String,
}

#[derive(Serialize)]
struct ExtractConfig {
    field: String,
}

#[derive(Serialize)]
struct QuorumConfig {
    required: i64,
}

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/constructor-contract")
}

fn stage_into(work: &tempfile::TempDir, providers: &std::path::Path, crate_name: &str) {
    let archive = work.path().join(format!("{crate_name}.cloacina"));
    let opts = ProviderPackageOptions {
        crate_dir: examples_dir().join(crate_name),
        output: Some(archive.clone()),
        sign_key: None,
        manifest_bin: "emit_manifest".to_string(),
        runtime: cloacina_constructor_contract::ProviderRuntime::Wasm,
        release: true,
    };
    package_constructor_provider(&opts).expect("package_constructor_provider");
    unpack_provider_archive(&archive, providers, &[]).expect("unpack provider archive");
}

/// Build + stage the three seed providers ONCE for the whole test binary.
fn providers_dir() -> &'static PathBuf {
    static PROVIDERS: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();
    &PROVIDERS
        .get_or_init(|| {
            let work = tempfile::TempDir::new().unwrap();
            let providers = work.path().join("providers");
            std::fs::create_dir_all(&providers).unwrap();
            stage_into(&work, &providers, "cloacina-provider-sensor");
            stage_into(&work, &providers, "cloacina-provider-extract");
            stage_into(&work, &providers, "cloacina-provider-quorum");
            (work, providers)
        })
        .1
}

/// The file sensor fires when the granted path exists, skips when it doesn't,
/// and — default-closed — never fires without an fs grant even when the host
/// file DOES exist.
#[tokio::test]
async fn file_present_trigger_is_grant_gated() {
    let watched = tempfile::TempDir::new().unwrap();
    let flag = watched.path().join("ready.flag");
    let flag_str = flag.to_string_lossy().to_string();

    let ro_grant = translate(&GrantSpec::from_lists(
        vec![],
        vec![],
        vec![format!("ro:{}", watched.path().display())],
        vec![],
        vec![],
    ))
    .expect("translate ro grant");

    let load = |grants: &ResolvedGrants| {
        load_trigger_constructor(
            providers_dir(),
            "cloacina-provider-sensor",
            "file_present",
            &SensorConfig {
                path: flag_str.clone(),
            },
            TriggerBinding::default(),
            grants,
        )
        .expect("load file_present")
    };

    // Granted + file absent → skip.
    let trigger = load(&ro_grant);
    let result = trigger.poll().await.expect("poll");
    assert!(
        matches!(result, TriggerResult::Skip),
        "absent file must not fire, got {result:?}"
    );

    // Granted + file present → fire, carrying the path in the fire context.
    std::fs::write(&flag, b"go").unwrap();
    let result = trigger.poll().await.expect("poll");
    assert!(result.should_fire(), "present file must fire");
    let ctx = result.into_context().expect("fire carries a context");
    assert_eq!(
        ctx.get("path"),
        Some(&serde_json::json!(flag_str)),
        "fire context carries the sensed path"
    );

    // NO grant + file present → the sandbox can't see it → never fires
    // (default-closed).
    let denied = load(&ResolvedGrants::deny_all());
    let result = denied.poll().await.expect("poll");
    assert!(
        matches!(result, TriggerResult::Skip),
        "an ungranted sensor must fail closed by never firing, got {result:?}"
    );
}

/// The extract accumulator projects the configured field into the boundary and
/// buffers events that lack it.
#[tokio::test]
async fn extract_accumulator_projects_configured_field() {
    let mut acc = load_accumulator_constructor(
        providers_dir(),
        "cloacina-provider-extract",
        "extract",
        &ExtractConfig {
            field: "order_id".into(),
        },
        &ResolvedGrants::deny_all(),
    )
    .expect("load extract");
    assert_eq!(acc.name(), "extract");

    // Event carrying the field → boundary with JUST the projection.
    let event = serde_json::json!({ "order_id": 42, "noise": "ignored" });
    let boundary = acc
        .process(serde_json::to_vec(&event).unwrap())
        .expect("event with the field must emit a boundary");
    let boundary: serde_json::Value = serde_json::from_slice(&boundary).unwrap();
    assert_eq!(boundary, serde_json::json!({ "order_id": 42 }));

    // Event without the field → buffered (no boundary).
    let event = serde_json::json!({ "unrelated": true });
    assert!(
        acc.process(serde_json::to_vec(&event).unwrap()).is_none(),
        "an event without the configured field must buffer"
    );
}

/// The quorum reactor fires once the held-boundary count reaches `required`.
#[tokio::test]
async fn quorum_reactor_fires_at_the_configured_count() {
    let load = |required: i64| {
        load_reactor_constructor(
            providers_dir(),
            "cloacina-provider-quorum",
            "quorum",
            &QuorumConfig { required },
            &ResolvedGrants::deny_all(),
        )
        .expect("load quorum")
    };

    let two_held = serde_json::json!({
        "orders": { "value": 1 },
        "payments": { "value": 2 },
    })
    .to_string();

    // required = 2, held = 2 → fire, payload reports the quorum.
    let outcome = load(2)
        .evaluate(two_held.clone())
        .await
        .expect("evaluate quorum=2");
    assert!(outcome.fire, "2-of-2 must fire");
    let payload: serde_json::Value =
        serde_json::from_str(&outcome.context_json.expect("fire carries a payload")).unwrap();
    assert_eq!(payload.get("quorum").and_then(|v| v.as_i64()), Some(2));

    // required = 3, held = 2 → hold.
    let outcome = load(3).evaluate(two_held).await.expect("evaluate quorum=3");
    assert!(!outcome.fire, "2-of-3 must hold");
}
