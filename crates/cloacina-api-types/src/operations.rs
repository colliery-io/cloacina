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

//! Operational metrics event (CLOACI-T-0718).
//!
//! A single snapshot of the control plane's own health — server liveness/
//! readiness, the build pipeline, the execution-agent fleet, and the registry
//! reconciler — pushed to the Operations UI over the WS substrate (direct
//! in-memory publish, not the durable outbox; ephemeral latest-snapshot
//! semantics). Replaces the per-tile ~5s REST pollers.

use serde::{Deserialize, Serialize};

use crate::compiler::CompilerStatus;
use crate::fleet::AgentInfo;

/// One operational-metrics snapshot. The `payload` of an `ops_metrics` push
/// frame on the substrate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct OpsMetricsEvent {
    /// Server liveness + readiness (the `/health` + `/ready` rollup).
    pub server: ServerHealthLite,
    /// Build-pipeline status (same shape as `GET /v1/compiler/status`).
    pub compiler: CompilerStatus,
    /// Execution-agent fleet roster (same shape as `GET /v1/agents`).
    pub fleet: Vec<AgentInfo>,
    /// Registry reconciler / package-availability status.
    pub reconciler: ReconcilerStatus,
    /// RFC 3339 timestamp this snapshot was gathered.
    pub ts: String,
}

/// Server liveness + readiness, mirroring what the UI derived from `/health`
/// and `/ready`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ServerHealthLite {
    /// Process is up (always true if this event arrived, but explicit for the UI).
    pub alive: bool,
    /// DB pool reachable and no crashed computation graphs.
    pub ready: bool,
    /// Why not ready, when `ready == false`.
    pub reason: Option<String>,
}

/// Registry reconciler status (absorbs CLOACI-T-0717). v1 reports the
/// DB-derivable package-availability signal: how many packages built
/// successfully (available to load), how many failed to build, and when the
/// most recent successful build landed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ReconcilerStatus {
    /// Coarse status: `"ok"` (no build failures) or `"errors"`.
    pub status: String,
    /// Packages with a successful, non-superseded build — available to load.
    pub built: u64,
    /// Packages whose latest build failed.
    pub failed: u64,
    /// RFC 3339 timestamp of the most recent successful build, if any.
    pub last_built_at: Option<String>,
}
