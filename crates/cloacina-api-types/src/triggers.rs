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

//! Trigger schedule types — read-only listing of cron and trigger schedules.

use serde::{Deserialize, Serialize};

/// Query string for `GET /tenants/{tenant_id}/triggers`
/// (CLOACI-T-0596 / API-10 pagination).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema, utoipa::IntoParams))]
#[cfg_attr(feature = "openapi", into_params(parameter_in = Query))]
pub struct ListTriggersQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// One row in the trigger list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TriggerScheduleSummary {
    /// Schedule UUID.
    pub id: String,
    /// `cron` or `trigger`.
    pub schedule_type: String,
    pub workflow_name: String,
    pub enabled: bool,
    pub cron_expression: Option<String>,
    pub trigger_name: Option<String>,
    pub poll_interval_ms: Option<i64>,
    /// RFC 3339 timestamp.
    pub next_run_at: Option<String>,
    /// RFC 3339 timestamp.
    pub last_run_at: Option<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// Whether the schedule is paused (CLOACI-T-0749). A paused schedule is not
    /// fired by the scheduler until resumed; distinct from `enabled`.
    #[serde(default)]
    pub paused: bool,
    /// RFC 3339 timestamp of when it was paused, if paused.
    #[serde(default)]
    pub paused_at: Option<String>,
}

/// Schedule fields in the trigger detail response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TriggerScheduleInfo {
    /// Schedule UUID.
    pub id: String,
    /// `cron` or `trigger`.
    pub schedule_type: String,
    pub workflow_name: String,
    pub enabled: bool,
    pub cron_expression: Option<String>,
    pub trigger_name: Option<String>,
    /// Poll interval in milliseconds for `trigger`-type schedules (the
    /// custom-poll cadence). `None` for cron schedules. Lets the Triggers
    /// detail view show "polls every Ns" (CLOACI-I-0124 / WS-6).
    pub poll_interval_ms: Option<i64>,
    /// Whether the schedule is paused (CLOACI-T-0749).
    #[serde(default)]
    pub paused: bool,
    /// RFC 3339 timestamp of when it was paused, if paused.
    #[serde(default)]
    pub paused_at: Option<String>,
}

/// `POST /tenants/{tenant_id}/triggers/{name}/pause` and `/resume` response
/// (CLOACI-T-0749).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TriggerPauseResponse {
    pub tenant_id: String,
    /// Schedule UUID.
    pub id: String,
    /// The name the schedule was addressed by (trigger or workflow name).
    pub name: String,
    /// `"paused"` or `"resumed"`.
    pub status: String,
    /// Current paused state after the operation.
    pub paused: bool,
}

/// `POST /tenants/{tenant_id}/triggers/{name}/fire` request (CLOACI-T-0777).
/// Manually push an event to a trigger; it fans out to every subscribed workflow.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct FireTriggerRequest {
    /// Optional typed event merged into each fired workflow's context, validated
    /// against the trigger's declared params (CLOACI-T-0777 P2). Omit to fire with
    /// just the trigger metadata.
    #[serde(default)]
    pub event: Option<serde_json::Value>,
}

/// `POST /tenants/{tenant_id}/triggers/{name}/fire` response (CLOACI-T-0777).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct FireTriggerResponse {
    pub tenant_id: String,
    /// The trigger name fired.
    pub trigger: String,
    /// How many subscribed workflows were fired (the fan-out count).
    pub fired: u32,
    /// The started executions: `(workflow_name, execution_id)`.
    pub executions: Vec<FiredExecution>,
}

/// One workflow fired by a manual trigger fire (CLOACI-T-0777).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct FiredExecution {
    pub workflow_name: String,
    pub execution_id: String,
}

/// One row in `recent_executions` of the trigger detail response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TriggerExecution {
    /// Schedule-execution UUID.
    pub id: String,
    /// RFC 3339 timestamp.
    pub scheduled_time: Option<String>,
    /// RFC 3339 timestamp.
    pub started_at: String,
    /// RFC 3339 timestamp.
    pub completed_at: Option<String>,
}

/// `GET /tenants/{tenant_id}/triggers/{name}` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TriggerDetailResponse {
    pub tenant_id: String,
    pub schedule: TriggerScheduleInfo,
    pub recent_executions: Vec<TriggerExecution>,
}
