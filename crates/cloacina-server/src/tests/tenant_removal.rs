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
//!
//! CLOACINA-T-1095: a delete leaves nothing of the tenant that a tenant made
//! again with the same name, or anyone else, can use; a key of the deleted
//! tenant fails on another server; a delete stopped between its steps is
//! finished by a second one; a lock held in the schema does not hold the
//! delete open.

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

// ---------------------------------------------------------------------------
// CLOACINA-T-1095
// ---------------------------------------------------------------------------

/// The rows of `tenant` in the admin-schema tables that delete must empty.
async fn tenant_rows(state: &AppState, tenant: &str) -> Vec<(&'static str, i64)> {
    let mut out = Vec::new();
    for (table, sql) in [
        (
            "api_keys",
            "SELECT COUNT(*) AS n FROM api_keys WHERE tenant_id = $1",
        ),
        (
            "local_accounts",
            "SELECT COUNT(*) AS n FROM local_accounts WHERE tenant_id = $1",
        ),
        (
            "agent_capacity_limits",
            "SELECT COUNT(*) AS n FROM agent_capacity_limits WHERE tenant_id = $1",
        ),
        (
            "agent_desired_counts",
            "SELECT COUNT(*) AS n FROM agent_desired_counts WHERE tenant_id = $1",
        ),
        (
            "agent_pools",
            "SELECT COUNT(*) AS n FROM agent_pools WHERE tenant_id = $1",
        ),
        (
            "fleet_agents",
            "SELECT COUNT(*) AS n FROM fleet_agents WHERE tenant_id = $1",
        ),
        (
            "ws_tickets",
            "SELECT COUNT(*) AS n FROM ws_tickets WHERE tenant_id = $1",
        ),
    ] {
        out.push((table, count(state, sql, tenant).await));
    }
    let org = cloacina::security::tenant_org_id(tenant).0.to_string();
    out.push((
        "trusted_keys",
        count(
            state,
            "SELECT COUNT(*) AS n FROM trusted_keys WHERE org_id = $1::uuid",
            &org,
        )
        .await,
    ));
    out
}

fn no_rows(rows: &[(&'static str, i64)]) -> bool {
    rows.iter().all(|(_, n)| *n == 0)
}

/// Give `tenant` one row in each admin-schema table a tenant can fill: a local
/// account, an API key, a limit, a pool, an agent, a WebSocket ticket and a
/// trusted key (its desired count comes with the create). Returns the key.
async fn fill_tenant(state: &AppState, admin_token: &str, tenant: &str) -> String {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    dal.local_accounts()
        .create("robin", "old-password-1", Some(tenant), "admin")
        .await
        .expect("account");
    dal.agent_limits()
        .set_tenant_limit(tenant, 6)
        .await
        .expect("limit");
    dal.agent_pools()
        .set(tenant, "big", "{}")
        .await
        .expect("pool");
    run_sql(
        state,
        format!(
            "INSERT INTO fleet_agents (agent_id, tenant_id, target_triple, max_concurrency, \
             registered_at, last_heartbeat_at) \
             VALUES ('{tenant}-agent', '{tenant}', 'x86_64-unknown-linux-gnu', 1, now(), now())"
        ),
    )
    .await;
    let key = create_tenant_api_key(state, tenant, "admin").await;
    let auth = crate::routes::auth::validate_token(state, &key)
        .await
        .expect("the new key works");
    state.ws_tickets.issue(auth).await;

    let public_key = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let request = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/v1/tenants/{tenant}/trusted-keys"))
        .header("Authorization", format!("Bearer {admin_token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({ "name": "old", "public_key": public_key }).to_string(),
        ))
        .unwrap();
    let (status, body) = send_request(build_router(state.clone()), request).await;
    assert_eq!(status, StatusCode::CREATED, "trusted key: {body:?}");
    key
}

async fn get(state: &AppState, token: &str, uri: String) -> (StatusCode, serde_json::Value) {
    let request = axum::http::Request::builder()
        .method("GET")
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    send_request(build_router(state.clone()), request).await
}

fn actions(body: &serde_json::Value) -> Vec<String> {
    body["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|i| i["action"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// After a delete, no row of the tenant is left in the admin schema, and a
/// tenant made again with the same name gets nothing of the old one: no
/// login, no key, no trusted key, no audit rows of its own.
#[tokio::test]
#[serial]
async fn a_tenant_made_again_finds_nothing_of_the_old_one() {
    let state = test_state().await;
    let token = create_test_api_key(&state).await;
    let tenant = format!("t1095_again_{}", uuid::Uuid::new_v4().simple());

    assert_eq!(create(&state, &token, &tenant).await, StatusCode::CREATED);
    let old_key = fill_tenant(&state, &token, &tenant).await;
    assert!(
        tenant_rows(&state, &tenant)
            .await
            .iter()
            .all(|(_, n)| *n > 0),
        "the fixture fills each table: {:?}",
        tenant_rows(&state, &tenant).await
    );

    let (status, body) = delete(&state, &token, &tenant).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    let left = tenant_rows(&state, &tenant).await;
    assert!(no_rows(&left), "rows of the deleted tenant: {left:?}");

    // The same name again.
    assert_eq!(create(&state, &token, &tenant).await, StatusCode::CREATED);
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let login = dal
        .local_accounts()
        .authenticate("robin", "old-password-1", Some(&tenant))
        .await
        .expect("authenticate");
    assert!(
        matches!(
            login,
            cloacina::dal::unified::local_accounts::LoginOutcome::Denied
        ),
        "the old account logs in to the new tenant"
    );
    // A fresh server state: no cache can answer for the key.
    let fresh = test_state().await;
    assert!(
        crate::routes::auth::validate_token(&fresh, &old_key)
            .await
            .is_err(),
        "the old key works for the new tenant"
    );
    let (status, body) = get(&state, &token, format!("/v1/tenants/{tenant}/trusted-keys")).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    assert_eq!(
        body["items"].as_array().map(Vec::len),
        Some(0),
        "the new tenant has the old trusted key: {body:?}"
    );

    // The audit rows stay, for the platform admin; the new tenant's own
    // record starts with it.
    let (status, body) = get(&state, &token, format!("/v1/tenants/{tenant}/audit")).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    let own = actions(&body);
    assert!(
        !own.contains(&"trusted_key.add".to_string()),
        "the new tenant reads the old tenant's audit rows: {own:?}"
    );
    let (status, body) = get(&state, &token, format!("/v1/audit?tenant={tenant}")).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    let all = actions(&body);
    assert!(
        all.contains(&"trusted_key.add".to_string()) && all.contains(&"tenant.removed".to_string()),
        "the platform record keeps the old rows and the boundary: {all:?}"
    );

    assert_eq!(delete(&state, &token, &tenant).await.0, StatusCode::OK);
    assert_eq!(schemas_named(&state, &tenant).await, 0);
}

/// A key of a deleted tenant fails on another server that had it in its key
/// cache, as soon as the revocation notice arrives (not after the 30 s of the
/// cache).
#[tokio::test]
#[serial]
async fn a_key_of_a_deleted_tenant_is_refused_on_another_server() {
    let deleting = test_state().await;
    let other = test_state().await;
    let (_stop, stop_rx) = tokio::sync::watch::channel(false);
    crate::routes::auth::spawn_key_revocation_listener(
        TEST_DB_URL.to_string(),
        other.key_cache.clone(),
        stop_rx,
    );
    // Let the listener connect: its first wake clears the cache once.
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let token = create_test_api_key(&deleting).await;
    let tenant = format!("t1095_cache_{}", uuid::Uuid::new_v4().simple());
    assert_eq!(
        create(&deleting, &token, &tenant).await,
        StatusCode::CREATED
    );
    let key = create_tenant_api_key(&deleting, &tenant, "write").await;
    crate::routes::auth::validate_token(&other, &key)
        .await
        .expect("the other server takes the key and caches it");

    assert_eq!(delete(&deleting, &token, &tenant).await.0, StatusCode::OK);

    let started = std::time::Instant::now();
    let refused = loop {
        if crate::routes::auth::validate_token(&other, &key)
            .await
            .is_err()
        {
            break true;
        }
        if started.elapsed() > std::time::Duration::from_secs(5) {
            break false;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    };
    assert!(
        refused,
        "the other server still takes the deleted tenant's key 5 s after the delete"
    );
}

/// An actuator whose agent removal fails once: the delete stops at step 4.
struct AgentsFailOnce(std::sync::atomic::AtomicBool);

#[async_trait::async_trait]
impl crate::actuator::FleetActuator for AgentsFailOnce {
    async fn reconcile(
        &self,
        _tenant_id: &str,
        desired: u32,
    ) -> Result<crate::actuator::ReconcileOutcome, crate::actuator::ActuatorError> {
        Ok(crate::actuator::ReconcileOutcome {
            spawned: 0,
            stopped: 0,
            running: desired,
        })
    }

    async fn remove_agents(&self, _tenant_id: &str) -> Result<(), crate::actuator::ActuatorError> {
        if self.0.swap(false, std::sync::atomic::Ordering::SeqCst) {
            return Err(crate::actuator::ActuatorError::Substrate(
                "stopped here by the test".to_string(),
            ));
        }
        Ok(())
    }

    fn kind(&self) -> &'static str {
        "test"
    }
}

/// A delete that stops after the keys are revoked and before the agents are
/// removed answers 500 and leaves the pool and the schema; a second delete
/// finishes the job.
#[tokio::test]
#[serial]
async fn a_delete_stopped_between_steps_is_finished_by_a_second_one() {
    let mut state = test_state().await;
    state.fleet_actuator = Arc::new(AgentsFailOnce(std::sync::atomic::AtomicBool::new(true)));
    let token = create_test_api_key(&state).await;
    let tenant = format!("t1095_retry_{}", uuid::Uuid::new_v4().simple());

    assert_eq!(create(&state, &token, &tenant).await, StatusCode::CREATED);
    let key = fill_tenant(&state, &token, &tenant).await;

    let (status, body) = delete(&state, &token, &tenant).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "body: {body:?}");
    // Step 1 is done: the key fails, here and on a fresh server.
    assert!(
        crate::routes::auth::validate_token(&state, &key)
            .await
            .is_err()
    );
    assert!(
        crate::routes::auth::validate_token(&test_state().await, &key)
            .await
            .is_err()
    );
    // Step 4 is not: the pool and the schema are there.
    assert_eq!(
        count(
            &state,
            "SELECT COUNT(*) AS n FROM agent_pools WHERE tenant_id = $1",
            &tenant
        )
        .await,
        1
    );
    assert_eq!(schemas_named(&state, &tenant).await, 1);
    // While part-done, the name is still taken.
    assert_eq!(create(&state, &token, &tenant).await, StatusCode::CONFLICT);

    let (status, body) = delete(&state, &token, &tenant).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    let left = tenant_rows(&state, &tenant).await;
    assert!(no_rows(&left), "rows after the second delete: {left:?}");
    assert_eq!(schemas_named(&state, &tenant).await, 0);
    assert_eq!(roles_named(&state, &tenant).await, 0);
    // And a third one is safe.
    assert_eq!(delete(&state, &token, &tenant).await.0, StatusCode::OK);
}

/// A connection that holds a lock in the tenant's schema does not hold the
/// delete open: it answers 500 within the lock timeout (5 s per lock wait),
/// with the agents' rows already gone, and a delete after the lock is
/// released finishes.
#[tokio::test]
#[serial]
async fn a_lock_in_the_schema_ends_the_delete_in_bounded_time() {
    let state = test_state().await;
    let token = create_test_api_key(&state).await;
    let tenant = format!("t1095_lock_{}", uuid::Uuid::new_v4().simple());
    assert_eq!(create(&state, &token, &tenant).await, StatusCode::CREATED);
    fill_tenant(&state, &token, &tenant).await;

    let (locked_tx, locked_rx) = std::sync::mpsc::channel::<()>();
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let table = format!("{tenant}.contexts");
    let holder = std::thread::spawn(move || {
        use diesel::Connection;
        let mut conn = diesel::PgConnection::establish(TEST_DB_URL).expect("connect");
        diesel::sql_query("BEGIN")
            .execute(&mut conn)
            .expect("begin");
        diesel::sql_query(format!("LOCK TABLE {table} IN ACCESS SHARE MODE"))
            .execute(&mut conn)
            .expect("lock");
        locked_tx.send(()).expect("tell");
        let _ = release_rx.recv();
        diesel::sql_query("ROLLBACK")
            .execute(&mut conn)
            .expect("rollback");
    });
    locked_rx.recv().expect("the lock is held");

    let started = std::time::Instant::now();
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        delete(&state, &token, &tenant),
    )
    .await;
    let took = started.elapsed();
    release_tx.send(()).expect("release");
    holder.join().expect("holder");

    let (status, body) = outcome.expect("the delete waits on the lock with no end");
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "body: {body:?}");
    assert!(
        took < std::time::Duration::from_secs(30),
        "the delete took {took:?}"
    );
    assert_eq!(schemas_named(&state, &tenant).await, 1);
    assert_eq!(
        count(
            &state,
            "SELECT COUNT(*) AS n FROM agent_pools WHERE tenant_id = $1",
            &tenant
        )
        .await,
        0,
        "the fleet rows go before the drop"
    );

    let (status, body) = delete(&state, &token, &tenant).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    assert_eq!(schemas_named(&state, &tenant).await, 0);
    assert_eq!(roles_named(&state, &tenant).await, 0);
}

/// The library allows a database user whose name is not the schema name; the
/// removal drops both.
#[tokio::test]
#[serial]
async fn a_user_name_other_than_the_schema_name_is_dropped_too() {
    let state = test_state().await;
    let schema = format!("t1095_schema_{}", uuid::Uuid::new_v4().simple());
    let user = format!("{schema}_user");
    let admin = cloacina::database::DatabaseAdmin::new(state.database.clone());
    admin
        .create_tenant(cloacina::database::TenantConfig {
            schema_name: schema.clone(),
            username: user.clone(),
            password: "testpass123".to_string(),
        })
        .await
        .expect("create");
    assert_eq!(
        (
            schemas_named(&state, &schema).await,
            roles_named(&state, &user).await
        ),
        (1, 1)
    );

    admin.remove_tenant(&schema, &user).await.expect("remove");
    assert_eq!(
        (
            schemas_named(&state, &schema).await,
            roles_named(&state, &user).await
        ),
        (0, 0)
    );
}
