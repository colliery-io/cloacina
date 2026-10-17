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

//! The dispatcher on an embedded SQLite runner (COLLIERY-I-0632 /
//! CLOACINA-T-0977): a three-segment pass over the echo source, serial and in
//! order, the cursor moving once per segment, the pass ending when the source
//! reports its end; a second pass finding nothing new; a stop between segments.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};

use cloacina::dal::DAL;
use cloacina::registry::loader::set_provider_search_path;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina_constructor_contract::sync::{
    ConfiguredStream, ConnectorRole, MappingSpec, SyncMode, WriteMode,
};
use cloacina_constructor_contract::{
    CONNECTOR_INTERFACE, CONNECTOR_INTERFACE_VERSION, ConfigField, ConnectorMeta,
    ConstructorManifest, PrimitiveKind, ProviderManifest, ProviderRuntime,
};
use cloacina_sync::dispatcher::{Dispatcher, PassOutcome, SyncDefinition};
use cloacina_sync::params::keys;
use serde_json::{Value, json};

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../examples/constructor-contract/{name}"))
}

fn component() -> &'static [u8] {
    static ECHO: OnceLock<Vec<u8>> = OnceLock::new();
    ECHO.get_or_init(|| {
        let fixture = fixture_dir("connector-echo-fixture");
        let status = Command::new("cargo")
            .args(["build", "--lib", "--target", "wasm32-wasip2", "--release"])
            .current_dir(&fixture)
            .status()
            .expect("spawn cargo build --target wasm32-wasip2");
        assert!(status.success(), "echo wasm build failed");
        std::fs::read(fixture.join("target/wasm32-wasip2/release/connector_echo_fixture.wasm"))
            .expect("read built wasm component")
    })
}

fn member(name: &str, kind: PrimitiveKind, roles: Vec<ConnectorRole>) -> ConstructorManifest {
    ConstructorManifest {
        name: name.into(),
        version: "0.1.0".into(),
        primitive_kind: kind,
        interface: CONNECTOR_INTERFACE.into(),
        interface_version: CONNECTOR_INTERFACE_VERSION,
        params: vec![],
        config_fields: vec![ConfigField {
            name: "json".into(),
            ty: "String".into(),
        }],
        dependencies: vec![],
        description: None,
        author: None,
        connector: Some(ConnectorMeta {
            roles,
            supported_sync_modes: vec![SyncMode::FullRefresh],
            config_schema: "{\"type\":\"object\"}".into(),
        }),
    }
}

fn stage(root: &Path) {
    let pkg = "connector-echo-fixture";
    let dir = root.join(pkg);
    std::fs::create_dir_all(&dir).unwrap();
    let wasm = "connector_echo_fixture.wasm".to_string();
    std::fs::write(dir.join(&wasm), component()).unwrap();
    std::fs::write(
        dir.join("package.toml"),
        format!(
            "[package]\nname = \"{pkg}\"\nversion = \"0.1.0\"\ninterface = \"connector\"\n\
             interface_version = 1\nruntime = \"wasm\"\n\n[metadata]\ncategory = \"constructor\"\n\n\
             [wasm]\ncomponent = \"{wasm}\"\n"
        ),
    )
    .unwrap();
    let provider = ProviderManifest {
        name: pkg.into(),
        version: "0.1.0".into(),
        component: wasm,
        runtime: ProviderRuntime::Wasm,
        constructors: vec![
            member("source", PrimitiveKind::Source, vec![ConnectorRole::Source]),
            member(
                "destination",
                PrimitiveKind::Destination,
                vec![ConnectorRole::Destination],
            ),
        ],
    };
    std::fs::write(dir.join("provider.json"), provider.to_json().unwrap()).unwrap();
}

fn providers() {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!(
            "cloacina-sync-dispatcher-providers-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        stage(&dir);
        set_provider_search_path(dir.clone());
        dir
    });
}

static RUNNER_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn definition(sync: &str, source_cfg: &str) -> SyncDefinition {
    let stream = ConfiguredStream {
        stream: "echo".into(),
        sync_mode: SyncMode::Incremental,
        cursor_field: Some("n".into()),
        primary_key: Some(vec!["id".into()]),
        write_mode: WriteMode::Upsert {
            business_keys: vec!["id".into()],
        },
        mapping: MappingSpec::default(),
    };
    let mut p: HashMap<String, Value> = HashMap::new();
    p.insert(keys::SYNC.into(), json!(sync));
    p.insert(
        keys::SOURCE_PROVIDER.into(),
        json!("connector-echo-fixture"),
    );
    p.insert(keys::SOURCE_MEMBER.into(), json!("source"));
    p.insert(keys::SOURCE_CONFIG.into(), json!(source_cfg));
    p.insert(
        keys::DESTINATION_PROVIDER.into(),
        json!("connector-echo-fixture"),
    );
    p.insert(keys::DESTINATION_MEMBER.into(), json!("destination"));
    p.insert(keys::DESTINATION_CONFIG.into(), json!("{}"));
    p.insert(
        keys::STREAM.into(),
        json!(serde_json::to_string(&stream).unwrap()),
    );
    SyncDefinition::new(p)
}

async fn runner() -> DefaultRunner {
    cloacina_sync::ensure_linked();
    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .build()
        .unwrap();
    DefaultRunner::with_config(":memory:", config)
        .await
        .expect("sqlite runner")
}

#[tokio::test]
async fn a_pass_runs_three_segments_in_order_and_ends_when_the_source_does() {
    let _guard = RUNNER_LOCK.lock().await;
    providers();
    let runner = runner().await;
    let dal = DAL::new(runner.database().clone());
    let dispatcher = Dispatcher::new(Arc::new(runner.clone()), Arc::new(dal.clone()));
    let (_tx, stop) = tokio::sync::watch::channel(false);

    // The echo source yields three segments, each one row and a checkpoint
    // whose cursor counts "1", "2", "3", and resumes after the cursor it is
    // given.
    let def = definition("three", r#"{"segments":3,"rows":["{\"id\":1}"]}"#);
    let outcome = dispatcher.run_pass(&def, stop.clone()).await.expect("pass");
    assert_eq!(outcome, PassOutcome::Exhausted { segments: 3 });

    let state = dal
        .sync_stream_state()
        .get("three", "echo", "")
        .await
        .unwrap()
        .expect("committed state");
    assert_eq!(state.cursor.as_deref(), Some("\"3\""));
    assert!(state.exhausted.0);

    // Three executions of the Sync workflow ran, serially: none overlap, each
    // one completed before the next started.
    let mut runs: Vec<_> = dal
        .workflow_execution()
        .list_recent(50)
        .await
        .unwrap()
        .into_iter()
        .filter(|e| e.workflow_name == cloacina_sync::WORKFLOW_NAME && e.status == "Completed")
        .collect();
    runs.sort_by_key(|e| e.started_at.0);
    assert_eq!(runs.len(), 3, "{runs:?}");
    for pair in runs.windows(2) {
        let done = pair[0].completed_at.expect("completed").0;
        assert!(
            done <= pair[1].started_at.0,
            "segments overlapped: {pair:?}"
        );
    }

    // A second pass finds nothing new: one empty segment, still exhausted,
    // cursor unchanged.
    let outcome = dispatcher.run_pass(&def, stop).await.expect("second pass");
    assert_eq!(outcome, PassOutcome::Exhausted { segments: 1 });
    let state = dal
        .sync_stream_state()
        .get("three", "echo", "")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.cursor.as_deref(), Some("\"3\""));

    runner.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_stop_between_segments_ends_the_pass_with_the_cursor_committed_so_far() {
    let _guard = RUNNER_LOCK.lock().await;
    providers();
    let runner = runner().await;
    let dal = DAL::new(runner.database().clone());
    let dispatcher = Dispatcher::new(Arc::new(runner.clone()), Arc::new(dal.clone()));
    let (tx, stop) = tokio::sync::watch::channel(false);

    // Stop is raised before the pass starts its first segment check: the
    // dispatcher sees it between segments. Here that is before any segment.
    tx.send(true).unwrap();
    let def = definition("stopped", r#"{"segments":3}"#);
    let outcome = dispatcher.run_pass(&def, stop).await.expect("pass");
    assert_eq!(outcome, PassOutcome::Stopped { segments: 0 });
    assert!(
        dal.sync_stream_state()
            .get("stopped", "echo", "")
            .await
            .unwrap()
            .is_none()
    );

    runner.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_definition_without_a_name_is_refused_before_anything_runs() {
    let _guard = RUNNER_LOCK.lock().await;
    providers();
    let runner = runner().await;
    let dal = DAL::new(runner.database().clone());
    let dispatcher = Dispatcher::new(Arc::new(runner.clone()), Arc::new(dal.clone()));
    let (_tx, stop) = tokio::sync::watch::channel(false);
    let def = SyncDefinition::new(HashMap::new());
    let err = dispatcher.run_pass(&def, stop).await.expect_err("refused");
    assert!(err.to_string().contains("sync"), "{err}");
    runner.shutdown().await.unwrap();
}
