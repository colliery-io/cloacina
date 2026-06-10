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

//! Execution API types — trigger workflows and query execution status.

use serde::{Deserialize, Serialize};

/// Request body for `POST /tenants/{tenant_id}/workflows/{name}/execute`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ExecuteRequest {
    /// Optional JSON context to pass to the workflow.
    #[serde(default)]
    pub context: Option<serde_json::Value>,
}

/// `202 Accepted` body for a scheduled workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ExecuteResponse {
    /// UUID of the scheduled execution.
    pub execution_id: String,
    pub workflow_name: String,
    pub tenant_id: String,
    /// Always `"scheduled"` at accept time.
    pub status: String,
}

/// Query string for `GET /tenants/{tenant_id}/executions`
/// (CLOACI-T-0594 / API-02).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema, utoipa::IntoParams))]
#[cfg_attr(feature = "openapi", into_params(parameter_in = Query))]
pub struct ListExecutionsQuery {
    pub status: Option<String>,
    pub workflow: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// One row in the executions list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ExecutionSummary {
    /// Execution UUID.
    pub id: String,
    pub workflow_name: String,
    pub status: String,
    /// RFC 3339 timestamp.
    pub started_at: String,
    /// RFC 3339 timestamp; `null` while still running.
    pub completed_at: Option<String>,
}

/// `GET /tenants/{tenant_id}/executions/{id}` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ExecutionDetail {
    pub tenant_id: String,
    pub execution_id: String,
    pub status: String,
}

/// One row in the execution event log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ExecutionEvent {
    /// Event UUID.
    pub id: String,
    pub event_type: String,
    /// JSON-encoded additional data for the event; `null` when absent.
    pub event_data: Option<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
    pub sequence_num: i64,
}

/// `GET /tenants/{tenant_id}/executions/{id}/events` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ExecutionEventsResponse {
    pub tenant_id: String,
    pub execution_id: String,
    pub events: Vec<ExecutionEvent>,
}
