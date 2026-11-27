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

//! Backfill types (CLOACINA-T-1020).
//!
//! A *backfill* is a bounded second schedule for one workflow: it runs the
//! workflow once for each fire time of a cron expression from `start` to
//! `end`, oldest first, with its own limit on active runs. Each fire time is
//! one interval: it ends at the fire time and starts at the fire time before
//! it. Each run gets `interval_start`, `interval_end` and `backfill_id` in
//! its context.

use serde::{Deserialize, Serialize};

/// Body for `POST /tenants/{tenant_id}/workflows/{name}/backfills`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateBackfillRequest {
    /// Cron expression that gives the intervals. When omitted: the
    /// expression and timezone of the cron schedule of the workflow, when it
    /// has exactly one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cron: Option<String>,
    /// IANA timezone for `cron`. Defaults to `UTC` when `cron` is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    /// Start of the range, an RFC 3339 timestamp. The backfill takes each
    /// fire time `F` of the expression with `start <= F <= end`.
    pub start: String,
    /// End of the range, an RFC 3339 timestamp. For one interval, set
    /// `start` and `end` to its fire time.
    pub end: String,
    /// The most runs of this backfill that may be active at once, 1 or more.
    /// Defaults to 1. The limit is apart from the limit of any other
    /// schedule of the workflow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "openapi", schema(minimum = 1))]
    pub max_active_runs: Option<u32>,
    /// Parameter values for each run, validated against the workflow's
    /// declared `params(...)` slots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// Body for `POST /tenants/{tenant_id}/workflows/{name}/backfills/{id}/retry`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RetryBackfillRequest {
    /// The limit on active runs of the new backfill. Defaults to the limit
    /// of the backfill that is retried.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "openapi", schema(minimum = 1))]
    pub max_active_runs: Option<u32>,
}

/// How far a backfill is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct BackfillProgress {
    /// Intervals in the backfill.
    pub total: u64,
    /// Intervals whose run ended `Completed`. Counted from the run history:
    /// a run that the retention job deleted is not counted.
    pub completed: u64,
    /// Intervals whose run ended `Failed` or `Cancelled`, or could not be
    /// started. Counted from the run history, like `completed`.
    pub failed: u64,
    /// Intervals whose run has not ended.
    pub running: u64,
    /// Intervals with no run yet.
    pub remaining: u64,
    /// The position: the end of the last interval that got a run. RFC 3339.
    /// Absent before the first run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_interval_end: Option<String>,
    /// The end of the next interval to get a run. RFC 3339. Absent when no
    /// interval remains.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_interval_end: Option<String>,
}

/// One backfill: a row in the list, and the response of create, cancel and
/// retry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct BackfillSummary {
    /// Backfill id (a UUID). Each run has it in its context as `backfill_id`.
    pub id: String,
    pub workflow_name: String,
    pub cron_expression: String,
    pub timezone: String,
    /// Start of the range, RFC 3339.
    pub start: String,
    /// End of the range, RFC 3339.
    pub end: String,
    /// The limit on active runs of this backfill.
    pub max_active_runs: u32,
    /// Parameter values for each run, as stored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    /// `running`, `paused`, `completed` or `cancelled`. `completed` means
    /// each interval got a run and each run ended; see `progress.failed`
    /// for the runs that failed.
    pub status: String,
    pub progress: BackfillProgress,
    /// The id of the backfill whose failed intervals this one runs again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_of: Option<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
}

/// One failed interval of a backfill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct BackfillFailedInterval {
    /// RFC 3339 timestamp.
    pub interval_start: String,
    /// RFC 3339 timestamp: the fire time of the interval.
    pub interval_end: String,
    /// The workflow execution of the interval. Absent when no run started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<String>,
    /// `Failed`, `Cancelled`, or `NotStarted` when no run could be started.
    pub status: String,
}

/// Response of `GET /tenants/{tenant_id}/workflows/{name}/backfills/{id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct BackfillDetail {
    pub backfill: BackfillSummary,
    /// The failed intervals, oldest first, at most 1000.
    pub failed_intervals: Vec<BackfillFailedInterval>,
    /// True when the backfill has more failed intervals than the list holds.
    pub failed_intervals_truncated: bool,
}

/// Query string for the backfill list.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema, utoipa::IntoParams))]
#[cfg_attr(feature = "openapi", into_params(parameter_in = Query))]
pub struct ListBackfillsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
