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

//! Resume on the embedded runner (COLLIERY-I-0632 / CLOACINA-T-0980): the
//! write task dies after the destination accepted the segment and before the
//! completion writes, the exact window a crash must not lose a record in. The
//! retry writes the segment again, upsert absorbs it, the cursor advances once.
//!
//! The fault is deterministic: the `fault` param, honoured only with the
//! `fault-injection` feature the tests enable. On SQLite the echo connector
//! shows the retry through the write task's attempt count; against the dev
//! Postgres (ignored unless run with `--ignored`) the destination row count
//! shows upsert absorbing the second write.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use cloacina::dal::DAL;
use cloacina::registry::loader::set_provider_search_path;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina_constructor_contract::sync::{ConfiguredStream, MappingSpec, SyncMode, WriteMode};
use cloacina_sync::dispatcher::{Dispatcher, PassOutcome, SyncDefinition};
use cloacina_sync::params::keys;
use serde_json::{Value, json};

fn providers() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!(
            "cloacina-sync-resume-providers-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let examples =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/constructor-contract");
        cloacina_testing::connectors::stage_connector(
            &dir,
            &examples.join("connector-echo-fixture"),
        );
        if std::env::var_os("CLOACINA_TEST_PG").is_some() {
            let pg = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../connectors/postgres");
            cloacina_testing::connectors::stage_connector(&dir, &pg);
        }
        set_provider_search_path(dir.clone());
        dir
    })
}

static RUNNER_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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

fn echo_definition(sync: &str, fault: &str) -> SyncDefinition {
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
    p.insert(
        keys::SOURCE_CONFIG.into(),
        json!(r#"{"segments":2,"rows":["{\"id\":1}","{\"id\":2}"]}"#),
    );
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
    p.insert(keys::FAULT.into(), json!(fault));
    SyncDefinition::new(p)
}

#[tokio::test]
async fn a_write_task_that_dies_after_the_write_is_retried_and_the_cursor_advances_once() {
    let _guard = RUNNER_LOCK.lock().await;
    providers();
    let runner = runner().await;
    let dal = Arc::new(DAL::new(runner.database().clone()));
    let dispatcher = Dispatcher::new(Arc::new(runner.clone()), dal.clone());
    let (_tx, stop) = tokio::sync::watch::channel(false);

    let def = echo_definition("dies-once", "fail_after_write_once");
    let outcome = dispatcher
        .run_pass(&def, stop)
        .await
        .expect("pass survives the fault");
    assert_eq!(outcome, PassOutcome::Exhausted { segments: 2 });

    // The cursor is the second segment's, committed once per segment.
    let state = dal
        .sync_stream_state()
        .get("dies-once", "echo", "")
        .await
        .unwrap()
        .expect("state");
    assert_eq!(state.cursor.as_deref(), Some("\"2\""));
    assert!(state.exhausted.0);

    // The first segment's write task ran twice: the injected death, then the
    // retry that landed the completion writes. The second segment's ran once.
    let runs: Vec<_> = dal
        .workflow_execution()
        .list_recent(10)
        .await
        .unwrap()
        .into_iter()
        .filter(|e| e.workflow_name == cloacina_sync::WORKFLOW_NAME)
        .collect();
    assert_eq!(runs.len(), 2, "{runs:?}");
    let mut attempts = Vec::new();
    for run in &runs {
        let tasks = dal
            .task_execution()
            .get_all_tasks_for_workflow(run.id)
            .await
            .unwrap();
        let write = tasks
            .iter()
            .find(|t| t.task_name.ends_with("::write"))
            .expect("write task");
        assert_eq!(write.status, "Completed");
        attempts.push(write.attempt);
    }
    attempts.sort();
    assert_eq!(
        attempts,
        vec![1, 2],
        "one write task died once and was retried; the other ran clean"
    );

    runner.shutdown().await.unwrap();
}

#[tokio::test]
#[ignore = "needs the dev Postgres on 15432 and CLOACINA_TEST_PG=1 (angreal services up)"]
async fn against_postgres_the_second_write_is_absorbed_by_upsert() {
    let _guard = RUNNER_LOCK.lock().await;
    providers();
    let url = std::env::var("CLOACINA_TEST_PG_URL")
        .unwrap_or_else(|_| "postgres://cloacina:cloacina@localhost:15432/cloacina".to_string());
    let url_plain = format!("{url}?sslmode=disable");
    let port: u16 = url
        .rsplit(':')
        .next()
        .and_then(|t| t.split('/').next())
        .and_then(|p| p.parse().ok())
        .unwrap_or(5432);
    {
        let u = url.clone();
        tokio::task::spawn_blocking(move || {
            let mut c = postgres::Client::connect(&u, postgres::NoTls).unwrap();
            c.batch_execute("DROP TABLE IF EXISTS rs_src; DROP TABLE IF EXISTS rs_dst; CREATE TABLE rs_src (n BIGINT PRIMARY KEY, name TEXT)").unwrap();
            for i in 1..=4 {
                c.execute("INSERT INTO rs_src (n, name) VALUES ($1, $2)", &[&(i as i64), &format!("row {i}")]).unwrap();
            }
        })
        .await
        .unwrap();
    }
    let runner = runner().await;
    let dal = Arc::new(DAL::new(runner.database().clone()));
    let dispatcher = Dispatcher::new(Arc::new(runner.clone()), dal.clone());
    let (_tx, stop) = tokio::sync::watch::channel(false);

    let stream = ConfiguredStream {
        stream: "rs_src".into(),
        sync_mode: SyncMode::Incremental,
        cursor_field: Some("n".into()),
        primary_key: Some(vec!["n".into()]),
        write_mode: WriteMode::Upsert {
            business_keys: vec!["n".into()],
        },
        mapping: MappingSpec::default(),
    };
    let grants = json!({ "tcp": [format!("*:{port}")] });
    let mut p: HashMap<String, Value> = HashMap::new();
    p.insert(keys::SYNC.into(), json!("pg-dies-once"));
    p.insert(
        keys::SOURCE_PROVIDER.into(),
        json!("cloacina-connector-postgres"),
    );
    p.insert(keys::SOURCE_MEMBER.into(), json!("source"));
    p.insert(
        keys::SOURCE_CONFIG.into(),
        json!(json!({"url": url_plain, "table": "rs_src"}).to_string()),
    );
    p.insert(keys::SOURCE_GRANTS.into(), grants.clone());
    p.insert(
        keys::DESTINATION_PROVIDER.into(),
        json!("cloacina-connector-postgres"),
    );
    p.insert(keys::DESTINATION_MEMBER.into(), json!("destination"));
    p.insert(
        keys::DESTINATION_CONFIG.into(),
        json!(json!({"url": url_plain, "table": "rs_dst"}).to_string()),
    );
    p.insert(keys::DESTINATION_GRANTS.into(), grants);
    p.insert(
        keys::STREAM.into(),
        json!(serde_json::to_string(&stream).unwrap()),
    );
    p.insert(keys::FAULT.into(), json!("fail_after_write_once"));
    let def = SyncDefinition::new(p);

    let outcome = dispatcher
        .run_pass(&def, stop)
        .await
        .expect("pass survives the fault");
    assert!(
        matches!(outcome, PassOutcome::Exhausted { .. }),
        "{outcome:?}"
    );

    let count = tokio::task::spawn_blocking(move || {
        postgres::Client::connect(&url, postgres::NoTls)
            .unwrap()
            .query_one("SELECT count(*) FROM rs_dst", &[])
            .unwrap()
            .get::<_, i64>(0)
    })
    .await
    .unwrap();
    assert_eq!(
        count, 4,
        "the segment was written twice; upsert keeps four rows"
    );
    let state = dal
        .sync_stream_state()
        .get("pg-dies-once", "rs_src", "")
        .await
        .unwrap()
        .expect("state");
    assert_eq!(state.cursor.as_deref(), Some("4"));

    runner.shutdown().await.unwrap();
}
