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

//! Execution-agent fleet API types (CLOACI-I-0124 / WS-0b).

use serde::{Deserialize, Serialize};

/// One registered execution agent in the in-memory fleet roster.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AgentInfo {
    pub agent_id: String,
    pub target_triple: String,
    pub max_concurrency: u32,
    pub in_flight: u32,
    pub available_capacity: u32,
    /// Seconds since this agent's last heartbeat — the liveness signal an
    /// operator reads (the underlying record stores a monotonic `Instant`,
    /// not a wall-clock time).
    pub seconds_since_heartbeat: u64,
    pub capabilities: Vec<String>,
    /// Tenant scope the agent registered under, if any.
    pub tenant_id: Option<String>,
}
