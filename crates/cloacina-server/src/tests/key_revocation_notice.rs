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

//! CLOACINA-T-1128: a key revoked on one server fails on another server that
//! had it in its key cache, as soon as the revocation notice arrives. Key
//! revoke, tenant key revoke, logout and refresh rotation each send it. They
//! need Postgres, like the other router tests.

use super::*;

/// A second server state whose key cache is cleared by the revocation notice.
/// Keep the returned sender alive for the test: dropping it stops the listener.
async fn other_server() -> (AppState, tokio::sync::watch::Sender<bool>) {
    let other = test_state().await;
    let (stop, stop_rx) = tokio::sync::watch::channel(false);
    crate::routes::auth::spawn_key_revocation_listener(
        TEST_DB_URL.to_string(),
        other.key_cache.clone(),
        stop_rx,
    );
    // Let the listener connect: its first wake clears the cache once.
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    (other, stop)
}

/// True when `other` refuses `key` within 5 s.
async fn refused_within_5s(other: &AppState, key: &str) -> bool {
    let started = std::time::Instant::now();
    loop {
        if crate::routes::auth::validate_token(other, key)
            .await
            .is_err()
        {
            return true;
        }
        if started.elapsed() > std::time::Duration::from_secs(5) {
            return false;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

async fn cache_on(other: &AppState, key: &str) {
    crate::routes::auth::validate_token(other, key)
        .await
        .expect("the other server takes the key and caches it");
}

async fn key_id(state: &AppState, key: &str) -> uuid::Uuid {
    crate::routes::auth::validate_token(state, key)
        .await
        .expect("the key works")
        .key_id
}

async fn send(
    state: &AppState,
    method: &str,
    uri: String,
    token: &str,
) -> (StatusCode, serde_json::Value) {
    let request = axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    send_request(build_router(state.clone()), request).await
}

#[tokio::test]
#[serial]
async fn a_revoked_key_is_refused_on_another_server() {
    let (other, _stop) = other_server().await;
    let state = test_state().await;
    let admin = create_test_api_key(&state).await;
    let key = create_tenant_api_key(&state, "public", "read").await;
    let id = key_id(&state, &key).await;
    cache_on(&other, &key).await;

    let (status, body) = send(&state, "DELETE", format!("/v1/auth/keys/{id}"), &admin).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    assert!(
        refused_within_5s(&other, &key).await,
        "the other server still takes the revoked key 5 s after the revoke"
    );
}

#[tokio::test]
#[serial]
async fn a_revoked_tenant_key_is_refused_on_another_server() {
    let (other, _stop) = other_server().await;
    let state = test_state().await;
    let admin = create_test_api_key(&state).await;
    let key = create_tenant_api_key(&state, "public", "read").await;
    let id = key_id(&state, &key).await;
    cache_on(&other, &key).await;

    let (status, body) = send(
        &state,
        "DELETE",
        format!("/v1/tenants/public/keys/{id}"),
        &admin,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    assert!(
        refused_within_5s(&other, &key).await,
        "the other server still takes the revoked tenant key 5 s after the revoke"
    );
}

#[tokio::test]
#[serial]
async fn a_logged_out_key_is_refused_on_another_server() {
    let (other, _stop) = other_server().await;
    let state = test_state().await;
    let (key, _) = mint_oidc_session(
        &state,
        Some("public"),
        "read",
        std::time::Duration::from_secs(900),
        chrono::Utc::now() + chrono::Duration::hours(8),
    )
    .await;
    cache_on(&other, &key).await;

    let (status, body) = send(&state, "POST", "/v1/auth/logout".to_string(), &key).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    assert!(
        refused_within_5s(&other, &key).await,
        "the other server still takes the logged-out key 5 s after the logout"
    );
}

#[tokio::test]
#[serial]
async fn a_key_rotated_out_by_refresh_is_refused_on_another_server() {
    let (other, _stop) = other_server().await;
    let state = test_state().await;
    let (key, _) = mint_oidc_session(
        &state,
        Some("public"),
        "read",
        std::time::Duration::from_secs(900),
        chrono::Utc::now() + chrono::Duration::hours(8),
    )
    .await;
    cache_on(&other, &key).await;

    let (status, body) = send_request(build_router(state.clone()), refresh_request(&key)).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    let new_key = body["key"].as_str().expect("a new key").to_string();
    assert!(
        refused_within_5s(&other, &key).await,
        "the other server still takes the rotated-out key 5 s after the refresh"
    );
    assert!(
        crate::routes::auth::validate_token(&other, &new_key)
            .await
            .is_ok(),
        "the other server takes the new key"
    );
    let _ = send(&state, "POST", "/v1/auth/logout".to_string(), &new_key).await;
}
