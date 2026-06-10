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

//! Live-server contract suite for the Rust client (CLOACI-I-0113 / REQ-007).
//!
//! DTO sharing with the server prevents schema drift, but does not prove
//! handler semantics match the documented contract — this suite does.
//! Every documented endpoint is exercised; endpoints whose success path
//! needs a compiled `.cloacina` package assert their documented error
//! contract instead (full execute→push flow lands in `angreal test
//! sdk-contract`, T-0648).
//!
//! Skipped unless both env vars are set:
//!   CLOACINA_SERVER_URL  e.g. http://localhost:8080
//!   CLOACINA_API_KEY     a god-mode (bootstrap) key

use cloacina_client::{types, Client, ClientBuilder, ClientError};

const RANDOM_UUID: &str = "00000000-0000-4000-8000-000000000000";

fn live_client() -> Option<(Client, String)> {
    let server = std::env::var("CLOACINA_SERVER_URL").ok()?;
    let key = std::env::var("CLOACINA_API_KEY").ok()?;
    let tenant = format!(
        "sdk_rs_contract_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let client = ClientBuilder::new(server)
        .api_key(key)
        .tenant(&tenant)
        .build()
        .expect("client builds");
    Some((client, tenant))
}

macro_rules! require_live {
    () => {
        match live_client() {
            Some(v) => v,
            None => {
                eprintln!("skipping: CLOACINA_SERVER_URL / CLOACINA_API_KEY not set");
                return;
            }
        }
    };
}

#[tokio::test]
async fn full_rest_surface_contract() {
    let (client, tenant) = require_live!();

    // ---- operational ----
    let health = client.health().await.expect("health");
    assert_eq!(health.get("status").and_then(|s| s.as_str()), Some("ok"));
    let (ready_status, _) = client.ready().await.expect("ready");
    assert!(ready_status == 200 || ready_status == 503);

    // ---- tenants: create / list ----
    client
        .create_tenant(&types::CreateTenantRequest {
            name: tenant.clone(),
            description: Some("rust sdk contract".into()),
            password: None,
        })
        .await
        .expect("create_tenant");
    let tenants = client.list_tenants().await.expect("list_tenants");
    assert!(tenants.items.iter().any(|t| t.name == tenant));

    // ---- keys ----
    let created = client
        .create_key(&format!("rs-contract-{tenant}"), types::KeyRole::Read)
        .await
        .expect("create_key");
    assert!(!created.key.is_empty(), "plaintext returned exactly once");
    assert_eq!(created.permissions, "read");

    let keys = client.list_keys().await.expect("list_keys");
    assert!(keys.total > 0);
    let mine = keys
        .items
        .iter()
        .find(|k| k.id == created.id)
        .expect("listed");
    assert_eq!(mine.permissions, "read");

    let revoked = client.revoke_key(&created.id).await.expect("revoke_key");
    assert_eq!(revoked.status, "revoked");

    let tenant_key = client
        .create_tenant_key(
            &format!("rs-contract-t-{tenant}"),
            types::KeyRole::Write,
            None,
        )
        .await
        .expect("create_tenant_key");
    assert_eq!(tenant_key.tenant_id.as_deref(), Some(tenant.as_str()));
    client
        .revoke_key(&tenant_key.id)
        .await
        .expect("revoke tenant key");

    let ticket = client.create_ws_ticket().await.expect("ws ticket");
    assert!(!ticket.ticket.is_empty());
    assert!(ticket.expires_in_seconds > 0);

    // ---- workflows ----
    let upload_err = client
        .upload_workflow(b"not a real package".to_vec(), None)
        .await
        .expect_err("garbage package must be rejected");
    assert!(matches!(upload_err, ClientError::InvalidRequest(_)));

    let workflows = client.list_workflows(None).await.expect("list_workflows");
    assert_eq!(workflows.tenant_id, tenant);
    assert_eq!(workflows.total, workflows.items.len());

    let missing = client.get_workflow("does-not-exist", None).await;
    assert!(matches!(missing, Err(ClientError::NotFound(_))));

    // Documented contract decision (T-0645): unregister is idempotent.
    let deleted = client
        .delete_workflow("does-not-exist", "0.0.0", None)
        .await
        .expect("idempotent delete");
    assert_eq!(deleted.status, "deleted");

    // ---- triggers ----
    let triggers = client
        .list_triggers(Some(10), Some(0), None)
        .await
        .expect("list_triggers");
    assert_eq!(triggers.tenant_id, tenant);

    let bad_page = client.list_triggers(Some(100_000), None, None).await;
    assert!(matches!(bad_page, Err(ClientError::InvalidRequest(_))));

    let no_trigger = client.get_trigger("does-not-exist", None).await;
    assert!(matches!(no_trigger, Err(ClientError::NotFound(_))));

    // ---- executions ----
    let exec_err = client
        .execute_workflow("does-not-exist", serde_json::json!({"k": "v"}))
        .await
        .expect_err("unknown workflow rejected");
    assert!(matches!(exec_err, ClientError::InvalidRequest(_)));

    let executions = client
        .list_executions(
            &types::ListExecutionsQuery {
                status: Some("Completed".into()),
                limit: Some(5),
                ..Default::default()
            },
            None,
        )
        .await
        .expect("list_executions");
    assert_eq!(executions.tenant_id, tenant);

    let bad_id = client.get_execution("not-a-uuid", None).await;
    assert!(matches!(bad_id, Err(ClientError::InvalidRequest(_))));

    let missing_exec = client.get_execution(RANDOM_UUID, None).await;
    assert!(matches!(missing_exec, Err(ClientError::NotFound(_))));

    // Events endpoint returns an empty envelope (not 404) for a valid
    // unknown UUID — documented contract.
    let events = client
        .get_execution_events(RANDOM_UUID, None)
        .await
        .expect("events envelope");
    assert_eq!(events.execution_id, RANDOM_UUID);
    assert!(events.events.is_empty());

    // ---- computation-graph health ----
    let accs = client.list_accumulators().await.expect("accumulators");
    assert_eq!(accs.total, accs.items.len());
    let graphs = client.list_graphs().await.expect("graphs");
    assert_eq!(graphs.total, graphs.items.len());
    let no_graph = client.get_graph("does-not-exist").await;
    assert!(matches!(no_graph, Err(ClientError::NotFound(_))));

    // ---- cleanup ----
    let removed = client.remove_tenant(&tenant).await.expect("remove_tenant");
    assert_eq!(removed.status, "removed");
}

#[tokio::test]
async fn ws_subscription_lifecycle() {
    use futures_util::StreamExt;

    let (client, _tenant) = require_live!();

    // Subscribing to a quiet recipient: connection must establish (welcome
    // + hello round-trip inside the stream) and then sit idle — no error
    // within the probe window proves the lifecycle.
    let options = cloacina_client::SubscribeOptions {
        reconnect: false,
        ..Default::default()
    };
    let stream = client.subscribe_delivery(&format!("exec_events:{RANDOM_UUID}"), options);
    let mut stream = std::pin::pin!(stream);

    let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next()).await;
    match outcome {
        // Quiet recipient: no frame within the window — connected and idle.
        Err(_elapsed) => {}
        // A frame somehow arrived — it must at least be well-formed.
        Ok(Some(Ok(_push))) => {}
        Ok(Some(Err(e))) => panic!("WS lifecycle failed: {e}"),
        Ok(None) => panic!("WS stream ended unexpectedly"),
    }
}

/// NFR-002: the client adds < 5ms over raw reqwest for a localhost
/// round-trip. Compares medians over 20 calls to /health.
#[tokio::test]
async fn overhead_under_5ms_vs_raw_reqwest() {
    let (client, _tenant) = require_live!();
    let server = std::env::var("CLOACINA_SERVER_URL").unwrap();

    let raw = reqwest::Client::new();
    let median = |mut samples: Vec<u128>| {
        samples.sort_unstable();
        samples[samples.len() / 2]
    };

    // Warm up both paths (connection pools).
    let _ = client.health().await.unwrap();
    let _ = raw.get(format!("{server}/health")).send().await.unwrap();

    let mut raw_samples = Vec::new();
    for _ in 0..20 {
        let t = std::time::Instant::now();
        let r = raw.get(format!("{server}/health")).send().await.unwrap();
        let _ = r.bytes().await.unwrap();
        raw_samples.push(t.elapsed().as_micros());
    }

    let mut client_samples = Vec::new();
    for _ in 0..20 {
        let t = std::time::Instant::now();
        let _ = client.health().await.unwrap();
        client_samples.push(t.elapsed().as_micros());
    }

    let raw_median = median(raw_samples);
    let client_median = median(client_samples);
    let overhead_us = client_median.saturating_sub(raw_median);
    eprintln!("median raw={raw_median}us client={client_median}us overhead={overhead_us}us");
    assert!(
        overhead_us < 5_000,
        "client overhead {overhead_us}us exceeds 5ms (NFR-002)"
    );
}
