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

//! Task Execution Model
//!
//! This module defines domain structures for tracking task executions.
//! These are API-level types; backend-specific models handle database storage.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};

/// Represents a task execution record (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecution {
    pub id: UniversalUuid,
    pub workflow_execution_id: UniversalUuid,
    pub task_name: String,
    pub status: String,
    pub started_at: Option<UniversalTimestamp>,
    pub completed_at: Option<UniversalTimestamp>,
    pub attempt: i32,
    pub max_attempts: i32,
    pub error_details: Option<String>,
    pub trigger_rules: String,
    pub task_configuration: String,
    pub retry_at: Option<UniversalTimestamp>,
    pub last_error: Option<String>,
    pub recovery_attempts: i32,
    pub last_recovery_at: Option<UniversalTimestamp>,
    pub sub_status: Option<String>,
    pub claimed_by: Option<UniversalUuid>,
    pub heartbeat_at: Option<UniversalTimestamp>,
    /// The time from which the task waits for capacity (CLOACINA-T-1036).
    /// Set when the task becomes `Ready`; a retry gets its due time, so a
    /// retry in backoff has a value in the future. A dispatch that is put
    /// off for capacity does not move it. `None` once the task starts or
    /// ends.
    #[serde(default)]
    pub waiting_since: Option<UniversalTimestamp>,
    /// The time of the first dispatch of this attempt that an agent refused
    /// because it could not fetch the package artifact (CLOACINA-T-1064).
    /// The later refusals of the attempt keep it. `None` when the attempt had
    /// no such refusal; cleared when a retry is scheduled.
    #[serde(default)]
    pub artifact_fetch_wait_since: Option<UniversalTimestamp>,
    /// The agent pool the task runs in on a fleet (CLOACINA-T-1027). `None`
    /// means the pool `default`.
    #[serde(default)]
    pub pool: Option<String>,
    /// The task's CPU request in millicores (CLOACINA-T-1027). `None` = none.
    #[serde(default)]
    pub cpu_request_millis: Option<i64>,
    /// The task's memory request in bytes (CLOACINA-T-1027). `None` = none.
    #[serde(default)]
    pub memory_request_bytes: Option<i64>,
    /// The agent the fleet placed the task on, while the dispatch holds its
    /// claim (CLOACINA-T-1027). `None` when the task is on no agent.
    #[serde(default)]
    pub fleet_agent_id: Option<String>,
    /// The `dispatch_id` of the fleet dispatch the task is in, while the
    /// dispatch holds its claim (CLOACINA-T-1084). `None` when the task is in
    /// no fleet dispatch.
    #[serde(default)]
    pub fleet_dispatch_id: Option<String>,
    /// The `delivery_outbox` row (admin schema) of that dispatch's work
    /// packet (CLOACINA-T-1084).
    #[serde(default)]
    pub fleet_packet_row_id: Option<i64>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
    /// The priority of the task's workflow, as given: 0 (the highest) to 10
    /// (CLOACINA-T-1045). Waiting work is ordered by this, then by
    /// `task_priority`, then by `waiting_since`.
    #[serde(default = "default_priority")]
    pub workflow_priority: i32,
    /// The task's own priority, as given (CLOACINA-T-1045). A task that
    /// declares none has its workflow's.
    #[serde(default = "default_priority")]
    pub task_priority: i32,
    /// The runner whose claim the fleet dispatch was recorded under
    /// (CLOACINA-T-1136). A replica takes over the wait of a stale dispatch
    /// only while this equals `claimed_by`. `None` on a row of the previous
    /// release, or in no fleet dispatch.
    #[serde(default)]
    pub fleet_dispatch_claimed_by: Option<UniversalUuid>,
}

fn default_priority() -> i32 {
    i32::from(cloacina_api_types::DEFAULT_PRIORITY)
}

/// Structure for creating new task executions (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTaskExecution {
    pub workflow_execution_id: UniversalUuid,
    pub task_name: String,
    pub status: String,
    pub attempt: i32,
    pub max_attempts: i32,
    pub trigger_rules: String,
    pub task_configuration: String,
}
