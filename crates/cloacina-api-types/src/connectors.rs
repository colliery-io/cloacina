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

//! The Sync connector catalog (COLLIERY-I-0634 / CLOACINA-T-0993).

use serde::{Deserialize, Serialize};

/// One connector a server can run: compiled (a provider package) or
/// declarative (a connector definition on the REST runtime).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ConnectorSummary {
    /// The name a sync names it by: `*_provider` for a compiled connector,
    /// `*_definition` for a declarative one.
    pub name: String,
    /// `compiled` or `declarative`.
    pub kind: String,
    /// The provider package that runs it.
    pub provider: String,
    pub version: Option<String>,
    /// `Source`, `Destination`.
    pub roles: Vec<String>,
    pub sync_modes: Vec<String>,
    /// A definition's streams, or a destination's objects. Empty for a
    /// compiled connector, whose streams are what `discover` finds.
    pub streams: Vec<String>,
}
