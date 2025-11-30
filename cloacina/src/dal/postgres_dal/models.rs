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

//! PostgreSQL-specific database models
//!
//! This module contains Diesel model definitions that use native PostgreSQL types.
//! These models are used internally by the PostgreSQL DAL implementation and
//! converted to/from domain types at the DAL boundary.

use crate::database::schema::postgres::*;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use uuid::Uuid;

// ============================================================================
// Context Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = contexts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PgDbContext {
    pub id: Uuid,
    pub value: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = contexts)]
pub struct NewPgDbContext {
    pub value: String,
}

// ============================================================================
// Pipeline Execution Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = pipeline_executions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PgPipelineExecution {
    pub id: Uuid,
    pub pipeline_name: String,
    pub pipeline_version: String,
    pub status: String,
    pub context_id: Option<Uuid>,
    pub started_at: NaiveDateTime,
    pub completed_at: Option<NaiveDateTime>,
    pub error_details: Option<String>,
    pub recovery_attempts: i32,
    pub last_recovery_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = pipeline_executions)]
pub struct NewPgPipelineExecution {
    pub pipeline_name: String,
    pub pipeline_version: String,
    pub status: String,
    pub context_id: Option<Uuid>,
}

// ============================================================================
// Task Execution Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = task_executions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PgTaskExecution {
    pub id: Uuid,
    pub pipeline_execution_id: Uuid,
    pub task_name: String,
    pub status: String,
    pub started_at: Option<NaiveDateTime>,
    pub completed_at: Option<NaiveDateTime>,
    pub attempt: i32,
    pub max_attempts: i32,
    pub error_details: Option<String>,
    pub trigger_rules: String,
    pub task_configuration: String,
    pub retry_at: Option<NaiveDateTime>,
    pub last_error: Option<String>,
    pub recovery_attempts: i32,
    pub last_recovery_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = task_executions)]
pub struct NewPgTaskExecution {
    pub pipeline_execution_id: Uuid,
    pub task_name: String,
    pub status: String,
    pub attempt: i32,
    pub max_attempts: i32,
    pub trigger_rules: String,
    pub task_configuration: String,
}

// ============================================================================
// Task Execution Metadata Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = task_execution_metadata)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PgTaskExecutionMetadata {
    pub id: Uuid,
    pub task_execution_id: Uuid,
    pub pipeline_execution_id: Uuid,
    pub task_name: String,
    pub context_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = task_execution_metadata)]
pub struct NewPgTaskExecutionMetadata {
    pub task_execution_id: Uuid,
    pub pipeline_execution_id: Uuid,
    pub task_name: String,
    pub context_id: Option<Uuid>,
}

// ============================================================================
// Recovery Event Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = recovery_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PgRecoveryEvent {
    pub id: Uuid,
    pub pipeline_execution_id: Uuid,
    pub task_execution_id: Option<Uuid>,
    pub recovery_type: String,
    pub recovered_at: NaiveDateTime,
    pub details: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = recovery_events)]
pub struct NewPgRecoveryEvent {
    pub pipeline_execution_id: Uuid,
    pub task_execution_id: Option<Uuid>,
    pub recovery_type: String,
    pub details: Option<String>,
}

// ============================================================================
// Cron Schedule Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = cron_schedules)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PgCronSchedule {
    pub id: Uuid,
    pub workflow_name: String,
    pub cron_expression: String,
    pub timezone: String,
    pub enabled: bool,
    pub catchup_policy: String,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub next_run_at: NaiveDateTime,
    pub last_run_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = cron_schedules)]
pub struct NewPgCronSchedule {
    pub workflow_name: String,
    pub cron_expression: String,
    pub timezone: String,
    pub enabled: bool,
    pub catchup_policy: String,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub next_run_at: NaiveDateTime,
}

// ============================================================================
// Cron Execution Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = cron_executions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PgCronExecution {
    pub id: Uuid,
    pub schedule_id: Uuid,
    pub pipeline_execution_id: Option<Uuid>,
    pub scheduled_time: NaiveDateTime,
    pub claimed_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = cron_executions)]
pub struct NewPgCronExecution {
    pub schedule_id: Uuid,
    pub pipeline_execution_id: Option<Uuid>,
    pub scheduled_time: NaiveDateTime,
    pub claimed_at: NaiveDateTime,
}

// ============================================================================
// Workflow Registry Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = workflow_registry)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PgWorkflowRegistryEntry {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub data: Vec<u8>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = workflow_registry)]
pub struct NewPgWorkflowRegistryEntry {
    pub data: Vec<u8>,
}

// ============================================================================
// Workflow Packages Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = workflow_packages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PgWorkflowPackage {
    pub id: Uuid,
    pub registry_id: Uuid,
    pub package_name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub metadata: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = workflow_packages)]
pub struct NewPgWorkflowPackage {
    pub registry_id: Uuid,
    pub package_name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub metadata: String,
}

// ============================================================================
// Conversion Implementations: PostgreSQL models <-> Domain models
// ============================================================================

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::models::context::DbContext;
use crate::models::cron_execution::CronExecution;
use crate::models::cron_schedule::CronSchedule;
use crate::models::pipeline_execution::PipelineExecution;
use crate::models::recovery_event::RecoveryEvent;
use crate::models::task_execution::TaskExecution;
use crate::models::task_execution_metadata::TaskExecutionMetadata;
use crate::models::workflow_packages::WorkflowPackage;
use crate::models::workflow_registry::WorkflowRegistryEntry;
use chrono::{TimeZone, Utc};

impl From<PgDbContext> for DbContext {
    fn from(pg: PgDbContext) -> Self {
        DbContext {
            id: UniversalUuid(pg.id),
            value: pg.value,
            created_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.created_at)),
            updated_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.updated_at)),
        }
    }
}

impl From<PgPipelineExecution> for PipelineExecution {
    fn from(pg: PgPipelineExecution) -> Self {
        PipelineExecution {
            id: UniversalUuid(pg.id),
            pipeline_name: pg.pipeline_name,
            pipeline_version: pg.pipeline_version,
            status: pg.status,
            context_id: pg.context_id.map(UniversalUuid),
            started_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.started_at)),
            completed_at: pg
                .completed_at
                .map(|dt| UniversalTimestamp(Utc.from_utc_datetime(&dt))),
            error_details: pg.error_details,
            recovery_attempts: pg.recovery_attempts,
            last_recovery_at: pg
                .last_recovery_at
                .map(|dt| UniversalTimestamp(Utc.from_utc_datetime(&dt))),
            created_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.created_at)),
            updated_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.updated_at)),
        }
    }
}

impl From<PgTaskExecution> for TaskExecution {
    fn from(pg: PgTaskExecution) -> Self {
        TaskExecution {
            id: UniversalUuid(pg.id),
            pipeline_execution_id: UniversalUuid(pg.pipeline_execution_id),
            task_name: pg.task_name,
            status: pg.status,
            started_at: pg
                .started_at
                .map(|dt| UniversalTimestamp(Utc.from_utc_datetime(&dt))),
            completed_at: pg
                .completed_at
                .map(|dt| UniversalTimestamp(Utc.from_utc_datetime(&dt))),
            attempt: pg.attempt,
            max_attempts: pg.max_attempts,
            error_details: pg.error_details,
            trigger_rules: pg.trigger_rules,
            task_configuration: pg.task_configuration,
            retry_at: pg
                .retry_at
                .map(|dt| UniversalTimestamp(Utc.from_utc_datetime(&dt))),
            last_error: pg.last_error,
            recovery_attempts: pg.recovery_attempts,
            last_recovery_at: pg
                .last_recovery_at
                .map(|dt| UniversalTimestamp(Utc.from_utc_datetime(&dt))),
            created_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.created_at)),
            updated_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.updated_at)),
        }
    }
}

impl From<PgTaskExecutionMetadata> for TaskExecutionMetadata {
    fn from(pg: PgTaskExecutionMetadata) -> Self {
        TaskExecutionMetadata {
            id: UniversalUuid(pg.id),
            task_execution_id: UniversalUuid(pg.task_execution_id),
            pipeline_execution_id: UniversalUuid(pg.pipeline_execution_id),
            task_name: pg.task_name,
            context_id: pg.context_id.map(UniversalUuid),
            created_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.created_at)),
            updated_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.updated_at)),
        }
    }
}

impl From<PgRecoveryEvent> for RecoveryEvent {
    fn from(pg: PgRecoveryEvent) -> Self {
        RecoveryEvent {
            id: UniversalUuid(pg.id),
            pipeline_execution_id: UniversalUuid(pg.pipeline_execution_id),
            task_execution_id: pg.task_execution_id.map(UniversalUuid),
            recovery_type: pg.recovery_type,
            recovered_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.recovered_at)),
            details: pg.details,
            created_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.created_at)),
            updated_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.updated_at)),
        }
    }
}

impl From<PgCronSchedule> for CronSchedule {
    fn from(pg: PgCronSchedule) -> Self {
        CronSchedule {
            id: UniversalUuid(pg.id),
            workflow_name: pg.workflow_name,
            cron_expression: pg.cron_expression,
            timezone: pg.timezone,
            enabled: pg.enabled,
            catchup_policy: pg.catchup_policy,
            start_date: pg
                .start_date
                .map(|dt| UniversalTimestamp(Utc.from_utc_datetime(&dt))),
            end_date: pg
                .end_date
                .map(|dt| UniversalTimestamp(Utc.from_utc_datetime(&dt))),
            next_run_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.next_run_at)),
            last_run_at: pg
                .last_run_at
                .map(|dt| UniversalTimestamp(Utc.from_utc_datetime(&dt))),
            created_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.created_at)),
            updated_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.updated_at)),
        }
    }
}

impl From<PgCronExecution> for CronExecution {
    fn from(pg: PgCronExecution) -> Self {
        CronExecution {
            id: UniversalUuid(pg.id),
            schedule_id: UniversalUuid(pg.schedule_id),
            pipeline_execution_id: pg.pipeline_execution_id.map(UniversalUuid),
            scheduled_time: UniversalTimestamp(Utc.from_utc_datetime(&pg.scheduled_time)),
            claimed_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.claimed_at)),
            created_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.created_at)),
            updated_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.updated_at)),
        }
    }
}

impl From<PgWorkflowRegistryEntry> for WorkflowRegistryEntry {
    fn from(pg: PgWorkflowRegistryEntry) -> Self {
        WorkflowRegistryEntry {
            id: UniversalUuid(pg.id),
            created_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.created_at)),
            data: pg.data,
        }
    }
}

impl From<PgWorkflowPackage> for WorkflowPackage {
    fn from(pg: PgWorkflowPackage) -> Self {
        WorkflowPackage {
            id: UniversalUuid(pg.id),
            registry_id: UniversalUuid(pg.registry_id),
            package_name: pg.package_name,
            version: pg.version,
            description: pg.description,
            author: pg.author,
            metadata: pg.metadata,
            created_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.created_at)),
            updated_at: UniversalTimestamp(Utc.from_utc_datetime(&pg.updated_at)),
        }
    }
}
