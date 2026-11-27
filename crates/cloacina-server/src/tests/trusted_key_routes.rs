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

//! Router tests of the trusted keys of package signing (CLOACINA-T-1061,
//! CLOACINA-A-0018): who may add, list and revoke a tenant's keys, and the
//! upload check against the tenant's org first, then the server org. They
//! need Postgres, like the other router tests of this crate.
//!
//! No route stores a signature yet (CLOACINA-T-1062): the upload tests write
//! the signature row through the library. Every key is made in the test.

use super::*;

use base64::Engine as _;
use cloacina::crypto::{GeneratedKeypair, generate_signing_keypair};
use cloacina::security::{DbKeyManager, DbPackageSigner, KeyManager, PackageSigner, tenant_org_id};

/// The codes of an upload that the signature check refused.
const VERIFICATION_CODES: [&str; 5] = [
    "signature_not_found",
    "package_tampered",
    "untrusted_signer",
    "invalid_signature",
    "signature_verification_error",
];

/// A tenant name no other test run has. The trusted key routes need no
/// schema: a tenant's keys are rows of the admin schema.
fn tenant_name(label: &str) -> String {
    format!("t1061-{label}-{}", uuid::Uuid::new_v4().simple())
}

async fn key_of(
    state: &AppState,
    role: &str,
    tenant: Option<&str>,
    platform_admin: bool,
) -> (String, String) {
    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();
    let name = format!("t1061-test-{}", uuid::Uuid::new_v4().simple());
    cloacina::dal::DAL::new(state.database.clone())
        .api_keys()
        .create_key(&hash, &name, tenant, platform_admin, role)
        .await
        .expect("create trusted-key test key");
    (plaintext, name)
}

async fn send_json(
    state: &AppState,
    method: &str,
    uri: &str,
    token: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let req = axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    send_request(build_router(state.clone()), req).await
}

async fn add_key(
    state: &AppState,
    tenant: &str,
    token: &str,
    keypair: &GeneratedKeypair,
) -> (StatusCode, serde_json::Value) {
    send_json(
        state,
        "POST",
        &format!("/v1/tenants/{tenant}/trusted-keys"),
        token,
        serde_json::json!({
            "name": "t1061-test-key",
            "public_key": hex::encode(&keypair.public_key),
        }),
    )
    .await
}

async fn list_keys(state: &AppState, tenant: &str, token: &str) -> (StatusCode, serde_json::Value) {
    send_json(
        state,
        "GET",
        &format!("/v1/tenants/{tenant}/trusted-keys"),
        token,
        serde_json::json!({}),
    )
    .await
}

async fn revoke_key(
    state: &AppState,
    tenant: &str,
    token: &str,
    key_id: &str,
) -> (StatusCode, serde_json::Value) {
    send_json(
        state,
        "DELETE",
        &format!("/v1/tenants/{tenant}/trusted-keys/{key_id}"),
        token,
        serde_json::json!({}),
    )
    .await
}

/// The PEM form of an Ed25519 public key (SubjectPublicKeyInfo).
fn pem_of(public_key: &[u8]) -> String {
    let mut der = vec![
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    der.extend_from_slice(public_key);
    format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
        base64::engine::general_purpose::STANDARD.encode(der)
    )
}

fn row_of<'a>(list: &'a serde_json::Value, id: &str) -> Option<&'a serde_json::Value> {
    list["items"]
        .as_array()
        .and_then(|items| items.iter().find(|k| k["id"] == id))
}

// ----- the routes -----

#[tokio::test]
#[serial]
async fn test_a_tenant_admin_adds_lists_and_revokes_a_trusted_key() {
    let state = test_state().await;
    let tenant = tenant_name("a");
    let (admin, admin_name) = key_of(&state, "admin", Some(&tenant), false).await;
    let first = generate_signing_keypair();
    let second = generate_signing_keypair();

    // An empty list at first.
    let (status, body) = list_keys(&state, &tenant, &admin).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    assert_eq!(body["total"], 0);

    // Add from the raw key: the answer has the id and the fingerprint.
    let (status, body) = add_key(&state, &tenant, &admin, &first).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body:?}");
    assert_eq!(body["fingerprint"], first.fingerprint.as_str());
    assert_eq!(body["name"], "t1061-test-key");
    assert_eq!(body["revoked"], false);
    let first_id = body["id"].as_str().expect("id").to_string();

    // Add from PEM, with no name.
    let (status, body) = send_json(
        &state,
        "POST",
        &format!("/v1/tenants/{tenant}/trusted-keys"),
        &admin,
        serde_json::json!({ "public_key_pem": pem_of(&second.public_key) }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body:?}");
    assert_eq!(body["fingerprint"], second.fingerprint.as_str());
    assert!(body["name"].is_null());
    let second_id = body["id"].as_str().expect("id").to_string();

    // The key is in the org of the tenant.
    let km = DbKeyManager::new(cloacina::dal::DAL::new(state.database.clone()));
    let found = km
        .find_trusted_key(tenant_org_id(&tenant), &first.fingerprint)
        .await
        .unwrap();
    assert!(found.is_some(), "the key is not in the tenant's org");

    // The same key again: 409.
    let (status, body) = add_key(&state, &tenant, &admin, &first).await;
    assert_eq!(status, StatusCode::CONFLICT, "body: {body:?}");
    assert_eq!(body["code"], "trusted_key_exists");

    // A body with no key, with two keys, or with a key that is not one: 400.
    for bad in [
        serde_json::json!({ "name": "no key" }),
        serde_json::json!({
            "public_key": hex::encode(&first.public_key),
            "public_key_pem": pem_of(&first.public_key),
        }),
        serde_json::json!({ "public_key": "not hex" }),
        serde_json::json!({ "public_key": "abcd" }),
        serde_json::json!({ "public_key_pem": "not a pem" }),
    ] {
        let (status, body) = send_json(
            &state,
            "POST",
            &format!("/v1/tenants/{tenant}/trusted-keys"),
            &admin,
            bad.clone(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}: {body:?}");
    }

    // Both keys are listed, live.
    let (status, list) = list_keys(&state, &tenant, &admin).await;
    assert_eq!(status, StatusCode::OK, "body: {list:?}");
    assert_eq!(list["total"], 2);
    for id in [&first_id, &second_id] {
        let row = row_of(&list, id).expect("key in the list");
        assert_eq!(row["revoked"], false);
        assert!(row["revoked_at"].is_null());
    }

    // Revoke the first: the list marks it, and the second stays live.
    let (status, body) = revoke_key(&state, &tenant, &admin, &first_id).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    assert_eq!(body["status"], "revoked");
    assert_eq!(body["id"], first_id.as_str());
    let (_, list) = list_keys(&state, &tenant, &admin).await;
    assert_eq!(list["total"], 2);
    let row = row_of(&list, &first_id).expect("revoked key in the list");
    assert_eq!(row["revoked"], true);
    assert!(row["revoked_at"].is_string());
    assert_eq!(row_of(&list, &second_id).unwrap()["revoked"], false);

    // A revoked key cannot be added again.
    let (status, body) = add_key(&state, &tenant, &admin, &first).await;
    assert_eq!(status, StatusCode::CONFLICT, "body: {body:?}");

    // An unknown id is not found; an id that is no UUID is a bad request.
    let unknown = uuid::Uuid::new_v4().to_string();
    let (status, body) = revoke_key(&state, &tenant, &admin, &unknown).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body:?}");
    assert_eq!(body["code"], "trusted_key_not_found");
    let (status, _) = revoke_key(&state, &tenant, &admin, "not-a-uuid").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // The two writes are in the audit record of the tenant.
    let (status, audit) = send_json(
        &state,
        "GET",
        &format!("/v1/tenants/{tenant}/audit?actor={admin_name}&limit=200"),
        &admin,
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {audit:?}");
    let rows = audit["items"].as_array().cloned().unwrap_or_default();
    let added = rows
        .iter()
        .find(|r| r["action"] == "trusted_key.add" && r["target"] == first_id.as_str())
        .unwrap_or_else(|| panic!("no trusted_key.add row for the first key: {rows:?}"));
    assert_eq!(added["outcome"], "ok");
    assert_eq!(added["tenant_id"], tenant.as_str());
    assert_eq!(added["target_kind"], "trusted_key");
    assert_eq!(added["detail"]["fingerprint"], first.fingerprint.as_str());
    let revoked = rows
        .iter()
        .find(|r| r["action"] == "trusted_key.revoke" && r["target"] == first_id.as_str())
        .unwrap_or_else(|| panic!("no trusted_key.revoke row: {rows:?}"));
    assert_eq!(revoked["outcome"], "ok");
    // The refused adds and the list wrote no `ok` row.
    assert_eq!(
        rows.iter()
            .filter(|r| r["action"] == "trusted_key.add" && r["outcome"] == "ok")
            .count(),
        2
    );
}

#[tokio::test]
#[serial]
async fn test_a_key_of_another_tenant_cannot_list_add_or_revoke_trusted_keys() {
    let state = test_state().await;
    let tenant = tenant_name("a");
    let other = tenant_name("b");
    let (admin, _) = key_of(&state, "admin", Some(&tenant), false).await;
    let (other_admin, _) = key_of(&state, "admin", Some(&other), false).await;
    let keypair = generate_signing_keypair();

    let (status, body) = add_key(&state, &tenant, &admin, &keypair).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body:?}");
    let key_id = body["id"].as_str().expect("id").to_string();

    // The admin of another tenant, on this tenant's routes: refused by the
    // check, for the tenant.
    let answers = [
        list_keys(&state, &tenant, &other_admin).await,
        add_key(&state, &tenant, &other_admin, &generate_signing_keypair()).await,
        revoke_key(&state, &tenant, &other_admin, &key_id).await,
    ];
    for (status, body) in &answers {
        assert_eq!(*status, StatusCode::FORBIDDEN, "body: {body:?}");
        assert_eq!(body["code"], "tenant_access_denied");
    }

    // The same id on the other tenant's own route: not found, and not revoked.
    let (status, body) = revoke_key(&state, &other, &other_admin, &key_id).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body:?}");
    assert_eq!(body["code"], "trusted_key_not_found");

    // The other tenant sees none of this tenant's keys.
    let (status, list) = list_keys(&state, &other, &other_admin).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["total"], 0);

    // The key is still live in its own tenant.
    let (_, list) = list_keys(&state, &tenant, &admin).await;
    assert_eq!(list["total"], 1);
    assert_eq!(row_of(&list, &key_id).expect("key")["revoked"], false);

    // The same public key may be trusted by two tenants: one row for each.
    let (status, body) = add_key(&state, &other, &other_admin, &keypair).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body:?}");
    assert_ne!(body["id"], key_id.as_str());
}

#[tokio::test]
#[serial]
async fn test_read_write_and_agent_roles_are_refused_on_trusted_keys() {
    let state = test_state().await;
    let tenant = tenant_name("a");
    let (admin, _) = key_of(&state, "admin", Some(&tenant), false).await;
    let (status, body) = add_key(&state, &tenant, &admin, &generate_signing_keypair()).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body:?}");
    let key_id = body["id"].as_str().expect("id").to_string();

    for role in ["read", "write", "agent"] {
        let (token, name) = key_of(&state, role, Some(&tenant), false).await;
        let answers = [
            list_keys(&state, &tenant, &token).await,
            add_key(&state, &tenant, &token, &generate_signing_keypair()).await,
            revoke_key(&state, &tenant, &token, &key_id).await,
        ];
        for (status, body) in &answers {
            assert_eq!(*status, StatusCode::FORBIDDEN, "{role}: {body:?}");
            assert_eq!(body["code"], "insufficient_permissions", "{role}");
        }

        // The refused writes are in the audit record; the refused read is not.
        let (status, audit) = send_json(
            &state,
            "GET",
            &format!("/v1/tenants/{tenant}/audit?actor={name}&limit=200"),
            &admin,
            serde_json::json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "body: {audit:?}");
        let rows = audit["items"].as_array().cloned().unwrap_or_default();
        assert_eq!(rows.len(), 2, "{role}: {rows:?}");
        for action in ["trusted_key.add", "trusted_key.revoke"] {
            let row = rows
                .iter()
                .find(|r| r["action"] == action)
                .unwrap_or_else(|| panic!("{role}: no {action} row"));
            assert_eq!(row["outcome"], "denied");
            assert_eq!(row["detail"]["reason"], "insufficient_role");
        }
    }

    // Nothing was added and nothing was revoked.
    let (_, list) = list_keys(&state, &tenant, &admin).await;
    assert_eq!(list["total"], 1);
    assert_eq!(row_of(&list, &key_id).expect("key")["revoked"], false);

    // No key at all: 401.
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/v1/tenants/{tenant}/trusted-keys"))
        .body(Body::empty())
        .unwrap();
    let (status, _) = send_request(build_router(state.clone()), req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn test_a_platform_admin_manages_the_trusted_keys_of_any_tenant() {
    let state = test_state().await;
    let tenant = tenant_name("a");
    // The shape of the bootstrap key: no tenant, the platform admin flag.
    let (platform, _) = key_of(&state, "admin", None, true).await;
    let (tenant_admin, _) = key_of(&state, "admin", Some(&tenant), false).await;

    let (status, body) = add_key(&state, &tenant, &platform, &generate_signing_keypair()).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body:?}");
    let key_id = body["id"].as_str().expect("id").to_string();

    // The tenant admin sees the key the platform admin added.
    let (status, list) = list_keys(&state, &tenant, &tenant_admin).await;
    assert_eq!(status, StatusCode::OK);
    assert!(row_of(&list, &key_id).is_some());

    let (status, list) = list_keys(&state, &tenant, &platform).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["total"], 1);

    let (status, body) = revoke_key(&state, &tenant, &platform, &key_id).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    let (_, list) = list_keys(&state, &tenant, &tenant_admin).await;
    assert_eq!(row_of(&list, &key_id).expect("key")["revoked"], true);
}

// ----- the upload check -----

/// A state that requires signatures, with `server_org` as the server-wide
/// org, or with none.
async fn state_requiring_signatures(server_org: Option<cloacina::UniversalUuid>) -> AppState {
    let mut state = test_state().await;
    state.security_config = SecurityConfig {
        require_signatures: true,
        verification_org_id: server_org,
        ..SecurityConfig::default()
    };
    state
}

/// Bytes no other run has, with a signature row by `keypair`, written through
/// the library as the signing flow does.
async fn signed_payload(state: &AppState, keypair: &GeneratedKeypair) -> Vec<u8> {
    let bytes = format!("t1061 signed payload {}", uuid::Uuid::new_v4()).into_bytes();
    let signer = DbPackageSigner::new(cloacina::dal::DAL::new(state.database.clone()));
    let signature = signer
        .sign_package_data(&bytes, &keypair.private_key, &keypair.public_key)
        .expect("sign the payload");
    signer
        .store_signature(&signature)
        .await
        .expect("store the signature row");
    bytes
}

async fn upload(
    state: &AppState,
    tenant: &str,
    token: &str,
    bytes: &[u8],
) -> (StatusCode, serde_json::Value) {
    let (boundary, body) = multipart_file_body(bytes);
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/v1/tenants/{tenant}/workflows"))
        .header("Authorization", format!("Bearer {token}"))
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();
    send_request(build_router(state.clone()), req).await
}

/// True when the signature check let the upload through. The bytes are no
/// real archive, so the registration after the check refuses them with 400
/// `upload_failed`: that answer is past the check.
fn passed_the_signature_check(answer: &(StatusCode, serde_json::Value)) -> bool {
    let (status, body) = answer;
    assert_ne!(*status, StatusCode::UNAUTHORIZED, "body: {body:?}");
    let code = body["code"].as_str().unwrap_or("");
    if *status == StatusCode::FORBIDDEN {
        assert!(
            VERIFICATION_CODES.contains(&code),
            "a 403 that is not from the signature check: {body:?}"
        );
        return false;
    }
    assert_eq!(code, "upload_failed", "status {status}, body: {body:?}");
    true
}

fn refused_as(answer: &(StatusCode, serde_json::Value), code: &str) -> bool {
    answer.0 == StatusCode::FORBIDDEN && answer.1["code"] == code
}

/// The uploads go to the tenant `public`: its schema exists. Each run makes
/// its own keys, and revokes the ones it put in the org of `public`.
const UPLOAD_TENANT: &str = "public";

#[tokio::test]
#[serial]
async fn test_upload_signed_by_a_key_of_the_tenant_passes_and_a_revoked_key_is_refused() {
    // A server org is set, and holds no key of this test: the first step of
    // the lookup, the tenant's org, decides.
    let state = state_requiring_signatures(Some(cloacina::UniversalUuid::new_v4())).await;
    let (admin, _) = key_of(&state, "admin", Some(UPLOAD_TENANT), false).await;
    let keypair = generate_signing_keypair();

    // Before the tenant trusts the key: refused.
    let bytes = signed_payload(&state, &keypair).await;
    let answer = upload(&state, UPLOAD_TENANT, &admin, &bytes).await;
    assert!(refused_as(&answer, "untrusted_signer"), "{answer:?}");

    // The tenant admin adds the key through the route: the same bytes pass.
    let (status, body) = add_key(&state, UPLOAD_TENANT, &admin, &keypair).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body:?}");
    let key_id = body["id"].as_str().expect("id").to_string();
    let answer = upload(&state, UPLOAD_TENANT, &admin, &bytes).await;
    assert!(passed_the_signature_check(&answer), "{answer:?}");

    // Bytes with no signature row are refused, key or no key.
    let answer = upload(&state, UPLOAD_TENANT, &admin, b"t1061 unsigned bytes").await;
    assert!(refused_as(&answer, "signature_not_found"), "{answer:?}");

    // After the revoke: refused, for the old bytes and for new ones.
    let (status, body) = revoke_key(&state, UPLOAD_TENANT, &admin, &key_id).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    let answer = upload(&state, UPLOAD_TENANT, &admin, &bytes).await;
    assert!(refused_as(&answer, "untrusted_signer"), "{answer:?}");
    let fresh = signed_payload(&state, &keypair).await;
    let answer = upload(&state, UPLOAD_TENANT, &admin, &fresh).await;
    assert!(refused_as(&answer, "untrusted_signer"), "{answer:?}");
}

#[tokio::test]
#[serial]
async fn test_upload_signed_by_a_key_of_the_server_org_passes_for_a_tenant() {
    let server_org = cloacina::UniversalUuid::new_v4();
    let state = state_requiring_signatures(Some(server_org)).await;
    let (admin, _) = key_of(&state, "admin", Some(UPLOAD_TENANT), false).await;
    let keypair = generate_signing_keypair();

    // The key is in the server org only: the tenant's org does not hold it.
    let km = DbKeyManager::new(cloacina::dal::DAL::new(state.database.clone()));
    let server_key = km
        .trust_public_key(server_org, &keypair.public_key, Some("t1061-test-key"))
        .await
        .expect("trust the key in the server org");
    let in_tenant = km
        .find_trusted_key(tenant_org_id(UPLOAD_TENANT), &keypair.fingerprint)
        .await
        .unwrap();
    assert!(in_tenant.is_none());

    // The second step of the lookup accepts it.
    let bytes = signed_payload(&state, &keypair).await;
    let answer = upload(&state, UPLOAD_TENANT, &admin, &bytes).await;
    assert!(passed_the_signature_check(&answer), "{answer:?}");

    // Revoked in the server org: refused.
    km.revoke_trusted_key(server_org, server_key.id)
        .await
        .expect("revoke in the server org");
    let answer = upload(&state, UPLOAD_TENANT, &admin, &bytes).await;
    assert!(refused_as(&answer, "untrusted_signer"), "{answer:?}");
}

#[tokio::test]
#[serial]
async fn test_upload_signed_by_a_key_of_another_tenant_only_is_refused() {
    let state = state_requiring_signatures(Some(cloacina::UniversalUuid::new_v4())).await;
    let (admin, _) = key_of(&state, "admin", Some(UPLOAD_TENANT), false).await;
    let other = tenant_name("b");
    let (other_admin, _) = key_of(&state, "admin", Some(&other), false).await;
    let keypair = generate_signing_keypair();

    // Another tenant trusts the key; this tenant and the server org do not.
    let (status, body) = add_key(&state, &other, &other_admin, &keypair).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body:?}");

    let bytes = signed_payload(&state, &keypair).await;
    let answer = upload(&state, UPLOAD_TENANT, &admin, &bytes).await;
    assert!(refused_as(&answer, "untrusted_signer"), "{answer:?}");
}

#[tokio::test]
#[serial]
async fn test_upload_with_no_server_org_is_checked_against_the_tenant_only() {
    // `--require-signatures` with no `--verification-org-id`.
    let state = state_requiring_signatures(None).await;
    let (admin, _) = key_of(&state, "admin", Some(UPLOAD_TENANT), false).await;
    let keypair = generate_signing_keypair();

    // No key yet: the upload is refused for its signer, not for the config.
    let bytes = signed_payload(&state, &keypair).await;
    let answer = upload(&state, UPLOAD_TENANT, &admin, &bytes).await;
    assert!(refused_as(&answer, "untrusted_signer"), "{answer:?}");
    let answer = upload(&state, UPLOAD_TENANT, &admin, b"t1061 unsigned bytes").await;
    assert!(refused_as(&answer, "signature_not_found"), "{answer:?}");

    // With the tenant's key: passes.
    let (status, body) = add_key(&state, UPLOAD_TENANT, &admin, &keypair).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body:?}");
    let key_id = body["id"].as_str().expect("id").to_string();
    let answer = upload(&state, UPLOAD_TENANT, &admin, &bytes).await;
    assert!(passed_the_signature_check(&answer), "{answer:?}");

    let (status, _) = revoke_key(&state, UPLOAD_TENANT, &admin, &key_id).await;
    assert_eq!(status, StatusCode::OK);
}
