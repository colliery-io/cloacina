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

//! Two tenants never share a Kubernetes namespace (CLOACINA-T-1070). Tenant
//! create refuses a name whose namespace is another tenant's. They need
//! Postgres, like the other router tests.

use super::*;

async fn create(state: &AppState, token: &str, tenant: &str) -> (StatusCode, serde_json::Value) {
    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/tenants")
        .header("Authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({ "name": tenant, "password": "testpass123" }).to_string(),
        ))
        .unwrap();
    send_request(build_router(state.clone()), request).await
}

async fn delete(state: &AppState, token: &str, tenant: &str) -> StatusCode {
    let request = axum::http::Request::builder()
        .method("DELETE")
        .uri(format!("/v1/tenants/{tenant}"))
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    send_request(build_router(state.clone()), request).await.0
}

/// `t1070_<id>` and `t1070_<id>_` are two valid tenant names with one
/// namespace. The second create is a 409 that names the first tenant, and
/// makes no tenant.
#[tokio::test]
#[serial]
async fn a_name_whose_namespace_is_taken_is_a_conflict() {
    let state = test_state().await;
    let token = create_test_api_key(&state).await;
    let first = format!("t1070_{}", uuid::Uuid::new_v4().simple());
    let second = format!("{first}_");
    assert_eq!(
        crate::actuator::kubernetes::tenant_namespace(&first),
        crate::actuator::kubernetes::tenant_namespace(&second)
    );

    let (status, body) = create(&state, &token, &first).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body:?}");

    let (status, body) = create(&state, &token, &second).await;
    // Clean up before the asserts, so a failed assert leaves less.
    let second_status = if status == StatusCode::CREATED {
        Some(delete(&state, &token, &second).await)
    } else {
        None
    };
    assert_eq!(delete(&state, &token, &first).await, StatusCode::OK);

    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "two tenants were given one namespace; body: {body:?}, cleanup: {second_status:?}"
    );
    assert_eq!(body["code"], "tenant_namespace_taken", "body: {body:?}");
    let message = body["error"].as_str().unwrap_or_default();
    assert!(message.contains(&first), "message: {message}");
    assert!(
        message.contains(&crate::actuator::kubernetes::tenant_namespace(&first)),
        "message: {message}"
    );
}

/// With the first tenant gone, the namespace is free and the second name is
/// taken.
#[tokio::test]
#[serial]
async fn a_namespace_is_free_again_after_its_tenant_is_removed() {
    let state = test_state().await;
    let token = create_test_api_key(&state).await;
    let first = format!("t1070_{}", uuid::Uuid::new_v4().simple());
    let second = format!("_{first}");

    assert_eq!(create(&state, &token, &first).await.0, StatusCode::CREATED);
    assert_eq!(delete(&state, &token, &first).await, StatusCode::OK);
    let (status, body) = create(&state, &token, &second).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body:?}");
    assert_eq!(delete(&state, &token, &second).await, StatusCode::OK);
}
