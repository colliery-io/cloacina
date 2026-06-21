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

//! Reactor WebSocket command protocol (`GET /v1/ws/reactor/{name}`) and the
//! REST manual-fire surface (`POST /v1/health/reactors/{name}/fire`).
//!
//! The WS types moved here from `cloacina::computation_graph::reactor` in
//! T-0642 — they are the client-facing operator commands, not engine
//! internals. The REST request/response types (CLOACI-T-0751) wrap the same
//! `FireWith` / `ForceFire` mechanics in an operator-friendly, typed-JSON
//! surface so operators never hand-craft raw boundary bytes.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Commands sent by WebSocket operators to a reactor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum ReactorCommand {
    ForceFire,
    FireWith { cache: HashMap<String, Vec<u8>> },
    GetState,
    Pause,
    Resume,
}

/// Responses sent back to WebSocket operators.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReactorResponse {
    Fired,
    State { cache: HashMap<String, String> },
    Paused,
    Resumed,
    Error { message: String },
}

/// How a manual REST fire should populate the reactor's input cache.
///
/// CLOACI-T-0751. Mirrors the two WS write commands (`ForceFire` /
/// `FireWith`) but with operator-friendly, typed input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum FireMode {
    /// Fire the graph with the reactor's *current* cache, untouched.
    /// Equivalent to the WS `ForceFire` command; `inputs` is ignored.
    #[default]
    ForceFire,
    /// Replace the reactor's cache with the supplied typed `inputs`, then
    /// fire. Equivalent to the WS `FireWith` command. Full-replace only —
    /// the existing cache is discarded (`replace_all`), there is no
    /// partial/merge mode in v1.
    FireWith,
}

/// Request body for `POST /v1/health/reactors/{name}/fire` (CLOACI-T-0751).
///
/// Operators supply typed JSON per source; the server serializes each value
/// to the boundary wire encoding so callers never deal in raw `Vec<u8>`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct FireReactorRequest {
    /// Whether to fire with the current cache (`force_fire`) or to inject
    /// `inputs` first (`fire_with`). Defaults to `force_fire`.
    #[serde(default)]
    pub mode: FireMode,
    /// Per-source typed payloads, keyed by accumulator source name. Each
    /// JSON value is serialized to the boundary encoding server-side.
    /// Required (and non-empty) when `mode` is `fire_with`; ignored for
    /// `force_fire`. Each value may be any JSON.
    #[serde(default)]
    pub inputs: HashMap<String, serde_json::Value>,
}

/// Response body for a successful manual reactor fire (CLOACI-T-0751).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct FireReactorResponse {
    /// Echoes the reactor name that was fired.
    pub reactor: String,
    /// The mode that was applied.
    pub mode: FireMode,
    /// Source names whose values were injected (empty for `force_fire`).
    pub sources_injected: Vec<String>,
}

/// Request body for `POST /v1/health/accumulators/{name}/inject` (CLOACI-T-0753)
/// — push a single typed event into a running accumulator, the operator-facing
/// REST analogue of the WS accumulator-push path. The JSON `event` is serialized
/// to the boundary wire encoding server-side, so operators never craft raw
/// `Vec<u8>`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct InjectAccumulatorRequest {
    /// The event payload (any JSON) to push to the accumulator.
    pub event: serde_json::Value,
}

/// Response body for a successful accumulator inject (CLOACI-T-0753).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct InjectAccumulatorResponse {
    /// Echoes the accumulator name the event was pushed to.
    pub accumulator: String,
    /// Number of receivers the event was delivered to.
    pub delivered: usize,
}
