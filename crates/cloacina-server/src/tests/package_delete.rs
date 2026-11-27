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

//! Router tests of the package delete in each build state (CLOACINA-T-1122).
//! They need Postgres, like the other router tests of this crate. The router
//! tests have no compiler, so an upload stays `pending`; the other states are
//! set with SQL.

use super::*;
use diesel::RunQueryDsl;

/// A platform admin key with a name no other run has: the audit rows of a
/// test are read back by it.
async fn admin_key(state: &AppState) -> (String, String) {
    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();
    let name = format!("t1122-{}", uuid::Uuid::new_v4().simple());
    cloacina::dal::DAL::new(state.database.clone())
        .api_keys()
        .create_key(&hash, &name, None, true, "admin")
        .await
        .expect("create the test key");
    (plaintext, name)
}

/// The Python fixture packed again under a package name no other run has, so
/// its upload makes a new row.
fn fixture_of_this_run() -> (Vec<u8>, String) {
    use cloacina::cloacina_workflow_plugin::{fidius_core, unpack_limit};

    let name = format!("t1122-pkg-{}", uuid::Uuid::new_v4().simple());
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

fn call(method: &str, uri: &str, token: &str) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

async fn upload(state: &AppState, token: &str, package_data: &[u8]) -> String {
    let (boundary, body_bytes) = multipart_file_body(package_data);
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
    body["package_id"].as_str().expect("package_id").to_string()
}

async fn delete(state: &AppState, token: &str, name: &str) -> (StatusCode, serde_json::Value) {
    send_request(
        build_router(state.clone()),
        call(
            "DELETE",
            &format!("/v1/tenants/public/workflows/{name}/1.0.0"),
            token,
        ),
    )
    .await
}

/// The `package.delete` rows written by the key named `actor`.
async fn delete_rows(state: &AppState, token: &str, actor: &str) -> Vec<serde_json::Value> {
    let (status, body) = send_request(
        build_router(state.clone()),
        call(
            "GET",
            &format!("/v1/tenants/public/audit?actor={actor}&action=package.delete&limit=200"),
            token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    body["items"].as_array().cloned().unwrap_or_default()
}

#[derive(diesel::QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    n: i64,
}

/// Run one SQL statement. The text holds only values this file made: UUIDs
/// and package names of the form `t1122-pkg-<hex>`.
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

/// The package rows of `name` and the stored archives they point to.
async fn rows_and_archives(state: &AppState, name: &str) -> (i64, i64) {
    let rows = count(
        state,
        format!("SELECT count(*) AS n FROM workflow_packages WHERE package_name = '{name}'"),
    )
    .await;
    let archives = count(
        state,
        format!(
            "SELECT count(*) AS n FROM workflow_registry WHERE id IN \
             (SELECT registry_id FROM workflow_packages WHERE package_name = '{name}')"
        ),
    )
    .await;
    (rows, archives)
}

/// Remove what a test left: the rows of `name` and their archives.
async fn remove_rows_of(state: &AppState, name: &str) {
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

async fn set_status(state: &AppState, package_id: &str, status: &str) {
    let id = uuid::Uuid::parse_str(package_id).expect("a UUID");
    sql(
        state,
        format!("UPDATE workflow_packages SET build_status = '{status}' WHERE id = '{id}'"),
    )
    .await;
}

/// The archive id of a package row, read before a delete.
async fn archive_count_of_id(state: &AppState, package_id: &str) -> i64 {
    let id = uuid::Uuid::parse_str(package_id).expect("a UUID");
    count(
        state,
        format!(
            "SELECT count(*) AS n FROM workflow_registry WHERE id IN \
             (SELECT registry_id FROM workflow_packages WHERE id = '{id}')"
        ),
    )
    .await
}

/// CLOACINA-T-1122: a `pending` package (it never built) is removed: its row
/// and its archive. One `package.delete` row is written. A second delete
/// answers 404 and writes none. A new upload of the same bytes gets a new row.
#[tokio::test]
#[serial]
async fn test_t1122_a_pending_package_is_removed() {
    let state = test_state().await;
    let (token, key_name) = admin_key(&state).await;
    let (bytes, name) = fixture_of_this_run();
    let package_id = upload(&state, &token, &bytes).await;
    assert_eq!(rows_and_archives(&state, &name).await, (1, 1));

    let (status, body) = delete(&state, &token, &name).await;
    let after = rows_and_archives(&state, &name).await;
    let audit = delete_rows(&state, &token, &key_name).await;
    let (again, again_body) = delete(&state, &token, &name).await;
    let audit_again = delete_rows(&state, &token, &key_name).await;
    let new_id = upload(&state, &token, &bytes).await;
    remove_rows_of(&state, &name).await;

    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    assert_eq!(after, (0, 0), "the row and its archive are removed");
    assert_eq!(audit.len(), 1, "one package.delete row: {audit:?}");
    assert_eq!(audit[0]["target"], name.as_str());
    assert_eq!(again, StatusCode::NOT_FOUND, "body: {again_body:?}");
    assert_eq!(again_body["code"], "workflow_not_found");
    assert_eq!(audit_again.len(), 1, "a 404 writes no audit row");
    assert_ne!(new_id, package_id, "a new upload gets a new row");
}

/// CLOACINA-T-1122: a `failed` package is removed, and a built one as before.
#[tokio::test]
#[serial]
async fn test_t1122_a_failed_or_built_package_is_removed() {
    let state = test_state().await;
    let (token, key_name) = admin_key(&state).await;
    let mut results = Vec::new();
    for status in ["failed", "success"] {
        let (bytes, name) = fixture_of_this_run();
        let package_id = upload(&state, &token, &bytes).await;
        set_status(&state, &package_id, status).await;
        let (answer, body) = delete(&state, &token, &name).await;
        let after = rows_and_archives(&state, &name).await;
        remove_rows_of(&state, &name).await;
        results.push((status, answer, body, after));
    }
    let audit = delete_rows(&state, &token, &key_name).await;

    for (status, answer, body, after) in results {
        assert_eq!(answer, StatusCode::OK, "{status}: {body:?}");
        assert_eq!(
            after,
            (0, 0),
            "{status}: the row and its archive are removed"
        );
    }
    assert_eq!(audit.len(), 2, "one package.delete row each: {audit:?}");
}

/// CLOACINA-T-1122: a package the compiler is building is not removed: 409
/// `package_building`, the row stays, no audit row.
#[tokio::test]
#[serial]
async fn test_t1122_a_building_package_is_refused() {
    let state = test_state().await;
    let (token, key_name) = admin_key(&state).await;
    let (bytes, name) = fixture_of_this_run();
    let package_id = upload(&state, &token, &bytes).await;
    set_status(&state, &package_id, "building").await;

    let (status, body) = delete(&state, &token, &name).await;
    let after = rows_and_archives(&state, &name).await;
    let archive = archive_count_of_id(&state, &package_id).await;
    let audit = delete_rows(&state, &token, &key_name).await;
    remove_rows_of(&state, &name).await;

    assert_eq!(status, StatusCode::CONFLICT, "body: {body:?}");
    assert_eq!(body["code"], "package_building");
    assert_eq!(after, (1, 1), "nothing is removed");
    assert_eq!(archive, 1);
    assert!(audit.is_empty(), "a refusal writes no audit row: {audit:?}");
}

/// CLOACINA-T-1122: a name and version that does not exist answers 404 and
/// writes no audit row.
#[tokio::test]
#[serial]
async fn test_t1122_an_unknown_package_is_404() {
    let state = test_state().await;
    let (token, key_name) = admin_key(&state).await;
    let name = format!("t1122-pkg-{}", uuid::Uuid::new_v4().simple());

    let (status, body) = delete(&state, &token, &name).await;
    let audit = delete_rows(&state, &token, &key_name).await;

    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body:?}");
    assert_eq!(body["code"], "workflow_not_found");
    assert!(audit.is_empty(), "rows: {audit:?}");
}
