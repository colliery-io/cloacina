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

//! CLOACINA-T-1065: a platform admin makes a second platform admin key with
//! `POST /v1/auth/keys` and `"platform_admin": true`, so the bootstrap key can
//! be rotated with no restart. They need Postgres, like the other router tests.

use super::*;

async fn post_json(
    state: &AppState,
    uri: &str,
    token: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let request = axum::http::Request::builder()
        .method("POST")
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    send_request(build_router(state.clone()), request).await
}

#[tokio::test]
#[serial]
async fn a_platform_admin_makes_a_platform_admin_key() {
    let state = test_state().await;
    let admin = create_test_api_key(&state).await;

    let (status, body) = post_json(
        &state,
        "/v1/auth/keys",
        &admin,
        serde_json::json!({ "name": "second-admin", "role": "admin", "platform_admin": true }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert_eq!(body["is_admin"], true, "body: {body}");
    assert_eq!(body["tenant_id"], serde_json::Value::Null, "body: {body}");
    assert_eq!(body["permissions"], "admin", "body: {body}");
}

/// A tenant `admin`, `write`, `read` and `agent` key is refused; so is a key
/// with no tenant and no flag. None of them gets a key with the flag.
#[tokio::test]
#[serial]
async fn only_a_platform_admin_makes_a_platform_admin_key() {
    let state = test_state().await;
    let mut callers = Vec::new();
    for role in ["admin", "write", "read", "agent"] {
        callers.push((role, create_tenant_api_key(&state, "public", role).await));
    }
    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();
    cloacina::dal::DAL::new(state.database.clone())
        .api_keys()
        .create_key(&hash, "no-tenant-no-flag", None, false, "admin")
        .await
        .expect("create key");
    callers.push(("no tenant, no flag", plaintext));

    for (who, token) in &callers {
        let (status, body) = post_json(
            &state,
            "/v1/auth/keys",
            token,
            serde_json::json!({ "name": "escalate", "role": "admin", "platform_admin": true }),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{who}: {body}");
        assert!(body.get("key").is_none(), "{who}: {body}");
    }

    // A tenant admin's own key route refuses the flag with 400.
    let (_, tenant_admin) = &callers[0];
    let (status, body) = post_json(
        &state,
        "/v1/tenants/public/keys",
        tenant_admin,
        serde_json::json!({ "name": "escalate", "role": "admin", "platform_admin": true }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["code"], "invalid_request");
}

/// The flag needs role `admin`; a request with another role is a 400.
#[tokio::test]
#[serial]
async fn a_platform_admin_key_has_role_admin() {
    let state = test_state().await;
    let admin = create_test_api_key(&state).await;
    for role in ["read", "write", "agent"] {
        let (status, body) = post_json(
            &state,
            "/v1/auth/keys",
            &admin,
            serde_json::json!({ "name": "odd", "role": role, "platform_admin": true }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{role}: {body}");
        assert_eq!(body["code"], "invalid_request");
    }
    // With no flag the route makes a `public` key, as before.
    let (status, body) = post_json(
        &state,
        "/v1/auth/keys",
        &admin,
        serde_json::json!({ "name": "plain", "role": "read" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert_eq!(body["tenant_id"], "public");
    assert_eq!(body["is_admin"], false);
}

/// Rotation: make a new platform admin key, revoke the old one with it, and
/// the new key passes a platform route. The audit row names the actor and
/// the new key's id, with the flag in its detail.
#[tokio::test]
#[serial]
async fn the_bootstrap_key_is_rotated_with_no_restart() {
    let state = test_state().await;
    let old_name = format!("rotate-old-{}", uuid::Uuid::new_v4().simple());
    let (old_key, old_hash) = cloacina::security::api_keys::generate_api_key();
    let old = cloacina::dal::DAL::new(state.database.clone())
        .api_keys()
        .create_key(&old_hash, &old_name, None, true, "admin")
        .await
        .expect("create old admin key");

    let (status, body) = post_json(
        &state,
        "/v1/auth/keys",
        &old_key,
        serde_json::json!({ "name": "rotated-admin", "role": "admin", "platform_admin": true }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    let new_key = body["key"].as_str().expect("a key").to_string();
    let new_id = body["id"].as_str().expect("an id").to_string();

    let revoke = axum::http::Request::builder()
        .method("DELETE")
        .uri(format!("/v1/auth/keys/{}", old.id))
        .header("Authorization", format!("Bearer {new_key}"))
        .body(Body::empty())
        .unwrap();
    let (status, body) = send_request(build_router(state.clone()), revoke).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");

    // The old key is refused; the new one passes a platform route.
    let list = |token: &str| {
        axum::http::Request::builder()
            .uri("/v1/auth/keys")
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap()
    };
    let (status, _) = send_request(build_router(state.clone()), list(&old_key)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = send_request(build_router(state.clone()), list(&new_key)).await;
    assert_eq!(status, StatusCode::OK);

    // The audit row of the create.
    let audit = axum::http::Request::builder()
        .uri(format!(
            "/v1/audit?actor={old_name}&action=key.create&limit=10"
        ))
        .header("Authorization", format!("Bearer {new_key}"))
        .body(Body::empty())
        .unwrap();
    let (status, page) = send_request(build_router(state.clone()), audit).await;
    assert_eq!(status, StatusCode::OK, "body: {page}");
    let rows = page["items"].as_array().cloned().unwrap_or_default();
    assert_eq!(rows.len(), 1, "rows: {rows:?}");
    assert_eq!(rows[0]["actor_key_name"], old_name.as_str());
    assert_eq!(rows[0]["target"], new_id.as_str());
    assert_eq!(rows[0]["outcome"], "ok");
    assert_eq!(rows[0]["detail"]["platform_admin"], true);
    assert!(rows[0]["detail"].get("key").is_none());
}
