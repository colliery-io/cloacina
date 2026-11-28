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

//! Router tests of the refusal of an undeclared secret binding
//! (CLOACINA-T-1134). They need Postgres, like the other router tests of this
//! crate. The router tests have no compiler, so the package row is set to
//! `success` and given a metadata document with SQL.

use super::*;
use diesel::RunQueryDsl;

/// The Python fixture packed again under a package name no other run has.
fn fixture_of_this_run() -> (Vec<u8>, String) {
    use cloacina::cloacina_workflow_plugin::{fidius_core, unpack_limit};

    let name = format!("t1134-pkg-{}", uuid::Uuid::new_v4().simple());
    let work = tempfile::tempdir().expect("temp dir");
    let source =
        unpack_limit::unpack_package(&fixture_path("python-workflow.cloacina"), work.path())
            .expect("unpack the fixture");
    let manifest_path = source.join("package.toml");
    let manifest = std::fs::read_to_string(&manifest_path).expect("read package.toml");
    let old_name = "name = \"test-fixture-python\"";
    assert!(manifest.contains(old_name), "manifest: {manifest}");
    std::fs::write(
        &manifest_path,
        manifest.replace(old_name, &format!("name = \"{name}\"")),
    )
    .expect("write package.toml");
    let archive = work.path().join("of-this-run.cloacina");
    fidius_core::package::pack_package(&source, Some(&archive)).expect("pack the fixture");
    (std::fs::read(&archive).expect("read the archive"), name)
}

async fn admin_key(state: &AppState) -> String {
    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();
    let name = format!("t1134-{}", uuid::Uuid::new_v4().simple());
    cloacina::dal::DAL::new(state.database.clone())
        .api_keys()
        .create_key(&hash, &name, None, true, "admin")
        .await
        .expect("create the test key");
    plaintext
}

#[derive(diesel::QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    n: i64,
}

/// Run one SQL statement. The text holds only values this file made: UUIDs,
/// package names of the form `t1134-pkg-<hex>` and fixed JSON.
async fn sql(state: &AppState, statement: String) {
    let conn = state
        .database
        .get_postgres_connection()
        .await
        .expect("connection");
    conn.interact(move |conn| diesel::sql_query(statement).execute(conn))
        .await
        .expect("interact")
        .expect("statement");
}

async fn count(state: &AppState, query: String) -> i64 {
    let conn = state
        .database
        .get_postgres_connection()
        .await
        .expect("connection");
    conn.interact(move |conn| diesel::sql_query(query).get_result::<Count>(conn))
        .await
        .expect("interact")
        .expect("count")
        .n
}

/// Upload the fixture as a new package that declares the secret `warehouse`,
/// built, in the tenant `public`. Returns the package name.
async fn package_declaring_warehouse(state: &AppState, token: &str) -> String {
    let (bytes, name) = fixture_of_this_run();
    let (boundary, body_bytes) = multipart_file_body(&bytes);
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/tenants/public/workflows")
        .header("Authorization", format!("Bearer {token}"))
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body_bytes))
        .unwrap();
    let (status, body) = send_request(build_router(state.clone()), req).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body:?}");
    sql(
        state,
        format!(
            "UPDATE workflow_packages SET build_status = 'success', \
             metadata_document = '{{\"format_version\": 1, \"declared_secrets\": [\"warehouse\"]}}' \
             WHERE package_name = '{name}'"
        ),
    )
    .await;
    name
}

async fn remove_rows_of(state: &AppState, name: &str) {
    sql(
        state,
        format!("DELETE FROM schedules WHERE workflow_name = '{name}'"),
    )
    .await;
    sql(
        state,
        format!(
            "WITH gone AS (DELETE FROM workflow_packages WHERE package_name = '{name}' \
             RETURNING registry_id) \
             DELETE FROM workflow_registry WHERE id IN (SELECT registry_id FROM gone)"
        ),
    )
    .await;
}

fn post(uri: String, token: &str, body: serde_json::Value) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .method("POST")
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn runs_of(state: &AppState, name: &str) -> i64 {
    count(
        state,
        format!("SELECT count(*) AS n FROM workflow_executions WHERE workflow_name = '{name}'"),
    )
    .await
}

/// CLOACINA-T-1134: an execute request that binds a name the package does
/// not declare is a 400 that names the binding and the package, and no run is
/// made. A request that binds only the declared name passes the check.
#[tokio::test]
#[serial]
async fn test_t1134_execute_refuses_an_undeclared_binding() {
    let state = test_state().await;
    let token = admin_key(&state).await;
    let name = package_declaring_warehouse(&state, &token).await;
    let uri = format!("/v1/tenants/public/workflows/{name}/execute");

    let (refused, refused_body) = send_request(
        build_router(state.clone()),
        post(
            uri.clone(),
            &token,
            serde_json::json!({"context": {
                "warehouse": {"$secret": "wh_prod"},
                "lake": {"$secret": "lake_prod"},
            }}),
        ),
    )
    .await;
    let runs_after_refusal = runs_of(&state, &name).await;

    let (_, declared_body) = send_request(
        build_router(state.clone()),
        post(
            uri,
            &token,
            serde_json::json!({"context": {"warehouse": {"$secret": "wh_prod"}}}),
        ),
    )
    .await;
    remove_rows_of(&state, &name).await;

    assert_eq!(refused, StatusCode::BAD_REQUEST, "body: {refused_body:?}");
    assert_eq!(
        refused_body["code"], "secret_not_declared",
        "{refused_body:?}"
    );
    let message = refused_body["error"].as_str().expect("error");
    assert!(message.contains("`lake`"), "{message}");
    assert!(message.contains(&name), "{message}");
    assert!(message.contains("secrets(lake)"), "{message}");
    assert_eq!(runs_after_refusal, 0, "a refused request makes no run");
    // The declared binding is not refused by this check. (The router tests
    // have no loaded workflow, so the execution itself may still fail.)
    assert_ne!(
        declared_body["code"], "secret_not_declared",
        "{declared_body:?}"
    );
}

/// CLOACINA-T-1134: an instance that binds a name the package does not
/// declare is a 400 and no schedule row is made; one that binds only the
/// declared name is created.
#[tokio::test]
#[serial]
async fn test_t1134_instance_refuses_an_undeclared_binding() {
    let state = test_state().await;
    let token = admin_key(&state).await;
    let name = package_declaring_warehouse(&state, &token).await;
    let uri = format!("/v1/tenants/public/workflows/{name}/instances");

    let (refused, refused_body) = send_request(
        build_router(state.clone()),
        post(
            uri.clone(),
            &token,
            serde_json::json!({
                "instance_name": "nightly",
                "params": {"lake": {"$secret": "lake_prod"}},
                "enabled": false,
            }),
        ),
    )
    .await;
    let schedules_after_refusal = count(
        &state,
        format!("SELECT count(*) AS n FROM schedules WHERE workflow_name = '{name}'"),
    )
    .await;

    let (created, created_body) = send_request(
        build_router(state.clone()),
        post(
            uri,
            &token,
            serde_json::json!({
                "instance_name": "nightly",
                "params": {"warehouse": {"$secret": "wh_prod"}},
                "enabled": false,
            }),
        ),
    )
    .await;
    remove_rows_of(&state, &name).await;

    assert_eq!(refused, StatusCode::BAD_REQUEST, "body: {refused_body:?}");
    assert_eq!(
        refused_body["code"], "secret_not_declared",
        "{refused_body:?}"
    );
    let message = refused_body["error"].as_str().expect("error");
    assert!(message.contains("`lake`"), "{message}");
    assert!(message.contains(&name), "{message}");
    assert_eq!(schedules_after_refusal, 0, "a refused request makes no row");
    assert!(
        created.is_success(),
        "create failed: {created} {created_body}"
    );
}
