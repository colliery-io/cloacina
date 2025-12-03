/*
 *  Copyright 2025 Colliery Software
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

//! Recovery Event Model
//!
//! This module defines domain structures and types for tracking recovery events.
//! These are API-level types; backend-specific models handle database storage.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};

/// Represents a recovery event record (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryEvent {
    pub id: UniversalUuid,
    pub pipeline_execution_id: UniversalUuid,
    pub task_execution_id: Option<UniversalUuid>,
    pub recovery_type: String,
    pub recovered_at: UniversalTimestamp,
    pub details: Option<String>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

/// Structure for creating new recovery event records (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRecoveryEvent {
    pub pipeline_execution_id: UniversalUuid,
    pub task_execution_id: Option<UniversalUuid>,
    pub recovery_type: String,
    pub details: Option<String>,
}

/// Enumeration of possible recovery types in the system.
#[derive(Debug, Clone)]
pub enum RecoveryType {
    TaskReset,
    TaskAbandoned,
    PipelineFailed,
    WorkflowUnavailable,
}

impl RecoveryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecoveryType::TaskReset => "task_reset",
            RecoveryType::TaskAbandoned => "task_abandoned",
            RecoveryType::PipelineFailed => "pipeline_failed",
            RecoveryType::WorkflowUnavailable => "workflow_unavailable",
        }
    }
}

impl From<RecoveryType> for String {
    fn from(recovery_type: RecoveryType) -> Self {
        recovery_type.as_str().to_string()
    }
}
