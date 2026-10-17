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

//! The Postgres connector against the dev database on port 15432
//! (COLLIERY-I-0632 / CLOACINA-T-0979): spec, check, discover, a full-refresh
//! read, upsert and append writes through the connector host, an incremental
//! read that advances the cursor and resumes with only the delta, and one
//! Postgres-to-Postgres sync through the Sync workflow and the dispatcher.
//!
//! Ignored by default: run with `-- --ignored` once `angreal services up` has
//! the database (or set `CLOACINA_TEST_PG_URL`). The lane that runs it also
//! needs the `wasm32-wasip2` target, since it builds the guest.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use cloacina::dal::DAL;
use cloacina::registry::loader::connector_host::load_connector;
use cloacina::registry::loader::grants::ResolvedGrants;
use cloacina::registry::loader::set_provider_search_path;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina_constructor_contract::PrimitiveKind;
use cloacina_constructor_contract::sync::{
    Config, ConfiguredStream, DiscoverOutcome, MappingSpec, Partition, ReadContext, ReadMessage,
    RecordBatch, StreamState, SyncMode, WriteContext, WriteMode, WriteResult,
};
use cloacina_sync::dispatcher::{Dispatcher, PassOutcome, SyncDefinition};
use cloacina_sync::params::keys;
use cloacina_sync::workflow::grant_spec_from_json;
use serde_json::{Value, json};

const PKG: &str = "cloacina-connector-postgres";

fn pg_url() -> String {
    std::env::var("CLOACINA_TEST_PG_URL")
        .unwrap_or_else(|_| "postgres://cloacina:cloacina@localhost:15432/cloacina".to_string())
}

fn pg_port() -> u16 {
    pg_url()
        .rsplit(':')
        .next()
        .and_then(|tail| tail.split('/').next())
        .and_then(|p| p.parse().ok())
        .unwrap_or(5432)
}

/// The connector talks plain TCP to the dev database: `sslmode=disable`.
fn connector_config(extra: &str) -> String {
    let url = format!("{}?sslmode=disable", pg_url());
    if extra.is_empty() {
        json!({ "url": url }).to_string()
    } else {
        let mut v: serde_json::Map<String, Value> = serde_json::from_str(extra).unwrap();
        v.insert("url".into(), json!(url));
        Value::Object(v).to_string()
    }
}

fn grants() -> ResolvedGrants {
    let spec = grant_spec_from_json(&json!({ "tcp": [format!("*:{}", pg_port())] })).unwrap();
    cloacina::registry::loader::grants::translate(&spec).unwrap()
}

/// Stage the connector once per process under the provider search path.
fn providers() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir =
            std::env::temp_dir().join(format!("cloacina-sync-pg-providers-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../connectors/postgres");
        cloacina_testing::connectors::stage_connector(&dir, &crate_dir);
        set_provider_search_path(dir.clone());
        dir
    })
}

fn raw() -> postgres::Client {
    postgres::Client::connect(&pg_url(), postgres::NoTls).expect("raw pg client")
}

fn reset_tables(tables: &[&str]) {
    let mut c = raw();
    for t in tables {
        c.batch_execute(&format!("DROP TABLE IF EXISTS {t}"))
            .unwrap();
    }
}

fn stream(name: &str, mode: SyncMode, write: WriteMode, cursor: Option<&str>) -> ConfiguredStream {
    ConfiguredStream {
        stream: name.into(),
        sync_mode: mode,
        cursor_field: cursor.map(str::to_string),
        primary_key: Some(vec!["n".into()]),
        write_mode: write,
        mapping: MappingSpec::default(),
    }
}

fn read_ctx(s: &ConfiguredStream, state: StreamState) -> ReadContext {
    ReadContext {
        stream: s.clone(),
        partition: Partition {
            id: String::new(),
            bounds: None,
        },
        state,
    }
}

fn rows(n: std::ops::RangeInclusive<i64>) -> Vec<String> {
    n.map(|i| json!({ "n": i, "name": format!("row {i}") }).to_string())
        .collect()
}

/// Read everything a source yields: (rows, last checkpoint state).
fn read_all(
    source: &cloacina::registry::loader::connector_host::ConnectorHandle,
    ctx: &ReadContext,
) -> (Vec<String>, StreamState) {
    let mut out = Vec::new();
    let mut state = ctx.state.clone();
    for msg in source.read_blocking(ctx).unwrap() {
        match msg.unwrap() {
            ReadMessage::Records(RecordBatch::Rows(r)) => out.extend(r),
            ReadMessage::Records(RecordBatch::Changes(c)) => {
                out.extend(c.into_iter().map(|c| c.data))
            }
            ReadMessage::Records(RecordBatch::Arrow(_)) => {}
            ReadMessage::Checkpoint(s) => state = s,
            ReadMessage::Fatal(e) => panic!("read failed: {e}"),
            ReadMessage::Log(_) | ReadMessage::DeadLettered(_) => {}
        }
    }
    (out, state)
}

fn n_of(row: &str) -> i64 {
    serde_json::from_str::<Value>(row).unwrap()["n"]
        .as_i64()
        .unwrap()
}

#[test]
#[ignore = "needs the dev Postgres on 15432 (angreal services up) and the wasm32-wasip2 target"]
fn spec_check_discover_and_a_write_then_read_roundtrip() {
    reset_tables(&["pg_rt"]);
    let root = providers();
    let g = grants();

    let destination = load_connector(
        root,
        PKG,
        "destination",
        &Config {
            json: connector_config(r#"{"table":"pg_rt"}"#),
        },
        &g,
    )
    .unwrap();
    assert_eq!(destination.kind(), PrimitiveKind::Destination);
    let spec = destination.spec().unwrap();
    assert_eq!(spec.name, "postgres");
    assert!(
        destination
            .check(&Config {
                json: connector_config("")
            })
            .unwrap()
            .success
    );
    // The connector binds its config at load, so a wrong password is a badly
    // configured handle, whose check fails.
    let bad_cfg = Config {
        json: json!({"url": format!("postgres://nobody:wrong@localhost:{}/cloacina?sslmode=disable", pg_port())}).to_string(),
    };
    let bad = load_connector(root, PKG, "destination", &bad_cfg, &g).unwrap();
    assert!(
        !bad.check(&bad_cfg).unwrap().success,
        "a wrong password fails check"
    );

    let st = stream("pg_rt", SyncMode::FullRefresh, WriteMode::Append, None);
    let outcome = destination
        .write(
            &WriteContext { stream: st.clone() },
            vec![RecordBatch::Rows(rows(1..=3))],
        )
        .unwrap();
    match outcome.result {
        WriteResult::Ok(r) => assert_eq!(r.accepted, 3),
        WriteResult::Err(e) => panic!("write failed: {e}"),
    }

    let source = load_connector(
        root,
        PKG,
        "source",
        &Config {
            json: connector_config(r#"{"table":"pg_rt"}"#),
        },
        &g,
    )
    .unwrap();
    match source
        .discover(&Config {
            json: connector_config(""),
        })
        .unwrap()
    {
        DiscoverOutcome::Catalog(c) => {
            assert!(c.streams.iter().any(|s| s.name == "pg_rt"), "{c:?}")
        }
        DiscoverOutcome::Error(e) => panic!("discover failed: {e}"),
    }
    let (got, _) = read_all(&source, &read_ctx(&st, StreamState::default()));
    let mut ns: Vec<i64> = got.iter().map(|r| n_of(r)).collect();
    ns.sort();
    assert_eq!(
        ns,
        vec![1, 2, 3],
        "three rows round-tripped through postgres"
    );
}

#[test]
#[ignore = "needs the dev Postgres on 15432 (angreal services up) and the wasm32-wasip2 target"]
fn upsert_is_idempotent_and_incremental_resumes_with_the_delta() {
    reset_tables(&["pg_inc"]);
    let root = providers();
    let g = grants();
    let cfg = Config {
        json: connector_config(r#"{"table":"pg_inc"}"#),
    };
    let destination = load_connector(root, PKG, "destination", &cfg, &g).unwrap();
    let source = load_connector(root, PKG, "source", &cfg, &g).unwrap();

    let up = stream(
        "pg_inc",
        SyncMode::FullRefresh,
        WriteMode::Upsert {
            business_keys: vec!["n".into()],
        },
        None,
    );
    for _ in 0..2 {
        let out = destination
            .write(
                &WriteContext { stream: up.clone() },
                vec![RecordBatch::Rows(rows(1..=3))],
            )
            .unwrap();
        assert!(matches!(out.result, WriteResult::Ok(_)));
    }
    let (got, _) = read_all(&source, &read_ctx(&up, StreamState::default()));
    assert_eq!(got.len(), 3, "upsert twice keeps 3 rows, not 6");

    // Incremental by cursor `n`: the first read sees 1..3 and leaves cursor 3;
    // after two more rows land, a read from that cursor sees only 4..5.
    let inc = stream(
        "pg_inc",
        SyncMode::Incremental,
        WriteMode::Append,
        Some("n"),
    );
    let (first, state) = read_all(&source, &read_ctx(&inc, StreamState::default()));
    assert_eq!(first.len(), 3);
    assert_eq!(
        state.cursor.as_deref(),
        Some("3"),
        "cursor advanced to max n"
    );
    destination
        .write(
            &WriteContext { stream: up.clone() },
            vec![RecordBatch::Rows(rows(4..=5))],
        )
        .unwrap();
    let (delta, state2) = read_all(&source, &read_ctx(&inc, state));
    let mut ns: Vec<i64> = delta.iter().map(|r| n_of(r)).collect();
    ns.sort();
    assert_eq!(ns, vec![4, 5], "only the new rows, no re-emission of 1..3");
    assert_eq!(state2.cursor.as_deref(), Some("5"));
}

static RUNNER_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// The synchronous Postgres client cannot run on the async test's runtime.
async fn sql<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f).await.unwrap()
}

#[tokio::test]
#[ignore = "needs the dev Postgres on 15432 (angreal services up) and the wasm32-wasip2 target"]
async fn a_postgres_to_postgres_sync_runs_through_the_workflow_and_the_dispatcher() {
    let _guard = RUNNER_LOCK.lock().await;
    sql(|| {
        reset_tables(&["pg_src", "pg_dst"]);
        providers();
        let mut c = raw();
        c.batch_execute("CREATE TABLE pg_src (n BIGINT PRIMARY KEY, name TEXT)")
            .unwrap();
        for i in 1..=5 {
            c.execute(
                "INSERT INTO pg_src (n, name) VALUES ($1, $2)",
                &[&(i as i64), &format!("row {i}")],
            )
            .unwrap();
        }
    })
    .await;

    cloacina_sync::ensure_linked();
    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .build()
        .unwrap();
    let runner = DefaultRunner::with_config(":memory:", config)
        .await
        .expect("sqlite runner");
    let dal = Arc::new(DAL::new(runner.database().clone()));
    let dispatcher = Dispatcher::new(Arc::new(runner.clone()), dal.clone());

    let grants_json = json!({ "tcp": [format!("*:{}", pg_port())] });
    let src = stream(
        "pg_src",
        SyncMode::Incremental,
        WriteMode::Upsert {
            business_keys: vec!["n".into()],
        },
        Some("n"),
    );
    let mut p: HashMap<String, Value> = HashMap::new();
    p.insert(keys::SYNC.into(), json!("pg-to-pg"));
    p.insert(keys::SOURCE_PROVIDER.into(), json!(PKG));
    p.insert(keys::SOURCE_MEMBER.into(), json!("source"));
    p.insert(
        keys::SOURCE_CONFIG.into(),
        json!(connector_config(r#"{"table":"pg_src"}"#)),
    );
    p.insert(keys::SOURCE_GRANTS.into(), grants_json.clone());
    p.insert(keys::DESTINATION_PROVIDER.into(), json!(PKG));
    p.insert(keys::DESTINATION_MEMBER.into(), json!("destination"));
    p.insert(
        keys::DESTINATION_CONFIG.into(),
        json!(connector_config(r#"{"table":"pg_dst"}"#)),
    );
    p.insert(keys::DESTINATION_GRANTS.into(), grants_json);
    p.insert(
        keys::STREAM.into(),
        json!(serde_json::to_string(&src).unwrap()),
    );
    let def = SyncDefinition::new(p);

    let (_tx, stop) = tokio::sync::watch::channel(false);
    let outcome = dispatcher.run_pass(&def, stop.clone()).await.expect("pass");
    assert!(
        matches!(outcome, PassOutcome::Exhausted { .. }),
        "{outcome:?}"
    );

    let count = sql(|| {
        raw()
            .query_one("SELECT count(*) FROM pg_dst", &[])
            .unwrap()
            .get::<_, i64>(0)
    })
    .await;
    assert_eq!(count, 5, "five rows landed in the destination");
    let state = dal
        .sync_stream_state()
        .get("pg-to-pg", "pg_src", "")
        .await
        .unwrap()
        .expect("state");
    assert_eq!(state.cursor.as_deref(), Some("5"));
    assert!(state.exhausted.0);

    // Two more rows; a second pass moves only them, and upsert keeps the count honest.
    sql(|| {
        raw()
            .batch_execute("INSERT INTO pg_src (n, name) VALUES (6, 'row 6'), (7, 'row 7')")
            .unwrap()
    })
    .await;
    let outcome = dispatcher.run_pass(&def, stop).await.expect("second pass");
    assert!(matches!(outcome, PassOutcome::Exhausted { .. }));
    let count = sql(|| {
        raw()
            .query_one("SELECT count(*) FROM pg_dst", &[])
            .unwrap()
            .get::<_, i64>(0)
    })
    .await;
    assert_eq!(count, 7);
    let state = dal
        .sync_stream_state()
        .get("pg-to-pg", "pg_src", "")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.cursor.as_deref(), Some("7"));

    runner.shutdown().await.unwrap();
}
