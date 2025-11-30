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

//! SQLite-specific database models
//!
//! This module contains Diesel model definitions that use SQLite-compatible types.
//! UUIDs are stored as BLOB (Vec<u8>), timestamps as TEXT (RFC3339 strings),
//! and booleans as INTEGER (0/1).
//!
//! These models are used internally by the SQLite DAL implementation and
//! converted to/from domain types at the DAL boundary.

use crate::database::schema::sqlite::*;
use diesel::prelude::*;

// ============================================================================
// Context Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = contexts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SqliteDbContext {
    pub id: Vec<u8>,
    pub value: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = contexts)]
pub struct NewSqliteDbContext {
    pub id: Vec<u8>,
    pub value: String,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// Pipeline Execution Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = pipeline_executions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SqlitePipelineExecution {
    pub id: Vec<u8>,
    pub pipeline_name: String,
    pub pipeline_version: String,
    pub status: String,
    pub context_id: Option<Vec<u8>>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error_details: Option<String>,
    pub recovery_attempts: i32,
    pub last_recovery_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = pipeline_executions)]
pub struct NewSqlitePipelineExecution {
    pub id: Vec<u8>,
    pub pipeline_name: String,
    pub pipeline_version: String,
    pub status: String,
    pub context_id: Option<Vec<u8>>,
    pub started_at: String,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// Task Execution Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = task_executions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SqliteTaskExecution {
    pub id: Vec<u8>,
    pub pipeline_execution_id: Vec<u8>,
    pub task_name: String,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub attempt: i32,
    pub max_attempts: i32,
    pub error_details: Option<String>,
    pub trigger_rules: String,
    pub task_configuration: String,
    pub retry_at: Option<String>,
    pub last_error: Option<String>,
    pub recovery_attempts: i32,
    pub last_recovery_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = task_executions)]
pub struct NewSqliteTaskExecution {
    pub id: Vec<u8>,
    pub pipeline_execution_id: Vec<u8>,
    pub task_name: String,
    pub status: String,
    pub attempt: i32,
    pub max_attempts: i32,
    pub trigger_rules: String,
    pub task_configuration: String,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// Task Execution Metadata Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = task_execution_metadata)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SqliteTaskExecutionMetadata {
    pub id: Vec<u8>,
    pub task_execution_id: Vec<u8>,
    pub pipeline_execution_id: Vec<u8>,
    pub task_name: String,
    pub context_id: Option<Vec<u8>>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = task_execution_metadata)]
pub struct NewSqliteTaskExecutionMetadata {
    pub id: Vec<u8>,
    pub task_execution_id: Vec<u8>,
    pub pipeline_execution_id: Vec<u8>,
    pub task_name: String,
    pub context_id: Option<Vec<u8>>,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// Recovery Event Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = recovery_events)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SqliteRecoveryEvent {
    pub id: Vec<u8>,
    pub pipeline_execution_id: Vec<u8>,
    pub task_execution_id: Option<Vec<u8>>,
    pub recovery_type: String,
    pub recovered_at: String,
    pub details: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = recovery_events)]
pub struct NewSqliteRecoveryEvent {
    pub id: Vec<u8>,
    pub pipeline_execution_id: Vec<u8>,
    pub task_execution_id: Option<Vec<u8>>,
    pub recovery_type: String,
    pub recovered_at: String,
    pub details: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// Cron Schedule Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = cron_schedules)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SqliteCronSchedule {
    pub id: Vec<u8>,
    pub workflow_name: String,
    pub cron_expression: String,
    pub timezone: String,
    pub enabled: i32,
    pub catchup_policy: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub next_run_at: String,
    pub last_run_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = cron_schedules)]
pub struct NewSqliteCronSchedule {
    pub id: Vec<u8>,
    pub workflow_name: String,
    pub cron_expression: String,
    pub timezone: String,
    pub enabled: i32,
    pub catchup_policy: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub next_run_at: String,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// Cron Execution Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = cron_executions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SqliteCronExecution {
    pub id: Vec<u8>,
    pub schedule_id: Vec<u8>,
    pub pipeline_execution_id: Option<Vec<u8>>,
    pub scheduled_time: String,
    pub claimed_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = cron_executions)]
pub struct NewSqliteCronExecution {
    pub id: Vec<u8>,
    pub schedule_id: Vec<u8>,
    pub pipeline_execution_id: Option<Vec<u8>>,
    pub scheduled_time: String,
    pub claimed_at: String,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// Workflow Registry Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = workflow_registry)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SqliteWorkflowRegistryEntry {
    pub id: Vec<u8>,
    pub created_at: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = workflow_registry)]
pub struct NewSqliteWorkflowRegistryEntry {
    pub id: Vec<u8>,
    pub created_at: String,
    pub data: Vec<u8>,
}

// ============================================================================
// Workflow Packages Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = workflow_packages)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SqliteWorkflowPackage {
    pub id: Vec<u8>,
    pub registry_id: Vec<u8>,
    pub package_name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = workflow_packages)]
pub struct NewSqliteWorkflowPackage {
    pub id: Vec<u8>,
    pub registry_id: Vec<u8>,
    pub package_name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// Conversion Utilities
// ============================================================================

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Convert a UUID to SQLite BLOB format (Vec<u8>)
pub fn uuid_to_blob(uuid: &Uuid) -> Vec<u8> {
    uuid.as_bytes().to_vec()
}

/// Convert SQLite BLOB to UUID
pub fn blob_to_uuid(blob: &[u8]) -> Result<Uuid, uuid::Error> {
    Uuid::from_slice(blob)
}

/// Convert DateTime<Utc> to RFC3339 string for SQLite storage
pub fn datetime_to_string(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

/// Parse RFC3339 string from SQLite to DateTime<Utc>
pub fn string_to_datetime(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc))
}

/// Current timestamp as RFC3339 string
pub fn current_timestamp_string() -> String {
    Utc::now().to_rfc3339()
}

// ============================================================================
// Conversion Implementations: SQLite models <-> Domain models
// ============================================================================

use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
use crate::models::context::DbContext;
use crate::models::cron_execution::CronExecution;
use crate::models::cron_schedule::CronSchedule;
use crate::models::pipeline_execution::PipelineExecution;
use crate::models::recovery_event::RecoveryEvent;
use crate::models::task_execution::TaskExecution;
use crate::models::task_execution_metadata::TaskExecutionMetadata;
use crate::models::workflow_packages::WorkflowPackage;
use crate::models::workflow_registry::WorkflowRegistryEntry;

impl From<SqliteDbContext> for DbContext {
    fn from(s: SqliteDbContext) -> Self {
        DbContext {
            id: UniversalUuid(blob_to_uuid(&s.id).expect("Invalid UUID in database")),
            value: s.value,
            created_at: UniversalTimestamp(
                string_to_datetime(&s.created_at).expect("Invalid timestamp in database"),
            ),
            updated_at: UniversalTimestamp(
                string_to_datetime(&s.updated_at).expect("Invalid timestamp in database"),
            ),
        }
    }
}

impl From<SqlitePipelineExecution> for PipelineExecution {
    fn from(s: SqlitePipelineExecution) -> Self {
        PipelineExecution {
            id: UniversalUuid(blob_to_uuid(&s.id).expect("Invalid UUID in database")),
            pipeline_name: s.pipeline_name,
            pipeline_version: s.pipeline_version,
            status: s.status,
            context_id: s
                .context_id
                .map(|b| UniversalUuid(blob_to_uuid(&b).expect("Invalid UUID in database"))),
            started_at: UniversalTimestamp(
                string_to_datetime(&s.started_at).expect("Invalid timestamp in database"),
            ),
            completed_at: s
                .completed_at
                .map(|ts| UniversalTimestamp(string_to_datetime(&ts).expect("Invalid timestamp"))),
            error_details: s.error_details,
            recovery_attempts: s.recovery_attempts,
            last_recovery_at: s
                .last_recovery_at
                .map(|ts| UniversalTimestamp(string_to_datetime(&ts).expect("Invalid timestamp"))),
            created_at: UniversalTimestamp(
                string_to_datetime(&s.created_at).expect("Invalid timestamp in database"),
            ),
            updated_at: UniversalTimestamp(
                string_to_datetime(&s.updated_at).expect("Invalid timestamp in database"),
            ),
        }
    }
}

impl From<SqliteTaskExecution> for TaskExecution {
    fn from(s: SqliteTaskExecution) -> Self {
        TaskExecution {
            id: UniversalUuid(blob_to_uuid(&s.id).expect("Invalid UUID in database")),
            pipeline_execution_id: UniversalUuid(
                blob_to_uuid(&s.pipeline_execution_id).expect("Invalid UUID in database"),
            ),
            task_name: s.task_name,
            status: s.status,
            started_at: s
                .started_at
                .map(|ts| UniversalTimestamp(string_to_datetime(&ts).expect("Invalid timestamp"))),
            completed_at: s
                .completed_at
                .map(|ts| UniversalTimestamp(string_to_datetime(&ts).expect("Invalid timestamp"))),
            attempt: s.attempt,
            max_attempts: s.max_attempts,
            error_details: s.error_details,
            trigger_rules: s.trigger_rules,
            task_configuration: s.task_configuration,
            retry_at: s
                .retry_at
                .map(|ts| UniversalTimestamp(string_to_datetime(&ts).expect("Invalid timestamp"))),
            last_error: s.last_error,
            recovery_attempts: s.recovery_attempts,
            last_recovery_at: s
                .last_recovery_at
                .map(|ts| UniversalTimestamp(string_to_datetime(&ts).expect("Invalid timestamp"))),
            created_at: UniversalTimestamp(
                string_to_datetime(&s.created_at).expect("Invalid timestamp in database"),
            ),
            updated_at: UniversalTimestamp(
                string_to_datetime(&s.updated_at).expect("Invalid timestamp in database"),
            ),
        }
    }
}

impl From<SqliteTaskExecutionMetadata> for TaskExecutionMetadata {
    fn from(s: SqliteTaskExecutionMetadata) -> Self {
        TaskExecutionMetadata {
            id: UniversalUuid(blob_to_uuid(&s.id).expect("Invalid UUID in database")),
            task_execution_id: UniversalUuid(
                blob_to_uuid(&s.task_execution_id).expect("Invalid UUID in database"),
            ),
            pipeline_execution_id: UniversalUuid(
                blob_to_uuid(&s.pipeline_execution_id).expect("Invalid UUID in database"),
            ),
            task_name: s.task_name,
            context_id: s
                .context_id
                .map(|b| UniversalUuid(blob_to_uuid(&b).expect("Invalid UUID in database"))),
            created_at: UniversalTimestamp(
                string_to_datetime(&s.created_at).expect("Invalid timestamp in database"),
            ),
            updated_at: UniversalTimestamp(
                string_to_datetime(&s.updated_at).expect("Invalid timestamp in database"),
            ),
        }
    }
}

impl From<SqliteRecoveryEvent> for RecoveryEvent {
    fn from(s: SqliteRecoveryEvent) -> Self {
        RecoveryEvent {
            id: UniversalUuid(blob_to_uuid(&s.id).expect("Invalid UUID in database")),
            pipeline_execution_id: UniversalUuid(
                blob_to_uuid(&s.pipeline_execution_id).expect("Invalid UUID in database"),
            ),
            task_execution_id: s
                .task_execution_id
                .map(|b| UniversalUuid(blob_to_uuid(&b).expect("Invalid UUID in database"))),
            recovery_type: s.recovery_type,
            recovered_at: UniversalTimestamp(
                string_to_datetime(&s.recovered_at).expect("Invalid timestamp in database"),
            ),
            details: s.details,
            created_at: UniversalTimestamp(
                string_to_datetime(&s.created_at).expect("Invalid timestamp in database"),
            ),
            updated_at: UniversalTimestamp(
                string_to_datetime(&s.updated_at).expect("Invalid timestamp in database"),
            ),
        }
    }
}

impl From<SqliteCronSchedule> for CronSchedule {
    fn from(s: SqliteCronSchedule) -> Self {
        CronSchedule {
            id: UniversalUuid(blob_to_uuid(&s.id).expect("Invalid UUID in database")),
            workflow_name: s.workflow_name,
            cron_expression: s.cron_expression,
            timezone: s.timezone,
            enabled: UniversalBool(s.enabled != 0),
            catchup_policy: s.catchup_policy,
            start_date: s
                .start_date
                .map(|ts| UniversalTimestamp(string_to_datetime(&ts).expect("Invalid timestamp"))),
            end_date: s
                .end_date
                .map(|ts| UniversalTimestamp(string_to_datetime(&ts).expect("Invalid timestamp"))),
            next_run_at: UniversalTimestamp(
                string_to_datetime(&s.next_run_at).expect("Invalid timestamp in database"),
            ),
            last_run_at: s
                .last_run_at
                .map(|ts| UniversalTimestamp(string_to_datetime(&ts).expect("Invalid timestamp"))),
            created_at: UniversalTimestamp(
                string_to_datetime(&s.created_at).expect("Invalid timestamp in database"),
            ),
            updated_at: UniversalTimestamp(
                string_to_datetime(&s.updated_at).expect("Invalid timestamp in database"),
            ),
        }
    }
}

impl From<SqliteCronExecution> for CronExecution {
    fn from(s: SqliteCronExecution) -> Self {
        CronExecution {
            id: UniversalUuid(blob_to_uuid(&s.id).expect("Invalid UUID in database")),
            schedule_id: UniversalUuid(
                blob_to_uuid(&s.schedule_id).expect("Invalid UUID in database"),
            ),
            pipeline_execution_id: s
                .pipeline_execution_id
                .map(|b| UniversalUuid(blob_to_uuid(&b).expect("Invalid UUID in database"))),
            scheduled_time: UniversalTimestamp(
                string_to_datetime(&s.scheduled_time).expect("Invalid timestamp in database"),
            ),
            claimed_at: UniversalTimestamp(
                string_to_datetime(&s.claimed_at).expect("Invalid timestamp in database"),
            ),
            created_at: UniversalTimestamp(
                string_to_datetime(&s.created_at).expect("Invalid timestamp in database"),
            ),
            updated_at: UniversalTimestamp(
                string_to_datetime(&s.updated_at).expect("Invalid timestamp in database"),
            ),
        }
    }
}

impl From<SqliteWorkflowRegistryEntry> for WorkflowRegistryEntry {
    fn from(s: SqliteWorkflowRegistryEntry) -> Self {
        WorkflowRegistryEntry {
            id: UniversalUuid(blob_to_uuid(&s.id).expect("Invalid UUID in database")),
            created_at: UniversalTimestamp(
                string_to_datetime(&s.created_at).expect("Invalid timestamp in database"),
            ),
            data: s.data,
        }
    }
}

impl From<SqliteWorkflowPackage> for WorkflowPackage {
    fn from(s: SqliteWorkflowPackage) -> Self {
        WorkflowPackage {
            id: UniversalUuid(blob_to_uuid(&s.id).expect("Invalid UUID in database")),
            registry_id: UniversalUuid(
                blob_to_uuid(&s.registry_id).expect("Invalid UUID in database"),
            ),
            package_name: s.package_name,
            version: s.version,
            description: s.description,
            author: s.author,
            metadata: s.metadata,
            created_at: UniversalTimestamp(
                string_to_datetime(&s.created_at).expect("Invalid timestamp in database"),
            ),
            updated_at: UniversalTimestamp(
                string_to_datetime(&s.updated_at).expect("Invalid timestamp in database"),
            ),
        }
    }
}
