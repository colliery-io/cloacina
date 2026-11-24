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

//! A sync definition is an instance of the Sync workflow, and a fire of it is a
//! pass (COLLIERY-I-0632 / CLOACINA-T-0978): `run_pass` by instance name on an
//! embedded runner, and `fire_instance` through the installed hook returning
//! the first execution while the rest of the pass runs on.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use cloacina::Context;
use cloacina::dal::DAL;
use cloacina::database::universal_types::UniversalBool;
use cloacina::models::schedule::NewSchedule;
use cloacina::registry::loader::set_provider_search_path;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina_constructor_contract::sync::{
    ConfiguredStream, ConnectorRole, MappingSpec, SyncMode, WriteMode,
};
use cloacina_constructor_contract::{
    CONNECTOR_INTERFACE, CONNECTOR_INTERFACE_VERSION, ConfigField, ConnectorMeta,
    ConstructorManifest, PrimitiveKind, ProviderManifest, ProviderRuntime,
};
use cloacina_sync::dispatcher::PassOutcome;
use cloacina_sync::params::keys;
use serde_json::json;

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
            "cloacina-sync-instance-providers-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        stage(&dir);
        set_provider_search_path(dir.clone());
        dir
    });
}

static RUNNER_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn params_json(sync: &str, segments: u64) -> String {
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
    json!({
        keys::SYNC: sync,
        keys::SOURCE_PROVIDER: "connector-echo-fixture",
        keys::SOURCE_MEMBER: "source",
        keys::SOURCE_CONFIG: format!(r#"{{"segments":{segments},"rows":["{{\"id\":1}}"]}}"#),
        keys::DESTINATION_PROVIDER: "connector-echo-fixture",
        keys::DESTINATION_MEMBER: "destination",
        keys::DESTINATION_CONFIG: "{}",
        keys::STREAM: serde_json::to_string(&stream).unwrap(),
    })
    .to_string()
}

async fn runner() -> DefaultRunner {
    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .build()
        .unwrap();
    DefaultRunner::with_config(":memory:", config)
        .await
        .expect("sqlite runner")
}

/// A sync definition: an unscheduled instance of the Sync workflow.
async fn define(dal: &DAL, instance: &str, params: &str) {
    dal.schedule()
        .create(NewSchedule {
            schedule_type: "cron".into(),
            workflow_name: cloacina_sync::WORKFLOW_NAME.into(),
            enabled: Some(UniversalBool(true)),
            cron_expression: None,
            timezone: None,
            catchup_policy: None,
            start_date: None,
            end_date: None,
            trigger_name: None,
            poll_interval_ms: None,
            allow_concurrent: None,
            next_run_at: None,
            params: Some(params.to_string()),
            instance_name: Some(instance.to_string()),
        })
        .await
        .expect("create the sync definition");
}

#[tokio::test]
async fn run_pass_by_instance_name_runs_the_definition_from_its_schedule_row() {
    let _guard = RUNNER_LOCK.lock().await;
    providers();
    let runner = runner().await;
    let dal = DAL::new(runner.database().clone());
    cloacina_sync::install(&runner, None, None);
    define(&dal, "orders", &params_json("orders", 3)).await;

    let (_tx, stop) = tokio::sync::watch::channel(false);
    let outcome = cloacina_sync::run_pass(&runner, "orders", stop)
        .await
        .expect("pass");
    assert_eq!(outcome, PassOutcome::Exhausted { segments: 3 });
    let state = dal
        .sync_stream_state()
        .get("orders", "echo", "")
        .await
        .unwrap()
        .expect("state");
    assert_eq!(state.cursor.as_deref(), Some("\"3\""));

    let (_tx, stop) = tokio::sync::watch::channel(false);
    let err = cloacina_sync::run_pass(&runner, "nope", stop)
        .await
        .expect_err("unknown definition");
    assert!(err.to_string().contains("nope"), "{err}");

    runner.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_fire_through_the_hook_returns_the_first_segment_and_the_pass_runs_on() {
    let _guard = RUNNER_LOCK.lock().await;
    providers();
    let runner = runner().await;
    let dal = DAL::new(runner.database().clone());
    let hook = cloacina_sync::install(&runner, None, None);

    // What the cron scheduler or the manual-run route hands the runner: the
    // instance params merged into a context.
    let mut context = Context::new();
    cloacina::workflow_instance::merge_instance_params(&mut context, &params_json("fired", 3))
        .unwrap();
    let first = runner
        .fire_instance(cloacina_sync::WORKFLOW_NAME, context)
        .await
        .expect("hook took the fire");
    assert_eq!(first.workflow_name, cloacina_sync::WORKFLOW_NAME);

    // The pass runs on after the first execution is handed back.
    let deadline = Instant::now() + Duration::from_secs(120);
    while !hook.running().is_empty() {
        assert!(Instant::now() < deadline, "the pass did not finish");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let state = dal
        .sync_stream_state()
        .get("fired", "echo", "")
        .await
        .unwrap()
        .expect("state");
    assert_eq!(state.cursor.as_deref(), Some("\"3\""));
    assert!(state.exhausted.0);
    let runs: Vec<_> = dal
        .workflow_execution()
        .list_recent(50)
        .await
        .unwrap()
        .into_iter()
        .filter(|e| e.workflow_name == cloacina_sync::WORKFLOW_NAME)
        .collect();
    assert_eq!(runs.len(), 3, "{runs:?}");
    assert!(runs.iter().all(|r| r.status == "Completed"));
    assert!(
        runs.iter().any(|r| r.id.0 == first.execution_id),
        "the handed-back execution is the first segment"
    );

    // A fire of any other workflow is left to the runner: the hook returns None
    // and the runner executes it once. There is no such workflow registered, so
    // the runner refuses it, and the error is the runner's, not the hook's.
    match runner.fire_instance("not_sync", Context::new()).await {
        Ok(_) => panic!("not a registered workflow"),
        Err(err) => assert!(err.to_string().contains("not found"), "{err}"),
    }

    runner.shutdown().await.unwrap();
}
