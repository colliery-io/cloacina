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

//! Reactor WebSocket command protocol (`GET /v1/ws/reactor/{name}`).
//!
//! Moved here from `cloacina::computation_graph::reactor` in T-0642 — these
//! are the client-facing operator commands, not engine internals.

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
