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
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use tracing::{debug, info, warn};

use cloacina::computation_graph::reactor::{ManualCommand, ReactorCommand, ReactorResponse};
use cloacina::computation_graph::registry::{EndpointRegistry, KeyContext};
use cloacina::computation_graph::types::InputCache;
use cloacina::computation_graph::SourceName;

use super::auth::{validate_token, AuthenticatedKey};
use crate::commands::serve::AppState;
use crate::server::error::ApiError;

/// Query parameter for passing a single-use ticket on WebSocket upgrade.
///
/// WebSocket clients can't set custom headers on the upgrade request in
/// browsers, so we accept a **ticket** (not an API key) as a query parameter:
/// `ws://host/v1/ws/accumulator/alpha?token=<ticket>`
///
/// Tickets are obtained via `POST /auth/ws-ticket` and are single-use with
/// a short TTL. Raw API keys are NOT accepted in query parameters.
#[derive(Deserialize)]
pub struct WsAuthQuery {
    pub token: Option<String>,
}

/// Where the auth credential came from — determines validation strategy.
enum WsTokenSource {
    /// Bearer token from Authorization header — validated as API key.
    Header(String),
    /// Single-use ticket from query parameter — consumed from WsTicketStore.
    QueryTicket(String),
}

/// Extract the auth token from either the Authorization header or query param.
fn extract_ws_token(headers: &axum::http::HeaderMap, query: &WsAuthQuery) -> Option<WsTokenSource> {
    // Prefer header — accepts Bearer API keys
    if let Some(val) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(s) = val.to_str() {
            if let Some(token) = s.strip_prefix("Bearer ") {
                return Some(WsTokenSource::Header(token.to_string()));
            }
        }
    }
    // Query param — treated as a single-use ticket, NOT a raw API key
    query
        .token
        .as_ref()
        .map(|t| WsTokenSource::QueryTicket(t.clone()))
}

/// Authenticate a WebSocket upgrade request using the appropriate strategy.
async fn authenticate_ws(
    state: &AppState,
    source: WsTokenSource,
) -> Result<AuthenticatedKey, ApiError> {
    match source {
        WsTokenSource::Header(token) => validate_token(state, &token)
            .await
            .map_err(|_| ApiError::unauthorized("invalid bearer token")),
        WsTokenSource::QueryTicket(ticket) => state
            .ws_tickets
            .consume(&ticket)
            .await
            .ok_or_else(|| ApiError::unauthorized("invalid or expired WebSocket ticket")),
    }
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
    let source = match extract_ws_token(request.headers(), &query) {
        Some(s) => s,
        None => {
            return ApiError::unauthorized("missing auth token").into_response();
        }
    };

    let auth = match authenticate_ws(&state, source).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };

    // Per-endpoint authorization check
    let ctx = KeyContext {
        key_id: &auth.key_id,
        tenant_id: auth.tenant_id.as_deref(),
        is_admin: auth.is_admin,
    };
    if let Err(_e) = state
        .endpoint_registry
        .check_accumulator_auth(&name, &ctx)
        .await
    {
        return ApiError::forbidden(
            "endpoint_access_denied",
            format!("not authorized for accumulator '{}'", name),
        )
        .into_response();
    }

    info!(
        accumulator = %name,
        key = %auth.name,
        "WebSocket upgrade accepted for accumulator"
    );

    let registry = state.endpoint_registry.clone();
    ws.on_upgrade(move |socket| handle_accumulator_socket(socket, name, auth, registry))
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
    let source = match extract_ws_token(request.headers(), &query) {
        Some(s) => s,
        None => {
            return ApiError::unauthorized("missing auth token").into_response();
        }
    };

    let auth = match authenticate_ws(&state, source).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };

    // Per-endpoint authorization check
    let ctx = KeyContext {
        key_id: &auth.key_id,
        tenant_id: auth.tenant_id.as_deref(),
        is_admin: auth.is_admin,
    };
    if let Err(_e) = state
        .endpoint_registry
        .check_reactor_auth(&name, &ctx)
        .await
    {
        return ApiError::forbidden(
            "endpoint_access_denied",
            format!("not authorized for reactor '{}'", name),
        )
        .into_response();
    }

    info!(
        reactor = %name,
        key = %auth.name,
        "WebSocket upgrade accepted for reactor"
    );

    let registry = state.endpoint_registry.clone();
    ws.on_upgrade(move |socket| handle_reactor_socket(socket, name, auth, registry))
}

/// Handle an accepted accumulator WebSocket connection.
///
/// Reads incoming messages and forwards them to all accumulators registered
/// under this name via the EndpointRegistry. External clients send JSON;
/// the accumulator socket receiver handles deserialization.
async fn handle_accumulator_socket(
    mut socket: axum::extract::ws::WebSocket,
    name: String,
    auth: AuthenticatedKey,
    registry: EndpointRegistry,
) {
    debug!(accumulator = %name, key = %auth.name, "accumulator WebSocket connected");

    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(axum::extract::ws::Message::Binary(data)) => {
                // Forward raw bytes to accumulator — it deserializes JSON internally.
                let bytes: Vec<u8> = data.into();
                match registry.send_to_accumulator(&name, bytes).await {
                    Ok(count) => {
                        debug!(accumulator = %name, recipients = count, "forwarded binary message");
                    }
                    Err(e) => {
                        warn!(accumulator = %name, error = %e, "failed to forward message");
                        let _ = socket
                            .send(axum::extract::ws::Message::Close(Some(
                                axum::extract::ws::CloseFrame {
                                    code: 4404,
                                    reason: format!("accumulator '{}' not registered", name).into(),
                                },
                            )))
                            .await;
                        break;
                    }
                }
            }
            Ok(axum::extract::ws::Message::Text(_)) => {
                warn!(accumulator = %name, "text frames not supported — send JSON as binary frame");
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
/// Receives JSON `ReactorCommand` messages, dispatches to the reactor via
/// the EndpointRegistry (ForceFire/FireWith) or ReactorHandle (GetState/Pause/Resume),
/// and sends back `ReactorResponse` JSON.
async fn handle_reactor_socket(
    mut socket: axum::extract::ws::WebSocket,
    name: String,
    auth: AuthenticatedKey,
    registry: EndpointRegistry,
) {
    debug!(reactor = %name, key = %auth.name, "reactor WebSocket connected");

    // Get the reactor handle for GetState/Pause/Resume
    let handle = registry.get_reactor_handle(&name).await;

    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                let response = match serde_json::from_str::<ReactorCommand>(&text) {
                    Ok(cmd) => {
                        process_reactor_command(
                            &name,
                            cmd,
                            &registry,
                            &handle,
                            auth.key_id,
                            auth.tenant_id.clone(),
                            auth.is_admin,
                        )
                        .await
                    }
                    Err(e) => ReactorResponse::Error {
                        message: format!("invalid command: {}", e),
                    },
                };

                let json = serde_json::to_string(&response).unwrap_or_else(|e| {
                    format!(
                        "{{\"type\":\"error\",\"message\":\"serialization failed: {}\"}}",
                        e
                    )
                });
                if socket
                    .send(axum::extract::ws::Message::Text(json.into()))
                    .await
                    .is_err()
                {
                    break;
                }
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

/// Map a ReactorCommand to its corresponding ReactorOp for authZ checks.
fn command_to_op(cmd: &ReactorCommand) -> cloacina::computation_graph::registry::ReactorOp {
    use cloacina::computation_graph::registry::ReactorOp;
    match cmd {
        ReactorCommand::ForceFire => ReactorOp::ForceFire,
        ReactorCommand::FireWith { .. } => ReactorOp::FireWith,
        ReactorCommand::GetState => ReactorOp::GetState,
        ReactorCommand::Pause => ReactorOp::Pause,
        ReactorCommand::Resume => ReactorOp::Resume,
    }
}

/// Process a single reactor command and return the response.
async fn process_reactor_command(
    name: &str,
    cmd: ReactorCommand,
    registry: &EndpointRegistry,
    handle: &Option<cloacina::computation_graph::reactor::ReactorHandle>,
    key_id: uuid::Uuid,
    key_tenant_id: Option<String>,
    key_is_admin: bool,
) -> ReactorResponse {
    // Per-command authZ check
    let op = command_to_op(&cmd);
    let ctx = KeyContext {
        key_id: &key_id,
        tenant_id: key_tenant_id.as_deref(),
        is_admin: key_is_admin,
    };
    if let Err(_e) = registry.check_reactor_op_auth(name, &ctx, &op).await {
        return ReactorResponse::Error {
            message: format!("operation {:?} not permitted on reactor '{}'", op, name),
        };
    }

    match cmd {
        ReactorCommand::ForceFire => {
            match registry
                .send_to_reactor(name, ManualCommand::ForceFire)
                .await
            {
                Ok(()) => ReactorResponse::Fired,
                Err(e) => ReactorResponse::Error {
                    message: e.to_string(),
                },
            }
        }
        ReactorCommand::FireWith { cache } => {
            let mut input_cache = InputCache::new();
            for (k, v) in cache {
                input_cache.update(SourceName::new(&k), v);
            }
            match registry
                .send_to_reactor(name, ManualCommand::FireWith(input_cache))
                .await
            {
                Ok(()) => ReactorResponse::Fired,
                Err(e) => ReactorResponse::Error {
                    message: e.to_string(),
                },
            }
        }
        ReactorCommand::GetState => match handle {
            Some(h) => {
                let state = h.get_state().await;
                ReactorResponse::State { cache: state }
            }
            None => ReactorResponse::Error {
                message: format!("no reactor handle for '{}'", name),
            },
        },
        ReactorCommand::Pause => match handle {
            Some(h) => {
                h.pause();
                ReactorResponse::Paused
            }
            None => ReactorResponse::Error {
                message: format!("no reactor handle for '{}'", name),
            },
        },
        ReactorCommand::Resume => match handle {
            Some(h) => {
                h.resume();
                ReactorResponse::Resumed
            }
            None => ReactorResponse::Error {
                message: format!("no reactor handle for '{}'", name),
            },
        },
    }
}
