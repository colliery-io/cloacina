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

//! A declarative sync end to end (COLLIERY-I-0634 / CLOACINA-T-0993): the
//! corpus's `github` definition, its `repositories` stream, synced to
//! Postgres through the Sync workflow and the dispatcher. The API is the mock
//! JSON API standing in for GitHub; the token lives in cloacina's secret
//! store, is read on the runner, and is added to each request on the host.
//!
//! Ignored by default: it needs the dev Postgres on 15432 and the
//! `wasm32-wasip2` target. The Postgres integration lane runs it.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use cloacina::UniversalUuid;
use cloacina::dal::DAL;
use cloacina::registry::loader::set_provider_search_path;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::security::{SecretStore, SecretStoreResolver};
use cloacina_constructor_contract::sync::{
    ConfiguredStream, FieldType, MappingSpec, SyncMode, WriteMode,
};
use cloacina_sync::corpus::Definitions;
use cloacina_sync::definition::compile_source;
use cloacina_sync::dispatcher::{Dispatcher, PassOutcome, SyncDefinition};
use cloacina_sync::params::keys;
use cloacina_testing::connectors::rest_mock::{Endpoint, Filter, Paging, RestMock};
use cloacina_testing::connectors::stage_connector;
use serde_json::{Map, Value, json};

mod support;
use support::repo;

const TOKEN: &str = "ghp_test-token";

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
            "cloacina-sync-declarative-providers-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        stage_connector(&dir, &repo("connectors/rest"));
        stage_connector(&dir, &repo("connectors/postgres"));
        set_provider_search_path(dir.clone());
        dir
    })
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f).await.unwrap()
}

/// A repository record with every field the definition's schema declares, as
/// the GitHub API would send it.
fn repository(n: i64) -> Value {
    let defs = Definitions::corpus().unwrap();
    let mut user = Map::new();
    user.insert("base_url".into(), json!("http://x"));
    let schema = compile_source(
        defs.source("github").unwrap(),
        "repositories",
        &user,
        Some("s"),
    )
    .unwrap()
    .schema;
    let mut rec = Map::new();
    for f in &schema.fields {
        let v = match f.field_type {
            FieldType::Integer => json!(n),
            FieldType::Float => json!(1.5),
            FieldType::Boolean => json!(false),
            FieldType::Timestamp => json!(format!("2026-01-{n:02}T00:00:00Z")),
            _ => json!(format!("{}-{n}", f.name)),
        };
        rec.insert(f.name.clone(), v);
    }
    rec.insert("id".into(), json!(n));
    rec.insert(
        "updated_at".into(),
        json!(format!("2026-01-{n:02}T00:00:00Z")),
    );
    Value::Object(rec)
}

fn serve(mock: &RestMock, n: i64) {
    mock.serve(
        "/user/repos",
        Endpoint {
            records: (1..=n).map(repository).collect(),
            paging: Paging::Page {
                page_param: "page".into(),
                size_param: "per_page".into(),
            },
            filter: Some(Filter {
                param: "since".into(),
                field: "updated_at".into(),
            }),
            require_header: Some(("Authorization".into(), format!("Bearer {TOKEN}"))),
            ..Default::default()
        },
    );
}

#[tokio::test]
#[ignore = "needs the dev Postgres on 15432 and the wasm32-wasip2 target"]
async fn a_corpus_source_syncs_to_postgres_with_a_stored_secret() {
    let mock = RestMock::start();
    serve(&mock, 3);
    blocking(|| {
        providers();
        pg().batch_execute("DROP TABLE IF EXISTS gh_repos").unwrap();
    })
    .await;

    // The token in cloacina's secret store, read by the runner's resolver.
    let url = format!(
        "file:declarative_sync_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4()
    );
    let seed_db = cloacina::Database::new(&url, "", 5);
    seed_db.run_migrations().await.expect("migrations");
    let org_id = UniversalUuid::new_v4();
    let kek = vec![9u8; 32];
    SecretStore::new(DAL::new(seed_db.clone()))
        .create_secret(
            org_id,
            "github-token",
            &BTreeMap::from([("api_key".to_string(), TOKEN.to_string())]),
            &kek,
        )
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
        stream: "repositories".into(),
        sync_mode: SyncMode::Incremental,
        cursor_field: Some("updated_at".into()),
        primary_key: Some(vec!["id".into()]),
        write_mode: WriteMode::Upsert {
            business_keys: vec!["id".into()],
        },
        mapping: MappingSpec::default(),
    };
    // The source names the corpus definition, not a provider: no provider,
    // no grants, no credential spec. The definition supplies them all.
    let p: HashMap<String, Value> = HashMap::from([
        (keys::SYNC.to_string(), json!("github-to-pg")),
        (keys::SOURCE_DEFINITION.to_string(), json!("github")),
        (
            keys::SOURCE_CONFIG.to_string(),
            json!({ "base_url": mock.base_url() }),
        ),
        (keys::SOURCE_SECRET.to_string(), json!("github-token")),
        (
            keys::DESTINATION_PROVIDER.to_string(),
            json!("cloacina-connector-postgres"),
        ),
        (keys::DESTINATION_MEMBER.to_string(), json!("destination")),
        (
            keys::DESTINATION_CONFIG.to_string(),
            json!(
                json!({ "url": format!("{}?sslmode=disable", pg_url()), "table": "gh_repos" })
                    .to_string()
            ),
        ),
        (
            keys::DESTINATION_GRANTS.to_string(),
            json!({ "tcp": ["*"] }),
        ),
        (
            keys::STREAM.to_string(),
            json!(serde_json::to_string(&stream).unwrap()),
        ),
    ]);
    // What the server checks when the sync definition is created.
    let as_map: Map<String, Value> = p.clone().into_iter().collect();
    cloacina_sync::params::validate("github-to-pg", &as_map).expect("the definition validates");
    let def = SyncDefinition::new(p);

    let count = || {
        blocking(|| {
            pg().query_one("SELECT count(*) FROM gh_repos", &[])
                .unwrap()
                .get::<_, i64>(0)
        })
    };
    let cursor = || async {
        dal.sync_stream_state()
            .get("github-to-pg", "repositories", "")
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
    assert_eq!(count().await, 3, "three repositories landed in Postgres");
    assert_eq!(
        cursor().await.as_deref(),
        Some(json!("2026-01-03T00:00:00Z").to_string().as_str())
    );

    // Two more repositories; the second pass sends `since` and reads only them.
    serve(&mock, 5);
    let outcome = dispatcher.run_pass(&def, stop).await.expect("second pass");
    assert!(matches!(outcome, PassOutcome::Exhausted { .. }));
    assert_eq!(count().await, 5);
    let last = mock.requests_to("/user/repos").pop().expect("a request");
    assert_eq!(
        last.params.get("since").map(String::as_str),
        Some("2026-01-03T00:00:00Z")
    );
    assert_eq!(
        last.headers.get("authorization").map(String::as_str),
        Some(format!("Bearer {TOKEN}").as_str()),
        "the stored token was added on the host"
    );
    runner.shutdown().await.unwrap();
}

#[test]
fn a_bad_declarative_side_is_refused_before_it_is_created() {
    let stream = json!(
        serde_json::to_string(&ConfiguredStream {
            stream: "repos".into(),
            sync_mode: SyncMode::FullRefresh,
            cursor_field: None,
            primary_key: None,
            write_mode: WriteMode::Append,
            mapping: MappingSpec::default(),
        })
        .unwrap()
    );
    let params = |def: &str| -> Map<String, Value> {
        [
            (keys::SOURCE_DEFINITION.to_string(), json!(def)),
            (keys::DESTINATION_PROVIDER.to_string(), json!("p")),
            (keys::DESTINATION_MEMBER.to_string(), json!("destination")),
            (keys::STREAM.to_string(), stream.clone()),
        ]
        .into_iter()
        .collect()
    };
    let err = cloacina_sync::params::validate("s", &params("githubb")).unwrap_err();
    assert!(err.to_string().contains("githubb"), "{err}");
    // A stream the definition does not have is named with the ones it has.
    let err = cloacina_sync::params::validate("s", &params("github")).unwrap_err();
    assert!(err.to_string().contains("repositories"), "{err}");
}
