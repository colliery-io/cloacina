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

//! Router tests of the authorization model (CLOACINA-T-1055): the `agent`
//! role, the routes `read` lost, the WebSocket upgrades, an unknown role, and
//! the audit rows the middleware writes. They need Postgres, like the other
//! router tests of this crate.

use super::*;

/// Make a key with a name no other test run has. `tenant` `None` with
/// `platform_admin` is the shape of the bootstrap key.
async fn key_of(
    state: &AppState,
    role: &str,
    tenant: Option<&str>,
    platform_admin: bool,
) -> (String, String) {
    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();
    let name = format!("authz-test-{}", uuid::Uuid::new_v4().simple());
    cloacina::dal::DAL::new(state.database.clone())
        .api_keys()
        .create_key(&hash, &name, tenant, platform_admin, role)
        .await
        .expect("create authz test key");
    (plaintext, name)
}

fn call(method: &str, uri: &str, token: &str) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap()
}

async fn answer_of(
    state: &AppState,
    method: &str,
    uri: &str,
    token: &str,
) -> (StatusCode, serde_json::Value) {
    send_request(build_router(state.clone()), call(method, uri, token)).await
}

async fn status_of(state: &AppState, method: &str, uri: &str, token: &str) -> StatusCode {
    answer_of(state, method, uri, token).await.0
}

/// The codes of a refusal by the authorization check (`deny_response`). A
/// handler has its own 403s with other codes: a reactor or an accumulator
/// with no policy answers `reactor_op_denied` or `accumulator_access_denied`.
const CHECK_REFUSALS: [&str; 3] = [
    "insufficient_permissions",
    "admin_required",
    "tenant_access_denied",
];

/// True when the authorization check let the request through: the handler
/// may still refuse the empty body, not find the name, or refuse by the
/// policy of the endpoint.
fn passed(answer: &(StatusCode, serde_json::Value)) -> bool {
    let (status, body) = answer;
    match *status {
        StatusCode::UNAUTHORIZED => false,
        StatusCode::FORBIDDEN => !CHECK_REFUSALS.contains(&body["code"].as_str().unwrap_or("")),
        _ => true,
    }
}

/// True when the authorization check refused the request for its role.
fn refused_for_role(answer: &(StatusCode, serde_json::Value)) -> bool {
    answer.0 == StatusCode::FORBIDDEN && answer.1["code"] == "insufficient_permissions"
}

async fn audit_rows_of(state: &AppState, admin: &str, actor: &str) -> Vec<serde_json::Value> {
    let (status, body) = send_request(
        build_router(state.clone()),
        call(
            "GET",
            &format!("/v1/tenants/public/audit?actor={actor}&limit=200"),
            admin,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    body["items"].as_array().cloned().unwrap_or_default()
}

#[tokio::test]
#[serial]
async fn test_an_agent_key_reaches_the_agent_routes_and_nothing_else() {
    let state = test_state().await;
    let (agent, _) = key_of(&state, "agent", Some("public"), false).await;

    // The agent set: the check passes (the empty body is refused later).
    for (method, uri) in [
        ("POST", "/v1/agent/register"),
        ("POST", "/v1/agent/heartbeat"),
        ("POST", "/v1/agent/keys"),
        ("POST", "/v1/agent/result"),
        ("POST", "/v1/auth/ws-ticket"),
    ] {
        let answer = answer_of(&state, method, uri, &agent).await;
        assert!(passed(&answer), "{method} {uri}: {answer:?}");
    }

    // Everything else: 403.
    for (method, uri) in [
        ("GET", "/v1/tenants/public/workflows"),
        ("GET", "/v1/tenants/public/executions"),
        ("GET", "/v1/tenants/public/triggers"),
        ("GET", "/v1/tenants/public/fleet"),
        ("GET", "/v1/tenants/public/keys"),
        ("POST", "/v1/tenants/public/workflows/x/execute"),
        ("GET", "/v1/tenants/public/workflows/x/backfills"),
        ("POST", "/v1/tenants/public/workflows/x/backfills"),
        ("GET", "/v1/health/graphs"),
        ("POST", "/v1/health/reactors/x/fire"),
        ("POST", "/v1/health/accumulators/x/inject"),
        ("GET", "/v1/auth/whoami"),
        ("POST", "/v1/auth/refresh"),
        ("GET", "/v1/agents"),
        ("GET", "/v1/tenants"),
        ("GET", "/v1/ws/reactor/x"),
        ("GET", "/v1/ws/accumulator/x"),
        ("GET", "/v1/ws/delivery/exec_events:1"),
        ("GET", "/v1/ws/delivery/ops_metrics:global"),
    ] {
        let status = status_of(&state, method, uri, &agent).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {uri}");
    }
}

#[tokio::test]
#[serial]
async fn test_no_other_key_acts_as_an_agent() {
    let state = test_state().await;
    let (reader, _) = key_of(&state, "read", Some("public"), false).await;
    let (writer, _) = key_of(&state, "write", Some("public"), false).await;
    let (admin, _) = key_of(&state, "admin", Some("public"), false).await;
    // The shape of the bootstrap key: the platform admin flag does not make
    // a key an agent.
    let (bootstrap, _) = key_of(&state, "admin", None, true).await;

    for token in [&reader, &writer, &admin, &bootstrap] {
        for (method, uri) in [
            ("POST", "/v1/agent/register"),
            ("POST", "/v1/agent/heartbeat"),
            ("POST", "/v1/agent/keys"),
            ("POST", "/v1/agent/result"),
            ("GET", "/v1/agent/artifact/abc"),
            ("GET", "/v1/ws/delivery/agent:abc"),
        ] {
            let status = status_of(&state, method, uri, token).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{method} {uri}");
        }
    }
    // The flag still passes every other route.
    assert_eq!(
        status_of(&state, "GET", "/v1/tenants", &bootstrap).await,
        StatusCode::OK
    );
}

#[tokio::test]
#[serial]
async fn test_read_cannot_fire_inject_or_push_and_write_can() {
    let state = test_state().await;
    let (reader, reader_name) = key_of(&state, "read", Some("public"), false).await;
    let (writer, _) = key_of(&state, "write", Some("public"), false).await;
    let (admin, _) = key_of(&state, "admin", Some("public"), false).await;

    for (method, uri) in [
        ("POST", "/v1/health/reactors/no-such-reactor/fire"),
        ("POST", "/v1/health/accumulators/no-such-accumulator/inject"),
        // The accumulator socket: every frame is an inject.
        ("GET", "/v1/ws/accumulator/no-such-accumulator"),
    ] {
        let answer = answer_of(&state, method, uri, &reader).await;
        assert!(
            refused_for_role(&answer),
            "read: {method} {uri}: {answer:?}"
        );
        let answer = answer_of(&state, method, uri, &writer).await;
        assert!(passed(&answer), "write: {method} {uri}: {answer:?}");
    }
    // A reader still reads, and may connect to a reactor to read its state.
    assert_eq!(
        status_of(&state, "GET", "/v1/health/graphs", &reader).await,
        StatusCode::OK
    );
    let answer = answer_of(&state, "GET", "/v1/ws/reactor/no-such-reactor", &reader).await;
    assert!(passed(&answer), "{answer:?}");

    // The refusals are in the audit record.
    let rows = audit_rows_of(&state, &admin, &reader_name).await;
    for action in ["reactor.fire", "accumulator.inject", "accumulator.connect"] {
        let row = rows
            .iter()
            .find(|r| r["action"] == action)
            .unwrap_or_else(|| panic!("no {action} row"));
        assert_eq!(row["outcome"], "denied");
        assert_eq!(row["detail"]["reason"], "insufficient_role");
    }
}

/// Backfills (CLOACINA-T-1020): `read` lists and reads one; create, cancel
/// and retry ask `workflows.run`, which `write` holds.
#[tokio::test]
#[serial]
async fn test_read_cannot_create_cancel_or_retry_a_backfill_and_write_can() {
    let state = test_state().await;
    let (reader, reader_name) = key_of(&state, "read", Some("public"), false).await;
    let (writer, _) = key_of(&state, "write", Some("public"), false).await;
    let (admin, _) = key_of(&state, "admin", Some("public"), false).await;
    let base = "/v1/tenants/public/workflows/no-such-workflow/backfills";
    let id = uuid::Uuid::new_v4();

    for uri in [
        base.to_string(),
        format!("{base}/{id}/cancel"),
        format!("{base}/{id}/retry"),
    ] {
        let answer = answer_of(&state, "POST", &uri, &reader).await;
        assert!(refused_for_role(&answer), "read: POST {uri}: {answer:?}");
        let answer = answer_of(&state, "POST", &uri, &writer).await;
        assert!(passed(&answer), "write: POST {uri}: {answer:?}");
    }
    // A reader lists and reads one (there is none under this id).
    for uri in [base.to_string(), format!("{base}/{id}")] {
        let answer = answer_of(&state, "GET", &uri, &reader).await;
        assert!(passed(&answer), "read: GET {uri}: {answer:?}");
    }
    // A key of another tenant is refused for the tenant, whatever its role.
    let (other, _) = key_of(&state, "admin", Some("acme"), false).await;
    let answer = answer_of(&state, "POST", base, &other).await;
    assert_eq!(answer.0, StatusCode::FORBIDDEN, "{answer:?}");
    assert_eq!(answer.1["code"], "tenant_access_denied", "{answer:?}");

    // The refusals are in the audit record.
    let rows = audit_rows_of(&state, &admin, &reader_name).await;
    for action in ["backfill.create", "backfill.cancel", "backfill.retry"] {
        let row = rows
            .iter()
            .find(|r| r["action"] == action)
            .unwrap_or_else(|| panic!("no {action} row"));
        assert_eq!(row["outcome"], "denied");
        assert_eq!(row["detail"]["reason"], "insufficient_role");
    }
}

#[tokio::test]
#[serial]
async fn test_a_websocket_upgrade_needs_a_credential_and_takes_a_ticket_once() {
    let state = test_state().await;
    let (reader, _) = key_of(&state, "read", Some("public"), false).await;

    // No credential: 401, before the handler.
    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/v1/ws/reactor/x")
        .body(Body::empty())
        .unwrap();
    let (status, _) = send_request(build_router(state.clone()), req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // A ticket carries the scope of the key that asked for it.
    let (status, body) = send_request(
        build_router(state.clone()),
        call("POST", "/v1/auth/ws-ticket", &reader),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    let ticket = body["ticket"].as_str().expect("ticket").to_string();
    let with_ticket = |path: &str| {
        axum::http::Request::builder()
            .method("GET")
            .uri(format!("{path}?token={ticket}"))
            .body(Body::empty())
            .unwrap()
    };
    // The reader's ticket does not open an agent's stream.
    let (status, _) = send_request(
        build_router(state.clone()),
        with_ticket("/v1/ws/delivery/agent:abc"),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // It was used: a second use is refused.
    let (status, _) =
        send_request(build_router(state.clone()), with_ticket("/v1/ws/reactor/x")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // A ticket is not a credential on a REST route.
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/v1/tenants/public/workflows?token={ticket}"))
        .body(Body::empty())
        .unwrap();
    let (status, _) = send_request(build_router(state.clone()), req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn test_a_stored_key_with_an_unknown_role_is_refused_everywhere() {
    let state = test_state().await;
    // A role string from outside the four: it read as `read` before.
    let (token, _) = key_of(&state, "owner", Some("public"), false).await;
    for (method, uri) in [
        ("GET", "/v1/auth/whoami"),
        ("GET", "/v1/tenants/public/workflows"),
        ("GET", "/v1/health/graphs"),
        ("POST", "/v1/auth/ws-ticket"),
        ("POST", "/v1/agent/register"),
    ] {
        assert_eq!(
            status_of(&state, method, uri, &token).await,
            StatusCode::FORBIDDEN,
            "{method} {uri}"
        );
    }
}

#[tokio::test]
#[serial]
async fn test_the_middleware_writes_the_audit_row_of_a_logout() {
    let state = test_state().await;
    let (admin, _) = key_of(&state, "admin", Some("public"), false).await;
    let (leaving, leaving_name) = key_of(&state, "read", Some("public"), false).await;

    // The handler of logout notes nothing: the row comes from the route's row.
    assert_eq!(
        status_of(&state, "POST", "/v1/auth/logout", &leaving).await,
        StatusCode::OK
    );
    let rows = audit_rows_of(&state, &admin, &leaving_name).await;
    let row = rows
        .iter()
        .find(|r| r["action"] == "auth.logout")
        .expect("an auth.logout row");
    assert_eq!(row["outcome"], "ok");
    assert_eq!(row["tenant_id"], "public");
    assert_eq!(row["target_kind"], "session");

    // A read writes no row.
    let (reader, reader_name) = key_of(&state, "read", Some("public"), false).await;
    assert_eq!(
        status_of(&state, "GET", "/v1/tenants/public/workflows", &reader).await,
        StatusCode::OK
    );
    assert!(audit_rows_of(&state, &admin, &reader_name).await.is_empty());
}

#[tokio::test]
#[serial]
async fn test_an_account_takes_a_role_a_person_may_hold() {
    let state = test_state().await;
    let (admin, _) = key_of(&state, "admin", Some("public"), false).await;
    for (role, want) in [
        ("agent", StatusCode::BAD_REQUEST),
        ("owner", StatusCode::BAD_REQUEST),
        ("read", StatusCode::CREATED),
    ] {
        let username = format!("robin-{}", uuid::Uuid::new_v4().simple());
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/tenants/public/accounts")
            .header("Authorization", format!("Bearer {admin}"))
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "username": username,
                    "password": "a long enough passphrase 1",
                    "role": role
                })
                .to_string(),
            ))
            .unwrap();
        let (status, body) = send_request(build_router(state.clone()), req).await;
        assert_eq!(status, want, "role {role}: {body:?}");
    }
}
