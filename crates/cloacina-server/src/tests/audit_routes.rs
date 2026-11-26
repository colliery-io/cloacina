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

//! Router tests of the audit record (CLOACINA-T-1024). They need Postgres,
//! like the other router tests of this crate.

use super::*;

/// Make a key in the `public` tenant with a name that no other test run has,
/// so a test can read back only its own audit rows with `actor=<name>`.
async fn audit_test_key(state: &AppState, role: &str, platform_admin: bool) -> (String, String) {
    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();
    let name = format!("audit-test-{}", uuid::Uuid::new_v4().simple());
    let tenant = if platform_admin { None } else { Some("public") };
    cloacina::dal::DAL::new(state.database.clone())
        .api_keys()
        .create_key(&hash, &name, tenant, platform_admin, role)
        .await
        .expect("create audit test key");
    (plaintext, name)
}

fn authed(method: &str, uri: &str, token: &str) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

/// The audit rows of the `public` tenant written by the key named `actor`,
/// newest first.
async fn audit_rows_of(state: &AppState, token: &str, actor: &str) -> Vec<serde_json::Value> {
    let (status, body) = send_request(
        build_router(state.clone()),
        authed(
            "GET",
            &format!("/v1/tenants/public/audit?actor={actor}&limit=200"),
            token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    body["items"].as_array().cloned().unwrap_or_default()
}

async fn upload_python_fixture(state: &AppState, token: &str) -> String {
    delete_workflow_if_exists(state, token, "test-fixture-python", "1.0.0").await;
    let package_data =
        std::fs::read(fixture_path("python-workflow.cloacina")).expect("fixture file not found");
    let (boundary, body_bytes) = multipart_file_body(&package_data);
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

#[tokio::test]
#[serial]
async fn test_audit_upload_writes_a_row_and_names_the_uploader() {
    let state = test_state().await;
    let (token, key_name) = audit_test_key(&state, "admin", true).await;

    let package_id = upload_python_fixture(&state, &token).await;

    let rows = audit_rows_of(&state, &token, &key_name).await;
    let upload = rows
        .iter()
        .find(|r| r["action"] == "package.upload")
        .expect("a package.upload row");
    assert_eq!(upload["tenant_id"], "public");
    assert_eq!(upload["target_kind"], "package");
    assert_eq!(upload["target"], package_id.as_str());
    assert_eq!(upload["outcome"], "ok");
    assert_eq!(upload["actor_key_name"], key_name.as_str());
    assert!(upload["actor_identity"].is_null(), "a manual key has none");
    assert_eq!(upload["detail"]["package_name"], "test-fixture-python");
    assert_eq!(upload["detail"]["version"], "1.0.0");
    assert_eq!(
        upload["detail"]["content_hash"].as_str().map(str::len),
        Some(64)
    );
    // The package bytes are not in the row.
    assert!(upload["detail"].get("file").is_none());

    // The package detail names the uploader, read from the audit record.
    let (status, detail) = send_request(
        build_router(state.clone()),
        authed(
            "GET",
            &format!("/v1/tenants/public/workflows/{package_id}"),
            &token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {detail:?}");
    assert_eq!(detail["uploaded_by"], key_name.as_str());

    // The delete is recorded too, by name and version.
    delete_workflow_if_exists(&state, &token, "test-fixture-python", "1.0.0").await;
    let rows = audit_rows_of(&state, &token, &key_name).await;
    let delete = rows
        .iter()
        .find(|r| r["action"] == "package.delete")
        .expect("a package.delete row");
    assert_eq!(delete["target"], "test-fixture-python");
    assert_eq!(delete["detail"]["version"], "1.0.0");
}

/// Pause and resume of a schedule. The target is a trigger schedule made
/// through the DAL: a workflow pause needs a built package, and the router
/// tests have no compiler.
#[tokio::test]
#[serial]
async fn test_audit_trigger_pause_and_resume_write_rows() {
    let state = test_state().await;
    let (token, key_name) = audit_test_key(&state, "admin", true).await;

    let trigger = format!("audit-trigger-{}", uuid::Uuid::new_v4().simple());
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let schedule = dal
        .schedule()
        .create(cloacina::models::schedule::NewSchedule::trigger(
            &trigger,
            "audit_no_such_workflow",
            std::time::Duration::from_secs(3600),
        ))
        .await
        .expect("create trigger schedule");

    for (verb, action) in [("pause", "trigger.pause"), ("resume", "trigger.resume")] {
        let (status, body) = send_request(
            build_router(state.clone()),
            authed(
                "POST",
                &format!("/v1/tenants/public/triggers/{trigger}/{verb}"),
                &token,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{verb} body: {body:?}");

        let rows = audit_rows_of(&state, &token, &key_name).await;
        // Newest first: the row of this call is the first one.
        assert_eq!(rows[0]["action"], action);
        assert_eq!(rows[0]["target_kind"], "trigger");
        assert_eq!(rows[0]["target"], trigger.as_str());
        assert_eq!(rows[0]["outcome"], "ok");
        assert_eq!(
            rows[0]["detail"]["schedule_id"],
            schedule.id.0.to_string().as_str()
        );
    }
    dal.schedule()
        .delete(schedule.id)
        .await
        .expect("delete trigger schedule");

    // A pause of a trigger that is not there fails and writes no row.
    let before = audit_rows_of(&state, &token, &key_name).await.len();
    let (status, _) = send_request(
        build_router(state.clone()),
        authed(
            "POST",
            "/v1/tenants/public/triggers/audit_no_such_trigger/pause",
            &token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(audit_rows_of(&state, &token, &key_name).await.len(), before);
}

/// A manual run. The router tests have no loaded workflow, so a run that
/// starts is not covered here; this covers the two other outcomes: a run that
/// fails writes no row, and a run that authorization refuses writes a
/// `denied` row.
#[tokio::test]
#[serial]
async fn test_audit_manual_run_failed_writes_none_and_denied_writes_denied() {
    let state = test_state().await;
    let (admin_token, admin_name) = audit_test_key(&state, "admin", true).await;
    let (read_token, read_name) = audit_test_key(&state, "read", false).await;

    let run = |token: &str| {
        axum::http::Request::builder()
            .method("POST")
            .uri("/v1/tenants/public/workflows/audit_no_such_workflow/execute")
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .body(Body::from("{}"))
            .unwrap()
    };

    // The admin key may run it; the workflow is not there, so the run fails.
    let (status, _) = send_request(build_router(state.clone()), run(&admin_token)).await;
    assert!(!status.is_success(), "status: {status}");
    let rows = audit_rows_of(&state, &admin_token, &admin_name).await;
    assert!(
        rows.iter().all(|r| r["action"] != "workflow.run"),
        "a failed run writes no row: {rows:?}"
    );

    // A read key may not run it: 403 and a `denied` row.
    let (status, _) = send_request(build_router(state.clone()), run(&read_token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let rows = audit_rows_of(&state, &admin_token, &read_name).await;
    assert_eq!(rows.len(), 1, "rows: {rows:?}");
    assert_eq!(rows[0]["action"], "workflow.run");
    assert_eq!(rows[0]["outcome"], "denied");
    assert_eq!(rows[0]["tenant_id"], "public");
    assert!(rows[0]["target"].is_null());
    assert_eq!(rows[0]["detail"]["reason"], "insufficient_role");
    assert_eq!(
        rows[0]["detail"]["path"],
        "/v1/tenants/public/workflows/audit_no_such_workflow/execute"
    );

    // A denied read writes no row.
    let (status, _) = send_request(
        build_router(state.clone()),
        authed("GET", "/v1/tenants", &read_token),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        audit_rows_of(&state, &admin_token, &read_name).await.len(),
        1
    );
}

#[tokio::test]
#[serial]
async fn test_audit_list_is_admin_only_and_pages() {
    let state = test_state().await;
    let (admin_token, admin_name) = audit_test_key(&state, "admin", true).await;
    let (tenant_admin_token, _) = audit_test_key(&state, "admin", false).await;
    let (write_token, _) = audit_test_key(&state, "write", false).await;

    // The tenant list: a tenant admin may read it, a write key may not.
    let tenant_list = |token: &str| authed("GET", "/v1/tenants/public/audit?limit=1", token);
    let (status, _) = send_request(
        build_router(state.clone()),
        tenant_list(&tenant_admin_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send_request(build_router(state.clone()), tenant_list(&write_token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // The global list: the platform admin only.
    let global = |token: &str, q: &str| authed("GET", &format!("/v1/audit{q}"), token);
    let (status, _) =
        send_request(build_router(state.clone()), global(&tenant_admin_token, "")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Two rows by this key (two keys made through the API), then one row per
    // page with the cursor.
    for _ in 0..2 {
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/tenants/public/keys")
            .header("Authorization", format!("Bearer {admin_token}"))
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"name":"audit-page-key","role":"read"}"#))
            .unwrap();
        let (status, body) = send_request(build_router(state.clone()), req).await;
        assert_eq!(status, StatusCode::CREATED, "body: {body:?}");
    }
    let q = format!("?actor={admin_name}&action=key.create&limit=1");
    let (status, page1) = send_request(build_router(state.clone()), global(&admin_token, &q)).await;
    assert_eq!(status, StatusCode::OK, "body: {page1:?}");
    assert_eq!(page1["total"], 1);
    let cursor = page1["next_cursor"]
        .as_i64()
        .expect("a full page has a cursor");
    assert_eq!(page1["items"][0]["id"], cursor);
    // The new key's plaintext is not in the row.
    assert!(page1["items"][0]["detail"].get("key").is_none());

    let q2 = format!("{q}&cursor={cursor}");
    let (_, page2) = send_request(build_router(state.clone()), global(&admin_token, &q2)).await;
    assert_eq!(page2["total"], 1);
    assert!(page2["items"][0]["id"].as_i64().unwrap() < cursor);

    // A bad limit and a bad time are 400.
    for bad in ["?limit=0", "?limit=201", "?since=yesterday"] {
        let (status, _) =
            send_request(build_router(state.clone()), global(&admin_token, bad)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
    }
}
