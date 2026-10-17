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

//! Host-side credential injection (COLLIERY-I-0633 / CLOACINA-T-0982): the
//! http fixture GETs a local HTTP server from its read, and the server sees
//! the header, the query parameter or the SigV4 signature the host added. The
//! guest only sees the response, never the secret.
//!
//! The fixtures are built to wasm32-wasip2 by this test; a `CARGO_TARGET_DIR`
//! in the environment moves it away from the path it reads.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};

use cloacina::Context;
use cloacina::executor::{WorkflowExecutor, WorkflowStatus};
use cloacina::registry::loader::credentials::Credential;
use cloacina::registry::loader::set_provider_search_path;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina_constructor_contract::sync::{
    ConfiguredStream, ConnectorRole, MappingSpec, Partition, RecordBatch, SyncMode, WriteMode,
};
use cloacina_constructor_contract::{
    CONNECTOR_INTERFACE, CONNECTOR_INTERFACE_VERSION, ConfigField, ConnectorMeta,
    ConstructorManifest, PrimitiveKind, ProviderManifest, ProviderRuntime,
};
use cloacina_sync::params::{Endpoint, SegmentInput, SyncParams, keys};
use cloacina_sync::workflow::read_segment_blocking;
use cloacina_workflow::secret::MapSecretResolver;
use serde_json::{Value, json};

/// The source: GETs its `fetch` URL. Imports wasi:http, so it loads only with
/// an http grant.
const HTTP: &str = "connector-http-fixture";
/// The destination: counts rows. Needs no grant.
const ECHO: &str = "connector-echo-fixture";

fn component(name: &str) -> Vec<u8> {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../examples/constructor-contract/{name}"));
    let status = Command::new("cargo")
        .args(["build", "--lib", "--target", "wasm32-wasip2", "--release"])
        .current_dir(&fixture)
        .status()
        .expect("spawn cargo build --target wasm32-wasip2");
    assert!(status.success(), "{name} wasm build failed");
    std::fs::read(fixture.join(format!(
        "target/wasm32-wasip2/release/{}.wasm",
        name.replace('-', "_")
    )))
    .expect("read built wasm component")
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

fn stage(root: &Path, pkg: &str) {
    let dir = root.join(pkg);
    std::fs::create_dir_all(&dir).unwrap();
    let wasm = format!("{}.wasm", pkg.replace('-', "_"));
    std::fs::write(dir.join(&wasm), component(pkg)).unwrap();
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

/// Stage both fixtures once and point the provider search path at them.
fn providers() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let root =
            std::env::temp_dir().join(format!("cloacina-sync-credentials-{}", std::process::id()));
        stage(&root, HTTP);
        stage(&root, ECHO);
        set_provider_search_path(root.clone());
        root
    })
}

/// A local HTTP server that answers `200 ok` and keeps each request's head
/// (request line and headers) in arrival order.
struct Recorder {
    port: u16,
    heads: Arc<Mutex<Vec<String>>>,
}

impl Recorder {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let heads = Arc::new(Mutex::new(Vec::new()));
        let seen = heads.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut head = String::new();
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    head.push_str(&line);
                }
                seen.lock().unwrap().push(head);
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                );
            }
        });
        Self { port, heads }
    }

    fn heads(&self) -> Vec<String> {
        self.heads.lock().unwrap().clone()
    }
}

fn stream() -> ConfiguredStream {
    ConfiguredStream {
        stream: "echo".into(),
        sync_mode: SyncMode::FullRefresh,
        cursor_field: None,
        primary_key: None,
        write_mode: WriteMode::Append,
        mapping: MappingSpec::default(),
    }
}

/// A sync whose source fetches `path` on the recorder. `grant` is the source's
/// http grant.
fn sync_params(rec: &Recorder, path: &str, grant: &str) -> SyncParams {
    SyncParams {
        sync: "creds".into(),
        source: Endpoint {
            provider: HTTP.into(),
            member: "source".into(),
            config: json!({"fetch": format!("http://127.0.0.1:{}{path}", rec.port)}).to_string(),
            grants: Some(json!({"http": [grant]})),
            credential: None,
        },
        destination: Endpoint {
            provider: ECHO.into(),
            member: "destination".into(),
            config: "{}".into(),
            grants: None,
            credential: None,
        },
        stream: stream(),
        partition: Partition {
            id: String::new(),
            bounds: None,
        },
        schema: None,
        segment_checkpoints: 1,
    }
}

/// Read one segment on a blocking thread and return the guest's rows.
async fn read_rows(p: SyncParams, credential: Option<Credential>) -> Vec<String> {
    let seg = tokio::task::spawn_blocking(move || {
        read_segment_blocking(&p, SegmentInput::default(), credential)
    })
    .await
    .unwrap()
    .expect("segment read");
    seg.batches
        .into_iter()
        .flat_map(|b| match b {
            RecordBatch::Rows(rows) => rows,
            _ => Vec::new(),
        })
        .collect()
}

fn header<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    head.lines().find_map(|l| {
        let (n, v) = l.split_once(':')?;
        n.trim().eq_ignore_ascii_case(name).then(|| v.trim())
    })
}

#[tokio::test]
async fn the_server_sees_the_header_and_the_guest_does_not() {
    providers();
    let rec = Recorder::start();
    let grant = format!("127.0.0.1:{}", rec.port);
    let rows = read_rows(
        sync_params(&rec, "/items", &grant),
        Some(Credential::Header {
            name: "Authorization".into(),
            value: "Bearer tok-SECRET-1".into(),
        }),
    )
    .await;

    let heads = rec.heads();
    assert_eq!(heads.len(), 1, "{heads:?}");
    assert_eq!(
        header(&heads[0], "authorization"),
        Some("Bearer tok-SECRET-1")
    );
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row: Value = serde_json::from_str(&rows[0]).unwrap();
    assert_eq!(row["status"], json!(200), "{row}");
    assert_eq!(row["body"], json!("ok"));
    assert!(!rows[0].contains("SECRET"), "{rows:?}");
}

#[tokio::test]
async fn the_server_sees_the_query_parameter() {
    providers();
    let rec = Recorder::start();
    let grant = format!("127.0.0.1:{}", rec.port);
    let rows = read_rows(
        sync_params(&rec, "/items?page=2", &grant),
        Some(Credential::Query {
            param: "api_key".into(),
            value: "k/SECRET 2".into(),
        }),
    )
    .await;

    let heads = rec.heads();
    assert_eq!(heads.len(), 1, "{heads:?}");
    let request_line = heads[0].lines().next().unwrap();
    assert!(
        request_line.starts_with("GET /items?page=2&api_key=k%2FSECRET%202 "),
        "{request_line}"
    );
    assert!(!rows[0].contains("SECRET"), "{rows:?}");
}

#[tokio::test]
async fn the_server_sees_a_sigv4_signature() {
    providers();
    let rec = Recorder::start();
    let grant = format!("127.0.0.1:{}", rec.port);
    let rows = read_rows(
        sync_params(&rec, "/bucket/key.json", &grant),
        Some(Credential::AwsSigV4 {
            access_key: "AKIDEXAMPLE".into(),
            secret_key: "wJalrXUtnFEMI/SECRET/bPxRfiCYEXAMPLEKEY".into(),
            region: "us-east-1".into(),
            service: "s3".into(),
        }),
    )
    .await;

    let heads = rec.heads();
    assert_eq!(heads.len(), 1, "{heads:?}");
    let auth = header(&heads[0], "authorization").expect("authorization header");
    assert!(
        auth.starts_with("AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/"),
        "{auth}"
    );
    assert!(auth.contains("/us-east-1/s3/aws4_request"), "{auth}");
    assert!(
        auth.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date"),
        "{auth}"
    );
    assert!(header(&heads[0], "x-amz-date").is_some(), "{}", heads[0]);
    assert!(!heads[0].contains("SECRET"), "the secret key never travels");
    assert!(!rows[0].contains("SECRET"), "{rows:?}");
}

#[tokio::test]
async fn a_credential_does_not_widen_the_grants() {
    providers();
    let rec = Recorder::start();
    // HTTP is granted, but to another port: the request is denied before the
    // credential is applied, and never reaches the recorder.
    let rows = read_rows(
        sync_params(&rec, "/items", "127.0.0.1:9"),
        Some(Credential::Header {
            name: "Authorization".into(),
            value: "Bearer tok-SECRET-3".into(),
        }),
    )
    .await;

    assert!(rec.heads().is_empty(), "no request left the host");
    let row: Value = serde_json::from_str(&rows[0]).unwrap();
    assert!(row.get("error").is_some(), "{row}");
    assert!(!rows[0].contains("SECRET"), "{rows:?}");
}

/// The whole path on an embedded runner: the sync definition names the
/// credential, the read task resolves the secret through the execution's
/// resolver, and the server sees the header. The secret never lands in the
/// saved context.
#[tokio::test]
async fn the_read_task_resolves_the_named_secret() {
    providers();
    let rec = Recorder::start();
    cloacina_sync::ensure_linked();
    let mut secrets = BTreeMap::new();
    secrets.insert(
        "api-prod".to_string(),
        BTreeMap::from([("token".to_string(), "tok-SECRET-4".to_string())]),
    );
    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .build()
        .unwrap();
    let runner = DefaultRunner::builder()
        .database_url(":memory:")
        .with_config(config)
        .secret_resolver(Arc::new(MapSecretResolver::new(secrets)))
        .build()
        .await
        .expect("sqlite runner");

    let mut ctx = Context::new();
    let mut put = |k: &str, v: Value| ctx.insert(k, v).unwrap();
    put(keys::SYNC, json!("creds"));
    put(keys::SOURCE_PROVIDER, json!(HTTP));
    put(keys::SOURCE_MEMBER, json!("source"));
    put(
        keys::SOURCE_CONFIG,
        json!(json!({"fetch": format!("http://127.0.0.1:{}/items", rec.port)}).to_string()),
    );
    put(
        keys::SOURCE_GRANTS,
        json!(json!({"http": [format!("127.0.0.1:{}", rec.port)]}).to_string()),
    );
    put(
        keys::SOURCE_CREDENTIAL,
        json!(
            json!({
                "kind": "header", "secret": "api-prod", "name": "Authorization",
                "field": "token", "prefix": "Bearer "
            })
            .to_string()
        ),
    );
    put(keys::DESTINATION_PROVIDER, json!(ECHO));
    put(keys::DESTINATION_MEMBER, json!("destination"));
    put(
        keys::STREAM,
        json!(serde_json::to_string(&stream()).unwrap()),
    );

    let result = runner
        .execute(cloacina_sync::WORKFLOW_NAME, ctx)
        .await
        .expect("sync ran");
    assert_eq!(result.status, WorkflowStatus::Completed, "{result:?}");
    assert_eq!(result.final_context.get(keys::ACCEPTED), Some(&json!(1)));

    let heads = rec.heads();
    assert_eq!(heads.len(), 1, "{heads:?}");
    assert_eq!(
        header(&heads[0], "authorization"),
        Some("Bearer tok-SECRET-4")
    );
    let saved = result.final_context.to_json().unwrap();
    assert!(!saved.contains("SECRET"), "{saved}");

    runner.shutdown().await.unwrap();
}
