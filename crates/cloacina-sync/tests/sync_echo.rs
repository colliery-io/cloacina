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

//! The Sync workflow end to end on an embedded SQLite runner
//! (COLLIERY-I-0632 / CLOACINA-T-0976): the echo connector as source and
//! destination, one segment read, mapped, written, and the stream state and
//! dead-letter rows landing with the write task's completion.
//!
//! The fixtures are built to wasm32-wasip2 by this test; a `CARGO_TARGET_DIR`
//! in the environment moves them away from the path it reads.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use cloacina::Context;
use cloacina::dal::DAL;
use cloacina::executor::{WorkflowExecutor, WorkflowStatus};
use cloacina::registry::loader::set_provider_search_path;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina_constructor_contract::sync::{
    ConfiguredStream, ConnectorRole, Field, FieldType, MappingOp, MappingSpec, StreamSchema,
    StreamState, SyncMode, WriteMode,
};
use cloacina_constructor_contract::{
    CONNECTOR_INTERFACE, CONNECTOR_INTERFACE_VERSION, ConfigField, ConnectorMeta,
    ConstructorManifest, PrimitiveKind, ProviderManifest, ProviderRuntime,
};
use cloacina_sync::params::keys;
use serde_json::{Value, json};

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../examples/constructor-contract/{name}"))
}

fn component(name: &'static str, cell: &'static OnceLock<Vec<u8>>) -> &'static [u8] {
    cell.get_or_init(|| {
        let fixture = fixture_dir(name);
        let status = Command::new("cargo")
            .args(["build", "--lib", "--target", "wasm32-wasip2", "--release"])
            .current_dir(&fixture)
            .status()
            .expect("spawn cargo build --target wasm32-wasip2");
        assert!(status.success(), "{name} wasm build failed");
        let lib = name.replace('-', "_");
        std::fs::read(fixture.join(format!("target/wasm32-wasip2/release/{lib}.wasm")))
            .expect("read built wasm component")
    })
}

static ECHO: OnceLock<Vec<u8>> = OnceLock::new();
static FAULTY: OnceLock<Vec<u8>> = OnceLock::new();

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

fn stage(root: &Path, pkg: &str, bytes: &[u8]) {
    let dir = root.join(pkg);
    std::fs::create_dir_all(&dir).unwrap();
    let wasm = format!("{}.wasm", pkg.replace('-', "_"));
    std::fs::write(dir.join(&wasm), bytes).unwrap();
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

/// The provider search path is process-wide; the tests stage every fixture
/// into one directory and set it once, under a lock so they do not race.
fn providers() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir =
            std::env::temp_dir().join(format!("cloacina-sync-providers-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        stage(
            &dir,
            "connector-echo-fixture",
            component("connector-echo-fixture", &ECHO),
        );
        stage(
            &dir,
            "connector-faulty-fixture",
            component("connector-faulty-fixture", &FAULTY),
        );
        set_provider_search_path(dir.clone());
        dir
    })
}

static RUNNER_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn stream(mapping: MappingSpec) -> ConfiguredStream {
    ConfiguredStream {
        stream: "echo".into(),
        sync_mode: SyncMode::FullRefresh,
        cursor_field: None,
        primary_key: Some(vec!["id".into()]),
        write_mode: WriteMode::Upsert {
            business_keys: vec!["id".into()],
        },
        mapping,
    }
}

fn params(
    sync: &str,
    source_pkg: &str,
    source_cfg: &str,
    dest_pkg: &str,
    dest_cfg: &str,
    stream: &ConfiguredStream,
    schema: Option<&StreamSchema>,
) -> Context<Value> {
    let mut c = Context::new();
    c.insert(keys::SYNC, json!(sync)).unwrap();
    c.insert(keys::SOURCE_PROVIDER, json!(source_pkg)).unwrap();
    c.insert(keys::SOURCE_MEMBER, json!("source")).unwrap();
    c.insert(keys::SOURCE_CONFIG, json!(source_cfg)).unwrap();
    c.insert(keys::DESTINATION_PROVIDER, json!(dest_pkg))
        .unwrap();
    c.insert(keys::DESTINATION_MEMBER, json!("destination"))
        .unwrap();
    c.insert(keys::DESTINATION_CONFIG, json!(dest_cfg)).unwrap();
    c.insert(keys::STREAM, json!(serde_json::to_string(stream).unwrap()))
        .unwrap();
    if let Some(s) = schema {
        c.insert(keys::SCHEMA, json!(serde_json::to_string(s).unwrap()))
            .unwrap();
    }
    c
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
async fn one_segment_moves_from_echo_to_echo_and_commits_the_cursor() {
    let _guard = RUNNER_LOCK.lock().await;
    providers();
    let runner = runner().await;
    let dal = DAL::new(runner.database().clone());

    // Three rows; the mapping renames `id` to `key`; the schema wants `key`
    // as an integer and lets `name` be absent.
    let mapping = MappingSpec {
        ops: vec![MappingOp::Rename {
            from: "id".into(),
            to: "key".into(),
        }],
    };
    let schema = StreamSchema {
        fields: vec![
            Field {
                name: "key".into(),
                field_type: FieldType::Integer,
                nullable: false,
            },
            Field {
                name: "name".into(),
                field_type: FieldType::Str,
                nullable: true,
            },
        ],
    };
    let ctx = params(
        "orders",
        "connector-echo-fixture",
        r#"{"rows":["{\"id\":1,\"name\":\"a\"}","{\"id\":\"2\"}","{\"id\":\"x\"}"],"cursor":"\"3\""}"#,
        "connector-echo-fixture",
        "{}",
        &stream(mapping),
        Some(&schema),
    );

    let result = runner
        .execute(cloacina_sync::WORKFLOW_NAME, ctx)
        .await
        .expect("sync ran");
    assert_eq!(result.status, WorkflowStatus::Completed, "{result:?}");

    // Two rows written (id 1 and the coerced "2"); "x" is a dead-letter.
    assert_eq!(result.final_context.get(keys::ACCEPTED), Some(&json!(2)));
    assert_eq!(
        result.final_context.get(keys::EXHAUSTED),
        Some(&json!(false)),
        "the echo source checkpoints once, so the segment closed at its budget"
    );
    assert!(
        result.final_context.get(keys::BATCHES).is_none(),
        "batches do not land in the saved context"
    );

    // The cursor landed with the completion, after the write.
    let state = dal
        .sync_stream_state()
        .get("orders", "echo", "")
        .await
        .unwrap()
        .expect("stream state row");
    assert_eq!(state.cursor.as_deref(), Some("\"3\""));
    assert!(!state.exhausted.0);

    let dead = dal
        .sync_dead_letters()
        .list_for_sync("orders", 10)
        .await
        .unwrap();
    assert_eq!(dead.len(), 1, "{dead:?}");
    assert!(
        dead[0].reason.contains("not a Integer") || dead[0].reason.contains("schema"),
        "{}",
        dead[0].reason
    );
    assert_eq!(dead[0].record, r#"{"id":"x"}"#);

    runner.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_source_that_ends_without_a_checkpoint_flushes_with_the_previous_state() {
    let _guard = RUNNER_LOCK.lock().await;
    providers();
    let runner = runner().await;
    let dal = DAL::new(runner.database().clone());

    // The faulty connector with no `fail` yields a dead-letter, one row and a
    // checkpoint with no cursor; with a budget of two checkpoints the segment
    // runs to the end of the stream, so the source is exhausted.
    let mut ctx = params(
        "faulty-sync",
        "connector-faulty-fixture",
        r#"{"dead_letter":true}"#,
        "connector-echo-fixture",
        "{}",
        &stream(MappingSpec::default()),
        None,
    );
    ctx.insert(keys::SEGMENT_CHECKPOINTS, json!(2)).unwrap();
    ctx.insert(
        keys::STATE,
        json!(
            serde_json::to_string(&StreamState {
                cursor: Some("\"prev\"".into()),
                opaque: vec![1],
            })
            .unwrap()
        ),
    )
    .unwrap();

    let result = runner
        .execute(cloacina_sync::WORKFLOW_NAME, ctx)
        .await
        .expect("sync ran");
    assert_eq!(result.status, WorkflowStatus::Completed, "{result:?}");
    assert_eq!(
        result.final_context.get(keys::EXHAUSTED),
        Some(&json!(true))
    );
    assert_eq!(result.final_context.get(keys::ACCEPTED), Some(&json!(1)));

    let state = dal
        .sync_stream_state()
        .get("faulty-sync", "echo", "")
        .await
        .unwrap()
        .expect("stream state row");
    // The faulty source's checkpoint carries no cursor, so the committed cursor is None.
    assert_eq!(state.cursor, None);
    assert!(state.exhausted.0);
    assert_eq!(
        dal.sync_dead_letters()
            .count_for_sync("faulty-sync")
            .await
            .unwrap(),
        1
    );

    runner.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_fatal_source_fails_the_execution_and_commits_nothing() {
    let _guard = RUNNER_LOCK.lock().await;
    providers();
    let runner = runner().await;
    let dal = DAL::new(runner.database().clone());

    let ctx = params(
        "doomed",
        "connector-faulty-fixture",
        r#"{"fail":"fatal"}"#,
        "connector-echo-fixture",
        "{}",
        &stream(MappingSpec::default()),
        None,
    );
    let result = runner
        .execute(cloacina_sync::WORKFLOW_NAME, ctx)
        .await
        .expect("sync ran to a terminal state");
    assert_ne!(result.status, WorkflowStatus::Completed);
    assert!(
        dal.sync_stream_state()
            .get("doomed", "echo", "")
            .await
            .unwrap()
            .is_none()
    );

    runner.shutdown().await.unwrap();
}
