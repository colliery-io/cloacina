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

//! CLOACINA-T-1082: an OIDC login session follows the current group map at
//! each refresh, and the groups at the IdP each hour. They need Postgres,
//! like the other router tests.

use super::*;

/// A login session of one tenant row, opened the way the callback opens it:
/// the claims on the row, a `login_id`. Returns the key, its id and the
/// login id.
async fn open_login_session(
    state: &AppState,
    role: &str,
    groups: &[&str],
    refresh_enc: Option<Vec<u8>>,
) -> (String, uuid::Uuid, uuid::Uuid, String) {
    let subject = format!("sub-{}", uuid::Uuid::new_v4());
    let principal = ResolvedPrincipal {
        tenant: Some("public".to_string()),
        role: role.to_string(),
        provenance: format!("oidc:{TEST_ISSUER}:{subject}"),
    };
    let (plaintext, info) =
        mint_for_principal(state, &principal, std::time::Duration::from_secs(900))
            .await
            .expect("mint login key");
    let login_id = uuid::Uuid::new_v4();
    let claims = serde_json::json!({
        "sub": subject,
        "email": null,
        "email_verified": null,
        "groups": groups,
    })
    .to_string();
    cloacina::dal::DAL::new(state.database.clone())
        .oidc_sessions()
        .create_login_session(
            info.id,
            TEST_ISSUER,
            chrono::Utc::now() + chrono::Duration::hours(8),
            login_id,
            &claims,
            refresh_enc,
        )
        .await
        .expect("open login session");
    (plaintext, info.id, login_id, subject)
}

fn with_map(state: &AppState, map: &str) -> AppState {
    let mut state = state.clone();
    state.oidc_policy = Arc::new(crate::oidc::MappingPolicy::parse(map));
    state
}

/// The map changed (a rolling restart): the next refresh applies it. A lower
/// role is the new role; a tenant no longer mapped ends the session.
#[tokio::test]
#[serial]
async fn a_changed_map_applies_at_the_next_refresh() {
    let state = with_map(&test_state().await, "group:eng=public:write");
    let (key, _, _, _) = open_login_session(&state, "write", &["eng"], None).await;

    let lower = with_map(&state, "group:eng=public:read");
    let (status, body) = send_request(build_router(lower.clone()), refresh_request(&key)).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["role"], "read", "body: {body}");
    let key = body["key"].as_str().expect("a key").to_string();

    let gone = with_map(&state, "group:other=public:write");
    let (status, body) = send_request(build_router(gone), refresh_request(&key)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body: {body}");
    assert!(
        crate::routes::auth::validate_token(&state, &key)
            .await
            .is_err(),
        "the key of an ended session is revoked"
    );
}

/// A mock IdP: answers each group check with the next answer, and counts the
/// calls and the tokens it was given.
struct MockIdp {
    answer: std::sync::Mutex<Option<crate::oidc::ClaimsCheck>>,
    calls: std::sync::atomic::AtomicUsize,
    tokens: std::sync::Mutex<Vec<String>>,
}

impl MockIdp {
    fn new(answer: crate::oidc::ClaimsCheck) -> Self {
        Self {
            answer: std::sync::Mutex::new(Some(answer)),
            calls: std::sync::atomic::AtomicUsize::new(0),
            tokens: std::sync::Mutex::new(Vec::new()),
        }
    }
    fn calls(&self) -> usize {
        self.calls.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl crate::oidc::IdpClaimsCheck for MockIdp {
    async fn check(&self, refresh_token: &str) -> crate::oidc::ClaimsCheck {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.tokens.lock().unwrap().push(refresh_token.to_string());
        self.answer
            .lock()
            .unwrap()
            .take()
            .unwrap_or(crate::oidc::ClaimsCheck::Unreachable(
                "no more answers".into(),
            ))
    }
}

fn current(subject: &str, groups: &[&str], token: Option<&str>) -> crate::oidc::ClaimsCheck {
    crate::oidc::ClaimsCheck::Current {
        claims: crate::oidc::IdentityClaims {
            subject: subject.to_string(),
            email: None,
            email_verified: None,
            groups: groups.iter().map(|g| g.to_string()).collect(),
        },
        refresh_token: token.map(str::to_string),
    }
}

/// A test KEK, set for the process. The tests are serial.
fn set_test_kek() -> Vec<u8> {
    use base64::Engine;
    let kek = vec![7u8; 32];
    // SAFETY: the tests that read the KEK are `#[serial]`.
    unsafe {
        std::env::set_var(
            cloacina::security::KEK_ENV_VAR,
            base64::engine::general_purpose::STANDARD.encode(&kek),
        )
    };
    kek
}

/// A login session with a stored refresh token whose last check was two
/// hours ago, so the next refresh checks the groups at the IdP.
async fn session_due_for_a_check(
    state: &AppState,
    groups: &[&str],
) -> (String, uuid::Uuid, String) {
    use diesel::RunQueryDsl;
    set_test_kek();
    let (key, _, login_id, subject) = open_login_session(state, "write", groups, None).await;
    let sealed =
        crate::routes::session::seal_refresh_token(Some("rt-1"), login_id).expect("seal the token");
    let conn = state
        .database
        .get_postgres_connection()
        .await
        .expect("connection");
    conn.interact(move |conn| {
        diesel::sql_query(
            "UPDATE oidc_sessions SET refresh_enc = $1, checked_at = now() - interval '2 hours' \
             WHERE login_id = $2",
        )
        .bind::<diesel::sql_types::Bytea, _>(sealed)
        .bind::<diesel::sql_types::Uuid, _>(login_id)
        .execute(conn)
    })
    .await
    .expect("interact")
    .expect("update");
    (key, login_id, subject)
}

async fn refresh_with_idp(
    state: &AppState,
    key: &str,
    idp: &MockIdp,
) -> (StatusCode, serde_json::Value) {
    let auth = crate::routes::auth::validate_token(state, key)
        .await
        .expect("the key works");
    let response = crate::routes::session::refresh_with(
        state,
        auth,
        Some(idp as &dyn crate::oidc::IdpClaimsCheck),
    )
    .await;
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::json!({})),
    )
}

/// The person left the group at the IdP: the next refresh after the check
/// ends the session. The IdP got the stored token.
#[tokio::test]
#[serial]
async fn a_group_removed_at_the_idp_ends_the_session() {
    let state = with_map(&test_state().await, "group:eng=public:write");
    let (key, _, subject) = session_due_for_a_check(&state, &["eng"]).await;
    let idp = MockIdp::new(current(&subject, &[], None));

    let (status, body) = refresh_with_idp(&state, &key, &idp).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body: {body}");
    assert_eq!(idp.calls(), 1);
    assert_eq!(idp.tokens.lock().unwrap().as_slice(), ["rt-1"]);
    assert!(
        crate::routes::auth::validate_token(&state, &key)
            .await
            .is_err()
    );
}

/// Moved to a group with a lower role: the new key has the lower role, and
/// the rotated refresh token is stored for the next check.
#[tokio::test]
#[serial]
async fn a_lower_group_at_the_idp_gives_the_lower_role() {
    let state = with_map(
        &test_state().await,
        "group:eng=public:write;group:viewers=public:read",
    );
    let (key, login_id, subject) = session_due_for_a_check(&state, &["eng"]).await;
    let idp = MockIdp::new(current(&subject, &["viewers"], Some("rt-2")));

    let (status, body) = refresh_with_idp(&state, &key, &idp).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["role"], "read");

    let new_key = body["key"].as_str().expect("a key").to_string();
    let new_id = crate::routes::auth::validate_token(&state, &new_key)
        .await
        .expect("new key")
        .key_id;
    let row = cloacina::dal::DAL::new(state.database.clone())
        .oidc_sessions()
        .get_login_session(new_id)
        .await
        .expect("read")
        .expect("the session moved to the new key");
    assert_eq!(row.login_id, Some(login_id));
    assert!(row.claims.as_deref().unwrap().contains("viewers"));
    let kek = set_test_kek();
    let token = cloacina::dal::unified::oidc_sessions::decrypt_token_bound(
        row.refresh_enc.as_deref().unwrap(),
        &kek,
        cloacina::dal::unified::oidc_sessions::refresh_token_aad(login_id).as_bytes(),
    )
    .unwrap();
    assert_eq!(token, b"rt-2");

    // The check was just done: the next refresh does not call the IdP.
    let idp2 = MockIdp::new(current(&subject, &[], None));
    let (status, body) = refresh_with_idp(&state, &new_key, &idp2).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["role"], "read");
    assert_eq!(idp2.calls(), 0);
}

/// The IdP cannot be reached: the session keeps its groups and its role.
#[tokio::test]
#[serial]
async fn an_unreachable_idp_keeps_the_session() {
    let state = with_map(&test_state().await, "group:eng=public:write");
    let (key, _, _) = session_due_for_a_check(&state, &["eng"]).await;
    let idp = MockIdp::new(crate::oidc::ClaimsCheck::Unreachable(
        "connection refused".into(),
    ));

    let (status, body) = refresh_with_idp(&state, &key, &idp).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["role"], "write");
    assert_eq!(idp.calls(), 1);
}

/// The IdP refuses the refresh token: every row of the login ends.
#[tokio::test]
#[serial]
async fn a_refused_refresh_token_ends_the_login() {
    let state = with_map(&test_state().await, "group:eng=public:write");
    let (key, login_id, _) = session_due_for_a_check(&state, &["eng"]).await;
    let idp = MockIdp::new(crate::oidc::ClaimsCheck::Refused("invalid_grant".into()));

    let (status, body) = refresh_with_idp(&state, &key, &idp).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body: {body}");
    assert!(
        crate::routes::auth::validate_token(&state, &key)
            .await
            .is_err()
    );
    let keys = cloacina::dal::DAL::new(state.database.clone())
        .oidc_sessions()
        .delete_login(login_id)
        .await
        .expect("read");
    assert!(keys.is_empty(), "no row of the login is left");
}

/// An answer for another subject ends the login.
#[tokio::test]
#[serial]
async fn an_answer_for_another_subject_ends_the_login() {
    let state = with_map(&test_state().await, "group:eng=public:write");
    let (key, _, _) = session_due_for_a_check(&state, &["eng"]).await;
    let idp = MockIdp::new(current("someone-else", &["eng"], None));

    let (status, body) = refresh_with_idp(&state, &key, &idp).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "body: {body}");
}

/// A session opened before the upgrade (no stored claims) keeps its role to
/// its deadline, whatever the map says now.
#[tokio::test]
#[serial]
async fn a_session_from_before_the_upgrade_keeps_its_role() {
    let state = with_map(&test_state().await, "group:other=public:read");
    let (key, _) = mint_oidc_session(
        &state,
        Some("public"),
        "write",
        std::time::Duration::from_secs(900),
        chrono::Utc::now() + chrono::Duration::hours(8),
    )
    .await;
    let idp = MockIdp::new(current("x", &[], None));
    let (status, body) = refresh_with_idp(&state, &key, &idp).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["role"], "write");
    assert_eq!(idp.calls(), 0);
}
