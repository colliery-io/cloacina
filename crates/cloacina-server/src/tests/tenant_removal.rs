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

//! Tenant delete answers "removed" only when the schema and the role are gone
//! (CLOACINA-T-1068). A drop that Postgres refuses answers an error, leaves the
//! tenant as it was, and a second delete finishes it. They need Postgres, like
//! the other router tests.

use super::*;

use diesel::RunQueryDsl;

#[derive(diesel::QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    n: i64,
}

/// Runs one SQL statement on the admin connection.
async fn run_sql(state: &AppState, sql: String) {
    let conn = state
        .database
        .get_postgres_connection()
        .await
        .expect("connection");
    conn.interact(move |conn| diesel::sql_query(sql).execute(conn))
        .await
        .expect("interact")
        .expect("sql");
}

/// The count that `sql` gives in its column `n`, with `name` as `$1`.
async fn count(state: &AppState, sql: &'static str, name: &str) -> i64 {
    let conn = state
        .database
        .get_postgres_connection()
        .await
        .expect("connection");
    let name = name.to_string();
    conn.interact(move |conn| {
        diesel::sql_query(sql)
            .bind::<diesel::sql_types::Text, _>(name)
            .get_result::<Count>(conn)
    })
    .await
    .expect("interact")
    .expect("count")
    .n
}

async fn schemas_named(state: &AppState, name: &str) -> i64 {
    count(
        state,
        "SELECT COUNT(*) AS n FROM pg_namespace WHERE nspname = $1",
        name,
    )
    .await
}

async fn roles_named(state: &AppState, name: &str) -> i64 {
    count(
        state,
        "SELECT COUNT(*) AS n FROM pg_roles WHERE rolname = $1",
        name,
    )
    .await
}

async fn create(state: &AppState, token: &str, tenant: &str) -> StatusCode {
    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/tenants")
        .header("Authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({ "name": tenant, "password": "testpass123" }).to_string(),
        ))
        .unwrap();
    send_request(build_router(state.clone()), request).await.0
}

async fn delete(state: &AppState, token: &str, tenant: &str) -> (StatusCode, serde_json::Value) {
    let request = axum::http::Request::builder()
        .method("DELETE")
        .uri(format!("/v1/tenants/{tenant}"))
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    send_request(build_router(state.clone()), request).await
}

/// A delete that answers "removed" has dropped the schema and the role, so
/// the name can be made again.
#[tokio::test]
#[serial]
async fn tenant_delete_drops_the_schema_and_the_role() {
    let state = test_state().await;
    let token = create_test_api_key(&state).await;
    let tenant = format!("t1068_gone_{}", uuid::Uuid::new_v4().simple());

    assert_eq!(create(&state, &token, &tenant).await, StatusCode::CREATED);
    assert_eq!(schemas_named(&state, &tenant).await, 1);
    assert_eq!(roles_named(&state, &tenant).await, 1);

    let (status, body) = delete(&state, &token, &tenant).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    assert_eq!(body["status"], "removed");
    assert_eq!(
        schemas_named(&state, &tenant).await,
        0,
        "the delete answered \"removed\" and the schema is still there"
    );
    assert_eq!(
        roles_named(&state, &tenant).await,
        0,
        "the delete answered \"removed\" and the role is still there"
    );

    // The name is free again, and a delete of a tenant that is gone is safe.
    assert_eq!(create(&state, &token, &tenant).await, StatusCode::CREATED);
    assert_eq!(delete(&state, &token, &tenant).await.0, StatusCode::OK);
    assert_eq!(delete(&state, &token, &tenant).await.0, StatusCode::OK);
    assert_eq!(schemas_named(&state, &tenant).await, 0);
    assert_eq!(roles_named(&state, &tenant).await, 0);
}

/// Postgres refuses to drop a role that owns a table in another schema. The
/// delete answers an error and leaves the schema and the role. With the cause
/// gone, the same delete finishes.
#[tokio::test]
#[serial]
async fn tenant_delete_reports_a_failed_drop_and_a_retry_finishes() {
    let state = test_state().await;
    let token = create_test_api_key(&state).await;
    let tenant = format!("t1068_held_{}", uuid::Uuid::new_v4().simple());
    let other = format!("{tenant}_x");

    assert_eq!(create(&state, &token, &tenant).await, StatusCode::CREATED);
    run_sql(&state, format!("CREATE SCHEMA {other}")).await;
    run_sql(&state, format!("CREATE TABLE {other}.held (id int)")).await;
    run_sql(
        &state,
        format!("ALTER TABLE {other}.held OWNER TO {tenant}"),
    )
    .await;

    let (status, body) = delete(&state, &token, &tenant).await;
    let left = (
        schemas_named(&state, &tenant).await,
        roles_named(&state, &tenant).await,
    );
    // Remove the cause before the asserts, so a failed assert leaves less.
    run_sql(&state, format!("DROP SCHEMA {other} CASCADE")).await;

    assert_eq!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "a drop that failed must not answer \"removed\"; body: {body:?}"
    );
    assert_eq!(left, (1, 1), "a failed drop leaves the schema and the role");

    let (status, body) = delete(&state, &token, &tenant).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    assert_eq!(schemas_named(&state, &tenant).await, 0);
    assert_eq!(roles_named(&state, &tenant).await, 0);
}
