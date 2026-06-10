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
