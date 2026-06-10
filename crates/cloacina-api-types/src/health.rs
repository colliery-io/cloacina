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

//! Computation-graph health endpoint types.

use serde::{Deserialize, Serialize};

/// One row in `GET /v1/health/accumulators`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AccumulatorStatus {
    pub name: String,
    /// Accumulator health as reported by the endpoint registry. Free-form
    /// JSON for now; structured in a later contract revision.
    pub status: serde_json::Value,
}

/// One row in `GET /v1/health/graphs`, and the `GET /v1/health/graphs/{name}`
/// response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct GraphStatus {
    pub name: String,
    /// Graph health snapshot; `{"state": "running" | "stopped"}` when no
    /// detailed health is available. Free-form JSON for now.
    pub health: serde_json::Value,
    /// Names of the accumulators feeding this graph.
    pub accumulators: Vec<String>,
    /// Pause state of the graph's reactor.
    pub paused: bool,
}
