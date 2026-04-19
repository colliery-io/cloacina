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

//! Unified database models using custom SQL types
//!
//! These models use the unified schema with DbUuid, DbTimestamp, DbBool custom
//! SQL types that work with both PostgreSQL and SQLite backends.

use crate::database::schema::unified::{
    accumulator_boundaries, accumulator_checkpoints, contexts, execution_events, key_trust_acls,
    package_signatures, reactor_state, recovery_events, schedule_executions, schedules,
    signing_keys, state_accumulator_buffers, task_execution_metadata, task_executions, task_outbox,
    trusted_keys, workflow_executions, workflow_packages, workflow_registry,
};
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
// Workflow Execution Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = workflow_executions)]
pub struct UnifiedWorkflowExecution {
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
}

#[derive(Debug, Insertable)]
#[diesel(table_name = workflow_executions)]
pub struct NewUnifiedWorkflowExecution {
    pub id: UniversalUuid,
    pub workflow_name: String,
    pub workflow_version: String,
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
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = task_executions)]
pub struct NewUnifiedTaskExecution {
    pub id: UniversalUuid,
    pub workflow_execution_id: UniversalUuid,
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
    pub workflow_execution_id: UniversalUuid,
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
    pub workflow_execution_id: UniversalUuid,
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
    pub workflow_execution_id: UniversalUuid,
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
    pub workflow_execution_id: UniversalUuid,
    pub task_execution_id: Option<UniversalUuid>,
    pub recovery_type: String,
    pub recovered_at: UniversalTimestamp,
    pub details: Option<String>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

// ============================================================================
// Execution Event Models
// ============================================================================

/// Unified execution event model for audit trail of state transitions.
/// Append-only: events are never updated after creation.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = execution_events)]
pub struct UnifiedExecutionEvent {
    pub id: UniversalUuid,
    pub workflow_execution_id: UniversalUuid,
    pub task_execution_id: Option<UniversalUuid>,
    pub event_type: String,
    pub event_data: Option<String>,
    pub worker_id: Option<String>,
    pub created_at: UniversalTimestamp,
    pub sequence_num: i64,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = execution_events)]
pub struct NewUnifiedExecutionEvent {
    pub id: UniversalUuid,
    pub workflow_execution_id: UniversalUuid,
    pub task_execution_id: Option<UniversalUuid>,
    pub event_type: String,
    pub event_data: Option<String>,
    pub worker_id: Option<String>,
    pub created_at: UniversalTimestamp,
}

// ============================================================================
// Task Outbox Models
// ============================================================================

/// Unified task outbox model for work distribution.
/// Transient: rows are deleted immediately upon claiming.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = task_outbox)]
pub struct UnifiedTaskOutbox {
    pub id: i64,
    pub task_execution_id: UniversalUuid,
    pub created_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = task_outbox)]
pub struct NewUnifiedTaskOutbox {
    pub task_execution_id: UniversalUuid,
    pub created_at: UniversalTimestamp,
}

// ============================================================================
// Unified Schedule Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = schedules)]
pub struct UnifiedSchedule {
    pub id: UniversalUuid,
    pub schedule_type: String,
    pub workflow_name: String,
    pub enabled: UniversalBool,
    pub cron_expression: Option<String>,
    pub timezone: Option<String>,
    pub catchup_policy: Option<String>,
    pub start_date: Option<UniversalTimestamp>,
    pub end_date: Option<UniversalTimestamp>,
    pub trigger_name: Option<String>,
    pub poll_interval_ms: Option<i32>,
    pub allow_concurrent: Option<UniversalBool>,
    pub next_run_at: Option<UniversalTimestamp>,
    pub last_run_at: Option<UniversalTimestamp>,
    pub last_poll_at: Option<UniversalTimestamp>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = schedules)]
pub struct NewUnifiedSchedule {
    pub id: UniversalUuid,
    pub schedule_type: String,
    pub workflow_name: String,
    pub enabled: UniversalBool,
    pub cron_expression: Option<String>,
    pub timezone: Option<String>,
    pub catchup_policy: Option<String>,
    pub start_date: Option<UniversalTimestamp>,
    pub end_date: Option<UniversalTimestamp>,
    pub trigger_name: Option<String>,
    pub poll_interval_ms: Option<i32>,
    pub allow_concurrent: Option<UniversalBool>,
    pub next_run_at: Option<UniversalTimestamp>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

// ============================================================================
// Unified Schedule Execution Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = schedule_executions)]
pub struct UnifiedScheduleExecution {
    pub id: UniversalUuid,
    pub schedule_id: UniversalUuid,
    pub workflow_execution_id: Option<UniversalUuid>,
    pub scheduled_time: Option<UniversalTimestamp>,
    pub claimed_at: Option<UniversalTimestamp>,
    pub context_hash: Option<String>,
    pub started_at: UniversalTimestamp,
    pub completed_at: Option<UniversalTimestamp>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = schedule_executions)]
pub struct NewUnifiedScheduleExecution {
    pub id: UniversalUuid,
    pub schedule_id: UniversalUuid,
    pub workflow_execution_id: Option<UniversalUuid>,
    pub scheduled_time: Option<UniversalTimestamp>,
    pub claimed_at: Option<UniversalTimestamp>,
    pub context_hash: Option<String>,
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
    pub tenant_id: Option<String>,
    pub content_hash: String,
    pub superseded: UniversalBool,
    pub compiled_data: Option<UniversalBinary>,
    pub build_status: String,
    pub build_error: Option<String>,
    pub build_claimed_at: Option<UniversalTimestamp>,
    pub compiled_at: Option<UniversalTimestamp>,
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
    pub tenant_id: Option<String>,
    pub content_hash: String,
    pub superseded: UniversalBool,
    pub compiled_data: Option<UniversalBinary>,
    pub build_status: String,
    pub build_error: Option<String>,
    pub build_claimed_at: Option<UniversalTimestamp>,
    pub compiled_at: Option<UniversalTimestamp>,
}

// ============================================================================
// Signing Key Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = signing_keys)]
pub struct UnifiedSigningKey {
    pub id: UniversalUuid,
    pub org_id: UniversalUuid,
    pub key_name: String,
    pub encrypted_private_key: UniversalBinary,
    pub public_key: UniversalBinary,
    pub key_fingerprint: String,
    pub created_at: UniversalTimestamp,
    pub revoked_at: Option<UniversalTimestamp>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = signing_keys)]
pub struct NewUnifiedSigningKey {
    pub id: UniversalUuid,
    pub org_id: UniversalUuid,
    pub key_name: String,
    pub encrypted_private_key: UniversalBinary,
    pub public_key: UniversalBinary,
    pub key_fingerprint: String,
    pub created_at: UniversalTimestamp,
}

// ============================================================================
// Trusted Key Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = trusted_keys)]
pub struct UnifiedTrustedKey {
    pub id: UniversalUuid,
    pub org_id: UniversalUuid,
    pub key_fingerprint: String,
    pub public_key: UniversalBinary,
    pub key_name: Option<String>,
    pub trusted_at: UniversalTimestamp,
    pub revoked_at: Option<UniversalTimestamp>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = trusted_keys)]
pub struct NewUnifiedTrustedKey {
    pub id: UniversalUuid,
    pub org_id: UniversalUuid,
    pub key_fingerprint: String,
    pub public_key: UniversalBinary,
    pub key_name: Option<String>,
    pub trusted_at: UniversalTimestamp,
}

// ============================================================================
// Key Trust ACL Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = key_trust_acls)]
pub struct UnifiedKeyTrustAcl {
    pub id: UniversalUuid,
    pub parent_org_id: UniversalUuid,
    pub child_org_id: UniversalUuid,
    pub granted_at: UniversalTimestamp,
    pub revoked_at: Option<UniversalTimestamp>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = key_trust_acls)]
pub struct NewUnifiedKeyTrustAcl {
    pub id: UniversalUuid,
    pub parent_org_id: UniversalUuid,
    pub child_org_id: UniversalUuid,
    pub granted_at: UniversalTimestamp,
}

// ============================================================================
// Package Signature Models
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = package_signatures)]
pub struct UnifiedPackageSignature {
    pub id: UniversalUuid,
    pub package_hash: String,
    pub key_fingerprint: String,
    pub signature: UniversalBinary,
    pub signed_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = package_signatures)]
pub struct NewUnifiedPackageSignature {
    pub id: UniversalUuid,
    pub package_hash: String,
    pub key_fingerprint: String,
    pub signature: UniversalBinary,
    pub signed_at: UniversalTimestamp,
}

// ============================================================================
// Conversion to Domain Models
// ============================================================================
// Since unified models use Universal* types directly, conversion to domain
// models is straightforward - mostly just field-by-field mapping.

use crate::models::context::DbContext;
use crate::models::execution_event::ExecutionEvent;
use crate::models::key_trust_acl::KeyTrustAcl;
use crate::models::package_signature::PackageSignature;
use crate::models::recovery_event::RecoveryEvent;
use crate::models::schedule::{Schedule, ScheduleExecution};
use crate::models::signing_key::SigningKey;
use crate::models::task_execution::TaskExecution;
use crate::models::task_execution_metadata::TaskExecutionMetadata;
use crate::models::trusted_key::TrustedKey;
use crate::models::workflow_execution::WorkflowExecutionRecord;
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

impl From<UnifiedWorkflowExecution> for WorkflowExecutionRecord {
    fn from(u: UnifiedWorkflowExecution) -> Self {
        WorkflowExecutionRecord {
            id: u.id,
            workflow_name: u.workflow_name,
            workflow_version: u.workflow_version,
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
            workflow_execution_id: u.workflow_execution_id,
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
            sub_status: u.sub_status,
            claimed_by: u.claimed_by,
            heartbeat_at: u.heartbeat_at,
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
            workflow_execution_id: u.workflow_execution_id,
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
            workflow_execution_id: u.workflow_execution_id,
            task_execution_id: u.task_execution_id,
            recovery_type: u.recovery_type,
            recovered_at: u.recovered_at,
            details: u.details,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

impl From<UnifiedExecutionEvent> for ExecutionEvent {
    fn from(u: UnifiedExecutionEvent) -> Self {
        ExecutionEvent {
            id: u.id,
            workflow_execution_id: u.workflow_execution_id,
            task_execution_id: u.task_execution_id,
            event_type: u.event_type,
            event_data: u.event_data,
            worker_id: u.worker_id,
            created_at: u.created_at,
            sequence_num: u.sequence_num,
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
            tenant_id: u.tenant_id,
            content_hash: u.content_hash,
            superseded: u.superseded.0,
            compiled_data: u.compiled_data.map(|b| b.into_inner()),
            build_status: u.build_status,
            build_error: u.build_error,
            build_claimed_at: u.build_claimed_at,
            compiled_at: u.compiled_at,
        }
    }
}

impl From<UnifiedSigningKey> for SigningKey {
    fn from(u: UnifiedSigningKey) -> Self {
        SigningKey {
            id: u.id,
            org_id: u.org_id,
            key_name: u.key_name,
            encrypted_private_key: u.encrypted_private_key.into_inner(),
            public_key: u.public_key.into_inner(),
            key_fingerprint: u.key_fingerprint,
            created_at: u.created_at,
            revoked_at: u.revoked_at,
        }
    }
}

impl From<UnifiedTrustedKey> for TrustedKey {
    fn from(u: UnifiedTrustedKey) -> Self {
        TrustedKey {
            id: u.id,
            org_id: u.org_id,
            key_fingerprint: u.key_fingerprint,
            public_key: u.public_key.into_inner(),
            key_name: u.key_name,
            trusted_at: u.trusted_at,
            revoked_at: u.revoked_at,
        }
    }
}

impl From<UnifiedKeyTrustAcl> for KeyTrustAcl {
    fn from(u: UnifiedKeyTrustAcl) -> Self {
        KeyTrustAcl {
            id: u.id,
            parent_org_id: u.parent_org_id,
            child_org_id: u.child_org_id,
            granted_at: u.granted_at,
            revoked_at: u.revoked_at,
        }
    }
}

impl From<UnifiedPackageSignature> for PackageSignature {
    fn from(u: UnifiedPackageSignature) -> Self {
        PackageSignature {
            id: u.id,
            package_hash: u.package_hash,
            key_fingerprint: u.key_fingerprint,
            signature: u.signature.into_inner(),
            signed_at: u.signed_at,
        }
    }
}

impl From<UnifiedSchedule> for Schedule {
    fn from(u: UnifiedSchedule) -> Self {
        Schedule {
            id: u.id,
            schedule_type: u.schedule_type,
            workflow_name: u.workflow_name,
            enabled: u.enabled,
            cron_expression: u.cron_expression,
            timezone: u.timezone,
            catchup_policy: u.catchup_policy,
            start_date: u.start_date,
            end_date: u.end_date,
            trigger_name: u.trigger_name,
            poll_interval_ms: u.poll_interval_ms,
            allow_concurrent: u.allow_concurrent,
            next_run_at: u.next_run_at,
            last_run_at: u.last_run_at,
            last_poll_at: u.last_poll_at,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

impl From<UnifiedScheduleExecution> for ScheduleExecution {
    fn from(u: UnifiedScheduleExecution) -> Self {
        ScheduleExecution {
            id: u.id,
            schedule_id: u.schedule_id,
            workflow_execution_id: u.workflow_execution_id,
            scheduled_time: u.scheduled_time,
            claimed_at: u.claimed_at,
            context_hash: u.context_hash,
            started_at: u.started_at,
            completed_at: u.completed_at,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

// ============================================================================
// Computation Graph State Models
// ============================================================================

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = accumulator_checkpoints)]
pub struct UnifiedAccumulatorCheckpoint {
    pub id: UniversalUuid,
    pub graph_name: String,
    pub accumulator_name: String,
    pub checkpoint_data: UniversalBinary,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = accumulator_checkpoints)]
pub struct NewUnifiedAccumulatorCheckpoint {
    pub id: UniversalUuid,
    pub graph_name: String,
    pub accumulator_name: String,
    pub checkpoint_data: UniversalBinary,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = accumulator_boundaries)]
pub struct UnifiedAccumulatorBoundary {
    pub id: UniversalUuid,
    pub graph_name: String,
    pub accumulator_name: String,
    pub boundary_data: UniversalBinary,
    pub sequence_number: i64,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = accumulator_boundaries)]
pub struct NewUnifiedAccumulatorBoundary {
    pub id: UniversalUuid,
    pub graph_name: String,
    pub accumulator_name: String,
    pub boundary_data: UniversalBinary,
    pub sequence_number: i64,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = reactor_state)]
pub struct UnifiedReactorState {
    pub id: UniversalUuid,
    pub graph_name: String,
    pub cache_data: UniversalBinary,
    pub dirty_flags: UniversalBinary,
    pub sequential_queue: Option<UniversalBinary>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = reactor_state)]
pub struct NewUnifiedReactorState {
    pub id: UniversalUuid,
    pub graph_name: String,
    pub cache_data: UniversalBinary,
    pub dirty_flags: UniversalBinary,
    pub sequential_queue: Option<UniversalBinary>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = state_accumulator_buffers)]
pub struct UnifiedStateAccumulatorBuffer {
    pub id: UniversalUuid,
    pub graph_name: String,
    pub accumulator_name: String,
    pub buffer_data: UniversalBinary,
    pub capacity: i32,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = state_accumulator_buffers)]
pub struct NewUnifiedStateAccumulatorBuffer {
    pub id: UniversalUuid,
    pub graph_name: String,
    pub accumulator_name: String,
    pub buffer_data: UniversalBinary,
    pub capacity: i32,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}
