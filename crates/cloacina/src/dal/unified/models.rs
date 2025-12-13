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

//! Unified database models using custom SQL types
//!
//! These models use the unified schema with DbUuid, DbTimestamp, DbBool custom
//! SQL types that work with both PostgreSQL and SQLite backends.

use crate::database::schema::unified::*;
use crate::database::universal_types::{
    UniversalBinary, UniversalBool, UniversalTimestamp, UniversalUuid,
};
use diesel::prelude::*;

// ============================================================================
// Context Models
// ============================================================================

/// Unified context model that works with both PostgreSQL and SQLite.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = contexts)]
pub struct UnifiedDbContext {
    pub id: UniversalUuid,
    pub value: String,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

/// Insertable context with explicit ID and timestamps (for SQLite compatibility).
#[derive(Debug, Insertable)]
#[diesel(table_name = contexts)]
pub struct NewUnifiedDbContext {
    pub id: UniversalUuid,
    pub value: String,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

// ============================================================================
// Pipeline Execution Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = pipeline_executions)]
pub struct UnifiedPipelineExecution {
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

#[derive(Debug, Insertable)]
#[diesel(table_name = pipeline_executions)]
pub struct NewUnifiedPipelineExecution {
    pub id: UniversalUuid,
    pub pipeline_name: String,
    pub pipeline_version: String,
    pub status: String,
    pub context_id: Option<UniversalUuid>,
    pub started_at: UniversalTimestamp,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

// ============================================================================
// Task Execution Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = task_executions)]
pub struct UnifiedTaskExecution {
    pub id: UniversalUuid,
    pub pipeline_execution_id: UniversalUuid,
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
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = task_executions)]
pub struct NewUnifiedTaskExecution {
    pub id: UniversalUuid,
    pub pipeline_execution_id: UniversalUuid,
    pub task_name: String,
    pub status: String,
    pub attempt: i32,
    pub max_attempts: i32,
    pub trigger_rules: String,
    pub task_configuration: String,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

// ============================================================================
// Task Execution Metadata Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = task_execution_metadata)]
pub struct UnifiedTaskExecutionMetadata {
    pub id: UniversalUuid,
    pub task_execution_id: UniversalUuid,
    pub pipeline_execution_id: UniversalUuid,
    pub task_name: String,
    pub context_id: Option<UniversalUuid>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = task_execution_metadata)]
pub struct NewUnifiedTaskExecutionMetadata {
    pub id: UniversalUuid,
    pub task_execution_id: UniversalUuid,
    pub pipeline_execution_id: UniversalUuid,
    pub task_name: String,
    pub context_id: Option<UniversalUuid>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

// ============================================================================
// Recovery Event Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = recovery_events)]
pub struct UnifiedRecoveryEvent {
    pub id: UniversalUuid,
    pub pipeline_execution_id: UniversalUuid,
    pub task_execution_id: Option<UniversalUuid>,
    pub recovery_type: String,
    pub recovered_at: UniversalTimestamp,
    pub details: Option<String>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = recovery_events)]
pub struct NewUnifiedRecoveryEvent {
    pub id: UniversalUuid,
    pub pipeline_execution_id: UniversalUuid,
    pub task_execution_id: Option<UniversalUuid>,
    pub recovery_type: String,
    pub recovered_at: UniversalTimestamp,
    pub details: Option<String>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

// ============================================================================
// Cron Schedule Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = cron_schedules)]
pub struct UnifiedCronSchedule {
    pub id: UniversalUuid,
    pub workflow_name: String,
    pub cron_expression: String,
    pub timezone: String,
    pub enabled: UniversalBool,
    pub catchup_policy: String,
    pub start_date: Option<UniversalTimestamp>,
    pub end_date: Option<UniversalTimestamp>,
    pub next_run_at: UniversalTimestamp,
    pub last_run_at: Option<UniversalTimestamp>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = cron_schedules)]
pub struct NewUnifiedCronSchedule {
    pub id: UniversalUuid,
    pub workflow_name: String,
    pub cron_expression: String,
    pub timezone: String,
    pub enabled: UniversalBool,
    pub catchup_policy: String,
    pub start_date: Option<UniversalTimestamp>,
    pub end_date: Option<UniversalTimestamp>,
    pub next_run_at: UniversalTimestamp,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

// ============================================================================
// Cron Execution Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = cron_executions)]
pub struct UnifiedCronExecution {
    pub id: UniversalUuid,
    pub schedule_id: UniversalUuid,
    pub pipeline_execution_id: Option<UniversalUuid>,
    pub scheduled_time: UniversalTimestamp,
    pub claimed_at: UniversalTimestamp,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = cron_executions)]
pub struct NewUnifiedCronExecution {
    pub id: UniversalUuid,
    pub schedule_id: UniversalUuid,
    pub pipeline_execution_id: Option<UniversalUuid>,
    pub scheduled_time: UniversalTimestamp,
    pub claimed_at: UniversalTimestamp,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

// ============================================================================
// Trigger Schedule Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = trigger_schedules)]
pub struct UnifiedTriggerSchedule {
    pub id: UniversalUuid,
    pub trigger_name: String,
    pub workflow_name: String,
    pub poll_interval_ms: i32,
    pub allow_concurrent: UniversalBool,
    pub enabled: UniversalBool,
    pub last_poll_at: Option<UniversalTimestamp>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = trigger_schedules)]
pub struct NewUnifiedTriggerSchedule {
    pub id: UniversalUuid,
    pub trigger_name: String,
    pub workflow_name: String,
    pub poll_interval_ms: i32,
    pub allow_concurrent: UniversalBool,
    pub enabled: UniversalBool,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

// ============================================================================
// Trigger Execution Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = trigger_executions)]
pub struct UnifiedTriggerExecution {
    pub id: UniversalUuid,
    pub trigger_name: String,
    pub context_hash: String,
    pub pipeline_execution_id: Option<UniversalUuid>,
    pub started_at: UniversalTimestamp,
    pub completed_at: Option<UniversalTimestamp>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = trigger_executions)]
pub struct NewUnifiedTriggerExecution {
    pub id: UniversalUuid,
    pub trigger_name: String,
    pub context_hash: String,
    pub pipeline_execution_id: Option<UniversalUuid>,
    pub started_at: UniversalTimestamp,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

// ============================================================================
// Workflow Registry Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = workflow_registry)]
pub struct UnifiedWorkflowRegistryEntry {
    pub id: UniversalUuid,
    pub created_at: UniversalTimestamp,
    pub data: UniversalBinary,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = workflow_registry)]
pub struct NewUnifiedWorkflowRegistryEntry {
    pub id: UniversalUuid,
    pub created_at: UniversalTimestamp,
    pub data: UniversalBinary,
}

// ============================================================================
// Workflow Packages Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = workflow_packages)]
pub struct UnifiedWorkflowPackage {
    pub id: UniversalUuid,
    pub registry_id: UniversalUuid,
    pub package_name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub metadata: String,
    pub storage_type: String,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = workflow_packages)]
pub struct NewUnifiedWorkflowPackage {
    pub id: UniversalUuid,
    pub registry_id: UniversalUuid,
    pub package_name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub metadata: String,
    pub storage_type: String,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

// ============================================================================
// Conversion to Domain Models
// ============================================================================
// Since unified models use Universal* types directly, conversion to domain
// models is straightforward - mostly just field-by-field mapping.

use crate::models::context::DbContext;
use crate::models::cron_execution::CronExecution;
use crate::models::cron_schedule::CronSchedule;
use crate::models::pipeline_execution::PipelineExecution;
use crate::models::recovery_event::RecoveryEvent;
use crate::models::task_execution::TaskExecution;
use crate::models::task_execution_metadata::TaskExecutionMetadata;
use crate::models::trigger_execution::TriggerExecution;
use crate::models::trigger_schedule::TriggerSchedule;
use crate::models::workflow_packages::WorkflowPackage;
use crate::models::workflow_registry::WorkflowRegistryEntry;

impl From<UnifiedDbContext> for DbContext {
    fn from(u: UnifiedDbContext) -> Self {
        DbContext {
            id: u.id,
            value: u.value,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

impl From<UnifiedPipelineExecution> for PipelineExecution {
    fn from(u: UnifiedPipelineExecution) -> Self {
        PipelineExecution {
            id: u.id,
            pipeline_name: u.pipeline_name,
            pipeline_version: u.pipeline_version,
            status: u.status,
            context_id: u.context_id,
            started_at: u.started_at,
            completed_at: u.completed_at,
            error_details: u.error_details,
            recovery_attempts: u.recovery_attempts,
            last_recovery_at: u.last_recovery_at,
            paused_at: u.paused_at,
            pause_reason: u.pause_reason,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

impl From<UnifiedTaskExecution> for TaskExecution {
    fn from(u: UnifiedTaskExecution) -> Self {
        TaskExecution {
            id: u.id,
            pipeline_execution_id: u.pipeline_execution_id,
            task_name: u.task_name,
            status: u.status,
            started_at: u.started_at,
            completed_at: u.completed_at,
            attempt: u.attempt,
            max_attempts: u.max_attempts,
            error_details: u.error_details,
            trigger_rules: u.trigger_rules,
            task_configuration: u.task_configuration,
            retry_at: u.retry_at,
            last_error: u.last_error,
            recovery_attempts: u.recovery_attempts,
            last_recovery_at: u.last_recovery_at,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

impl From<UnifiedTaskExecutionMetadata> for TaskExecutionMetadata {
    fn from(u: UnifiedTaskExecutionMetadata) -> Self {
        TaskExecutionMetadata {
            id: u.id,
            task_execution_id: u.task_execution_id,
            pipeline_execution_id: u.pipeline_execution_id,
            task_name: u.task_name,
            context_id: u.context_id,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

impl From<UnifiedRecoveryEvent> for RecoveryEvent {
    fn from(u: UnifiedRecoveryEvent) -> Self {
        RecoveryEvent {
            id: u.id,
            pipeline_execution_id: u.pipeline_execution_id,
            task_execution_id: u.task_execution_id,
            recovery_type: u.recovery_type,
            recovered_at: u.recovered_at,
            details: u.details,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

impl From<UnifiedCronSchedule> for CronSchedule {
    fn from(u: UnifiedCronSchedule) -> Self {
        CronSchedule {
            id: u.id,
            workflow_name: u.workflow_name,
            cron_expression: u.cron_expression,
            timezone: u.timezone,
            enabled: u.enabled,
            catchup_policy: u.catchup_policy,
            start_date: u.start_date,
            end_date: u.end_date,
            next_run_at: u.next_run_at,
            last_run_at: u.last_run_at,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

impl From<UnifiedCronExecution> for CronExecution {
    fn from(u: UnifiedCronExecution) -> Self {
        CronExecution {
            id: u.id,
            schedule_id: u.schedule_id,
            pipeline_execution_id: u.pipeline_execution_id,
            scheduled_time: u.scheduled_time,
            claimed_at: u.claimed_at,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

impl From<UnifiedWorkflowRegistryEntry> for WorkflowRegistryEntry {
    fn from(u: UnifiedWorkflowRegistryEntry) -> Self {
        WorkflowRegistryEntry {
            id: u.id,
            created_at: u.created_at,
            data: u.data.into_inner(),
        }
    }
}

impl From<UnifiedWorkflowPackage> for WorkflowPackage {
    fn from(u: UnifiedWorkflowPackage) -> Self {
        WorkflowPackage {
            id: u.id,
            registry_id: u.registry_id,
            package_name: u.package_name,
            version: u.version,
            description: u.description,
            author: u.author,
            metadata: u.metadata,
            storage_type: u.storage_type.parse().unwrap(),
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

impl From<UnifiedTriggerSchedule> for TriggerSchedule {
    fn from(u: UnifiedTriggerSchedule) -> Self {
        TriggerSchedule {
            id: u.id,
            trigger_name: u.trigger_name,
            workflow_name: u.workflow_name,
            poll_interval_ms: u.poll_interval_ms,
            allow_concurrent: u.allow_concurrent,
            enabled: u.enabled,
            last_poll_at: u.last_poll_at,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

impl From<UnifiedTriggerExecution> for TriggerExecution {
    fn from(u: UnifiedTriggerExecution) -> Self {
        TriggerExecution {
            id: u.id,
            trigger_name: u.trigger_name,
            context_hash: u.context_hash,
            pipeline_execution_id: u.pipeline_execution_id,
            started_at: u.started_at,
            completed_at: u.completed_at,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}
