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

//! The connector host end to end (COLLIERY-I-0632 / CLOACINA-T-0973): the echo
//! and faulty fixtures are built to wasm32-wasip2, staged as connector
//! provider packages with a `source` and a `destination` member each, and
//! loaded under deny-all grants. Spec, check, discover, a streaming read to its
//! end, a client-streaming write, the error paths, and the kind checks.
//!
//! Note for anyone rerunning: a `CARGO_TARGET_DIR` in the environment moves the
//! fixture's wasm build away from the path this test reads.

#![cfg(feature = "constructors-wasm")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use cloacina::registry::loader::connector_host::{ConnectorHostError, load_connector};
use cloacina::registry::loader::grants::ResolvedGrants;
use cloacina_constructor_contract::sync::{
    Config, ConfiguredStream, ConnectorRole, DiscoverOutcome, ErrorKind, MappingSpec, Partition,
    ReadContext, ReadMessage, RecordBatch, StreamState, SyncMode, WriteContext, WriteMode,
    WriteResult,
};
use cloacina_constructor_contract::{
    CONNECTOR_INTERFACE, CONNECTOR_INTERFACE_VERSION, ConfigField, ConnectorMeta,
    ConstructorManifest, PrimitiveKind, ProviderManifest, ProviderRuntime,
};

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../examples/constructor-contract/{name}"))
}

/// Build a fixture to a wasm component once per process; return its bytes.
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

/// Stage one connector provider package: the component, package.toml and a
/// provider.json with a source and a destination member over the same
/// component, plus a task member to prove the kind check.
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
    let mut task = member("not_a_connector", PrimitiveKind::Task, vec![]);
    task.interface = "task-constructor".into();
    task.connector = None;
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
            task,
        ],
    };
    std::fs::write(dir.join("provider.json"), provider.to_json().unwrap()).unwrap();
}

fn stream(name: &str) -> ConfiguredStream {
    ConfiguredStream {
        stream: name.into(),
        sync_mode: SyncMode::FullRefresh,
        cursor_field: None,
        primary_key: None,
        write_mode: WriteMode::Append,
        mapping: MappingSpec::default(),
    }
}

fn read_ctx(name: &str) -> ReadContext {
    ReadContext {
        stream: stream(name),
        partition: Partition {
            id: "0".into(),
            bounds: None,
        },
        state: StreamState::default(),
    }
}

fn config(json: &str) -> Config {
    Config { json: json.into() }
}

#[test]
fn echo_source_reads_rows_then_a_checkpoint_and_the_destination_counts_them() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(
        tmp.path(),
        "connector-echo-fixture",
        component("connector-echo-fixture", &ECHO),
    );
    let grants = ResolvedGrants::deny_all();
    let cfg = config(r#"{"rows":["{\"id\":1}","{\"id\":2}","{\"id\":3}"],"cursor":"\"3\""}"#);

    let source = load_connector(
        tmp.path(),
        "connector-echo-fixture",
        "source",
        &cfg,
        &grants,
    )
    .expect("load echo source");
    assert_eq!(source.kind(), PrimitiveKind::Source);

    let spec = source.spec().unwrap();
    assert_eq!(spec.name, "echo");
    assert_eq!(spec.contract_version, CONNECTOR_INTERFACE_VERSION);
    assert!(spec.roles.contains(&ConnectorRole::Source));

    assert!(source.check(&cfg).unwrap().success);
    assert!(!source.check(&config("not json")).unwrap().success);

    match source.discover(&cfg).unwrap() {
        DiscoverOutcome::Catalog(c) => assert_eq!(c.streams[0].name, "echo"),
        DiscoverOutcome::Error(e) => panic!("discover failed: {e}"),
    }

    // The streaming read, driven as a blocking iterator to its end.
    let msgs: Vec<ReadMessage> = source
        .read_blocking(&read_ctx("echo"))
        .unwrap()
        .map(|m| m.expect("read item"))
        .collect();
    assert_eq!(msgs.len(), 3, "rows, log, checkpoint: {msgs:?}");
    match &msgs[0] {
        ReadMessage::Records(RecordBatch::Rows(rows)) => {
            assert_eq!(rows, &["{\"id\":1}", "{\"id\":2}", "{\"id\":3}"]);
        }
        other => panic!("first message is not the rows: {other:?}"),
    }
    assert!(matches!(msgs[1], ReadMessage::Log(_)));
    match &msgs[2] {
        ReadMessage::Checkpoint(state) => assert_eq!(state.cursor.as_deref(), Some("\"3\"")),
        other => panic!("last message is not the checkpoint: {other:?}"),
    }

    // A source has no write; the host refuses before any call.
    let refused = source.write(
        &WriteContext {
            stream: stream("echo"),
        },
        vec![],
    );
    assert!(matches!(
        refused,
        Err(ConnectorHostError::WrongKind {
            method: "write",
            ..
        })
    ));

    // The destination member, same component, counts the rows it is given.
    let destination = load_connector(
        tmp.path(),
        "connector-echo-fixture",
        "destination",
        &cfg,
        &grants,
    )
    .expect("load echo destination");
    assert_eq!(destination.kind(), PrimitiveKind::Destination);
    let outcome = destination
        .write(
            &WriteContext {
                stream: stream("echo"),
            },
            vec![
                RecordBatch::Rows(vec!["{\"id\":1}".into(), "{\"id\":2}".into()]),
                RecordBatch::Rows(vec!["{\"id\":3}".into()]),
            ],
        )
        .unwrap();
    match outcome.result {
        WriteResult::Ok(receipt) => assert_eq!(receipt.accepted, 3),
        WriteResult::Err(e) => panic!("write failed: {e}"),
    }
    assert_eq!(outcome.state.cursor.as_deref(), Some("\"3\""));
    assert!(matches!(
        destination.discover(&cfg),
        Err(ConnectorHostError::WrongKind {
            method: "discover",
            ..
        })
    ));
}

#[test]
fn faulty_source_ends_in_fatal_after_its_dead_letter_and_the_destination_errs() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(
        tmp.path(),
        "connector-faulty-fixture",
        component("connector-faulty-fixture", &FAULTY),
    );
    let grants = ResolvedGrants::deny_all();
    let cfg = config(r#"{"fail":"fatal","dead_letter":true}"#);

    let source = load_connector(
        tmp.path(),
        "connector-faulty-fixture",
        "source",
        &cfg,
        &grants,
    )
    .expect("load faulty source");
    let msgs: Vec<ReadMessage> = source
        .read_blocking(&read_ctx("faulty"))
        .unwrap()
        .map(|m| m.expect("read item"))
        .collect();
    assert_eq!(msgs.len(), 2, "{msgs:?}");
    assert!(matches!(msgs[0], ReadMessage::DeadLettered(_)));
    match &msgs[1] {
        ReadMessage::Fatal(e) => assert_eq!(e.kind, ErrorKind::Fatal),
        other => panic!("read did not end in Fatal: {other:?}"),
    }

    let destination = load_connector(
        tmp.path(),
        "connector-faulty-fixture",
        "destination",
        &cfg,
        &grants,
    )
    .expect("load faulty destination");
    let outcome = destination
        .write(
            &WriteContext {
                stream: stream("faulty"),
            },
            vec![RecordBatch::Rows(vec!["{}".into()])],
        )
        .unwrap();
    assert_eq!(outcome.dead_letters.len(), 1);
    match outcome.result {
        WriteResult::Err(e) => assert_eq!(e.kind, ErrorKind::Fatal),
        WriteResult::Ok(_) => panic!("faulty write did not fail"),
    }

    // A transient failure keeps its retryable flag across the boundary.
    let transient = load_connector(
        tmp.path(),
        "connector-faulty-fixture",
        "source",
        &config(r#"{"fail":"transient"}"#),
        &grants,
    )
    .unwrap();
    let last = transient
        .read_blocking(&read_ctx("faulty"))
        .unwrap()
        .last()
        .unwrap()
        .unwrap();
    match last {
        ReadMessage::Fatal(e) => {
            assert_eq!(e.kind, ErrorKind::Transient);
            assert!(e.retryable);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_member_that_is_not_a_connector_is_refused_at_load() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(
        tmp.path(),
        "connector-echo-fixture",
        component("connector-echo-fixture", &ECHO),
    );
    let err = load_connector(
        tmp.path(),
        "connector-echo-fixture",
        "not_a_connector",
        &config("{}"),
        &ResolvedGrants::deny_all(),
    )
    .err()
    .expect("a task member must not load as a connector");
    assert!(
        err.to_string().contains("not a source or a destination"),
        "{err}"
    );

    let missing = load_connector(
        tmp.path(),
        "connector-echo-fixture",
        "nope",
        &config("{}"),
        &ResolvedGrants::deny_all(),
    )
    .err()
    .unwrap();
    assert!(
        missing.to_string().contains("has no constructor 'nope'"),
        "{missing}"
    );
}
