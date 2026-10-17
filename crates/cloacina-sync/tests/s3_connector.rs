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

//! The S3 connector (COLLIERY-I-0633 / CLOACINA-T-0985): its conformance file,
//! and one S3 to Postgres sync through the Sync workflow and the dispatcher,
//! with the MinIO keys in cloacina's secret store and SigV4 added on egress.
//!
//! Ignored by default. They need MinIO (`docker compose -f
//! .angreal/docker-compose.yaml --profile minio up -d --wait minio`), the dev
//! Postgres on 15432, and the `wasm32-wasip2` target. The Postgres integration
//! lane runs them.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use cloacina::UniversalUuid;
use cloacina::dal::DAL;
use cloacina::registry::loader::set_provider_search_path;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::security::{SecretStore, SecretStoreResolver};
use cloacina_constructor_contract::sync::{ConfiguredStream, MappingSpec, SyncMode, WriteMode};
use cloacina_sync::dispatcher::{Dispatcher, PassOutcome, SyncDefinition};
use cloacina_sync::params::keys;
use cloacina_testing::connectors::conformance;
use cloacina_testing::connectors::stage_connector;
use serde_json::{Value, json};

mod support;
use support::{ConnectorHost, minio, repo};

#[test]
#[ignore = "needs MinIO (compose profile minio) and the wasm32-wasip2 target"]
fn s3_conforms() {
    let report = conformance::run(
        &repo("connectors/s3/conformance.toml"),
        &ConnectorHost::new("s3"),
        Some(&minio::Minio),
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
            "cloacina-sync-s3-providers-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        stage_connector(&dir, &repo("connectors/s3"));
        stage_connector(&dir, &repo("connectors/postgres"));
        set_provider_search_path(dir.clone());
        dir
    })
}

/// The synchronous clients cannot run on the async test's runtime.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f).await.unwrap()
}

const BUCKET: &str = "cloacina-sync";

fn object(n: i64) -> String {
    format!("{{\"n\":{n},\"name\":\"row {n}\"}}\n")
}

#[tokio::test]
#[ignore = "needs MinIO (compose profile minio), the dev Postgres on 15432 and the wasm32-wasip2 target"]
async fn an_s3_to_postgres_sync_signs_with_a_stored_secret() {
    blocking(|| {
        providers();
        minio::create_bucket(BUCKET).unwrap();
        // A fresh prefix per run, so earlier runs' objects stay out of the pass.
        for n in 1..=3 {
            minio::put_object(BUCKET, &format!("{}/{n:03}.ndjson", run_prefix()), &object(n))
                .unwrap();
        }
        pg().batch_execute("DROP TABLE IF EXISTS s3_dst").unwrap();
    })
    .await;

    // The MinIO keys live in cloacina's secret store, encrypted under a KEK; the
    // runner resolves them for the read task through the store.
    let url = format!(
        "file:s3_sync_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4()
    );
    let seed_db = cloacina::Database::new(&url, "", 5);
    seed_db.run_migrations().await.expect("migrations");
    let org_id = UniversalUuid::new_v4();
    let kek = vec![7u8; 32];
    let fields = BTreeMap::from([
        ("access_key_id".to_string(), minio::ACCESS_KEY.to_string()),
        (
            "secret_access_key".to_string(),
            minio::SECRET_KEY.to_string(),
        ),
    ]);
    SecretStore::new(DAL::new(seed_db.clone()))
        .create_secret(org_id, "minio-keys", &fields, &kek)
        .await
        .expect("store the secret");
    let resolver =
        SecretStoreResolver::new(SecretStore::new(DAL::new(seed_db.clone())), org_id, kek)
            .into_arc();

    cloacina_sync::ensure_linked();
    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .build()
        .unwrap();
    let runner = DefaultRunner::builder()
        .database_url(&url)
        .with_config(config)
        .secret_resolver(resolver)
        .build()
        .await
        .expect("sqlite runner");
    let dal = Arc::new(DAL::new(runner.database().clone()));
    let dispatcher = Dispatcher::new(Arc::new(runner.clone()), dal.clone());

    let stream = ConfiguredStream {
        stream: "objects".into(),
        sync_mode: SyncMode::Incremental,
        cursor_field: None,
        primary_key: Some(vec!["n".into()]),
        write_mode: WriteMode::Upsert {
            business_keys: vec!["n".into()],
        },
        mapping: MappingSpec::default(),
    };
    let mut p: HashMap<String, Value> = HashMap::new();
    p.insert(keys::SYNC.into(), json!("s3-to-pg"));
    p.insert(keys::SOURCE_PROVIDER.into(), json!("cloacina-connector-s3"));
    p.insert(keys::SOURCE_MEMBER.into(), json!("source"));
    p.insert(
        keys::SOURCE_CONFIG.into(),
        json!(
            json!({ "endpoint": minio::endpoint(), "bucket": BUCKET, "prefix": format!("{}/", run_prefix()) })
                .to_string()
        ),
    );
    p.insert(
        keys::SOURCE_GRANTS.into(),
        json!({ "http": [format!("{}:{}", minio::host(), minio::port())] }),
    );
    p.insert(
        keys::SOURCE_CREDENTIAL.into(),
        json!(
            json!({ "kind": "aws_sigv4", "secret": "minio-keys", "region": minio::REGION })
                .to_string()
        ),
    );
    p.insert(
        keys::DESTINATION_PROVIDER.into(),
        json!("cloacina-connector-postgres"),
    );
    p.insert(keys::DESTINATION_MEMBER.into(), json!("destination"));
    p.insert(
        keys::DESTINATION_CONFIG.into(),
        json!(
            json!({ "url": format!("{}?sslmode=disable", pg_url()), "table": "s3_dst" })
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
            pg().query_one("SELECT count(*) FROM s3_dst", &[])
                .unwrap()
                .get::<_, i64>(0)
        })
    };
    let cursor = || async {
        dal.sync_stream_state()
            .get("s3-to-pg", "objects", "")
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
    assert_eq!(count().await, 3, "three objects' rows landed in Postgres");
    assert_eq!(
        cursor().await,
        Some(json!(format!("{}/003.ndjson", run_prefix())).to_string())
    );

    // One more object; the second pass reads only it.
    blocking(|| {
        minio::put_object(BUCKET, &format!("{}/004.ndjson", run_prefix()), &object(4)).unwrap()
    })
    .await;
    let outcome = dispatcher.run_pass(&def, stop).await.expect("second pass");
    assert!(matches!(outcome, PassOutcome::Exhausted { .. }));
    assert_eq!(count().await, 4);
    assert_eq!(
        cursor().await,
        Some(json!(format!("{}/004.ndjson", run_prefix())).to_string())
    );

    runner.shutdown().await.unwrap();
}

/// This process's object prefix.
fn run_prefix() -> &'static str {
    static P: OnceLock<String> = OnceLock::new();
    P.get_or_init(|| format!("run-{}", uuid::Uuid::new_v4()))
}
