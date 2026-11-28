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

//! Tests of the ownership checks on the agent routes and the delivery socket
//! (CLOACINA-T-1050, CLOACINA-T-1051, CLOACINA-T-1052). They drive the real
//! handlers against the shared roster and outbox, so they need Postgres, like
//! the other router tests of this crate.

use super::*;
use crate::routes::agent::{
    heartbeat_agent, register_agent, replenish_keys, report_result, report_undecodable,
};
use crate::routes::auth::AuthenticatedKey;
use crate::routes::delivery_ws::{AckOutcome, apply_client_ack, require_recipient};
use axum::Extension;
use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use cloacina::fleet::{
    AGENT_PROTOCOL_VERSION, AgentHeartbeatRequest, AgentKeyReplenishRequest, AgentOutcome,
    AgentRegisterRequest, AgentResultRequest, AgentUndecodableRequest, EphemeralKeyEntry,
};
use cloacina::security::ownership::{DeliveryScope, DispatchTarget};

/// A key of `tenant` (not a platform admin), as the auth layer hands it on.
fn tenant_key(tenant: &str) -> AuthenticatedKey {
    AuthenticatedKey {
        key_id: uuid::Uuid::new_v4(),
        name: format!("ownership-test-{tenant}"),
        permissions: "read".to_string(),
        tenant_id: Some(tenant.to_string()),
        is_admin: false,
    }
}

fn platform_admin_key() -> AuthenticatedKey {
    AuthenticatedKey {
        key_id: uuid::Uuid::new_v4(),
        name: "ownership-test-admin".to_string(),
        permissions: "admin".to_string(),
        tenant_id: None,
        is_admin: true,
    }
}

fn unique(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4().simple())
}

fn register_body(agent_id: Option<&str>) -> AgentRegisterRequest {
    serde_json::from_value(serde_json::json!({
        "protocol_version": AGENT_PROTOCOL_VERSION,
        "agent_id": agent_id,
        "target_triple": "aarch64-apple-darwin",
        "max_concurrency": 2,
        "capabilities": [],
    }))
    .expect("a register request")
}

/// Register through the real handler. `Ok(agent id)` or the HTTP status of
/// the refusal.
async fn register(
    state: &AppState,
    key: &AuthenticatedKey,
    agent_id: Option<&str>,
) -> Result<String, StatusCode> {
    register_agent(
        State(state.clone()),
        Extension(key.clone()),
        Json(register_body(agent_id)),
    )
    .await
    .map(|Json(resp)| resp.agent_id)
    .map_err(|e| e.into_response().status())
}

async fn heartbeat(state: &AppState, key: &AuthenticatedKey, agent_id: &str) -> StatusCode {
    let body: AgentHeartbeatRequest = serde_json::from_value(serde_json::json!({
        "protocol_version": AGENT_PROTOCOL_VERSION,
        "agent_id": agent_id,
        "in_flight": 0,
        "available_capacity": 2,
    }))
    .expect("a heartbeat request");
    match heartbeat_agent(State(state.clone()), Extension(key.clone()), Json(body)).await {
        Ok(_) => StatusCode::OK,
        Err(e) => e.into_response().status(),
    }
}

async fn top_up(state: &AppState, key: &AuthenticatedKey, agent_id: &str) -> StatusCode {
    let body = AgentKeyReplenishRequest {
        protocol_version: AGENT_PROTOCOL_VERSION,
        agent_id: agent_id.to_string(),
        keys: vec![EphemeralKeyEntry {
            key_id: unique("k"),
            public_key_b64: "AAAA".to_string(),
        }],
    };
    match replenish_keys(State(state.clone()), Extension(key.clone()), Json(body)).await {
        Ok(_) => StatusCode::OK,
        Err(e) => e.into_response().status(),
    }
}

async fn report(
    state: &AppState,
    key: &AuthenticatedKey,
    agent_id: &str,
    task_id: cloacina::UniversalUuid,
    dispatch_id: Option<&str>,
) -> StatusCode {
    let body = AgentResultRequest {
        protocol_version: AGENT_PROTOCOL_VERSION,
        agent_id: agent_id.to_string(),
        task_execution_id: task_id.0.to_string(),
        attempt: 1,
        dispatch_id: dispatch_id.map(str::to_string),
        duration_ms: 5,
        outcome: AgentOutcome::Success {
            context: serde_json::json!({"forged": true}),
        },
    };
    match report_result(State(state.clone()), Extension(key.clone()), Json(body)).await {
        Ok(_) => StatusCode::OK,
        Err(e) => e.into_response().status(),
    }
}

/// CLOACINA-T-1051: a register for an agent id that another tenant or
/// another key holds is refused. The holder registers again (an agent that
/// restarts) and keeps the row.
#[tokio::test]
#[serial]
async fn test_register_cannot_take_over_an_agent_id() {
    let state = test_state().await;
    let tenant_b = unique("own-b");
    let key_b = tenant_key(&tenant_b);
    let agent_b = unique("agent-b");

    assert_eq!(
        register(&state, &key_b, Some(&agent_b)).await.as_deref(),
        Ok(agent_b.as_str())
    );

    // Across tenants.
    let key_a = tenant_key(&unique("own-a"));
    assert_eq!(
        register(&state, &key_a, Some(&agent_b)).await,
        Err(StatusCode::CONFLICT)
    );
    // A platform admin key gets no bypass: it has no tenant, so it registers
    // no agent (CLOACINA-T-1118).
    assert_eq!(
        register(&state, &platform_admin_key(), Some(&agent_b)).await,
        Err(StatusCode::FORBIDDEN)
    );
    // Inside the tenant: another key.
    let other_key_b = tenant_key(&tenant_b);
    assert_eq!(
        register(&state, &other_key_b, Some(&agent_b)).await,
        Err(StatusCode::CONFLICT)
    );

    // The row is still B's.
    let owner = state
        .agent_registry
        .agent_owner(&agent_b)
        .await
        .expect("lookup")
        .expect("the row exists");
    assert_eq!(owner.tenant_id.as_deref(), Some(tenant_b.as_str()));
    assert_eq!(owner.owner_key_id, Some(key_b.key_id));
    assert_eq!(heartbeat(&state, &key_b, &agent_b).await, StatusCode::OK);

    // The agent restarts and registers again with its own id and key.
    assert_eq!(
        register(&state, &key_b, Some(&agent_b)).await.as_deref(),
        Ok(agent_b.as_str())
    );
    assert_eq!(heartbeat(&state, &key_b, &agent_b).await, StatusCode::OK);

    // With no id the server assigns a new one, for any key.
    let assigned = register(&state, &key_a, None).await.expect("a new id");
    assert_ne!(assigned, agent_b);

    let _ = state.agent_registry.deregister(&agent_b).await;
    let _ = state.agent_registry.deregister(&assigned).await;
}

/// CLOACINA-T-1050: heartbeat, result and key top-up are refused for an
/// unknown agent id, for an agent of another tenant, and for an agent that
/// another key of the same tenant holds.
#[tokio::test]
#[serial]
async fn test_only_the_owner_key_acts_as_an_agent() {
    let state = test_state().await;
    let tenant_b = unique("own-b");
    let key_b = tenant_key(&tenant_b);
    let agent_b = register(&state, &key_b, None).await.expect("register");
    let task_id = cloacina::UniversalUuid::new_v4();

    let ghost = unique("ghost");
    let key_a = tenant_key(&unique("own-a"));
    let other_key_b = tenant_key(&tenant_b);

    // An unknown agent id.
    for key in [&key_a, &key_b, &platform_admin_key()] {
        assert_eq!(heartbeat(&state, key, &ghost).await, StatusCode::NOT_FOUND);
        assert_eq!(top_up(&state, key, &ghost).await, StatusCode::NOT_FOUND);
        assert_eq!(
            report(&state, key, &ghost, task_id, None).await,
            StatusCode::NOT_FOUND
        );
    }
    // Another tenant's agent, another key's agent, and no bypass for a
    // platform admin key.
    for key in [&key_a, &other_key_b, &platform_admin_key()] {
        assert_eq!(
            heartbeat(&state, key, &agent_b).await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(top_up(&state, key, &agent_b).await, StatusCode::FORBIDDEN);
        assert_eq!(
            report(&state, key, &agent_b, task_id, Some("d1")).await,
            StatusCode::FORBIDDEN
        );
    }
    // The owner key.
    assert_eq!(heartbeat(&state, &key_b, &agent_b).await, StatusCode::OK);
    assert_eq!(top_up(&state, &key_b, &agent_b).await, StatusCode::OK);

    let _ = state.agent_registry.deregister(&agent_b).await;
}

/// CLOACINA-T-1050: a key of tenant A cannot end a task of tenant B. It
/// cannot name B's agent, an unregistered agent resolves nothing, and its
/// own registered agent is not the one the packet went to. A second agent of
/// tenant B that was not sent the packet cannot resolve the wait either.
#[tokio::test]
#[serial]
async fn test_a_result_resolves_a_wait_only_from_the_agent_the_packet_went_to() {
    let state = test_state().await;
    let inbox = cloacina::dal::DAL::new(state.database.clone());
    let tenant_b = unique("own-b");
    let key_b = tenant_key(&tenant_b);
    let agent_b = register(&state, &key_b, None).await.expect("register");
    let other_agent_b = register(&state, &tenant_key(&tenant_b), None)
        .await
        .expect("register");
    let key_a = tenant_key(&unique("own-a"));
    let agent_a = register(&state, &key_a, None).await.expect("register");

    // Tenant B's task is dispatched to agent_b.
    let task_id = cloacina::UniversalUuid::new_v4();
    let mut rx = state.fleet_coordinator.register_dispatch(
        task_id,
        DispatchTarget {
            tenant_id: Some(tenant_b.clone()),
            agent_id: agent_b.clone(),
            dispatch_id: Some("d1".to_string()),
        },
    );
    let packet = inbox
        .delivery_outbox()
        .enqueue(cloacina::models::delivery_outbox::NewDeliveryOutbox {
            recipient: format!("agent:{agent_b}"),
            kind: "work_packet".to_string(),
            tenant_id: Some(tenant_b.clone()),
            payload: b"{}".to_vec(),
        })
        .await
        .expect("enqueue");
    state.fleet_coordinator.packet_enqueued(task_id, packet.id);

    // Tenant A: the attack of the finding (an unregistered agent id, no
    // dispatch id), then its own agent with and without the dispatch id,
    // then B's agent id.
    assert_eq!(
        report(&state, &key_a, &unique("ghost"), task_id, None).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        report(&state, &key_a, &agent_a, task_id, None).await,
        StatusCode::OK
    );
    assert_eq!(
        report(&state, &key_a, &agent_a, task_id, Some("d1")).await,
        StatusCode::OK
    );
    assert_eq!(
        report(&state, &key_a, &agent_b, task_id, Some("d1")).await,
        StatusCode::FORBIDDEN
    );
    // Tenant B, an agent that was not sent the packet. Find its key: it is
    // the one that registered it, so use the owner row.
    let other_owner = state
        .agent_registry
        .agent_owner(&other_agent_b)
        .await
        .expect("lookup")
        .expect("row");
    let other_key_b = AuthenticatedKey {
        key_id: other_owner.owner_key_id.expect("an owner"),
        ..tenant_key(&tenant_b)
    };
    assert_eq!(
        report(&state, &other_key_b, &other_agent_b, task_id, Some("d1")).await,
        StatusCode::OK
    );

    assert_eq!(state.fleet_coordinator.pending_count(), 1);
    assert!(rx.try_recv().is_err(), "nothing resolved tenant B's wait");
    assert!(
        inbox
            .fleet_result_inbox()
            .list_for(&[task_id])
            .await
            .unwrap()
            .is_empty(),
        "a refused report is not stored"
    );

    // The agent the packet went to.
    assert_eq!(
        report(&state, &key_b, &agent_b, task_id, Some("d1")).await,
        StatusCode::OK
    );
    let got = rx.await.expect("the wait is resolved");
    assert_eq!(got.agent_id, agent_b);

    let _ = inbox.delivery_outbox().mark_acked(packet.id).await;
    for agent in [&agent_a, &agent_b, &other_agent_b] {
        let _ = state.agent_registry.deregister(agent).await;
    }
}

/// CLOACINA-T-1050: when the agent is evicted and its packet is re-targeted,
/// the report of the new agent resolves the wait.
#[tokio::test]
#[serial]
async fn test_a_result_from_the_agent_a_packet_was_retargeted_to_is_accepted() {
    let state = test_state().await;
    let inbox = cloacina::dal::DAL::new(state.database.clone());
    let tenant = unique("own-t");
    let dead_key = tenant_key(&tenant);
    let dead_agent = register(&state, &dead_key, None).await.expect("register");
    let live_key = tenant_key(&tenant);
    let live_agent = register(&state, &live_key, None).await.expect("register");

    let task_id = cloacina::UniversalUuid::new_v4();
    let rx = state.fleet_coordinator.register_dispatch(
        task_id,
        DispatchTarget {
            tenant_id: Some(tenant.clone()),
            agent_id: dead_agent.clone(),
            dispatch_id: Some("d1".to_string()),
        },
    );
    let packet = inbox
        .delivery_outbox()
        .enqueue(cloacina::models::delivery_outbox::NewDeliveryOutbox {
            recipient: format!("agent:{dead_agent}"),
            kind: "work_packet".to_string(),
            tenant_id: Some(tenant.clone()),
            payload: b"{}".to_vec(),
        })
        .await
        .expect("enqueue");
    state.fleet_coordinator.packet_enqueued(task_id, packet.id);

    // The sweeper evicts the agent and re-targets its open rows.
    state
        .agent_registry
        .deregister(&dead_agent)
        .await
        .expect("evict");
    inbox
        .delivery_outbox()
        .reassign_open_rows(
            &format!("agent:{dead_agent}"),
            &format!("agent:{live_agent}"),
        )
        .await
        .expect("reassign");

    // The evicted agent is unknown now.
    assert_eq!(
        report(&state, &dead_key, &dead_agent, task_id, Some("d1")).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        report(&state, &live_key, &live_agent, task_id, Some("d1")).await,
        StatusCode::OK
    );
    let got = tokio::time::timeout(std::time::Duration::from_secs(2), rx)
        .await
        .expect("the wait is resolved")
        .expect("a report");
    assert_eq!(got.agent_id, live_agent);

    // The ack follows the row too.
    assert_eq!(
        apply_client_ack(
            &inbox,
            &DeliveryScope::of_agent(&dead_agent, Some(tenant.clone())),
            packet.id
        )
        .await
        .unwrap(),
        AckOutcome::Refused
    );
    assert_eq!(
        apply_client_ack(
            &inbox,
            &DeliveryScope::of_agent(&live_agent, Some(tenant.clone())),
            packet.id
        )
        .await
        .unwrap(),
        AckOutcome::Applied
    );
    let _ = state.agent_registry.deregister(&live_agent).await;
}

/// CLOACINA-T-1051: `agent:<id>` on the delivery socket is open only to the
/// key that registered the agent. `exec_events:<id>` is open to a key with a
/// tenant; no other recipient has a socket.
#[tokio::test]
#[serial]
async fn test_the_delivery_socket_checks_who_may_open_a_recipient() {
    let state = test_state().await;
    let tenant_b = unique("own-b");
    let key_b = tenant_key(&tenant_b);
    let agent_b = register(&state, &key_b, None).await.expect("register");
    let recipient = format!("agent:{agent_b}");

    let status = |r: Result<(), crate::routes::error::ApiError>| match r {
        Ok(()) => StatusCode::OK,
        Err(e) => e.into_response().status(),
    };

    assert_eq!(
        status(require_recipient(&state, &key_b, &recipient).await),
        StatusCode::OK
    );
    // Across tenants, inside the tenant, a platform admin key, an unknown id.
    for key in [
        tenant_key(&unique("own-a")),
        tenant_key(&tenant_b),
        platform_admin_key(),
    ] {
        assert_eq!(
            status(require_recipient(&state, &key, &recipient).await),
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        status(require_recipient(&state, &key_b, &format!("agent:{}", unique("ghost"))).await),
        StatusCode::FORBIDDEN
    );

    // Execution events: any key with a tenant, and a platform admin key.
    let events = format!("exec_events:{}", uuid::Uuid::new_v4());
    for key in [key_b.clone(), tenant_key(&tenant_b), platform_admin_key()] {
        assert_eq!(
            status(require_recipient(&state, &key, &events).await),
            StatusCode::OK
        );
    }
    // A key with no tenant that is not a platform admin.
    let no_tenant = AuthenticatedKey {
        tenant_id: None,
        ..tenant_key("x")
    };
    assert_eq!(
        status(require_recipient(&state, &no_tenant, &events).await),
        StatusCode::FORBIDDEN
    );
    // Recipients that are read inside the server only.
    for other in ["alert_webhook", "accumulator:orders", "anything"] {
        assert_eq!(
            status(require_recipient(&state, &key_b, other).await),
            StatusCode::FORBIDDEN
        );
    }

    let _ = state.agent_registry.deregister(&agent_b).await;
}

/// CLOACINA-T-1052: a connection cannot ack a row of another tenant, nor a
/// row of another recipient of its own tenant.
#[tokio::test]
#[serial]
async fn test_a_connection_acks_only_its_own_rows() {
    let state = test_state().await;
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let tenant_a = unique("own-a");
    let tenant_b = unique("own-b");
    let enqueue = |recipient: String, tenant: String| {
        let dal = dal.clone();
        async move {
            dal.delivery_outbox()
                .enqueue(cloacina::models::delivery_outbox::NewDeliveryOutbox {
                    recipient,
                    kind: "work_packet".to_string(),
                    tenant_id: Some(tenant),
                    payload: b"{}".to_vec(),
                })
                .await
                .expect("enqueue")
                .id
        }
    };
    let mine = enqueue("agent:a1".to_string(), tenant_a.clone()).await;
    let same_tenant = enqueue("agent:a2".to_string(), tenant_a.clone()).await;
    let other_tenant = enqueue("agent:b1".to_string(), tenant_b.clone()).await;
    let scope = DeliveryScope::of_agent("a1", Some(tenant_a.clone()));

    for id in [same_tenant, other_tenant] {
        assert_eq!(
            apply_client_ack(&dal, &scope, id).await.unwrap(),
            AckOutcome::Refused
        );
        let row = dal.delivery_outbox().get(id).await.unwrap().unwrap();
        assert_eq!(row.delivery_state, "pending");
    }
    assert_eq!(
        apply_client_ack(&dal, &scope, mine).await.unwrap(),
        AckOutcome::Applied
    );
    assert_eq!(
        apply_client_ack(&dal, &scope, mine).await.unwrap(),
        AckOutcome::Noop
    );

    // Leave no open rows behind for the relay of other tests.
    for id in [same_tenant, other_tenant] {
        let _ = dal.delivery_outbox().mark_acked(id).await;
    }
}

/// CLOACINA-T-1118: a key with no tenant cannot register an agent, even with
/// the role `agent`. No route makes such a key, and the DAL refuses one, so
/// the row is written with SQL here, as a person with database access could.
#[tokio::test]
#[serial]
async fn test_t1118_an_agent_key_with_no_tenant_cannot_register_an_agent() {
    let state = test_state().await;
    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();
    let key_id = uuid::Uuid::new_v4();
    let name = unique("t1118-no-tenant-agent");
    let conn = state
        .database
        .get_postgres_connection()
        .await
        .expect("a connection");
    let insert = format!(
        "INSERT INTO api_keys (id, key_hash, name, permissions, tenant_id, is_admin) \
         VALUES ('{key_id}', '{hash}', '{name}', 'agent', NULL, false)"
    );
    conn.interact(move |conn| {
        use diesel::RunQueryDsl;
        diesel::sql_query(insert).execute(conn)
    })
    .await
    .expect("interact")
    .expect("insert the key row");

    let agent_id = unique("t1118-agent");
    let body = serde_json::json!({
        "protocol_version": AGENT_PROTOCOL_VERSION,
        "agent_id": agent_id,
        "target_triple": "aarch64-apple-darwin",
        "max_concurrency": 2,
        "capabilities": [],
    });
    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/agent/register")
        .header("Authorization", format!("Bearer {plaintext}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let (status, answer) = send_request(build_router(state.clone()), request).await;
    let row = state.agent_registry.agent_owner(&agent_id).await;

    let _ = state.agent_registry.deregister(&agent_id).await;
    let delete = format!("DELETE FROM api_keys WHERE id = '{key_id}'");
    let _ = conn
        .interact(move |conn| {
            use diesel::RunQueryDsl;
            diesel::sql_query(delete).execute(conn)
        })
        .await;

    assert_eq!(status, StatusCode::FORBIDDEN, "answer: {answer:?}");
    assert_eq!(answer["code"], "agent_no_tenant", "answer: {answer:?}");
    assert!(
        row.expect("lookup").is_none(),
        "no roster row is written for a key with no tenant"
    );
}

/// CLOACINA-T-1118: the two DAL calls that make a key refuse role `agent`
/// with no tenant. Every route and code path that makes a key goes through
/// one of them (`test_t1118_only_the_dal_writes_api_keys_rows`).
#[tokio::test]
#[serial]
async fn test_t1118_the_dal_makes_no_agent_key_with_no_tenant() {
    let state = test_state().await;
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let (_, hash) = cloacina::security::api_keys::generate_api_key();
    let name = unique("t1118-create");
    assert!(
        dal.api_keys()
            .create_key(&hash, &name, None, false, "agent")
            .await
            .is_err()
    );
    assert!(
        dal.api_keys()
            .create_key(&hash, &name, None, true, "agent")
            .await
            .is_err()
    );
    let (_, hash) = cloacina::security::api_keys::generate_api_key();
    let minted = unique("t1118-mint");
    assert!(
        dal.api_keys()
            .mint_key(
                &hash,
                &minted,
                None,
                "agent",
                chrono::Utc::now() + chrono::Duration::minutes(5),
                "test:t1118",
            )
            .await
            .is_err()
    );
    let leaked: Vec<_> = dal
        .api_keys()
        .list_keys()
        .await
        .expect("list keys")
        .into_iter()
        .filter(|k| k.name == name || k.name == minted)
        .collect();
    assert!(leaked.is_empty(), "no row was written: {leaked:?}");

    // With a tenant the agent key is made, as before.
    let (_, hash) = cloacina::security::api_keys::generate_api_key();
    let made = dal
        .api_keys()
        .create_key(
            &hash,
            &unique("t1118-tenant"),
            Some("public"),
            false,
            "agent",
        )
        .await
        .expect("an agent key of a tenant");
    let delete = format!("DELETE FROM api_keys WHERE id = '{}'", made.id);
    if let Ok(conn) = state.database.get_postgres_connection().await {
        let _ = conn
            .interact(move |conn| {
                use diesel::RunQueryDsl;
                diesel::sql_query(delete).execute(conn)
            })
            .await;
    }
}

/// CLOACINA-T-1118: only `api_keys/crud.rs` writes an `api_keys` row, so the
/// refusal of `check_key_scope` covers every route and code path that makes
/// a key. This fails when other code inserts into the table.
#[test]
fn test_t1118_only_the_dal_writes_api_keys_rows() {
    fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                rust_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crates folder");
    let mut files = Vec::new();
    rust_files(crates, &mut files);
    assert!(files.len() > 100, "the scan found the sources");

    // Split so that this file does not match itself.
    let patterns = [
        ["insert_into(", "api_keys"].concat(),
        ["insert into ", "api_keys"].concat(),
    ];
    // The DAL, and the test above that writes a row with SQL on purpose.
    let allowed = ["api_keys/crud.rs", "tests/agent_ownership.rs"];
    let mut writers = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file)
            .unwrap_or_default()
            .to_lowercase();
        let path = file.to_string_lossy().replace('\\', "/");
        if patterns.iter().any(|p| text.contains(p.as_str()))
            && !allowed.iter().any(|a| path.ends_with(a))
        {
            writers.push(path);
        }
    }
    assert!(
        writers.is_empty(),
        "code outside the API key DAL writes api_keys rows; route it through \
         ApiKeyDAL::create_key or mint_key: {writers:?}"
    );
}

async fn cannot_decode(
    state: &AppState,
    key: &AuthenticatedKey,
    agent_id: &str,
    push_id: i64,
) -> StatusCode {
    let body = AgentUndecodableRequest {
        protocol_version: AGENT_PROTOCOL_VERSION,
        agent_id: agent_id.to_string(),
        push_id,
        error: "deserialize WorkPacket JSON".to_string(),
        agent_version: None,
    };
    match report_undecodable(State(state.clone()), Extension(key.clone()), Json(body)).await {
        Ok(_) => StatusCode::OK,
        Err(e) => e.into_response().status(),
    }
}

/// CLOACINA-T-1058: an agent that cannot decode a packet names its delivery
/// row, and the dispatch of that packet fails at once. Only the agent the
/// row is addressed to can do that: a key of another tenant and another
/// agent of the tenant change nothing.
#[tokio::test]
#[serial]
async fn test_only_the_agent_of_a_packet_row_can_report_it_undecodable() {
    let state = test_state().await;
    let db = cloacina::dal::DAL::new(state.database.clone());
    let tenant_b = unique("own-b");
    let key_b = tenant_key(&tenant_b);
    let agent_b = register(&state, &key_b, None).await.expect("register");
    let other_key_b = tenant_key(&tenant_b);
    let other_agent_b = register(&state, &other_key_b, None)
        .await
        .expect("register");
    let key_a = tenant_key(&unique("own-a"));
    let agent_a = register(&state, &key_a, None).await.expect("register");

    // Tenant B's task is dispatched to agent_b.
    let task_id = cloacina::UniversalUuid::new_v4();
    let mut rx = state.fleet_coordinator.register_dispatch(
        task_id,
        DispatchTarget {
            tenant_id: Some(tenant_b.clone()),
            agent_id: agent_b.clone(),
            dispatch_id: Some("d1".to_string()),
        },
    );
    let payload = serde_json::json!({
        "task_execution_id": task_id.0.to_string(),
        "attempt": 1,
        "dispatch_id": "d1",
    });
    let packet = db
        .delivery_outbox()
        .enqueue(cloacina::models::delivery_outbox::NewDeliveryOutbox {
            recipient: format!("agent:{agent_b}"),
            kind: cloacina::fleet::WORK_PACKET_KIND.to_string(),
            tenant_id: Some(tenant_b.clone()),
            payload: serde_json::to_vec(&payload).unwrap(),
        })
        .await
        .expect("enqueue");
    state.fleet_coordinator.packet_enqueued(task_id, packet.id);

    // Another tenant: its own agent with B's row id, then B's agent id.
    assert_eq!(
        cannot_decode(&state, &key_a, &agent_a, packet.id).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        cannot_decode(&state, &key_a, &agent_b, packet.id).await,
        StatusCode::FORBIDDEN
    );
    // Another agent of tenant B, and a platform admin key.
    assert_eq!(
        cannot_decode(&state, &other_key_b, &other_agent_b, packet.id).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        cannot_decode(&state, &platform_admin_key(), &agent_b, packet.id).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(state.fleet_coordinator.pending_count(), 1);
    assert!(rx.try_recv().is_err(), "nothing resolved tenant B's wait");

    // The agent the row is addressed to.
    assert_eq!(
        cannot_decode(&state, &key_b, &agent_b, packet.id).await,
        StatusCode::OK
    );
    let report = rx.try_recv().expect("the wait is resolved at once");
    assert!(matches!(report.outcome, AgentOutcome::Failure { .. }));

    for agent in [&agent_a, &agent_b, &other_agent_b] {
        let _ = state.agent_registry.deregister(agent).await;
    }
}
