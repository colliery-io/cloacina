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

//! The MSSQL connector (COLLIERY-I-0633 / CLOACINA-T-0984): its conformance
//! file, and one MSSQL to Postgres sync through the Sync workflow and the
//! dispatcher.
//!
//! Ignored by default. They need SQL Server (`docker compose -f
//! .angreal/docker-compose.yaml --profile mssql up -d --wait mssql`), the dev
//! Postgres on 15432, and the `wasm32-wasip2` target. The Postgres integration
//! lane runs them.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use cloacina::dal::DAL;
use cloacina::registry::loader::set_provider_search_path;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina_constructor_contract::sync::{ConfiguredStream, MappingSpec, SyncMode, WriteMode};
use cloacina_sync::dispatcher::{Dispatcher, PassOutcome, SyncDefinition};
use cloacina_sync::params::keys;
use cloacina_testing::connectors::conformance;
use cloacina_testing::connectors::stage_connector;
use serde_json::{Value, json};

mod support;
use support::{ConnectorHost, mssql, repo};

#[test]
#[ignore = "needs SQL Server (compose profile mssql) and the wasm32-wasip2 target"]
fn mssql_conforms() {
    let report = conformance::run(
        &repo("connectors/mssql/conformance.toml"),
        &ConnectorHost::new("mssql"),
        Some(&mssql::Mssql),
    );
    report.assert_passed();
    println!("{report}");
}

fn pg_url() -> String {
    std::env::var("CLOACINA_TEST_PG_URL")
        .unwrap_or_else(|_| "postgres://cloacina:cloacina@localhost:15432/cloacina".to_string())
}

fn pg() -> postgres::Client {
    postgres::Client::connect(&pg_url(), postgres::NoTls).expect("raw pg client")
}

/// Both connectors staged once, under the provider search path the Sync
/// workflow loads from.
fn providers() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!(
            "cloacina-sync-mssql-providers-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        stage_connector(&dir, &repo("connectors/mssql"));
        stage_connector(&dir, &repo("connectors/postgres"));
        set_provider_search_path(dir.clone());
        dir
    })
}

/// The synchronous clients cannot run on the async test's runtime.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f).await.unwrap()
}

#[tokio::test]
#[ignore = "needs SQL Server (compose profile mssql), the dev Postgres on 15432 and the wasm32-wasip2 target"]
async fn an_mssql_to_postgres_sync_runs_through_the_workflow_and_the_dispatcher() {
    blocking(|| {
        providers();
        mssql::batch(
            "master",
            "IF DB_ID('cloacina_sync') IS NULL CREATE DATABASE cloacina_sync",
        )
        .unwrap();
        mssql::batch(
            "cloacina_sync",
            "DROP TABLE IF EXISTS dbo.sync_orders; \
             CREATE TABLE dbo.sync_orders (n INT NOT NULL PRIMARY KEY, name NVARCHAR(100) NOT NULL); \
             INSERT INTO dbo.sync_orders (n, name) VALUES \
             (1, 'row 1'), (2, 'row 2'), (3, 'row 3'), (4, 'row 4'), (5, 'row 5'), \
             (6, 'row 6'), (7, 'row 7'), (8, 'row 8'), (9, 'row 9');",
        )
        .unwrap();
        pg().batch_execute("DROP TABLE IF EXISTS mssql_dst").unwrap();
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

    let stream = ConfiguredStream {
        stream: "dbo.sync_orders".into(),
        sync_mode: SyncMode::Incremental,
        cursor_field: Some("n".into()),
        primary_key: Some(vec!["n".into()]),
        write_mode: WriteMode::Upsert {
            business_keys: vec!["n".into()],
        },
        mapping: MappingSpec::default(),
    };
    let mut p: HashMap<String, Value> = HashMap::new();
    p.insert(keys::SYNC.into(), json!("mssql-to-pg"));
    p.insert(
        keys::SOURCE_PROVIDER.into(),
        json!("cloacina-connector-mssql"),
    );
    p.insert(keys::SOURCE_MEMBER.into(), json!("source"));
    p.insert(
        keys::SOURCE_CONFIG.into(),
        json!(
            json!({
                "host": mssql::host(), "port": mssql::port(), "database": "cloacina_sync",
                "user": "sa", "password": mssql::PASSWORD, "tls_mode": "disable",
            })
            .to_string()
        ),
    );
    p.insert(
        keys::SOURCE_GRANTS.into(),
        json!({ "tcp": [format!("*:{}", mssql::port())] }),
    );
    p.insert(
        keys::DESTINATION_PROVIDER.into(),
        json!("cloacina-connector-postgres"),
    );
    p.insert(keys::DESTINATION_MEMBER.into(), json!("destination"));
    p.insert(
        keys::DESTINATION_CONFIG.into(),
        json!(
            json!({ "url": format!("{}?sslmode=disable", pg_url()), "table": "mssql_dst" })
                .to_string()
        ),
    );
    p.insert(keys::DESTINATION_GRANTS.into(), json!({ "tcp": ["*"] }));
    p.insert(
        keys::STREAM.into(),
        json!(serde_json::to_string(&stream).unwrap()),
    );
    let def = SyncDefinition::new(p);

    let count = || {
        blocking(|| {
            pg().query_one("SELECT count(*) FROM mssql_dst", &[])
                .unwrap()
                .get::<_, i64>(0)
        })
    };

    let (_tx, stop) = tokio::sync::watch::channel(false);
    let outcome = dispatcher.run_pass(&def, stop.clone()).await.expect("pass");
    assert!(
        matches!(outcome, PassOutcome::Exhausted { .. }),
        "{outcome:?}"
    );
    assert_eq!(count().await, 9, "nine rows landed in Postgres");
    let state = dal
        .sync_stream_state()
        .get("mssql-to-pg", "dbo.sync_orders", "")
        .await
        .unwrap()
        .expect("state");
    assert_eq!(state.cursor.as_deref(), Some("9"));

    // Three more rows past 9; the second pass moves only them.
    blocking(|| {
        mssql::batch(
            "cloacina_sync",
            "INSERT INTO dbo.sync_orders (n, name) VALUES (10, 'row 10'), (11, 'row 11'), (12, 'row 12')",
        )
        .unwrap()
    })
    .await;
    let outcome = dispatcher.run_pass(&def, stop).await.expect("second pass");
    assert!(matches!(outcome, PassOutcome::Exhausted { .. }));
    assert_eq!(count().await, 12);
    let state = dal
        .sync_stream_state()
        .get("mssql-to-pg", "dbo.sync_orders", "")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.cursor.as_deref(), Some("12"));

    runner.shutdown().await.unwrap();
}
