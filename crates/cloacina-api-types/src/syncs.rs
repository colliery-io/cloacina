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

//! The Sync read resource, `/tenants/{tenant_id}/syncs` (COLLIERY-I-0635 /
//! CLOACINA-T-1004). A sync is a workflow instance of the built-in `sync`
//! workflow; these views name syncs, not workflows. Writes stay on the
//! instances route.

use serde::{Deserialize, Serialize};

use crate::connectors::DiscoveredField;
use crate::executions::ExecutionSummary;

/// One side of a sync: a compiled connector, or a connector definition.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SyncSide {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub definition: Option<String>,
}

/// One pass of a sync: its segments' executions, shown as one.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SyncPass {
    pub pass_id: String,
    /// `Running`, `Completed` or `Failed`.
    pub status: String,
    /// RFC 3339: the first segment's start.
    pub started_at: String,
    /// RFC 3339: the last segment's end; `null` while running.
    #[serde(default)]
    pub completed_at: Option<String>,
    pub segments: usize,
    /// The cursor the pass's last segment ended on.
    #[serde(default)]
    pub cursor: Option<String>,
    /// The segments, oldest first.
    pub executions: Vec<ExecutionSummary>,
}

/// One sync in the list, and the head of its detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SyncSummary {
    /// The sync definition's name: the workflow instance's.
    pub name: String,
    /// The name its cursor and dead-letters are kept under.
    pub sync_name: String,
    pub source: SyncSide,
    pub destination: SyncSide,
    pub stream: String,
    /// `FullRefresh`, `Incremental` or `Cdc`.
    pub sync_mode: String,
    /// `Append`, `Upsert` or `Overwrite`.
    pub write_mode: String,
    #[serde(default)]
    pub cron: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub next_run_at: Option<String>,
    /// The latest pass, if any has run.
    #[serde(default)]
    pub last_pass: Option<SyncPass>,
    /// Whether a stream's schema is broken.
    pub schema_broken: bool,
    pub dead_letters: i64,
}

/// A sync and its params, as stored: the params name secrets, never hold them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SyncDetail {
    #[serde(flatten)]
    pub summary: SyncSummary,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

/// A record a sync set aside, with the reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SyncDeadLetter {
    pub id: String,
    pub stream: String,
    pub record: String,
    pub reason: String,
    pub created_at: String,
    #[serde(default)]
    pub execution_id: Option<String>,
}

/// A stream of a sync: its committed cursor and its captured schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SyncStream {
    pub stream: String,
    #[serde(default)]
    pub partition: String,
    #[serde(default)]
    pub cursor: Option<String>,
    pub exhausted: bool,
    /// RFC 3339: when the cursor last moved.
    #[serde(default)]
    pub updated_at: Option<String>,
    /// The captured schema's fields; empty before the first write, or after
    /// an accept.
    pub fields: Vec<DiscoveredField>,
    /// Why the schema is broken; `null` when it stands.
    #[serde(default)]
    pub broken: Option<String>,
}

/// Query string for the pass and dead-letter lists.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema, utoipa::IntoParams))]
#[cfg_attr(feature = "openapi", into_params(parameter_in = Query))]
pub struct SyncPageQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// `POST /tenants/{tenant_id}/syncs/{name}/schema/accept` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SchemaAcceptedResponse {
    pub name: String,
    pub stream: String,
    pub accepted: bool,
}
