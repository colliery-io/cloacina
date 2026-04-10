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
    pub pipeline_name: String,
    pub pipeline_version: String,
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
}

/// Structure for creating new workflow executions (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorkflowExecution {
    pub pipeline_name: String,
    pub pipeline_version: String,
    pub status: String,
    pub context_id: Option<UniversalUuid>,
}
