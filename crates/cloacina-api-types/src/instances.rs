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

//! Named workflow-instance types (CLOACI-T-0894).
//!
//! An *instance* is a persistent, named binding of a workflow to a set of
//! parameter values, optionally on a cron schedule. I-0116 shipped the engine
//! for this — `schedules` rows carry `params` JSON and `instance_name`, and the
//! fire-time merge delivers bound params as top-level context keys — but
//! registration existed only on the embedded runner. These types back the
//! server surface.

use serde::{Deserialize, Serialize};

/// Body for `POST /tenants/{tenant_id}/workflows/{name}/instances`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateInstanceRequest {
    /// Instance name, unique per `(workflow_name, instance_name)` within the
    /// tenant.
    pub instance_name: String,
    /// Parameter values bound to this instance, validated against the
    /// workflow's declared `params(...)` slots.
    #[serde(default)]
    pub params: Option<serde_json::Value>,
    /// Cron expression. When omitted the instance is created **unscheduled** —
    /// a durable named param binding that never fires on its own.
    #[serde(default)]
    pub cron: Option<String>,
    /// IANA timezone for `cron`. Defaults to `UTC`.
    #[serde(default)]
    pub timezone: Option<String>,
    /// Whether the schedule is enabled on creation. Defaults to `true`.
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// One row in the instance list, and the body of a single-instance GET.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WorkflowInstanceSummary {
    /// Underlying schedule UUID.
    pub id: String,
    pub workflow_name: String,
    pub instance_name: String,
    /// Bound parameter values, as stored.
    pub params: Option<serde_json::Value>,
    pub cron_expression: Option<String>,
    pub timezone: Option<String>,
    pub enabled: bool,
    /// Whether the schedule is paused (distinct from `enabled`).
    #[serde(default)]
    pub paused: bool,
    /// RFC 3339 timestamp.
    pub next_run_at: Option<String>,
    /// RFC 3339 timestamp.
    pub last_run_at: Option<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
}

/// Response for a delete.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DeleteInstanceResponse {
    pub tenant_id: String,
    pub workflow_name: String,
    pub instance_name: String,
    pub deleted: bool,
}

/// Query string for the instance list (pagination, matching the trigger list).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema, utoipa::IntoParams))]
#[cfg_attr(feature = "openapi", into_params(parameter_in = Query))]
pub struct ListInstancesQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
