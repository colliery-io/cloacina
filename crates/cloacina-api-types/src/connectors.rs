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

/// Body for `POST /tenants/{tenant_id}/connectors/check` and
/// `POST /tenants/{tenant_id}/connectors/discover` (CLOACINA-T-1002): one side
/// of a sync, as a sync definition gives it. A side names a compiled
/// connector by `provider` and `member`, or a connector definition by
/// `definition`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ConnectorProbeRequest {
    /// `source` or `destination`. Discover reads a source.
    pub side: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub member: Option<String>,
    #[serde(default)]
    pub definition: Option<String>,
    /// The connector's config, or a definition's user config.
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    /// The side's capability grants. A definition supplies its own.
    #[serde(default)]
    pub grants: Option<serde_json::Value>,
    /// The side's credential, a credential spec. A definition supplies its own.
    #[serde(default)]
    pub credential: Option<serde_json::Value>,
    /// The stored secret a definition's auth reads.
    #[serde(default)]
    pub secret: Option<String>,
}

/// What a connector's `check` said.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ConnectorCheckResponse {
    pub ok: bool,
    /// The connector's message, when it gives one.
    #[serde(default)]
    pub message: Option<String>,
}

/// A field of a discovered stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DiscoveredField {
    pub name: String,
    /// `Str`, `Integer`, `Float`, `Boolean`, `Timestamp` or `Json`.
    #[serde(rename = "type")]
    pub field_type: String,
    pub nullable: bool,
}

/// A stream a source offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DiscoveredStream {
    pub name: String,
    #[serde(default)]
    pub namespace: Option<String>,
    /// `FullRefresh`, `Incremental` or `Cdc`.
    pub sync_modes: Vec<String>,
    #[serde(default)]
    pub default_cursor_field: Option<String>,
    #[serde(default)]
    pub primary_key: Option<Vec<String>>,
    /// The stream's typed fields, when the source declares them. A compiled
    /// connector's are empty for now.
    #[serde(default)]
    pub fields: Vec<DiscoveredField>,
}

/// What a source's `discover` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ConnectorDiscoverResponse {
    pub streams: Vec<DiscoveredStream>,
}
