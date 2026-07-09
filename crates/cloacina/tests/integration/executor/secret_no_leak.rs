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

//! CLOACI-I-0133 / T-0858 — the NFR-001 leak test (the initiative gate).
//!
//! Runs a real in-process workflow (DefaultRunner, SQLite) whose task resolves a
//! secret via `context.secret(...)`, then asserts the plaintext value appears in
//! NONE of the durable surfaces: the persisted `contexts` rows (the durable
//! context / execution history), the serialized final `Context`, or
//! `schedules.params`. The task proves it *did* resolve the secret by writing
//! only non-secret derived facts (a boolean + the value's length) back into the
//! context.
//!
//! The test name contains `sqlite` so the SQLite integration lane selects it and
//! the Postgres lane skips it (`cargo test ... -- sqlite` / `--skip sqlite`); it
//! is self-contained (no shared fixture) so it needs no Docker/Postgres.

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use diesel::prelude::*;
use serde_json::Value;

use cloacina::dal::DAL;
use cloacina::executor::WorkflowExecutor;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::security::{SecretStore, SecretStoreResolver};
use cloacina::*;

/// The secret plaintext under test. If this string appears in any durable
/// surface after the run, the no-leak guarantee is broken.
const SECRET_PASSWORD: &str = "s3cr3t-p@ss-DO-NOT-LEAK";
const SECRET_HOST: &str = "db.internal.example";

/// A task that resolves a secret and uses it WITHOUT ever writing the plaintext
/// back into the (durable) context.
#[task(id = "use_secret_task", dependencies = [])]
async fn use_secret_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    // Resolve via the dedicated accessor (D-1). The returned fields live only in
    // this stack frame — they are never inserted into `context`.
    let password = context
        .secret_field("db_prod", "password")
        .await
        .map_err(|e| TaskError::Unknown {
            task_id: "use_secret_task".to_string(),
            message: format!("secret resolution failed: {e}"),
        })?;

    // Prove resolution worked using only NON-secret derived facts.
    context.insert("secret_resolved", serde_json::json!(true))?;
    context.insert("password_len", serde_json::json!(password.len()))?;
    Ok(())
}

/// Load every `value` from the durable `contexts` table (raw, backend-level).
async fn dump_contexts(database: &cloacina::Database) -> Vec<String> {
    #[derive(QueryableByName)]
    struct TextRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        v: String,
    }
    let conn = database.get_sqlite_connection().await.unwrap();
    conn.interact(|conn| diesel::sql_query("SELECT value AS v FROM contexts").load::<TextRow>(conn))
        .await
        .unwrap()
        .unwrap()
        .into_iter()
        .map(|r| r.v)
        .collect()
}

/// Load every non-null `schedules.params` (the plaintext-params sibling surface).
async fn dump_schedule_params(database: &cloacina::Database) -> Vec<String> {
    #[derive(QueryableByName)]
    struct TextRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        v: String,
    }
    let conn = database.get_sqlite_connection().await.unwrap();
    conn.interact(|conn| {
        diesel::sql_query("SELECT COALESCE(params, '') AS v FROM schedules").load::<TextRow>(conn)
    })
    .await
    .unwrap()
    .unwrap()
    .into_iter()
    .map(|r| r.v)
    .collect()
}

#[tokio::test]
async fn test_secret_resolution_does_not_leak_plaintext_sqlite() {
    // ── A shared-cache in-memory SQLite DB both the SecretStore and the runner
    //    open (shared cache => same DB across `Database` handles). `seed_db` is
    //    held for the whole test so the in-memory DB survives. ─────────────────
    let url = format!(
        "file:secret_leak_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4()
    );
    let seed_db = cloacina::Database::new(&url, "", 5);
    seed_db.run_migrations().await.expect("seed migrations");

    // Store the secret for a tenant under a KEK.
    let org_id = UniversalUuid::new_v4();
    let kek: Vec<u8> = vec![7u8; 32];
    let store = SecretStore::new(DAL::new(seed_db.clone()));
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("host".to_string(), SECRET_HOST.to_string());
    fields.insert("password".to_string(), SECRET_PASSWORD.to_string());
    store
        .create_secret(org_id, "db_prod", &fields, &kek)
        .await
        .expect("create secret");

    // Build the in-process resolver (embedded path: server IS the executor).
    let resolver: Arc<dyn SecretResolver> = SecretStoreResolver::new(
        SecretStore::new(DAL::new(seed_db.clone())),
        org_id,
        kek.clone(),
    )
    .into_arc();

    // ── Register the workflow + task in a scoped runtime. ─────────────────────
    let workflow_name = format!(
        "secret_leak_wf_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let workflow = Workflow::builder(&workflow_name)
        .description("NFR-001 secret no-leak test workflow")
        .add_task(Arc::new(use_secret_task_task()))
        .unwrap()
        .build()
        .unwrap();

    let runtime = cloacina::Runtime::empty();
    let namespace = TaskNamespace::new(
        workflow.tenant(),
        workflow.package(),
        workflow.name(),
        "use_secret_task",
    );
    runtime.register_task(namespace, || {
        Arc::new(use_secret_task_task()) as Arc<dyn cloacina::Task>
    });
    runtime.register_workflow(workflow_name.clone(), {
        let workflow = workflow.clone();
        move || workflow.clone()
    });

    // ── Runner wired with the secret resolver (T-0858). ───────────────────────
    let config = DefaultRunnerConfig::builder()
        .max_concurrent_tasks(1)
        .build()
        .unwrap();
    let runner = DefaultRunner::builder()
        .database_url(&url)
        .with_config(config)
        .runtime(runtime)
        .secret_resolver(resolver)
        .build()
        .await
        .unwrap();

    // ── Execute and wait. ─────────────────────────────────────────────────────
    let input_context = Context::new();
    let execution = runner
        .execute_async(&workflow_name, input_context)
        .await
        .unwrap();
    let result = execution.wait_for_completion().await.unwrap();
    assert_eq!(
        result.status,
        WorkflowStatus::Completed,
        "workflow should complete: {:?}",
        result.error_message
    );

    // The task actually resolved the secret (proof: derived, non-secret facts).
    let final_json = result.final_context.to_json().unwrap();
    assert!(
        final_json.contains("secret_resolved"),
        "task should have run and recorded resolution proof: {final_json}"
    );
    assert!(
        final_json.contains(&format!("{}", SECRET_PASSWORD.len())),
        "task should have recorded the resolved value's length"
    );

    // ── NFR-001: the plaintext must appear in NONE of the durable surfaces. ───
    assert!(
        !final_json.contains(SECRET_PASSWORD),
        "LEAK: secret plaintext in serialized final Context: {final_json}"
    );
    assert!(
        !final_json.contains(SECRET_HOST),
        "LEAK: secret host in serialized final Context: {final_json}"
    );

    // Scan every durable context row (via both the runner's and the seed DB
    // handles — same shared DB, but be explicit).
    for database in [runner.database(), &seed_db] {
        let contexts = dump_contexts(database).await;
        for row in &contexts {
            assert!(
                !row.contains(SECRET_PASSWORD),
                "LEAK: secret plaintext found in a durable contexts row: {row}"
            );
            assert!(
                !row.contains(SECRET_HOST),
                "LEAK: secret host found in a durable contexts row: {row}"
            );
        }

        // schedules.params — the plaintext-params sibling surface.
        let params = dump_schedule_params(database).await;
        for row in &params {
            assert!(
                !row.contains(SECRET_PASSWORD),
                "LEAK: secret plaintext found in schedules.params: {row}"
            );
        }
    }

    runner.shutdown().await.unwrap();
    // Keep seed_db alive until the very end so the shared in-memory DB persists.
    drop(seed_db);
}
