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

//! Workflow Execution Models
//!
//! This module defines domain structures for tracking workflow executions.
//! These are API-level types; backend-specific models handle database storage.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};

/// Represents a workflow execution record (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionRecord {
    pub id: UniversalUuid,
    pub workflow_name: String,
    pub workflow_version: String,
    pub status: String,
    pub context_id: Option<UniversalUuid>,
    pub started_at: UniversalTimestamp,
    pub completed_at: Option<UniversalTimestamp>,
    pub error_details: Option<String>,
    pub recovery_attempts: i32,
    pub last_recovery_at: Option<UniversalTimestamp>,
    pub paused_at: Option<UniversalTimestamp>,
    pub pause_reason: Option<String>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
    /// How this run was triggered (CLOACI-T-0776). `Some("manual")` for an
    /// operator run via the REST execute endpoint; `None` for cron/trigger/
    /// reactor-driven runs (and rows created before the column existed).
    pub trigger_origin: Option<String>,
    /// The workflow priority that was asked for (CLOACINA-T-1045): by the
    /// run request, the schedule, or the workflow's own declaration. `None`
    /// on a row from before the column.
    #[serde(default)]
    pub priority_asked: Option<i32>,
    /// The workflow priority the execution runs at: the asked one, held to
    /// the limit of who asked. 0 is the highest, 10 the lowest.
    #[serde(default = "default_priority")]
    pub priority: i32,
}

fn default_priority() -> i32 {
    i32::from(cloacina_api_types::DEFAULT_PRIORITY)
}

/// Structure for creating new workflow executions (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorkflowExecution {
    pub workflow_name: String,
    pub workflow_version: String,
    pub status: String,
    pub context_id: Option<UniversalUuid>,
}
