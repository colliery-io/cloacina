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

//! A signed upload, end to end through the upload route (CLOACINA-T-1062,
//! CLOACINA-A-0018): the signature is the `signature` field of the upload,
//! the row is stored in the tenant's schema with the org whose key matched,
//! and the tenant's reconciler verifies it at load. They need Postgres, like
//! the other router tests of this crate.
//!
//! Each test makes its own tenants (a schema each) and removes them. Every
//! key is made in the test.

use super::*;

use cloacina::crypto::{GeneratedKeypair, generate_signing_keypair};
use cloacina::dal::DAL;
use cloacina::registry::reconciler::RegistryReconciler;
use cloacina::registry::traits::WorkflowRegistry;
use cloacina::security::{
    DbKeyManager, DbPackageSigner, DetachedSignature, KeyManager, PackageSigner, tenant_org_id,
};
use sha2::{Digest, Sha256};

/// The name of every trusted key these tests add.
const KEY_NAME: &str = "t1062-test-key";

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// A real package archive: the upload registers it.
fn package_bytes() -> Vec<u8> {
    std::fs::read(fixture_path("python-workflow.cloacina")).expect("fixture")
}

/// A state that requires signatures or not, with `server_org` as the
/// server-wide org, or with none.
async fn state_with(require: bool, server_org: Option<cloacina::UniversalUuid>) -> AppState {
    let mut state = test_state().await;
    state.security_config = SecurityConfig {
        require_signatures: require,
        verification_org_id: server_org,
        ..SecurityConfig::default()
    };
    state
}

/// Makes a tenant schema.
async fn make_tenant(state: &AppState, label: &str) -> String {
    let name = format!("t1062_{}_{}", label, uuid::Uuid::new_v4().simple());
    cloacina::database::DatabaseAdmin::new(state.database.clone())
        .create_tenant(cloacina::database::TenantConfig {
            schema_name: name.clone(),
            username: name.clone(),
            password: "testpass123".to_string(),
        })
        .await
        .expect("create tenant");
    name
}

/// A database of the tenant's schema that no server state owns.
fn tenant_database(tenant: &str) -> cloacina::Database {
    cloacina::Database::try_new_with_schema(TEST_DB_URL, "cloacina", 2, Some(tenant))
        .expect("tenant database")
}

/// Stops the tenant runners of the state, revokes nothing, and removes the
/// tenants with their trusted keys.
async fn clean_up(state: &AppState, tenants: &[&String]) {
    let _ = state.tenant_runners.shutdown_all().await;
    let admin = cloacina::database::DatabaseAdmin::new(state.database.clone());
    for tenant in tenants {
        state.tenant_databases.evict(tenant).await;
        let _ = admin.remove_tenant(tenant, tenant).await;
    }
}

/// The detached signature of `bytes` by `keypair`, as `cloacinactl package
/// pack --sign` writes it: the text of the `signature` field.
fn sign(bytes: &[u8], keypair: &GeneratedKeypair) -> String {
    DetachedSignature::sign(bytes, &keypair.private_key)
        .expect("sign")
        .to_json()
        .expect("signature json")
}

/// The tenant trusts the key, through the route. Returns the key id.
async fn trust(state: &AppState, tenant: &str, token: &str, keypair: &GeneratedKeypair) -> String {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/v1/tenants/{tenant}/trusted-keys"))
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "name": KEY_NAME,
                "public_key": hex::encode(&keypair.public_key),
            })
            .to_string(),
        ))
        .unwrap();
    let (status, body) = send_request(build_router(state.clone()), req).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body:?}");
    body["id"].as_str().expect("id").to_string()
}

/// The tenant revokes the key, through the route.
async fn revoke(state: &AppState, tenant: &str, token: &str, key_id: &str) {
    let req = axum::http::Request::builder()
        .method("DELETE")
        .uri(format!("/v1/tenants/{tenant}/trusted-keys/{key_id}"))
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let (status, body) = send_request(build_router(state.clone()), req).await;
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
}

async fn upload(
    state: &AppState,
    tenant: &str,
    token: &str,
    bytes: &[u8],
    signature: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let (boundary, body) = multipart_upload_body(bytes, signature);
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

/// Every signature row of the bytes in one schema: `(fingerprint, org)`,
/// the org `None` for a row stored with none.
async fn signature_rows(
    database: &cloacina::Database,
    bytes: &[u8],
    orgs: &[cloacina::UniversalUuid],
) -> (usize, Vec<(String, cloacina::UniversalUuid)>) {
    let signer = DbPackageSigner::new(DAL::new(database.clone()));
    let hash = sha256_hex(bytes);
    let all = signer.find_signatures(&hash).await.expect("rows").len();
    let for_orgs = signer
        .find_signatures_for_orgs(&hash, orgs)
        .await
        .expect("rows of the orgs")
        .into_iter()
        .map(|(row, org)| (row.key_fingerprint, org))
        .collect();
    (all, for_orgs)
}

fn refused(answer: &(StatusCode, serde_json::Value), status: StatusCode, code: &str) -> bool {
    answer.0 == status && answer.1["code"] == code
}

// ----- the upload route -----

/// With signatures required, an upload signed by a trusted key of the tenant
/// is accepted. The row is in the tenant's schema, with the tenant's org, and
/// not in the admin schema. The audit row names the signer.
#[tokio::test]
#[serial]
async fn a_signed_upload_by_a_key_of_the_tenant_is_accepted_and_the_row_has_the_tenant_org() {
    let state = state_with(true, None).await;
    let token = create_test_api_key(&state).await;
    let tenant = make_tenant(&state, "ok").await;
    let org = tenant_org_id(&tenant);
    let keypair = generate_signing_keypair();
    trust(&state, &tenant, &token, &keypair).await;

    let bytes = package_bytes();
    let signature = sign(&bytes, &keypair);
    let answer = upload(&state, &tenant, &token, &bytes, Some(&signature)).await;
    assert_eq!(answer.0, StatusCode::CREATED, "body: {:?}", answer.1);
    let package_id = answer.1["package_id"].as_str().expect("package id");

    // One row, in the tenant's schema, for the tenant's org.
    let (all, for_org) = signature_rows(&tenant_database(&tenant), &bytes, &[org]).await;
    assert_eq!(all, 1);
    assert_eq!(for_org, vec![(keypair.fingerprint.clone(), org)]);
    // The admin schema holds no row for this tenant.
    let (_, in_admin) = signature_rows(&state.database, &bytes, &[org]).await;
    assert!(in_admin.is_empty(), "{in_admin:?}");

    // The audit row of the upload names the signer and the org that trusts it.
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/v1/tenants/{tenant}/audit?limit=200"))
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let (status, audit) = send_request(build_router(state.clone()), req).await;
    assert_eq!(status, StatusCode::OK, "body: {audit:?}");
    let rows = audit["items"].as_array().cloned().unwrap_or_default();
    let row = rows
        .iter()
        .find(|r| r["action"] == "package.upload" && r["target"] == package_id)
        .unwrap_or_else(|| panic!("no upload row: {rows:?}"));
    assert_eq!(
        row["detail"]["signer_fingerprint"],
        keypair.fingerprint.as_str()
    );
    assert_eq!(row["detail"]["signer_trusted_by_org"], org.to_string());

    // The same bytes again, signed by the same key: still one row.
    let _ = upload(&state, &tenant, &token, &bytes, Some(&signature)).await;
    let (all, _) = signature_rows(&tenant_database(&tenant), &bytes, &[org]).await;
    assert_eq!(all, 1);

    clean_up(&state, &[&tenant]).await;
}

/// With signatures required, each upload that must not pass is refused with
/// its own code, and none of them stores a row.
#[tokio::test]
#[serial]
async fn each_refused_upload_has_its_own_code() {
    let state = state_with(true, None).await;
    let token = create_test_api_key(&state).await;
    let tenant = make_tenant(&state, "refuse").await;
    let org = tenant_org_id(&tenant);
    let trusted = generate_signing_keypair();
    trust(&state, &tenant, &token, &trusted).await;
    let bytes = package_bytes();

    // No signature.
    let answer = upload(&state, &tenant, &token, &bytes, None).await;
    assert!(
        refused(&answer, StatusCode::FORBIDDEN, "signature_required"),
        "{answer:?}"
    );

    // Signed by a key nobody trusts.
    let stranger = generate_signing_keypair();
    let answer = upload(
        &state,
        &tenant,
        &token,
        &bytes,
        Some(&sign(&bytes, &stranger)),
    )
    .await;
    assert!(
        refused(&answer, StatusCode::FORBIDDEN, "untrusted_signer"),
        "{answer:?}"
    );

    // Signed by a key that only another tenant trusts: the same answer as
    // for a key nobody trusts.
    let other_tenant = format!("t1062_other_{}", uuid::Uuid::new_v4().simple());
    let others_key = generate_signing_keypair();
    trust(&state, &other_tenant, &token, &others_key).await;
    let answer = upload(
        &state,
        &tenant,
        &token,
        &bytes,
        Some(&sign(&bytes, &others_key)),
    )
    .await;
    assert!(
        refused(&answer, StatusCode::FORBIDDEN, "untrusted_signer"),
        "{answer:?}"
    );

    // A bad signature: the trusted key's fingerprint and the right hash, and
    // signature bytes that the key did not make.
    let mut forged = DetachedSignature::sign(&bytes, &trusted.private_key).expect("sign");
    forged.signature = DetachedSignature::sign(&bytes, &stranger.private_key)
        .expect("sign")
        .signature;
    let answer = upload(
        &state,
        &tenant,
        &token,
        &bytes,
        Some(&forged.to_json().unwrap()),
    )
    .await;
    assert!(
        refused(&answer, StatusCode::FORBIDDEN, "invalid_signature"),
        "{answer:?}"
    );

    // A signature of the trusted key for other bytes.
    let answer = upload(
        &state,
        &tenant,
        &token,
        &bytes,
        Some(&sign(b"t1062 other bytes", &trusted)),
    )
    .await;
    assert!(
        refused(&answer, StatusCode::FORBIDDEN, "package_tampered"),
        "{answer:?}"
    );

    // A field that is no signature, and a signature format this server does
    // not know.
    let answer = upload(&state, &tenant, &token, &bytes, Some("not a signature")).await;
    assert!(
        refused(&answer, StatusCode::BAD_REQUEST, "malformed_signature"),
        "{answer:?}"
    );
    let mut other_algorithm = DetachedSignature::sign(&bytes, &trusted.private_key).expect("sign");
    other_algorithm.algorithm = "rsa".to_string();
    let answer = upload(
        &state,
        &tenant,
        &token,
        &bytes,
        Some(&other_algorithm.to_json().unwrap()),
    )
    .await;
    assert!(
        refused(&answer, StatusCode::BAD_REQUEST, "malformed_signature"),
        "{answer:?}"
    );

    // Signed by a key the tenant revoked.
    let revoked = generate_signing_keypair();
    let revoked_id = trust(&state, &tenant, &token, &revoked).await;
    revoke(&state, &tenant, &token, &revoked_id).await;
    let answer = upload(
        &state,
        &tenant,
        &token,
        &bytes,
        Some(&sign(&bytes, &revoked)),
    )
    .await;
    assert!(
        refused(&answer, StatusCode::FORBIDDEN, "signer_revoked"),
        "{answer:?}"
    );

    // None of them stored a row.
    let (all, _) = signature_rows(&tenant_database(&tenant), &bytes, &[org]).await;
    assert_eq!(all, 0, "a refused upload stored a signature row");

    // The trusted key still passes.
    let answer = upload(
        &state,
        &tenant,
        &token,
        &bytes,
        Some(&sign(&bytes, &trusted)),
    )
    .await;
    assert_eq!(answer.0, StatusCode::CREATED, "body: {:?}", answer.1);

    clean_up(&state, &[&tenant]).await;
}

/// Two tenants upload the same bytes, each signed by its own key. Each keeps
/// its own row, with its own org. The signature of one tenant is not accepted
/// by the other.
#[tokio::test]
#[serial]
async fn the_same_bytes_in_two_tenants_keep_two_rows() {
    let state = state_with(true, None).await;
    let token = create_test_api_key(&state).await;
    let first = make_tenant(&state, "one").await;
    let second = make_tenant(&state, "two").await;
    let first_key = generate_signing_keypair();
    let second_key = generate_signing_keypair();
    trust(&state, &first, &token, &first_key).await;
    trust(&state, &second, &token, &second_key).await;
    let bytes = package_bytes();
    let orgs = [tenant_org_id(&first), tenant_org_id(&second)];

    // The first tenant's signature is no use to the second tenant.
    let first_signature = sign(&bytes, &first_key);
    let answer = upload(&state, &second, &token, &bytes, Some(&first_signature)).await;
    assert!(
        refused(&answer, StatusCode::FORBIDDEN, "untrusted_signer"),
        "{answer:?}"
    );

    let answer = upload(&state, &first, &token, &bytes, Some(&first_signature)).await;
    assert_eq!(answer.0, StatusCode::CREATED, "body: {:?}", answer.1);
    // The upload of the first tenant gave the second tenant nothing.
    let (all, _) = signature_rows(&tenant_database(&second), &bytes, &orgs).await;
    assert_eq!(all, 0);
    let answer = upload(&state, &second, &token, &bytes, None).await;
    assert!(
        refused(&answer, StatusCode::FORBIDDEN, "signature_required"),
        "{answer:?}"
    );

    let answer = upload(
        &state,
        &second,
        &token,
        &bytes,
        Some(&sign(&bytes, &second_key)),
    )
    .await;
    assert_eq!(answer.0, StatusCode::CREATED, "body: {:?}", answer.1);

    // Two rows: one in each schema, each with its tenant's org and key.
    let (all, rows) = signature_rows(&tenant_database(&first), &bytes, &orgs).await;
    assert_eq!(all, 1);
    assert_eq!(rows, vec![(first_key.fingerprint.clone(), orgs[0])]);
    let (all, rows) = signature_rows(&tenant_database(&second), &bytes, &orgs).await;
    assert_eq!(all, 1);
    assert_eq!(rows, vec![(second_key.fingerprint.clone(), orgs[1])]);

    clean_up(&state, &[&first, &second]).await;
}

/// The server-org fallback: a key of the server org signs for any tenant,
/// and the row carries the server org. A key the tenant revoked that is live
/// in the server org still passes. Revoked in the server org, it is refused.
#[tokio::test]
#[serial]
async fn a_key_of_the_server_org_signs_for_a_tenant_and_the_row_has_the_server_org() {
    let server_org = cloacina::UniversalUuid::new_v4();
    let state = state_with(true, Some(server_org)).await;
    let token = create_test_api_key(&state).await;
    let tenant = make_tenant(&state, "srv").await;
    let tenant_org = tenant_org_id(&tenant);
    let keypair = generate_signing_keypair();
    let km = DbKeyManager::new(DAL::new(state.database.clone()));
    let server_key = km
        .trust_public_key(server_org, &keypair.public_key, Some(KEY_NAME))
        .await
        .expect("trust the key in the server org");

    let bytes = package_bytes();
    let signature = sign(&bytes, &keypair);
    let answer = upload(&state, &tenant, &token, &bytes, Some(&signature)).await;
    assert_eq!(answer.0, StatusCode::CREATED, "body: {:?}", answer.1);
    let (all, rows) =
        signature_rows(&tenant_database(&tenant), &bytes, &[tenant_org, server_org]).await;
    assert_eq!(all, 1);
    assert_eq!(rows, vec![(keypair.fingerprint.clone(), server_org)]);

    // The tenant trusts the same key: the tenant's org is asked first, and
    // the row now carries it.
    let tenant_key = trust(&state, &tenant, &token, &keypair).await;
    let _ = upload(&state, &tenant, &token, &bytes, Some(&signature)).await;
    let (all, rows) =
        signature_rows(&tenant_database(&tenant), &bytes, &[tenant_org, server_org]).await;
    assert_eq!(all, 1);
    assert_eq!(rows, vec![(keypair.fingerprint.clone(), tenant_org)]);

    // The tenant revokes it; the server org still holds it live: it passes,
    // by the server org.
    revoke(&state, &tenant, &token, &tenant_key).await;
    let other_bytes = format!("t1062 server org payload {}", uuid::Uuid::new_v4()).into_bytes();
    let answer = upload(
        &state,
        &tenant,
        &token,
        &other_bytes,
        Some(&sign(&other_bytes, &keypair)),
    )
    .await;
    // The bytes are no archive: 400 `upload_failed` is past the check.
    assert!(
        refused(&answer, StatusCode::BAD_REQUEST, "upload_failed"),
        "{answer:?}"
    );
    let (_, rows) = signature_rows(
        &tenant_database(&tenant),
        &other_bytes,
        &[tenant_org, server_org],
    )
    .await;
    assert_eq!(rows, vec![(keypair.fingerprint.clone(), server_org)]);

    // Revoked in the server org too: refused.
    km.revoke_trusted_key(server_org, server_key.id)
        .await
        .expect("revoke in the server org");
    let answer = upload(
        &state,
        &tenant,
        &token,
        &other_bytes,
        Some(&sign(&other_bytes, &keypair)),
    )
    .await;
    assert!(
        refused(&answer, StatusCode::FORBIDDEN, "signer_revoked"),
        "{answer:?}"
    );

    clean_up(&state, &[&tenant]).await;
}

/// With signatures not required, an upload with no signature works as
/// before. A signature that is sent is checked all the same: a good one is
/// stored, a bad one is refused.
#[tokio::test]
#[serial]
async fn with_signatures_off_an_unsigned_upload_passes_and_a_sent_signature_is_checked() {
    let state = state_with(false, None).await;
    let token = create_test_api_key(&state).await;
    let unsigned = make_tenant(&state, "off_a").await;
    let signed = make_tenant(&state, "off_b").await;
    let bytes = package_bytes();

    // An old client: no field.
    let answer = upload(&state, &unsigned, &token, &bytes, None).await;
    assert_eq!(answer.0, StatusCode::CREATED, "body: {:?}", answer.1);
    let org = tenant_org_id(&unsigned);
    let (all, _) = signature_rows(&tenant_database(&unsigned), &bytes, &[org]).await;
    assert_eq!(all, 0);

    // A signature by a key the tenant does not trust: refused.
    let keypair = generate_signing_keypair();
    let signature = sign(&bytes, &keypair);
    let answer = upload(&state, &signed, &token, &bytes, Some(&signature)).await;
    assert!(
        refused(&answer, StatusCode::FORBIDDEN, "untrusted_signer"),
        "{answer:?}"
    );

    // Trusted: accepted, and the row is stored.
    trust(&state, &signed, &token, &keypair).await;
    let answer = upload(&state, &signed, &token, &bytes, Some(&signature)).await;
    assert_eq!(answer.0, StatusCode::CREATED, "body: {:?}", answer.1);
    let org = tenant_org_id(&signed);
    let (_, rows) = signature_rows(&tenant_database(&signed), &bytes, &[org]).await;
    assert_eq!(rows, vec![(keypair.fingerprint.clone(), org)]);

    clean_up(&state, &[&unsigned, &signed]).await;
}

// ----- the load check -----

/// The reconciler of a tenant's runner, as the server configures it: the
/// config of `runner_config_for_tenant_cache` for the tenant, over the
/// tenant's schema. A new one for each call, as after a restart.
fn tenant_reconciler(
    state: &AppState,
    tenant: &str,
    server_org: Option<cloacina::UniversalUuid>,
) -> RegistryReconciler {
    let mut config = runner_config_for_tenant_cache(
        None,
        "default",
        true,
        server_org.map(|o| o.0),
        Some(state.database.clone()),
        Default::default(),
        Default::default(),
    );
    config.set_tenant_id(tenant);
    let database = tenant_database(tenant);
    let registry: Arc<dyn WorkflowRegistry> = Arc::new(
        DAL::new(database.clone())
            .workflow_registry(cloacina::dal::UnifiedRegistryStorage::new(database)),
    );
    let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    RegistryReconciler::new(registry, config.reconciler_config(), shutdown_rx)
        .expect("reconciler")
        .with_runtime(Arc::new(cloacina::Runtime::empty()))
}

/// Marks the package built, as the compiler does, so the reconciler loads it.
async fn mark_built(tenant: &str, package_id: uuid::Uuid) {
    let database = tenant_database(tenant);
    let registry = DAL::new(database.clone())
        .workflow_registry(cloacina::dal::UnifiedRegistryStorage::new(database));
    registry.claim_next_build().await.expect("claim the build");
    registry
        .mark_build_success(package_id, Vec::new())
        .await
        .expect("mark built");
}

/// The failure of the load check in a reconcile result, if there is one.
fn load_check_failure(result: &cloacina::registry::ReconcileResult) -> Option<String> {
    result
        .packages_failed
        .iter()
        .map(|(_, message)| message.clone())
        .find(|message| message.contains("load check"))
}

/// A package uploaded signed to a tenant that requires signatures passes the
/// load check of the tenant's reconciler. After the tenant revokes the
/// signer's key, the next load refuses the package.
///
/// The fixture is a Python package and this test process has no Python
/// runtime, so the load stops after the signature check, at the step that
/// needs the runtime. That failure is past the check: it is the proof that
/// the check let the package through.
#[tokio::test]
#[serial]
async fn a_signed_package_passes_the_load_check_and_is_refused_at_the_next_load_after_a_revoke() {
    let state = state_with(true, None).await;
    let token = create_test_api_key(&state).await;
    let tenant = make_tenant(&state, "load").await;
    let keypair = generate_signing_keypair();
    let key_id = trust(&state, &tenant, &token, &keypair).await;

    let bytes = package_bytes();
    let answer = upload(
        &state,
        &tenant,
        &token,
        &bytes,
        Some(&sign(&bytes, &keypair)),
    )
    .await;
    assert_eq!(answer.0, StatusCode::CREATED, "body: {:?}", answer.1);
    let package_id =
        uuid::Uuid::parse_str(answer.1["package_id"].as_str().expect("package id")).expect("uuid");
    // The upload started the tenant's runner. Stop it: the test drives its
    // own reconciler, with the same config.
    let _ = state.tenant_runners.shutdown_all().await;
    mark_built(&tenant, package_id).await;

    let result = tenant_reconciler(&state, &tenant, None)
        .reconcile()
        .await
        .expect("reconcile");
    assert!(
        load_check_failure(&result).is_none(),
        "the load check refused the signed package: {:?}",
        result.packages_failed
    );
    // Past the check: loaded, or stopped at the step that needs the Python
    // runtime this process does not have.
    let past_the_check = result.packages_loaded == vec![package_id]
        || result
            .packages_failed
            .iter()
            .all(|(id, message)| *id == package_id && message.contains("no PythonRuntime"));
    assert!(past_the_check, "{result:?}");

    // The revoke. A reconciler that starts after it (a restart) refuses the
    // package at load.
    revoke(&state, &tenant, &token, &key_id).await;
    let result = tenant_reconciler(&state, &tenant, None)
        .reconcile()
        .await
        .expect("reconcile after the revoke");
    assert!(result.packages_loaded.is_empty(), "{result:?}");
    let failure = load_check_failure(&result).expect("the load check refused the package");
    assert!(failure.contains("revoked_signer"), "{failure}");

    clean_up(&state, &[&tenant]).await;
}

/// A package put into a tenant's tables without the upload route is refused
/// at load when signatures are required: with no row, with a row that has no
/// org, and with a row whose signer no org trusts.
#[tokio::test]
#[serial]
async fn an_unsigned_package_put_into_a_tenant_is_refused_at_load() {
    let state = state_with(true, None).await;
    let tenant = make_tenant(&state, "unsigned").await;
    let org = tenant_org_id(&tenant);
    let bytes = package_bytes();

    // The package goes straight into the tenant's registry.
    let database = tenant_database(&tenant);
    let mut registry = DAL::new(database.clone())
        .workflow_registry(cloacina::dal::UnifiedRegistryStorage::new(database.clone()));
    let package_id = registry
        .register_workflow_package(bytes.clone())
        .await
        .expect("register");
    mark_built(&tenant, package_id).await;

    // No row.
    let result = tenant_reconciler(&state, &tenant, None)
        .reconcile()
        .await
        .expect("reconcile");
    assert!(result.packages_loaded.is_empty(), "{result:?}");
    let failure = load_check_failure(&result).expect("refused with no row");
    assert!(failure.contains("missing_signature"), "{failure}");

    // A row with no org, as the library's `store_signature` writes: it does
    // not count.
    let keypair = generate_signing_keypair();
    let signer = DbPackageSigner::new(DAL::new(database.clone()));
    let row = signer
        .sign_package_data(&bytes, &keypair.private_key, &keypair.public_key)
        .expect("sign");
    signer.store_signature(&row).await.expect("store, no org");
    let result = tenant_reconciler(&state, &tenant, None)
        .reconcile()
        .await
        .expect("reconcile");
    let failure = load_check_failure(&result).expect("refused with a row that has no org");
    assert!(failure.contains("missing_signature"), "{failure}");

    // A row written for the tenant's org by someone who can write the
    // tenant's tables, signed by a key the tenant does not trust.
    signer
        .store_signature_for_org(org, &row)
        .await
        .expect("store for the org");
    let result = tenant_reconciler(&state, &tenant, None)
        .reconcile()
        .await
        .expect("reconcile");
    assert!(result.packages_loaded.is_empty(), "{result:?}");
    let failure = load_check_failure(&result).expect("refused with an untrusted signer");
    assert!(failure.contains("untrusted_signer"), "{failure}");

    clean_up(&state, &[&tenant]).await;
}
