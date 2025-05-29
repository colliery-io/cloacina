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
//! This module defines the data structures and types used for tracking recovery events
//! in the pipeline system. Recovery events are used to record and manage various types
//! of recovery actions that occur during pipeline execution.
//!
//! The module provides structures for both querying existing recovery events and
//! creating new ones, along with an enum defining the different types of recovery
//! actions that can be performed.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use crate::database::universal_types::{UniversalUuid, UniversalTimestamp};

/// Represents a recovery event record in the database.
///
/// This struct is used for querying existing recovery events and includes all fields
/// from the recovery_events table. It implements serialization and deserialization
/// for API responses.
#[derive(Debug, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::recovery_events)]
#[cfg_attr(feature = "postgres", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "sqlite", diesel(check_for_backend(diesel::sqlite::Sqlite)))]
pub struct RecoveryEvent {
    /// Unique identifier for the recovery event
    pub id: UniversalUuid,
    /// Reference to the pipeline execution that triggered the recovery
    pub pipeline_execution_id: UniversalUuid,
    /// Optional reference to a specific task execution if the recovery is task-specific
    pub task_execution_id: Option<UniversalUuid>,
    /// Type of recovery action performed
    pub recovery_type: String,
    /// Timestamp when the recovery was executed
    pub recovered_at: UniversalTimestamp,
    /// Additional JSON string details about the recovery event
    pub details: Option<String>,
    /// Timestamp when the record was created
    pub created_at: UniversalTimestamp,
    /// Timestamp when the record was last updated
    pub updated_at: UniversalTimestamp,
}

/// Structure for creating new recovery event records.
///
/// This struct is used when inserting new recovery events into the database.
/// It contains only the fields that are required for creating a new record.
#[derive(Debug, Insertable)]
#[diesel(table_name = crate::database::schema::recovery_events)]
pub struct NewRecoveryEvent {
    /// Reference to the pipeline execution that triggered the recovery
    pub pipeline_execution_id: UniversalUuid,
    /// Optional reference to a specific task execution if the recovery is task-specific
    pub task_execution_id: Option<UniversalUuid>,
    /// Type of recovery action performed
    pub recovery_type: String,
    /// Additional JSON string details about the recovery event
    pub details: Option<String>,
}

/// Enumeration of possible recovery types in the system.
///
/// This enum defines all the different types of recovery actions that can be
/// performed during pipeline execution. Each variant represents a specific
/// recovery scenario.
#[derive(Debug)]
pub enum RecoveryType {
    /// Recovery triggered by resetting a task
    TaskReset,
    /// Recovery triggered by abandoning a task
    TaskAbandoned,
    /// Recovery triggered by pipeline failure
    PipelineFailed,
    /// Recovery triggered when a workflow is not available during recovery
    WorkflowUnavailable,
}

impl RecoveryType {
    /// Converts a RecoveryType variant to its string representation.
    ///
    /// Returns a static string slice that represents the recovery type
    /// in the database and API responses.
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
    /// Converts a RecoveryType variant into a String.
    ///
    /// This implementation allows for easy conversion of RecoveryType variants
    /// into strings for database storage and API responses.
    fn from(recovery_type: RecoveryType) -> Self {
        recovery_type.as_str().to_string()
    }
}
