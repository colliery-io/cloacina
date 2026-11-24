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

//! The Snowflake connector against the mock SQL API (COLLIERY-I-0633 /
//! CLOACINA-T-0986): its conformance file, and a Postgres to Snowflake and a
//! Snowflake to Postgres sync through the Sync workflow and the dispatcher,
//! with the key-pair JWT added on egress.
//!
//! The conformance run needs only the `wasm32-wasip2` target. The syncs are
//! ignored by default: they need the dev Postgres on 15432 too, and the
//! Postgres integration lane runs them. A live Snowflake run is opt-in and not
//! here.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use cloacina::dal::DAL;
use cloacina::registry::loader::set_provider_search_path;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina_constructor_contract::sync::{ConfiguredStream, MappingSpec, SyncMode, WriteMode};
use cloacina_sync::dispatcher::{Dispatcher, PassOutcome, SyncDefinition};
use cloacina_sync::params::keys;
use cloacina_testing::connectors::conformance;
use cloacina_testing::connectors::snowflake_mock::{MockOptions, SnowflakeMock};
use cloacina_testing::connectors::stage_connector;
use cloacina_workflow::secret::MapSecretResolver;
use serde_json::{Value, json};

mod support;
use support::{ConnectorHost, repo};

const ACCOUNT: &str = "testorg-sync";
const USER: &str = "sync_test";
/// The public key fingerprint of `crates/cloacina/tests/fixtures/test_rsa_key.pem`,
/// a throwaway test key: `openssl pkey -pubout -outform DER | openssl dgst
/// -sha256 -binary | base64`.
const FINGERPRINT: &str = "SHA256:BgiJGcINPQuBjB0MFoHV/SW8vP5pzN90TXJe8SpDZtI=";

fn mock(asynchronous: bool, partition_rows: usize) -> SnowflakeMock {
    SnowflakeMock::start(MockOptions {
        fingerprint: Some(FINGERPRINT.to_string()),
        asynchronous,
        partition_rows,
        ..MockOptions::new(ACCOUNT, USER)
    })
}

#[test]
fn snowflake_conforms() {
    let mock = mock(true, 5);
    let vars = BTreeMap::from([
        ("MOCK_URL".to_string(), mock.base_url()),
        ("MOCK_GRANT".to_string(), mock.grant()),
    ]);
    let report = conformance::run_with(
        &repo("connectors/snowflake/conformance.toml"),
        &vars,
        &ConnectorHost::new("snowflake"),
        Some(&mock),
    );
    report.assert_passed();
    // The one rejected request is the check that signed with another key.
    let rejected = mock.rejected();
    assert_eq!(rejected.len(), 1, "{rejected:?}");
    assert!(rejected[0].contains("is not for the key"), "{rejected:?}");
    println!("{report}");
}

fn pg_url() -> String {
    std::env::var("CLOACINA_TEST_PG_URL")
        .unwrap_or_else(|_| "postgres://cloacina:cloacina@localhost:15432/cloacina".to_string())
}

fn pg() -> postgres::Client {
    postgres::Client::connect(&pg_url(), postgres::NoTls).expect("raw pg client")
}

fn providers() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!(
            "cloacina-sync-snowflake-providers-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        stage_connector(&dir, &repo("connectors/snowflake"));
        stage_connector(&dir, &repo("connectors/postgres"));
        set_provider_search_path(dir.clone());
        dir
    })
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f).await.unwrap()
}

static RUNNER_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// A runner whose secret resolver holds the Snowflake key as `snowflake-key`.
async fn runner() -> DefaultRunner {
    let key = std::fs::read_to_string(repo("crates/cloacina/tests/fixtures/test_rsa_key.pem"))
        .expect("the test key");
    let secrets = BTreeMap::from([(
        "snowflake-key".to_string(),
        BTreeMap::from([("private_key".to_string(), key)]),
    )]);
    cloacina_sync::ensure_linked();
    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .build()
        .unwrap();
    DefaultRunner::builder()
        .database_url(":memory:")
        .with_config(config)
        .secret_resolver(Arc::new(MapSecretResolver::new(secrets)))
        .build()
        .await
        .expect("sqlite runner")
}

fn snowflake_side(mock: &SnowflakeMock, table: &str) -> (Value, Value, Value) {
    (
        json!(
            json!({
                "account": ACCOUNT, "database": "SYNC_DB", "schema": "PUBLIC",
                "warehouse": "SYNC_WH", "base_url": mock.base_url(), "table": table,
            })
            .to_string()
        ),
        json!({ "http": [mock.grant()] }),
        json!(
            json!({
                "kind": "snowflake_keypair_jwt", "secret": "snowflake-key",
                "account": ACCOUNT, "user": USER,
            })
            .to_string()
        ),
    )
}

fn postgres_side(table: &str) -> (Value, Value) {
    (
        json!(
            json!({ "url": format!("{}?sslmode=disable", pg_url()), "table": table }).to_string()
        ),
        json!({ "tcp": ["*"] }),
    )
}

fn incremental(stream: &str, key: &str) -> Value {
    json!(
        serde_json::to_string(&ConfiguredStream {
            stream: stream.into(),
            sync_mode: SyncMode::Incremental,
            cursor_field: Some(key.into()),
            primary_key: Some(vec![key.into()]),
            write_mode: WriteMode::Upsert {
                business_keys: vec![key.into()],
            },
            mapping: MappingSpec::default(),
        })
        .unwrap()
    )
}

#[tokio::test]
#[ignore = "needs the dev Postgres on 15432 and the wasm32-wasip2 target"]
async fn a_postgres_to_snowflake_sync_writes_through_the_mock() {
    let _guard = RUNNER_LOCK.lock().await;
    let mock = mock(false, 0);
    blocking(|| {
        providers();
        let mut c = pg();
        c.batch_execute(
            "DROP TABLE IF EXISTS sf_src; \
             CREATE TABLE sf_src (n BIGINT PRIMARY KEY, name TEXT); \
             INSERT INTO sf_src (n, name) SELECT g, 'row ' || g FROM generate_series(1, 5) g;",
        )
        .unwrap();
    })
    .await;
    let runner = runner().await;
    let dal = Arc::new(DAL::new(runner.database().clone()));
    let dispatcher = Dispatcher::new(Arc::new(runner.clone()), dal.clone());

    let (src_cfg, src_grants) = postgres_side("sf_src");
    let (dst_cfg, dst_grants, dst_cred) = snowflake_side(&mock, "SF_DST");
    let p: HashMap<String, Value> = HashMap::from([
        (keys::SYNC.to_string(), json!("pg-to-sf")),
        (
            keys::SOURCE_PROVIDER.to_string(),
            json!("cloacina-connector-postgres"),
        ),
        (keys::SOURCE_MEMBER.to_string(), json!("source")),
        (keys::SOURCE_CONFIG.to_string(), src_cfg),
        (keys::SOURCE_GRANTS.to_string(), src_grants),
        (
            keys::DESTINATION_PROVIDER.to_string(),
            json!("cloacina-connector-snowflake"),
        ),
        (keys::DESTINATION_MEMBER.to_string(), json!("destination")),
        (keys::DESTINATION_CONFIG.to_string(), dst_cfg),
        (keys::DESTINATION_GRANTS.to_string(), dst_grants),
        (keys::DESTINATION_CREDENTIAL.to_string(), dst_cred),
        (keys::STREAM.to_string(), incremental("sf_src", "n")),
    ]);
    let def = SyncDefinition::new(p);

    let (_tx, stop) = tokio::sync::watch::channel(false);
    let outcome = dispatcher.run_pass(&def, stop).await.expect("pass");
    assert!(
        matches!(outcome, PassOutcome::Exhausted { .. }),
        "{outcome:?}"
    );
    let rows = mock.rows("SF_DST").expect("the destination table");
    assert_eq!(rows.len(), 5, "{rows:?}");
    assert!(
        rows.contains(&json!({ "n": 3, "name": "row 3" })),
        "{rows:?}"
    );
    assert!(mock.rejected().is_empty(), "{:?}", mock.rejected());
    let state = dal
        .sync_stream_state()
        .get("pg-to-sf", "sf_src", "")
        .await
        .unwrap()
        .expect("state");
    assert_eq!(state.cursor.as_deref(), Some("5"));
    runner.shutdown().await.unwrap();
}

#[tokio::test]
#[ignore = "needs the dev Postgres on 15432 and the wasm32-wasip2 target"]
async fn a_snowflake_to_postgres_sync_reads_through_the_mock() {
    let _guard = RUNNER_LOCK.lock().await;
    let mock = mock(true, 4);
    let orders = |n: i64| -> Vec<Vec<Value>> {
        (1..=n)
            .map(|i| vec![json!(i), json!(format!("order {i}"))])
            .collect()
    };
    mock.load_table("ORDERS", &["ID", "NAME"], orders(9));
    blocking(|| {
        providers();
        pg().batch_execute("DROP TABLE IF EXISTS sf_dst").unwrap();
    })
    .await;
    let runner = runner().await;
    let dal = Arc::new(DAL::new(runner.database().clone()));
    let dispatcher = Dispatcher::new(Arc::new(runner.clone()), dal.clone());

    let (src_cfg, src_grants, src_cred) = snowflake_side(&mock, "ORDERS");
    let (dst_cfg, dst_grants) = postgres_side("sf_dst");
    let p: HashMap<String, Value> = HashMap::from([
        (keys::SYNC.to_string(), json!("sf-to-pg")),
        (
            keys::SOURCE_PROVIDER.to_string(),
            json!("cloacina-connector-snowflake"),
        ),
        (keys::SOURCE_MEMBER.to_string(), json!("source")),
        (keys::SOURCE_CONFIG.to_string(), src_cfg),
        (keys::SOURCE_GRANTS.to_string(), src_grants),
        (keys::SOURCE_CREDENTIAL.to_string(), src_cred),
        (
            keys::DESTINATION_PROVIDER.to_string(),
            json!("cloacina-connector-postgres"),
        ),
        (keys::DESTINATION_MEMBER.to_string(), json!("destination")),
        (keys::DESTINATION_CONFIG.to_string(), dst_cfg),
        (keys::DESTINATION_GRANTS.to_string(), dst_grants),
        (keys::STREAM.to_string(), incremental("orders", "id")),
    ]);
    let def = SyncDefinition::new(p);
    let count = || {
        blocking(|| {
            pg().query_one("SELECT count(*) FROM sf_dst", &[])
                .unwrap()
                .get::<_, i64>(0)
        })
    };
    let cursor = || async {
        dal.sync_stream_state()
            .get("sf-to-pg", "orders", "")
            .await
            .unwrap()
            .expect("state")
            .cursor
    };

    let (_tx, stop) = tokio::sync::watch::channel(false);
    let outcome = dispatcher.run_pass(&def, stop.clone()).await.expect("pass");
    assert!(
        matches!(outcome, PassOutcome::Exhausted { .. }),
        "{outcome:?}"
    );
    assert_eq!(count().await, 9);
    assert_eq!(cursor().await.as_deref(), Some("\"9\""));

    // Three more rows past 9; the second pass reads only them.
    mock.load_table("ORDERS", &["ID", "NAME"], orders(12));
    let outcome = dispatcher.run_pass(&def, stop).await.expect("second pass");
    assert!(matches!(outcome, PassOutcome::Exhausted { .. }));
    assert_eq!(count().await, 12);
    assert_eq!(cursor().await.as_deref(), Some("\"12\""));
    let last = mock.statements().last().cloned().unwrap_or_default();
    assert!(last.contains("WHERE \"ID\" > ?"), "{last}");
    assert!(mock.rejected().is_empty(), "{:?}", mock.rejected());
    runner.shutdown().await.unwrap();
}
