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

//! Compiler / build-pipeline status (CLOACI-I-0124 / WS-0b).

use serde::{Deserialize, Serialize};

/// Build-pipeline state, derived from the build queue in the database — the
/// same rows the compiler's own `/v1/status` reports. The server reads them
/// directly, so this needs no HTTP coupling to the compiler service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CompilerStatus {
    /// Coarse roll-up: `"building"` (work in flight), `"backlogged"` (packages
    /// pending but none building — the compiler may be down), or `"idle"`
    /// (nothing queued; liveness is undeterminable from the queue alone).
    pub status: String,
    /// Packages awaiting compilation.
    pub pending: u64,
    /// Packages currently building.
    pub building: u64,
    /// Seconds since the compiler last claimed a build (its DB-visible
    /// heartbeat). Only meaningful while a build is in flight.
    pub seconds_since_heartbeat: Option<u64>,
    /// RFC 3339 timestamp of the most recent successful build, if any.
    pub last_success_at: Option<String>,
    /// RFC 3339 timestamp of the most recent failed build, if any.
    pub last_failure_at: Option<String>,
}
