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

//! Substrate delivery WebSocket endpoint
//! (CLOACI-I-0115 / spec S-0012 / ADR A-0006, task T-0627).
//!
//! `GET /v1/ws/delivery/{recipient}` — recipients connect here to receive
//! pushed [`cloacina::delivery::ServerMessage`] frames and acknowledge them.
//!
//! Auth reuses the existing pattern (`Authorization: Bearer …` header or
//! single-use `?token=…` ticket from `POST /auth/ws-ticket`). Tenant scoping is
//! enforced from the authenticated key's `tenant_id` — recipients can only
//! receive rows whose `delivery_outbox.tenant_id` matches their auth context.
//!
//! Who may open which recipient, and which rows a connection may ack, is
//! decided by [`cloacina::security::ownership`] (CLOACINA-T-1051,
//! CLOACINA-T-1052): `agent:<id>` is open only to the key that registered the
//! agent, and an ack applies only to a row addressed to the connection's own
//! recipient and tenant.
//!
//! Reconnect model: on accept the handler resets every `delivered`-state row
//! for `(recipient, tenant)` back to `pending` and wakes the relay. The relay
//! then re-pushes those rows through the normal sink path. There is no
//! separate resync frame — recipients just see `welcome` followed by `push`
//! messages, and must be idempotent on row id (NFR-1.1.2).

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::http::HeaderMap;
use axum::response::Response;
use cloacina::dal::DAL;
use cloacina::delivery::{ClientMessage, DELIVERY_PROTOCOL_VERSION, ServerMessage, WakeHandle};
use cloacina::security::ownership::{DeliveryScope, RecipientKind};
use tracing::{debug, info, warn};

use crate::AppState;
use crate::routes::auth::{AuthenticatedKey, validate_token};
use crate::routes::error::ApiError;
use crate::routes::ws::WsAuthQuery;

/// `GET /v1/ws/delivery/{recipient}` — substrate delivery subscription.
pub async fn delivery_ws(
    Path(recipient): Path<String>,
    Query(query): Query<WsAuthQuery>,
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    let auth = authenticate_substrate_ws(&state, &headers, &query).await?;
    require_recipient(&state, &auth, &recipient).await?;
    info!(
        recipient = %recipient,
        key = %auth.name,
        tenant = ?auth.tenant_id,
        "delivery WebSocket upgrade accepted"
    );
    let tenant = auth.tenant_id.clone();
    let dal = DAL::new(state.database.clone());
    let sink = state.delivery_sink.clone();
    let wake = state.delivery_wake.clone();
    Ok(ws.on_upgrade(move |socket| {
        handle_delivery_socket(socket, recipient, tenant, dal, sink, wake)
    }))
}

/// Refuse the upgrade unless the key may open `recipient` (CLOACINA-T-1051;
/// the rule is [`cloacina::security::ownership::Principal::may_subscribe`]).
/// For `agent:<id>` the roster row of the agent says who holds it.
pub(crate) async fn require_recipient(
    state: &AppState,
    auth: &AuthenticatedKey,
    recipient: &str,
) -> Result<(), ApiError> {
    let agent_row = match RecipientKind::of(recipient) {
        RecipientKind::Agent(agent_id) => state
            .agent_registry
            .agent_owner(agent_id)
            .await
            .map_err(|e| ApiError::internal_logged("agent roster lookup failed", &e))?,
        _ => None,
    };
    let principal = crate::routes::agent::principal_of(auth);
    if let Err(why) = principal.may_subscribe(recipient, agent_row.as_ref()) {
        metrics::counter!("cloacina_delivery_refused_subscriptions_total", "reason" => why.as_str())
            .increment(1);
        warn!(
            recipient = %recipient,
            key = %auth.name,
            tenant = ?auth.tenant_id,
            reason = %why,
            "delivery WebSocket refused: the key may not open this recipient"
        );
        return Err(ApiError::forbidden(
            "recipient_access_denied",
            "this key may not open this delivery recipient",
        ));
    }
    Ok(())
}

/// Token extraction + validation. Bearer header preferred; falls back to the
/// single-use query-string ticket the existing WS pattern uses for browsers.
async fn authenticate_substrate_ws(
    state: &AppState,
    headers: &HeaderMap,
    query: &WsAuthQuery,
) -> Result<AuthenticatedKey, ApiError> {
    if let Some(value) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(s) = value.to_str() {
            let token = s
                .strip_prefix("Bearer ")
                .or_else(|| s.strip_prefix("bearer "));
            if let Some(token) = token {
                return validate_token(state, token)
                    .await
                    .map_err(|_| ApiError::unauthorized("invalid Authorization bearer token"));
            }
        }
    }
    if let Some(ticket) = query.token.as_deref() {
        return state
            .ws_tickets
            .consume(ticket)
            .await
            .ok_or_else(|| ApiError::unauthorized("invalid or expired WebSocket ticket"));
    }
    Err(ApiError::unauthorized(
        "missing credential: provide Authorization bearer header or ?token=<ticket>",
    ))
}

/// Per-connection state machine.
async fn handle_delivery_socket(
    mut socket: WebSocket,
    recipient: String,
    tenant: Option<String>,
    dal: DAL,
    sink: std::sync::Arc<crate::delivery_sink::WsDeliverySink>,
    wake: WakeHandle,
) {
    debug!(recipient = %recipient, tenant = ?tenant, "delivery WS connected");
    // What this connection may ack: the rows of its own recipient and tenant.
    let scope = DeliveryScope::new(recipient.clone(), tenant.clone());

    // Register first so any NOTIFY-driven wake during catch-up lands somewhere.
    let (conn_id, mut rx) = sink.register(&recipient, tenant.as_deref());

    // Reset the stuck-`delivered` set for (recipient, tenant) → `pending`, so
    // the relay re-pushes them via the normal sink path on the next drain.
    // (Race-free: rows that were never delivered are already `pending`; rows
    // that the relay marks `delivered` AFTER this reset will be the new live
    // pushes, which is what we want.)
    match dal
        .delivery_outbox()
        .reset_delivered_to_pending_for_recipient(&recipient, tenant.as_deref())
        .await
    {
        Ok(n) if n > 0 => debug!(
            recipient = %recipient, count = n,
            "reset stuck-delivered rows for reconnect resync"
        ),
        Ok(_) => {}
        Err(e) => warn!(error = %e, "reset_delivered_to_pending failed; continuing"),
    }

    // Kick the relay so the just-reset rows (and anything else pending) drain.
    wake.wake();

    // Welcome frame.
    let welcome = ServerMessage::Welcome {
        protocol_version: DELIVERY_PROTOCOL_VERSION,
        // For v1 we omit the actual max id; the field is advisory and consumers
        // don't yet use it. Future: query a max(id) for this recipient/tenant.
        max_known_id: 0,
    };
    if !send_server_frame(&mut socket, &welcome).await {
        sink.unregister(&recipient, tenant.as_deref(), conn_id);
        return;
    }

    // Main loop: pump pushes from the relay-fed channel, accept acks from the
    // socket, exit cleanly on disconnect or fatal error.
    loop {
        tokio::select! {
            push = rx.recv() => match push {
                Some(frame) => {
                    if !send_server_frame(&mut socket, &frame).await {
                        break;
                    }
                }
                None => {
                    // Sink dropped our sender — either another connection took
                    // over for this (recipient, tenant) or the sink was torn
                    // down. Either way we're done.
                    debug!(recipient = %recipient, "sink sender closed");
                    break;
                }
            },
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    if let FrameOutcome::Close { code, reason } =
                        handle_client_frame(&text, &dal, &scope).await
                    {
                        let _ = socket.send(Message::Close(Some(axum::extract::ws::CloseFrame {
                            code,
                            reason: reason.into(),
                        }))).await;
                        break;
                    }
                }
                Some(Ok(Message::Binary(_))) => {
                    warn!(recipient = %recipient, "substrate WS expects JSON text frames, got binary");
                }
                Some(Ok(Message::Close(_))) | None => {
                    debug!(recipient = %recipient, "client closed connection");
                    break;
                }
                Some(Ok(_)) => {} // ping/pong handled by axum
                Some(Err(e)) => {
                    warn!(recipient = %recipient, error = %e, "WS recv error");
                    break;
                }
            }
        }
    }

    sink.unregister(&recipient, tenant.as_deref(), conn_id);
    debug!(recipient = %recipient, "delivery WS disconnected");
}

/// Serialize and send a `ServerMessage` as a text frame. Returns `false` on
/// send failure (caller should exit the loop).
async fn send_server_frame(socket: &mut WebSocket, msg: &ServerMessage) -> bool {
    match serde_json::to_string(msg) {
        Ok(json) => match socket.send(Message::Text(json.into())).await {
            Ok(()) => true,
            Err(e) => {
                warn!(error = %e, "delivery WS send failed");
                false
            }
        },
        Err(e) => {
            warn!(error = %e, "failed to serialize ServerMessage; closing");
            false
        }
    }
}

/// Result of processing one client frame: keep going, or close with a
/// specific code + reason.
enum FrameOutcome {
    Ok,
    Close { code: u16, reason: &'static str },
}

/// What an ack from a client did ([`apply_client_ack`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AckOutcome {
    /// The row is acked.
    Applied,
    /// The row is acked already, or no row has the id. Nothing changed.
    Noop,
    /// The row is addressed to another recipient or tenant. Nothing changed.
    Refused,
}

/// Apply an ack that a client sent on a connection with `scope`
/// (CLOACINA-T-1052). The row is acked only when it is addressed to the
/// connection's own recipient and tenant: the filter is in the update. An
/// ack for a row outside the scope changes nothing, is logged and is counted
/// in `cloacina_delivery_refused_acks_total`.
pub(crate) async fn apply_client_ack(
    dal: &DAL,
    scope: &DeliveryScope,
    id: i64,
) -> Result<AckOutcome, cloacina::error::ValidationError> {
    let outbox = dal.delivery_outbox();
    if outbox
        .mark_acked_for(id, &scope.recipient, scope.tenant_id.as_deref())
        .await?
    {
        return Ok(AckOutcome::Applied);
    }
    // Not applied: tell a repeated ack from an ack of somebody else's row.
    match outbox.get(id).await? {
        Some(row) if !scope.owns_row(&row.recipient, row.tenant_id.as_deref()) => {
            metrics::counter!("cloacina_delivery_refused_acks_total").increment(1);
            warn!(
                id,
                recipient = %scope.recipient,
                tenant = ?scope.tenant_id,
                "delivery WS: ack refused: the row is not addressed to this connection; \
                 nothing changed"
            );
            Ok(AckOutcome::Refused)
        }
        _ => Ok(AckOutcome::Noop),
    }
}

/// Parse and act on one client frame.
async fn handle_client_frame(text: &str, dal: &DAL, scope: &DeliveryScope) -> FrameOutcome {
    // T-0629 diagnostic: log every inbound client frame at info so the e2e
    // contract test can prove acks are arriving (or pinpoint where they
    // aren't). Truncated to keep noisy payloads out of the log.
    let preview: String = text.chars().take(256).collect();
    info!(frame = %preview, "delivery WS: client frame received");
    let msg = match serde_json::from_str::<ClientMessage>(text) {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, frame = %text, "delivery WS: invalid client frame");
            return FrameOutcome::Close {
                code: 4400,
                reason: "invalid client frame",
            };
        }
    };
    match msg {
        ClientMessage::Ack { id, .. } => {
            // Idempotent: an ack of a row that is acked already or does not
            // exist changes nothing. An ack of a row of another recipient or
            // tenant changes nothing either (CLOACINA-T-1052). The applied
            // ack is logged at info (T-0629 contract diagnostic) so server
            // stderr shows ack flow at the default RUST_LOG=info level.
            match apply_client_ack(dal, scope, id).await {
                Ok(AckOutcome::Applied) => info!(id, "delivery WS: ack applied"),
                Ok(AckOutcome::Noop) => {
                    debug!(
                        id,
                        "delivery WS: ack of an acked or unknown row; nothing changed"
                    )
                }
                Ok(AckOutcome::Refused) => {}
                Err(e) => warn!(id, error = %e, "delivery WS: ack failed"),
            }
        }
        ClientMessage::Hello {
            protocol_version, ..
        } => {
            // T-0644: the Hello handshake declares the client's protocol
            // version; reject versions this server doesn't speak so an old
            // server fails loudly against a newer SDK instead of silently
            // misinterpreting frames. (v1 still ignores the cursor — the
            // connect-time reset+wake covers replay.)
            if protocol_version != DELIVERY_PROTOCOL_VERSION {
                warn!(
                    got = protocol_version,
                    supported = DELIVERY_PROTOCOL_VERSION,
                    "delivery WS: unsupported protocol_version in hello"
                );
                return FrameOutcome::Close {
                    code: 4426,
                    reason: "unsupported protocol_version",
                };
            }
            debug!("delivery WS hello (cursor ignored in v1)");
        }
    }
    FrameOutcome::Ok
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use cloacina::models::delivery_outbox::NewDeliveryOutbox;

    async fn outbox_db() -> DAL {
        let url = format!(
            "file:delivery_ws_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = cloacina::database::Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    async fn enqueue(dal: &DAL, recipient: &str, tenant: Option<&str>) -> i64 {
        dal.delivery_outbox()
            .enqueue(NewDeliveryOutbox {
                recipient: recipient.to_string(),
                kind: "work_packet".to_string(),
                tenant_id: tenant.map(str::to_string),
                payload: b"{}".to_vec(),
            })
            .await
            .unwrap()
            .id
    }

    async fn state_of(dal: &DAL, id: i64) -> String {
        dal.delivery_outbox()
            .get(id)
            .await
            .unwrap()
            .unwrap()
            .delivery_state
    }

    fn scope(recipient: &str, tenant: Option<&str>) -> DeliveryScope {
        DeliveryScope::new(recipient, tenant.map(str::to_string))
    }

    /// CLOACINA-T-1052, across tenants: a connection of tenant A sends an ack
    /// frame with the row id of a work packet of tenant B. The row stays.
    #[tokio::test]
    async fn an_ack_frame_for_a_row_of_another_tenant_changes_nothing() {
        let dal = outbox_db().await;
        let theirs = enqueue(&dal, "agent:b1", Some("tenant-b")).await;
        let mine = scope("agent:a1", Some("tenant-a"));

        let frame = format!(r#"{{"type":"ack","protocol_version":1,"id":{theirs}}}"#);
        assert!(matches!(
            handle_client_frame(&frame, &dal, &mine).await,
            FrameOutcome::Ok
        ));
        assert_eq!(state_of(&dal, theirs).await, "pending");
        assert_eq!(
            apply_client_ack(&dal, &mine, theirs).await.unwrap(),
            AckOutcome::Refused
        );
        // The same recipient name in another tenant is another scope.
        assert_eq!(
            apply_client_ack(&dal, &scope("agent:b1", Some("tenant-a")), theirs)
                .await
                .unwrap(),
            AckOutcome::Refused
        );
        // A connection with no tenant does not own the rows of a tenant.
        assert_eq!(
            apply_client_ack(&dal, &scope("agent:b1", None), theirs)
                .await
                .unwrap(),
            AckOutcome::Refused
        );
        assert_eq!(state_of(&dal, theirs).await, "pending");
    }

    /// CLOACINA-T-1052, inside one tenant: a connection cannot ack a row of
    /// another recipient of its tenant (another agent's packet, an
    /// execution's events, an alert notice).
    #[tokio::test]
    async fn an_ack_for_a_row_of_another_recipient_of_the_tenant_changes_nothing() {
        let dal = outbox_db().await;
        let other_agent = enqueue(&dal, "agent:a2", Some("tenant-a")).await;
        let events = enqueue(&dal, "exec_events:e1", Some("tenant-a")).await;
        let notice = enqueue(&dal, "alert_webhook", Some("tenant-a")).await;
        let mine = scope("agent:a1", Some("tenant-a"));
        for id in [other_agent, events, notice] {
            assert_eq!(
                apply_client_ack(&dal, &mine, id).await.unwrap(),
                AckOutcome::Refused
            );
            assert_eq!(state_of(&dal, id).await, "pending");
        }
    }

    #[tokio::test]
    async fn an_ack_for_the_connections_own_row_applies_one_time() {
        let dal = outbox_db().await;
        let id = enqueue(&dal, "agent:a1", Some("tenant-a")).await;
        let mine = scope("agent:a1", Some("tenant-a"));
        let frame = format!(r#"{{"type":"ack","protocol_version":1,"id":{id}}}"#);
        assert!(matches!(
            handle_client_frame(&frame, &dal, &mine).await,
            FrameOutcome::Ok
        ));
        assert_eq!(state_of(&dal, id).await, "acked");
        assert_eq!(
            apply_client_ack(&dal, &mine, id).await.unwrap(),
            AckOutcome::Noop,
            "a second ack is a no-op"
        );
        assert_eq!(
            apply_client_ack(&dal, &mine, id + 1000).await.unwrap(),
            AckOutcome::Noop,
            "an unknown id is a no-op"
        );
    }

    /// A follower of an execution acks its tenant's events; the platform
    /// admin view (no tenant) acks rows with no tenant.
    #[tokio::test]
    async fn execution_event_rows_are_acked_by_a_connection_of_their_tenant() {
        let dal = outbox_db().await;
        let tenant_row = enqueue(&dal, "exec_events:e1", Some("tenant-a")).await;
        let admin_row = enqueue(&dal, "exec_events:e1", None).await;
        assert_eq!(
            apply_client_ack(&dal, &scope("exec_events:e1", Some("tenant-a")), tenant_row)
                .await
                .unwrap(),
            AckOutcome::Applied
        );
        assert_eq!(
            apply_client_ack(&dal, &scope("exec_events:e1", Some("tenant-a")), admin_row)
                .await
                .unwrap(),
            AckOutcome::Refused
        );
        assert_eq!(
            apply_client_ack(&dal, &scope("exec_events:e1", None), admin_row)
                .await
                .unwrap(),
            AckOutcome::Applied
        );
    }
}
