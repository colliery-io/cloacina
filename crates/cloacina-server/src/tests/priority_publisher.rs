/*
 *  Copyright 2026 Colliery Software
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

//! Who set a priority, and the limit that holds it (CLOACINA-T-1123): the
//! limit on a key and on an OIDC mapping rule, the limit of a publisher or a
//! creator as it is at each run start, the upload notice and the priority of
//! an instance. They need Postgres, like the other router tests of this
//! crate.

use super::*;

use crate::priority_authority::{PrincipalLookup, limit_now};
use cloacina::dal::DAL;
use cloacina::dal::unified::PrioritySetter;

fn json_request(
    method: &str,
    uri: &str,
    token: &str,
    body: serde_json::Value,
) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get_request(uri: &str, token: &str) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

/// A key of `tenant` with `role` and its own limit; the plaintext and the id.
async fn key_with_limit(
    state: &AppState,
    tenant: &str,
    role: &str,
    max_priority: Option<u8>,
) -> (String, uuid::Uuid) {
    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();
    let info = DAL::new(state.database.clone())
        .api_keys()
        .create_key_limited(
            &hash,
            "t1123-key",
            Some(tenant),
            false,
            role,
            None,
            max_priority,
        )
        .await
        .expect("create key");
    (plaintext, info.id)
}

fn lookup(state: &AppState, oidc_map: &str) -> PrincipalLookup {
    PrincipalLookup {
        admin: state.database.clone(),
        oidc_policy: Arc::new(crate::oidc::MappingPolicy::parse(oidc_map)),
    }
}

fn setter(by: String, role: &str) -> PrioritySetter {
    PrioritySetter {
        by,
        role: role.to_string(),
    }
}

/// A key's own limit, set at create, changed and cleared by the admin route:
/// 0 to 10, not better than the caller's own limit, not on a `read` key.
#[tokio::test]
#[serial]
async fn a_key_takes_its_own_limit_at_create_and_by_the_admin_route() {
    let state = test_state().await;
    let platform = create_test_api_key(&state).await;
    let keys = "/v1/tenants/public/keys";

    let (status, created) = send_request(
        build_router(state.clone()),
        json_request(
            "POST",
            keys,
            &platform,
            serde_json::json!({"name": "t1123-ops", "role": "write", "max_priority": 2}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["max_priority"], 2);
    let id = created["id"].as_str().unwrap().to_string();

    let (status, body) = send_request(
        build_router(state.clone()),
        json_request(
            "POST",
            keys,
            &platform,
            serde_json::json!({"name": "t1123-reader", "role": "read", "max_priority": 2}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    let (status, body) = send_request(
        build_router(state.clone()),
        json_request(
            "POST",
            keys,
            &platform,
            serde_json::json!({"name": "t1123-x", "role": "write", "max_priority": 11}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["code"], "invalid_priority");

    // A tenant admin limited to 3 may not give a key 1.
    let (limited_admin, _) = key_with_limit(&state, "public", "admin", Some(3)).await;
    let (status, body) = send_request(
        build_router(state.clone()),
        json_request(
            "POST",
            keys,
            &limited_admin,
            serde_json::json!({"name": "t1123-y", "role": "write", "max_priority": 1}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["code"], "priority_above_limit");

    // The route changes it, and clears it.
    let uri = format!("{keys}/{id}");
    let (status, body) = send_request(
        build_router(state.clone()),
        json_request(
            "PATCH",
            &uri,
            &platform,
            serde_json::json!({"max_priority": 0}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["max_priority"], 0);
    let stored = DAL::new(state.database.clone())
        .api_keys()
        .get_key(uuid::Uuid::parse_str(&id).unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.max_priority, Some(0));
    let (status, body) = send_request(
        build_router(state.clone()),
        json_request(
            "PATCH",
            &uri,
            &platform,
            serde_json::json!({"max_priority": null}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["max_priority"].is_null(), "{body}");

    // A key of another tenant is not found through this tenant.
    let (_, other) = key_with_limit(&state, "t1123_other_tenant", "write", None).await;
    let (status, _) = send_request(
        build_router(state.clone()),
        json_request(
            "PATCH",
            &format!("{keys}/{other}"),
            &platform,
            serde_json::json!({"max_priority": 3}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// The limit of a publisher at a run start: a live key has its own limit or
/// its role's; a key that is gone has the role recorded; a local account
/// has its role's; a disabled one the role recorded.
#[tokio::test]
#[serial]
async fn the_limit_of_a_key_or_an_account_is_read_as_it_is_now() {
    let state = test_state().await;
    let lookup = lookup(&state, "");
    let dal = DAL::new(state.database.clone());

    let (_, write) = key_with_limit(&state, "public", "write", None).await;
    let key = setter(format!("key:{write}"), "write");
    assert_eq!(limit_now(&lookup, "public", &key).await, 5);
    dal.api_keys()
        .set_max_priority(write, Some(0))
        .await
        .unwrap();
    assert_eq!(limit_now(&lookup, "public", &key).await, 0, "raised");
    dal.api_keys()
        .set_max_priority(write, Some(7))
        .await
        .unwrap();
    assert_eq!(limit_now(&lookup, "public", &key).await, 7, "lowered");
    dal.api_keys().revoke_key(write).await.unwrap();
    assert_eq!(
        limit_now(&lookup, "public", &key).await,
        5,
        "a key that is gone: the default of the role it had"
    );
    let gone_admin = setter(format!("key:{}", uuid::Uuid::new_v4()), "admin");
    assert_eq!(limit_now(&lookup, "public", &gone_admin).await, 0);

    let platform = create_test_api_key(&state).await;
    let hash = cloacina::security::api_keys::hash_api_key(&platform);
    let platform_id = dal
        .api_keys()
        .validate_hash(&hash)
        .await
        .unwrap()
        .unwrap()
        .id;
    let admin = setter(format!("key:{platform_id}"), "admin");
    assert_eq!(limit_now(&lookup, "public", &admin).await, 0);

    let account = dal
        .local_accounts()
        .create(
            &format!("t1123-{}", uuid::Uuid::new_v4().simple()),
            "a long test password",
            Some("public"),
            "admin",
        )
        .await
        .unwrap();
    let local = setter(format!("local:{}", account.id), "write");
    assert_eq!(
        limit_now(&lookup, "public", &local).await,
        0,
        "its role now"
    );
    dal.local_accounts()
        .set_status(account.id, "public", "disabled")
        .await
        .unwrap();
    assert_eq!(
        limit_now(&lookup, "public", &local).await,
        5,
        "disabled: the role recorded"
    );
}

/// The limit of a person who signs in with OIDC: the current map, run on
/// the claims of their newest login session in the tenant.
#[tokio::test]
#[serial]
async fn the_limit_of_an_oidc_person_comes_from_the_current_map() {
    let state = test_state().await;
    let dal = DAL::new(state.database.clone());
    let issuer = "https://idp.example.test";
    let subject = format!("sub-{}", uuid::Uuid::new_v4().simple());
    let provenance = format!("oidc:{issuer}:{subject}");
    let person = setter(provenance.clone(), "write");

    // No session: the role recorded.
    let map = "group:t1123-ops=public:write:2";
    assert_eq!(limit_now(&lookup(&state, map), "public", &person).await, 5);

    let (_, hash) = cloacina::security::api_keys::generate_api_key();
    let key = dal
        .api_keys()
        .mint_key_limited(
            &hash,
            &provenance,
            Some("public"),
            "write",
            chrono::Utc::now() + chrono::Duration::minutes(15),
            &provenance,
            Some(2),
        )
        .await
        .unwrap();
    let claims = serde_json::json!({"sub": subject, "groups": ["t1123-ops"]}).to_string();
    dal.oidc_sessions()
        .create_login_session(
            key.id,
            issuer,
            chrono::Utc::now() + chrono::Duration::hours(1),
            uuid::Uuid::new_v4(),
            &claims,
            None,
        )
        .await
        .unwrap();

    assert_eq!(limit_now(&lookup(&state, map), "public", &person).await, 2);
    // The map changes: the next run start reads the new rule.
    assert_eq!(
        limit_now(
            &lookup(&state, "group:t1123-ops=public:write:0"),
            "public",
            &person
        )
        .await,
        0
    );
    assert_eq!(
        limit_now(
            &lookup(&state, "group:t1123-ops=public:write"),
            "public",
            &person
        )
        .await,
        5,
        "a rule with no limit: the role's"
    );
    assert_eq!(
        limit_now(
            &lookup(&state, "group:other=public:admin"),
            "public",
            &person
        )
        .await,
        5,
        "no rule maps the person any more: the role recorded"
    );
    dal.oidc_sessions().delete(key.id).await.unwrap();
}

/// The upload answer gives the limit that holds the runs of the package,
/// with a notice when it is below 0, and the package keeps its publisher.
#[tokio::test]
#[serial]
async fn an_upload_records_the_publisher_and_answers_the_limit() {
    let state = test_state().await;
    let tenant = format!("t1123_up_{}", uuid::Uuid::new_v4().simple());
    let platform = create_test_api_key(&state).await;
    let (status, _) = send_request(
        build_router(state.clone()),
        json_request(
            "POST",
            "/v1/tenants",
            &platform,
            serde_json::json!({"name": tenant, "password": "testpass123"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (writer, writer_id) = key_with_limit(&state, &tenant, "write", None).await;

    let package = std::fs::read(fixture_path("python-workflow.cloacina")).expect("fixture");
    let upload = |token: &str| {
        let (boundary, body) = multipart_file_body(&package);
        axum::http::Request::builder()
            .method("POST")
            .uri(format!("/v1/tenants/{tenant}/workflows"))
            .header("Authorization", format!("Bearer {token}"))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .unwrap()
    };
    let (status, body) = send_request(build_router(state.clone()), upload(&writer)).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["priority_limit"], 5, "{body}");
    let notice = body["priority_notice"].as_str().expect("a notice");
    assert!(notice.contains("priority 5"), "{notice}");

    let tenant_db = state
        .tenant_databases
        .resolve(&tenant, &state.database)
        .await
        .unwrap();
    let rows = DAL::new(tenant_db)
        .priority_grants()
        .list(cloacina::dal::unified::priority_grants::KIND_PACKAGE)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].set_by, format!("key:{writer_id}"));
    assert_eq!(rows[0].set_by_role, "write");

    let _ = state.tenant_runners.shutdown_all().await;
    state.tenant_databases.evict(&tenant).await;
    let _ = cloacina::database::DatabaseAdmin::new(state.database.clone())
        .remove_tenant(&tenant, &tenant)
        .await;
}

/// An instance created with a priority keeps it and its creator; each view
/// gives the priority held to the creator's limit now.
#[tokio::test]
#[serial]
async fn an_instance_keeps_its_priority_and_is_held_to_its_creators_limit_now() {
    let state = test_state().await;
    let (writer, writer_id) = key_with_limit(&state, "public", "write", None).await;
    let workflow = format!("t1123_inst_{}", uuid::Uuid::new_v4().simple());
    let base = format!("/v1/tenants/public/workflows/{workflow}/instances");

    let (status, body) = send_request(
        build_router(state.clone()),
        json_request(
            "POST",
            &base,
            &writer,
            serde_json::json!({"instance_name": "nightly", "priority": 1}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["priority"]["asked"], 1, "{body}");
    assert_eq!(body["priority"]["given"], 5, "{body}");
    assert!(body["priority_notice"].as_str().is_some(), "{body}");
    let schedule_id = body["id"].as_str().unwrap().to_string();

    let row = DAL::new(state.database.clone())
        .priority_grants()
        .get(
            cloacina::dal::unified::priority_grants::KIND_SCHEDULE,
            &schedule_id,
        )
        .await
        .unwrap()
        .expect("a record");
    assert_eq!(row.asked(), Some(1));
    assert_eq!(row.set_by, format!("key:{writer_id}"));

    // The creator's limit is raised: the instance's fires get what it asks.
    DAL::new(state.database.clone())
        .api_keys()
        .set_max_priority(writer_id, Some(0))
        .await
        .unwrap();
    let (status, body) = send_request(
        build_router(state.clone()),
        get_request(&format!("{base}/nightly"), &writer),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["priority"]["given"], 1, "{body}");
    assert!(body["priority_notice"].is_null(), "{body}");

    let (status, body) = send_request(
        build_router(state.clone()),
        json_request(
            "POST",
            &base,
            &writer,
            serde_json::json!({"instance_name": "bad", "priority": 11}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    // A delete takes the record with it.
    let (status, _) = send_request(
        build_router(state.clone()),
        axum::http::Request::builder()
            .method("DELETE")
            .uri(format!("{base}/nightly"))
            .header("Authorization", format!("Bearer {writer}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        DAL::new(state.database.clone())
            .priority_grants()
            .get(
                cloacina::dal::unified::priority_grants::KIND_SCHEDULE,
                &schedule_id
            )
            .await
            .unwrap()
            .is_none()
    );
}
