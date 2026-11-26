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
    let tenant = connection_tenant(&auth)?;
    info!(
        recipient = %recipient,
        key = %auth.name,
        tenant = ?auth.tenant_id,
        "delivery WebSocket upgrade accepted"
    );
    let dal = DAL::new(state.database.clone());
    let sink = state.delivery_sink.clone();
    let wake = state.delivery_wake.clone();
    Ok(ws.on_upgrade(move |socket| {
        handle_delivery_socket(socket, recipient, tenant, dal, sink, wake)
    }))
}

/// The tenant a delivery connection is registered under (CLOACINA-T-1053).
///
/// The sink matches `(recipient, tenant)` exactly, and a connection with no
/// tenant is the admin view: it gets the rows with no tenant and the
/// all-tenant snapshot of `ops_metrics:global`. So "no tenant" on the
/// connection must mean "admin", never only "the key has no tenant":
///
/// - a key with a tenant: that tenant;
/// - an admin key with no tenant: no tenant, the admin view;
/// - a key with no tenant that is not an admin (an OIDC `_` rule): refused.
fn connection_tenant(auth: &AuthenticatedKey) -> Result<Option<String>, ApiError> {
    match (&auth.tenant_id, auth.is_admin) {
        (Some(tenant), _) => Ok(Some(tenant.clone())),
        (None, true) => Ok(None),
        (None, false) => Err(ApiError::forbidden(
            "tenant_access_denied",
            "key has no tenant",
        )),
    }
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
                        handle_client_frame(&text, &dal).await
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

/// Parse and act on one client frame.
async fn handle_client_frame(text: &str, dal: &DAL) -> FrameOutcome {
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
            // Idempotent: if the row is already acked or doesn't exist, the
            // compare-and-set returns InvalidStateTransition; log and move on.
            // Bumped to info/warn (T-0629 contract diagnostic) so server
            // stderr shows ack flow at the default RUST_LOG=info level.
            match dal.delivery_outbox().mark_acked(id).await {
                Ok(()) => info!(id, "delivery WS: ack applied"),
                Err(e) => warn!(id, error = %e, "delivery WS: mark_acked failed"),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn key(tenant: Option<&str>, is_admin: bool, role: &str) -> AuthenticatedKey {
        AuthenticatedKey {
            key_id: uuid::Uuid::new_v4(),
            name: "t".into(),
            permissions: role.into(),
            tenant_id: tenant.map(str::to_string),
            is_admin,
        }
    }

    // CLOACINA-T-1053: a connection with no tenant is the admin view, so only
    // an admin key gets one.
    #[test]
    fn admin_key_connects_with_no_tenant() {
        assert_eq!(connection_tenant(&key(None, true, "admin")).unwrap(), None);
    }

    #[test]
    fn tenant_key_connects_under_its_tenant() {
        for tenant in ["acme", "public"] {
            assert_eq!(
                connection_tenant(&key(Some(tenant), false, "read")).unwrap(),
                Some(tenant.to_string())
            );
        }
    }

    /// The key an OIDC `_` rule makes is refused, whatever its role: it would
    /// otherwise subscribe as the admin view and receive the operations
    /// stream of all tenants.
    #[test]
    fn no_tenant_non_admin_key_is_refused() {
        for role in ["read", "write", "admin"] {
            let err = connection_tenant(&key(None, false, role)).unwrap_err();
            assert_eq!(err.status, axum::http::StatusCode::FORBIDDEN);
            assert_eq!(err.code, "tenant_access_denied");
        }
    }
}
