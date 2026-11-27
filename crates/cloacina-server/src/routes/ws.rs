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
//! The upgrade request is authenticated and authorized by the `guard`
//! middleware like a REST route (CLOACINA-T-1055): a bearer key or a
//! single-use ticket, then the row of the route in the authorization table.
//! The handlers read the key from the request extensions. A reactor command
//! that changes state is checked and audited when it arrives.

use axum::{
    Extension,
    extract::{Path, State, WebSocketUpgrade},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use tracing::{debug, info, warn};

use cloacina::computation_graph::SourceName;
use cloacina::computation_graph::reactor::{ManualCommand, ReactorCommand, ReactorResponse};
use cloacina::computation_graph::registry::{EndpointRegistry, EndpointScope, KeyContext};
use cloacina::computation_graph::types::InputCache;

use super::auth::AuthenticatedKey;
use crate::AppState;
use crate::routes::audit::{self as audit_record, AuditEvent};
use crate::routes::authz::{DENY_ROLE, Permission, authorize_caller};
use crate::routes::error::ApiError;

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

/// Increment the `cloacina_ws_auth_failures_total` counter with a bounded
/// `reason` label. The four documented reasons (see I-0099 / T-0588):
/// `ticket_expired`, `invalid_signature`, `tenant_mismatch`, `not_authorized`.
pub(crate) fn record_ws_auth_failure(reason: &'static str) {
    metrics::counter!(
        "cloacina_ws_auth_failures_total",
        "reason" => reason,
    )
    .increment(1);
}

/// Record one inbound or outbound WebSocket message. `endpoint` is the
/// bounded `{accumulator, reactor}` enum; `direction` is `{in, out}`.
fn record_ws_message(endpoint: &'static str, direction: &'static str) {
    metrics::counter!(
        "cloacina_ws_messages_total",
        "endpoint" => endpoint,
        "direction" => direction,
    )
    .increment(1);
}

/// RAII guard for `cloacina_ws_connections_active`. Increments on
/// construction and decrements on Drop so a panic or early return inside
/// the handler still releases the slot — defends the gauge against leak
/// bugs of the kind that motivated T-0534.
struct WsConnectionGuard {
    endpoint: &'static str,
}

impl WsConnectionGuard {
    fn new(endpoint: &'static str) -> Self {
        metrics::gauge!(
            "cloacina_ws_connections_active",
            "endpoint" => endpoint,
        )
        .increment(1.0);
        Self { endpoint }
    }
}

impl Drop for WsConnectionGuard {
    fn drop(&mut self) {
        metrics::gauge!(
            "cloacina_ws_connections_active",
            "endpoint" => self.endpoint,
        )
        .decrement(1.0);
    }
}

/// A connection accepted for an accumulator whose reactor another replica
/// owns, when no owner address was published yet (CLOACI-T-0851).
struct ForeignReactor {
    /// The reactor's tenant. The frames of the connection are written for
    /// this tenant, not for the key's: an admin key has none (CLOACINA-T-1053).
    tenant_id: Option<String>,
}

/// The URL of the accumulator WebSocket on the replica at `owner_addr`: the
/// `Location` of the handshake redirect. It is the path of the route
/// `/v1/ws/accumulator/{name}`.
fn accumulator_ws_location(owner_addr: &str, name: &str) -> String {
    format!(
        "{}/v1/ws/accumulator/{}",
        owner_addr.trim_end_matches('/'),
        name
    )
}

/// WebSocket handler for accumulator endpoints.
///
/// `GET /v1/ws/accumulator/{name}` — upgrades to WebSocket after PAK auth.
/// Currently a stub: accepts the connection, logs, and closes.
/// Business logic (forwarding to registry) wired in T-0373.
pub async fn accumulator_ws(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Extension(auth): Extension<AuthenticatedKey>,
    ws: WebSocketUpgrade,
) -> Response {
    let ctx = KeyContext {
        key_id: &auth.key_id,
        tenant_id: auth.tenant_id.as_deref(),
        is_admin: auth.is_admin,
    };

    // CLOACI-T-0851 / A-0012 Amendment 3: if another replica owns the reactor
    // hosting this accumulator, redirect at the HANDSHAKE. The upgrade request
    // is ordinary HTTP, so a 307 here steers the client to the owner ONCE, and
    // the connection then stays pinned there for its lifetime — every
    // subsequent frame is an in-process channel send on the owner. A connection
    // accepted here instead would pay a per-event forward forever; a redirect
    // mid-stream is impossible (WS clients only follow redirects pre-upgrade).
    //
    // Runs BEFORE the local auth check, which fails CLOSED for foreign
    // accumulators (no local registration → no policy — observed live as a 403
    // at the non-owner for an inject the owner accepted with 200). Guarded by
    // the equivalent authz — admin, or tenant-match — so the redirect does not
    // become an unauthenticated probe for reactor placement. A key with no
    // tenant that is not an admin matches no reactor (CLOACINA-T-1053).
    // Checked BEFORE on_upgrade — after acceptance the 307 can never be sent.
    //
    let mut known_foreign: Option<ForeignReactor> = None;
    if let Some(reactor_key) = state
        .graph_scheduler
        .foreign_reactor_for_accumulator(
            cloacina::TenantScope {
                tenant_id: ctx.tenant_id,
                is_admin: ctx.is_admin,
            },
            &name,
        )
        .await
    {
        let authorized = ctx.may_act_for(reactor_key.tenant_id.as_deref());
        if authorized {
            let dal = cloacina::dal::unified::DAL::new(state.database.clone());
            match dal
                .reactor_owner_addresses()
                .lookup(reactor_key.tenant_id.as_deref(), &reactor_key.name)
                .await
            {
                Ok(Some(owner_addr)) => {
                    let location = accumulator_ws_location(&owner_addr, &name);
                    info!(
                        accumulator = %name,
                        owner = %location,
                        "accumulator owned by another replica; redirecting WS handshake"
                    );
                    return Response::builder()
                        .status(axum::http::StatusCode::TEMPORARY_REDIRECT)
                        .header(axum::http::header::LOCATION, location)
                        .body(axum::body::Body::empty())
                        .unwrap_or_else(|_| {
                            ApiError::internal("redirect construction failed").into_response()
                        });
                }
                // No address published (mid-takeover) — accept the connection in
                // fallback mode: frames go through the inject path (local try,
                // then durable outbox), so events are slow but never lost. The
                // client's next reconnect gets the redirect once the new owner
                // publishes.
                Ok(None) => {
                    known_foreign = Some(ForeignReactor {
                        tenant_id: reactor_key.tenant_id.clone(),
                    });
                }
                Err(e) => {
                    warn!(accumulator = %name, "owner-address lookup failed: {e}");
                }
            }
        }
    }

    // Per-endpoint authorization check (skipped only when the connection is
    // already known-foreign and authorized above — there is no local policy to
    // consult for an endpoint this replica does not host).
    if known_foreign.is_none() {
        if let Err(_e) = state
            .endpoint_registry
            .check_accumulator_auth(&name, &ctx)
            .await
        {
            record_ws_auth_failure("tenant_mismatch");
            return ApiError::forbidden(
                "endpoint_access_denied",
                format!("not authorized for accumulator '{}'", name),
            )
            .into_response();
        }
    }

    info!(
        accumulator = %name,
        key = %auth.name,
        "WebSocket upgrade accepted for accumulator"
    );

    let registry = state.endpoint_registry.clone();
    let dal = cloacina::dal::unified::DAL::new(state.database.clone());
    ws.on_upgrade(move |socket| {
        handle_accumulator_socket(socket, name, auth, registry, dal, known_foreign)
    })
}

/// WebSocket handler for reactor endpoints.
///
/// `GET /v1/ws/reactor/{name}` — upgrades to WebSocket after PAK auth.
/// Currently a stub: accepts the connection, logs, and closes.
/// Business logic (command handling) wired in T-0374.
pub async fn reactor_ws(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Extension(auth): Extension<AuthenticatedKey>,
    ws: WebSocketUpgrade,
) -> Response {
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
        record_ws_auth_failure("tenant_mismatch");
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
    ws.on_upgrade(move |socket| handle_reactor_socket(socket, name, auth, registry, state))
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
    dal: cloacina::dal::unified::DAL,
    // Decided at handshake: this accumulator's reactor is owned elsewhere and
    // no redirect was possible (no address published). Frames route through
    // the inject fallback rather than the direct registry send.
    known_foreign: Option<ForeignReactor>,
) {
    debug!(accumulator = %name, key = %auth.name, "accumulator WebSocket connected");
    let _conn_guard = WsConnectionGuard::new("accumulator");

    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(axum::extract::ws::Message::Binary(data)) => {
                record_ws_message("accumulator", "in");
                // Forward raw bytes to accumulator — it deserializes JSON internally.
                let bytes: Vec<u8> = data.into();
                // CLOACI-T-0921: `{name}` from the URL is resolved inside the
                // AUTHENTICATED key's tenant, so two tenants may both own an
                // accumulator called `ticks` without cross-wiring.
                let scope = EndpointScope {
                    tenant_id: auth.tenant_id.as_deref(),
                    is_admin: auth.is_admin,
                };
                // CLOACI-T-0851: a connection accepted for a KNOWN-FOREIGN
                // accumulator (mid-takeover — no owner address existed at
                // handshake, so no redirect was possible) routes through
                // `inject_event`: local first (covers a takeover TO this
                // replica), then the durable outbox — slow but never lost.
                // Decided ONCE at handshake, not per frame, so the ordinary
                // local path below pays nothing for this. If ownership is
                // lost MID-connection instead, the local send fails, the
                // 4404 close fires, and the client's reconnect handshake
                // performs the redirect — which is the correct recovery.
                if let Some(foreign) = &known_foreign {
                    match cloacina::computation_graph::reactor_routing::inject_event(
                        &registry,
                        Some(&dal),
                        &name,
                        foreign.tenant_id.as_deref(),
                        bytes,
                    )
                    .await
                    {
                        Ok(_) => {
                            record_ws_message("accumulator", "in");
                            continue;
                        }
                        Err(e) => {
                            warn!(accumulator = %name, error = %e, "foreign-frame forward failed; closing");
                        }
                    }
                    let _ = socket
                        .send(axum::extract::ws::Message::Close(Some(
                            axum::extract::ws::CloseFrame {
                                code: 4404,
                                reason: format!("accumulator '{}' not reachable", name).into(),
                            },
                        )))
                        .await;
                    break;
                }
                match registry.send_to_accumulator(&name, scope, bytes).await {
                    Ok(count) => {
                        debug!(accumulator = %name, recipients = count, "forwarded binary message");
                    }
                    Err(e) => {
                        warn!(accumulator = %name, error = %e, "failed to forward message");
                        if socket
                            .send(axum::extract::ws::Message::Close(Some(
                                axum::extract::ws::CloseFrame {
                                    code: 4404,
                                    reason: format!("accumulator '{}' not registered", name).into(),
                                },
                            )))
                            .await
                            .is_ok()
                        {
                            record_ws_message("accumulator", "out");
                        }
                        break;
                    }
                }
            }
            Ok(axum::extract::ws::Message::Text(_)) => {
                record_ws_message("accumulator", "in");
                warn!(accumulator = %name, "text frames not supported — send JSON as binary frame");
            }
            Ok(axum::extract::ws::Message::Close(_)) => {
                record_ws_message("accumulator", "in");
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
    state: AppState,
) {
    debug!(reactor = %name, key = %auth.name, "reactor WebSocket connected");
    let _conn_guard = WsConnectionGuard::new("reactor");

    // Get the reactor handle for GetState/Pause/Resume. CLOACI-T-0921: resolved
    // within the authenticated key's tenant.
    let scope = EndpointScope {
        tenant_id: auth.tenant_id.as_deref(),
        is_admin: auth.is_admin,
    };
    let handle = registry.get_reactor_handle(&name, scope).await;

    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                record_ws_message("reactor", "in");
                let response = match serde_json::from_str::<ReactorCommand>(&text) {
                    Ok(cmd) => {
                        // CLOACINA-T-1055: a command that changes state goes
                        // through the same matcher as a REST route, and writes
                        // the same audit row. Reading the state needs only the
                        // permission of the connect.
                        let action = command_action(&cmd);
                        let permitted = action.is_none()
                            || authorize_caller(&auth, Permission::ReactorsOperate).is_permit();
                        if let Some(action) = action
                            && !permitted
                        {
                            audit_record::record(
                                &state,
                                &auth,
                                auth.tenant_id.as_deref(),
                                AuditEvent::denied(
                                    action,
                                    Some(name.clone()),
                                    serde_json::json!({ "reason": DENY_ROLE, "via": "websocket" }),
                                ),
                            )
                            .await;
                            ReactorResponse::Error {
                                message: format!(
                                    "insufficient permissions: this command needs {}",
                                    Permission::ReactorsOperate.name()
                                ),
                            }
                        } else {
                            let response = process_reactor_command(
                                &name,
                                cmd,
                                &registry,
                                &handle,
                                auth.key_id,
                                auth.tenant_id.clone(),
                                auth.is_admin,
                            )
                            .await;
                            if let Some(action) = action
                                && !matches!(response, ReactorResponse::Error { .. })
                            {
                                audit_record::record(
                                    &state,
                                    &auth,
                                    auth.tenant_id.as_deref(),
                                    AuditEvent::ok(action, name.as_str())
                                        .detail(serde_json::json!({ "via": "websocket" })),
                                )
                                .await;
                            }
                            response
                        }
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
                record_ws_message("reactor", "out");
            }
            Ok(axum::extract::ws::Message::Close(_)) => {
                record_ws_message("reactor", "in");
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

/// The audited action of a reactor command, `None` for one that only reads.
/// A command with an action needs the permission `reactors.operate`.
fn command_action(cmd: &ReactorCommand) -> Option<audit_record::Action> {
    use audit_record::actions;
    match cmd {
        ReactorCommand::ForceFire | ReactorCommand::FireWith { .. } => Some(actions::REACTOR_FIRE),
        ReactorCommand::Pause => Some(actions::REACTOR_PAUSE),
        ReactorCommand::Resume => Some(actions::REACTOR_RESUME),
        ReactorCommand::GetState => None,
    }
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

    // CLOACI-T-0921: commands resolve the reactor inside the caller's tenant.
    let scope = EndpointScope::from_key_context(&ctx);

    match cmd {
        ReactorCommand::ForceFire => {
            match registry
                .send_to_reactor(name, scope, ManualCommand::ForceFire)
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
                .send_to_reactor(name, scope, ManualCommand::FireWith(input_cache))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The handshake redirect names the route the router serves
    /// (`/v1/ws/accumulator/{name}`, singular).
    #[test]
    fn the_accumulator_redirect_names_the_registered_route() {
        let location = accumulator_ws_location("http://replica-2:8080/", "orders");
        assert_eq!(location, "http://replica-2:8080/v1/ws/accumulator/orders");
        let table = crate::routes::authz::build_authz_table();
        let route = location
            .strip_prefix("http://replica-2:8080")
            .unwrap()
            .replace("orders", "{name}");
        assert!(
            table.get(&axum::http::Method::GET, &route).is_some(),
            "{route} is not a route of the server"
        );
    }

    #[test]
    fn a_reactor_command_that_changes_state_has_an_audit_action() {
        use crate::routes::audit::actions;
        assert_eq!(
            command_action(&ReactorCommand::ForceFire),
            Some(actions::REACTOR_FIRE)
        );
        assert_eq!(
            command_action(&ReactorCommand::FireWith {
                cache: Default::default()
            }),
            Some(actions::REACTOR_FIRE)
        );
        assert_eq!(
            command_action(&ReactorCommand::Pause),
            Some(actions::REACTOR_PAUSE)
        );
        assert_eq!(
            command_action(&ReactorCommand::Resume),
            Some(actions::REACTOR_RESUME)
        );
        assert_eq!(command_action(&ReactorCommand::GetState), None);
    }
}
