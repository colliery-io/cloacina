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
    /// Node/edge topology of the computation graph, for rendering its DAG.
    /// `None` for graphs predating topology emission. (CLOACI-T-0673)
    #[serde(default)]
    pub topology: Option<GraphTopology>,
    /// Name of the reactor this graph is bound to (the trigger that fires it).
    #[serde(default)]
    pub reactor: Option<String>,
    /// Reaction mode of the bound reactor: `"when_any"` | `"when_all"`.
    #[serde(default)]
    pub reaction_mode: Option<String>,
    /// Input strategy of the bound reactor: `"latest"` | `"sequential"`.
    #[serde(default)]
    pub input_strategy: Option<String>,
    /// Total graph fires since load — the reactor's live fire counter
    /// (CLOACI-I-0124 / WS-10). The UI derives recent throughput from the delta
    /// across successive polls.
    #[serde(default)]
    pub fires: u64,
    /// RFC 3339 timestamp of the last graph fire; `null` if it hasn't fired yet.
    #[serde(default)]
    pub last_fired_at: Option<String>,
}

/// Node/edge topology of a computation graph (CLOACI-T-0673).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct GraphTopology {
    pub nodes: Vec<GraphTopologyNode>,
    pub edges: Vec<GraphTopologyEdge>,
}

/// One compute node in a computation graph (CLOACI-T-0673).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct GraphTopologyNode {
    /// Node id (the compute function name).
    pub id: String,
    /// Accumulator names this node reads from the input cache (entry nodes).
    #[serde(default)]
    pub inputs: Vec<String>,
}

/// One directed edge in a computation graph (CLOACI-T-0673).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct GraphTopologyEdge {
    pub from: String,
    pub to: String,
    /// Routing-variant label for conditional edges; `None` for linear edges.
    #[serde(default)]
    pub label: Option<String>,
}
