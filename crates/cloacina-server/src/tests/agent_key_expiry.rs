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

//! CLOACINA-T-1083: key expiry. A key a person creates takes an optional
//! `expires_at`; an expired key is answered 401 `key_expired`, also when it
//! is in the key cache; the keys the Kubernetes actuator mints expire and the
//! Docker actuator's do not. They need Postgres, like the other router tests.

use super::*;
use crate::actuator::docker::{DalKeyMinter, KeyMinter};

async fn send(
    state: &AppState,
    method: &str,
    uri: &str,
    token: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let mut request = axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"));
    let body = match body {
        Some(json) => {
            request = request.header("Content-Type", "application/json");
            Body::from(json.to_string())
        }
        None => Body::empty(),
    };
    send_request(build_router(state.clone()), request.body(body).unwrap()).await
}

#[tokio::test]
#[serial]
async fn a_key_created_with_an_expiry_carries_it() {
    let state = test_state().await;
    let admin = create_test_api_key(&state).await;
    let at = (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339();
    for uri in ["/v1/auth/keys", "/v1/tenants/public/keys"] {
        let (status, body) = send(
            &state,
            "POST",
            uri,
            &admin,
            Some(serde_json::json!({ "name": "t1083-expiring", "role": "read", "expires_at": at })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{uri}: {body:?}");
        let got = chrono::DateTime::parse_from_rfc3339(body["expires_at"].as_str().unwrap())
            .unwrap()
            .with_timezone(&chrono::Utc);
        let want = chrono::DateTime::parse_from_rfc3339(&at)
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert!(
            (got - want).num_seconds().abs() <= 1,
            "{uri}: {got} vs {want}"
        );

        // The key list shows it.
        let id = body["id"].as_str().unwrap().to_string();
        let (status, list) = send(&state, "GET", uri, &admin, None).await;
        assert_eq!(status, StatusCode::OK, "{list:?}");
        let row = list["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|k| k["id"] == id.as_str())
            .expect("the new key is listed");
        assert!(row["expires_at"].is_string(), "{row:?}");
    }
}

#[tokio::test]
#[serial]
async fn a_key_created_with_no_expiry_has_none() {
    let state = test_state().await;
    let admin = create_test_api_key(&state).await;
    let (status, body) = send(
        &state,
        "POST",
        "/v1/tenants/public/keys",
        &admin,
        Some(serde_json::json!({ "name": "t1083-forever", "role": "read" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body:?}");
    assert!(body["expires_at"].is_null(), "{body:?}");
}

#[tokio::test]
#[serial]
async fn an_expiry_in_the_past_or_not_a_time_is_refused() {
    let state = test_state().await;
    let admin = create_test_api_key(&state).await;
    let past = (chrono::Utc::now() - chrono::Duration::minutes(1)).to_rfc3339();
    for value in [past.as_str(), "tomorrow"] {
        let (status, body) = send(
            &state,
            "POST",
            "/v1/auth/keys",
            &admin,
            Some(serde_json::json!({ "name": "t1083-bad", "role": "read", "expires_at": value })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{value}: {body:?}");
        assert_eq!(body["code"], "invalid_expires_at", "{value}: {body:?}");
    }
}

/// An expired key is answered 401 `key_expired`, also when this server has it
/// in its key cache from a request just before the expiry.
#[tokio::test]
#[serial]
async fn an_expired_key_is_answered_key_expired_and_leaves_the_cache() {
    let state = test_state().await;
    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();
    cloacina::dal::DAL::new(state.database.clone())
        .api_keys()
        .create_key_expiring(
            &hash,
            "t1083-short",
            Some("public"),
            false,
            "read",
            Some(chrono::Utc::now() + chrono::Duration::seconds(2)),
        )
        .await
        .unwrap();

    let (status, body) = send(&state, "GET", "/v1/auth/whoami", &plaintext, None).await;
    assert_eq!(status, StatusCode::OK, "before the expiry: {body:?}");
    assert!(state.key_cache.get(&hash).await.is_some(), "cached");

    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
    let (status, body) = send(&state, "GET", "/v1/auth/whoami", &plaintext, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body:?}");
    assert_eq!(body["code"], "key_expired", "{body:?}");
    assert!(state.key_cache.get(&hash).await.is_none(), "dropped");

    // A key that does not exist keeps the old answer.
    let (status, body) = send(&state, "GET", "/v1/auth/whoami", "clk_nope", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "unauthorized", "{body:?}");
}

/// The Kubernetes actuator's minter gives a key the expiry of the rule; the
/// Docker actuator's minter gives none.
#[tokio::test]
#[serial]
async fn an_actuator_key_carries_the_expiry_and_a_docker_key_has_none() {
    let state = test_state().await;
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let rule = crate::actuator::AgentKeyExpiry::default();

    let kube = DalKeyMinter::new(state.database.clone()).with_expiry(rule);
    let before = chrono::Utc::now();
    let key = kube.mint("public").await.unwrap();
    let info = dal
        .api_keys()
        .get_key(uuid::Uuid::parse_str(&key.id).unwrap())
        .await
        .unwrap()
        .unwrap();
    let at = info.expires_at.expect("an expiry");
    assert!(at >= before + chrono::Duration::days(30) - chrono::Duration::seconds(1));
    assert!(at <= chrono::Utc::now() + chrono::Duration::days(30));
    assert!(kube.is_live_agent_key("public", &key.id).await.unwrap());

    let docker = DalKeyMinter::new(state.database.clone());
    let key = docker.mint("public").await.unwrap();
    let info = dal
        .api_keys()
        .get_key(uuid::Uuid::parse_str(&key.id).unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(info.expires_at, None);
    assert!(docker.is_live_agent_key("public", &key.id).await.unwrap());
    // A key with no expiry is one from before the rule for the Kubernetes
    // minter: the first reconcile after the upgrade replaces it.
    assert!(!kube.is_live_agent_key("public", &key.id).await.unwrap());
}

/// The actuator's revoke of an old agent key sends the revocation notice: a
/// second server that had the key in its cache refuses it within 5 s, not
/// after its 30 s.
#[tokio::test]
#[serial]
async fn the_actuators_revoke_is_refused_on_another_server() {
    let other = test_state().await;
    let (_stop, stop_rx) = tokio::sync::watch::channel(false);
    crate::routes::auth::spawn_key_revocation_listener(
        TEST_DB_URL.to_string(),
        other.key_cache.clone(),
        stop_rx,
    );
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let state = test_state().await;
    let notice: std::sync::Arc<dyn crate::actuator::KeyRevocationNotice> =
        std::sync::Arc::new(crate::routes::auth::DbKeyRevocationNotice::new(
            state.database.clone(),
            state.key_cache.clone(),
        ));
    let minter = crate::actuator::docker::minter_with_notice(
        std::sync::Arc::new(
            DalKeyMinter::new(state.database.clone())
                .with_expiry(crate::actuator::AgentKeyExpiry::default()),
        ),
        Some(notice),
    );
    let old = minter.mint("public").await.unwrap();
    let new = minter.mint("public").await.unwrap();
    crate::routes::auth::validate_token(&other, &old.plaintext)
        .await
        .expect("the other server takes the old key and caches it");

    let revoked = minter
        .revoke_others("public", std::slice::from_ref(&new.id))
        .await
        .unwrap();
    assert!(revoked >= 1);

    let started = std::time::Instant::now();
    loop {
        if crate::routes::auth::validate_token(&other, &old.plaintext)
            .await
            .is_err()
        {
            break;
        }
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "the other server still takes the revoked agent key 5 s after the revoke"
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(
        crate::routes::auth::validate_token(&other, &new.plaintext)
            .await
            .is_ok()
    );
}

/// A key with less than the rotation window left is not live for the
/// Kubernetes minter: the next reconcile replaces it.
#[tokio::test]
#[serial]
async fn a_stored_key_with_9_days_left_is_due_for_rotation() {
    let state = test_state().await;
    let (_, hash) = cloacina::security::api_keys::generate_api_key();
    let info = cloacina::dal::DAL::new(state.database.clone())
        .api_keys()
        .create_key_expiring(
            &hash,
            &format!("agent:public:{}", uuid::Uuid::new_v4()),
            Some("public"),
            false,
            "agent",
            Some(chrono::Utc::now() + chrono::Duration::days(9)),
        )
        .await
        .unwrap();
    let kube = DalKeyMinter::new(state.database.clone())
        .with_expiry(crate::actuator::AgentKeyExpiry::default());
    assert!(
        !kube
            .is_live_agent_key("public", &info.id.to_string())
            .await
            .unwrap()
    );
}
