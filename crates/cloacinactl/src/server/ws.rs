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

//! WebSocket handlers for accumulator and reactor endpoints.
//!
//! - `/v1/ws/accumulator/{name}` — external producers push events to accumulators
//! - `/v1/ws/reactor/{name}` — operators send manual commands to reactors
//!
//! Auth is validated on the HTTP upgrade request before promoting to WebSocket.
//! Business logic (registry lookup, message forwarding) is wired in later tasks.

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use tracing::{debug, info, warn};

use super::auth::{validate_token, AuthenticatedKey};
use crate::commands::serve::AppState;

/// Query parameter for passing the auth token on WebSocket upgrade.
///
/// WebSocket clients can't set custom headers on the upgrade request in
/// browsers, so we accept the token as a query parameter as well:
/// `ws://host/v1/ws/accumulator/alpha?token=<pak>`
#[derive(Deserialize)]
pub struct WsAuthQuery {
    pub token: Option<String>,
}

/// Extract the auth token from either the Authorization header or query param.
fn extract_ws_token(headers: &axum::http::HeaderMap, query: &WsAuthQuery) -> Option<String> {
    // Prefer header
    if let Some(val) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(s) = val.to_str() {
            if let Some(token) = s.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }
    // Fall back to query param
    query.token.clone()
}

/// WebSocket handler for accumulator endpoints.
///
/// `GET /v1/ws/accumulator/{name}` — upgrades to WebSocket after PAK auth.
/// Currently a stub: accepts the connection, logs, and closes.
/// Business logic (forwarding to registry) wired in T-0373.
pub async fn accumulator_ws(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<WsAuthQuery>,
    ws: WebSocketUpgrade,
    request: axum::extract::Request,
) -> Response {
    let token = match extract_ws_token(request.headers(), &query) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "missing auth token"})),
            )
                .into_response();
        }
    };

    let auth = match validate_token(&state, &token).await {
        Ok(a) => a,
        Err(resp) => return resp.into_response(),
    };

    info!(
        accumulator = %name,
        key = %auth.name,
        "WebSocket upgrade accepted for accumulator"
    );

    ws.on_upgrade(move |socket| handle_accumulator_socket(socket, name, auth))
}

/// WebSocket handler for reactor endpoints.
///
/// `GET /v1/ws/reactor/{name}` — upgrades to WebSocket after PAK auth.
/// Currently a stub: accepts the connection, logs, and closes.
/// Business logic (command handling) wired in T-0374.
pub async fn reactor_ws(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<WsAuthQuery>,
    ws: WebSocketUpgrade,
    request: axum::extract::Request,
) -> Response {
    let token = match extract_ws_token(request.headers(), &query) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "missing auth token"})),
            )
                .into_response();
        }
    };

    let auth = match validate_token(&state, &token).await {
        Ok(a) => a,
        Err(resp) => return resp.into_response(),
    };

    info!(
        reactor = %name,
        key = %auth.name,
        "WebSocket upgrade accepted for reactor"
    );

    ws.on_upgrade(move |socket| handle_reactor_socket(socket, name, auth))
}

/// Handle an accepted accumulator WebSocket connection.
///
/// Stub implementation — logs connection lifecycle. Real forwarding in T-0373.
async fn handle_accumulator_socket(
    mut socket: axum::extract::ws::WebSocket,
    name: String,
    auth: AuthenticatedKey,
) {
    debug!(accumulator = %name, key = %auth.name, "accumulator WebSocket connected");

    // Read messages until client disconnects
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(axum::extract::ws::Message::Binary(data)) => {
                debug!(
                    accumulator = %name,
                    bytes = data.len(),
                    "received binary message (stub — not forwarded yet)"
                );
            }
            Ok(axum::extract::ws::Message::Text(text)) => {
                debug!(
                    accumulator = %name,
                    len = text.len(),
                    "received text message (stub — not forwarded yet)"
                );
            }
            Ok(axum::extract::ws::Message::Close(_)) => {
                debug!(accumulator = %name, "client sent close frame");
                break;
            }
            Ok(_) => {} // ping/pong handled by axum
            Err(e) => {
                warn!(accumulator = %name, error = %e, "WebSocket error");
                break;
            }
        }
    }

    debug!(accumulator = %name, "accumulator WebSocket disconnected");
}

/// Handle an accepted reactor WebSocket connection.
///
/// Stub implementation — logs connection lifecycle. Real command handling in T-0374.
async fn handle_reactor_socket(
    mut socket: axum::extract::ws::WebSocket,
    name: String,
    auth: AuthenticatedKey,
) {
    debug!(reactor = %name, key = %auth.name, "reactor WebSocket connected");

    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                debug!(
                    reactor = %name,
                    len = text.len(),
                    "received command (stub — not processed yet)"
                );
                // Stub: echo back an error response
                let resp = serde_json::json!({"error": "reactor WebSocket not yet wired"});
                let _ = socket
                    .send(axum::extract::ws::Message::Text(resp.to_string().into()))
                    .await;
            }
            Ok(axum::extract::ws::Message::Close(_)) => {
                debug!(reactor = %name, "client sent close frame");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                warn!(reactor = %name, error = %e, "WebSocket error");
                break;
            }
        }
    }

    debug!(reactor = %name, "reactor WebSocket disconnected");
}
