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
    /// The workflow priority this run asks for: an integer from 0 (the
    /// highest) to 10 (CLOACINA-T-1045). With none, the run gets the
    /// priority the workflow declares, or 5. A priority above what the
    /// caller may set is not refused: the run gets the caller's limit, and
    /// the answer says so in `priority` and `priority_notice`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
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
    /// The workflow priority of the run: the one asked for and the one
    /// given (CLOACINA-T-1045). `null` from a server before priorities.
    #[serde(default)]
    pub priority: Option<crate::priority::PriorityGrant>,
    /// Set when the given priority is lower than the one asked for: one
    /// sentence that says what was asked, what was given and why.
    #[serde(default)]
    pub priority_notice: Option<String>,
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
    /// How the run was triggered (CLOACI-T-0776): `"manual"` for an operator run
    /// via the REST execute endpoint; `null` for cron/trigger/reactor-driven runs.
    /// The UI marks manual runs with a "manual" pill.
    #[serde(default)]
    pub trigger_origin: Option<String>,
    /// The workflow priority of the execution: asked and given
    /// (CLOACINA-T-1045). `null` for an execution from before priorities.
    #[serde(default)]
    pub priority: Option<crate::priority::PriorityGrant>,
}

/// `GET /tenants/{tenant_id}/executions/{id}` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ExecutionDetail {
    pub tenant_id: String,
    pub execution_id: String,
    pub status: String,
    /// The workflow priority of the execution: asked and given
    /// (CLOACINA-T-1045). `null` for an execution from before priorities.
    #[serde(default)]
    pub priority: Option<crate::priority::PriorityGrant>,
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
    /// Local name of the task this event is about, resolved from the event's
    /// `task_execution_id`. `null` for workflow-scoped events (e.g.
    /// `workflow_completed`) or when the task can't be resolved
    /// (CLOACI-I-0124 / WS-9).
    pub task_name: Option<String>,
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

/// One per-task row of an execution (CLOACI-I-0124 / WS-1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TaskExecutionDetail {
    /// Task execution UUID.
    pub id: String,
    /// Task identifier within the workflow.
    pub task_name: String,
    pub status: String,
    /// RFC 3339 timestamp; `null` until the task starts.
    pub started_at: Option<String>,
    /// RFC 3339 timestamp; `null` while still running.
    pub completed_at: Option<String>,
    pub attempt: i32,
    pub max_attempts: i32,
    /// Row-created timestamp (RFC 3339) — always present; a fallback "start"
    /// when `started_at` is null (some runner configs don't stamp it).
    pub created_at: String,
    /// Row-updated timestamp (RFC 3339) — always present; a fallback "end".
    pub updated_at: String,
    /// `sub_status` qualifier (e.g. deferral), when present.
    pub sub_status: Option<String>,
    /// Last error message for the most recent failed attempt, when present.
    pub last_error: Option<String>,
    /// Details of the task's end, when present: the traceback of a failed
    /// Python task, the reason of a skipped or abandoned task.
    pub error_details: Option<String>,
    /// Serialized size in bytes of the context the task returned; `null`
    /// when the task has saved no context.
    #[serde(default)]
    pub context_size_bytes: Option<i64>,
    /// URL of this attempt's logs in the operator's log stack, when the
    /// server has a log URL template (`--task-log-url-template`).
    pub log_url: Option<String>,
    /// The error of each failed attempt, oldest first; empty when no attempt
    /// failed (CLOACINA-T-1100). The last entry of a failed task is the
    /// error in `last_error` and `error_details`.
    #[serde(default)]
    pub attempt_errors: Vec<TaskAttemptErrorDetail>,
    /// Why the task waits, while the fleet puts it off (CLOACINA-T-1148);
    /// `null` when it does not wait, once it is placed on an agent or
    /// starts, and after it ends.
    #[serde(default)]
    pub wait_reason: Option<TaskWaitReason>,
}

/// Why a task waits (CLOACINA-T-1148): a short code and the same in words.
///
/// The codes are an open set: a client shows an unknown code with its
/// message. The codes of this release: `plugin_interface`, `no_agent`,
/// `platform`, `too_large`, `no_capacity`, `agent_refused`,
/// `artifact_fetch`, `dispatch_retry`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TaskWaitReason {
    /// A short code, for example `no_capacity`.
    pub code: String,
    /// The reason in words, with the tenant, the pool and the numbers.
    pub message: String,
}

impl TaskWaitReason {
    /// No agent of the task's tenant and pool loads the package's plugin
    /// interface version.
    pub const PLUGIN_INTERFACE: &'static str = "plugin_interface";
    /// No live agent of the task's tenant and pool.
    pub const NO_AGENT: &'static str = "no_agent";
    /// Agents of the pool, but none on a platform the package is built for.
    pub const PLATFORM: &'static str = "platform";
    /// The task requests more CPU or memory than any agent of the pool holds.
    pub const TOO_LARGE: &'static str = "too_large";
    /// Agents of the pool can run the task; none has a free slot or room for
    /// its requests now.
    pub const NO_CAPACITY: &'static str = "no_capacity";
    /// The agent the task was sent to refused it: full or shutting down.
    pub const AGENT_REFUSED: &'static str = "agent_refused";
    /// The agent could not fetch the package artifact.
    pub const ARTIFACT_FETCH: &'static str = "artifact_fetch";
    /// The server could not read the tasks in flight or write the placement;
    /// it tries again.
    pub const DISPATCH_RETRY: &'static str = "dispatch_retry";

    /// Every code of this release.
    pub const CODES: &'static [&'static str] = &[
        Self::PLUGIN_INTERFACE,
        Self::NO_AGENT,
        Self::PLATFORM,
        Self::TOO_LARGE,
        Self::NO_CAPACITY,
        Self::AGENT_REFUSED,
        Self::ARTIFACT_FETCH,
        Self::DISPATCH_RETRY,
    ];

    /// A reason with this code and message.
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

/// The error of one failed attempt of a task (CLOACINA-T-1100).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TaskAttemptErrorDetail {
    /// The attempt that failed, from 1.
    pub attempt: i32,
    /// The error message of the attempt.
    pub message: String,
    /// The details of the failure (a Python traceback), at most 16 KiB; a
    /// cut keeps the end.
    pub details: Option<String>,
    /// RFC 3339 timestamp of the failure.
    pub failed_at: String,
}

/// `GET /tenants/{tenant_id}/executions/{id}/tasks` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ExecutionTasksResponse {
    pub tenant_id: String,
    pub execution_id: String,
    pub tasks: Vec<TaskExecutionDetail>,
}
