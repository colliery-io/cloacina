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

// Schema selection based on backend feature flags

// =============================================================================
// Unified Schema using Custom SQL Types
// =============================================================================
// This schema uses custom SQL types (DbUuid, DbTimestamp, DbBool) that work
// with both PostgreSQL and SQLite backends. Use this for new code.

mod unified_schema {
    diesel::table! {
        use diesel::sql_types::*;
        use crate::database::universal_types::{DbUuid, DbTimestamp, DbBool, DbBinary};

        contexts (id) {
            id -> DbUuid,
            value -> Text,
            created_at -> DbTimestamp,
            updated_at -> DbTimestamp,
        }
    }

    diesel::table! {
        use diesel::sql_types::*;
        use crate::database::universal_types::{DbUuid, DbTimestamp, DbBool, DbBinary};

        pipeline_executions (id) {
            id -> DbUuid,
            pipeline_name -> Text,
            pipeline_version -> Text,
            status -> Text,
            context_id -> Nullable<DbUuid>,
            started_at -> DbTimestamp,
            completed_at -> Nullable<DbTimestamp>,
            error_details -> Nullable<Text>,
            recovery_attempts -> Integer,
            last_recovery_at -> Nullable<DbTimestamp>,
            paused_at -> Nullable<DbTimestamp>,
            pause_reason -> Nullable<Text>,
            created_at -> DbTimestamp,
            updated_at -> DbTimestamp,
        }
    }

    diesel::table! {
        use diesel::sql_types::*;
        use crate::database::universal_types::{DbUuid, DbTimestamp, DbBool, DbBinary};

        task_executions (id) {
            id -> DbUuid,
            pipeline_execution_id -> DbUuid,
            task_name -> Text,
            status -> Text,
            started_at -> Nullable<DbTimestamp>,
            completed_at -> Nullable<DbTimestamp>,
            attempt -> Integer,
            max_attempts -> Integer,
            error_details -> Nullable<Text>,
            trigger_rules -> Text,
            task_configuration -> Text,
            retry_at -> Nullable<DbTimestamp>,
            last_error -> Nullable<Text>,
            recovery_attempts -> Integer,
            last_recovery_at -> Nullable<DbTimestamp>,
            created_at -> DbTimestamp,
            updated_at -> DbTimestamp,
        }
    }

    diesel::table! {
        use diesel::sql_types::*;
        use crate::database::universal_types::{DbUuid, DbTimestamp, DbBool, DbBinary};

        recovery_events (id) {
            id -> DbUuid,
            pipeline_execution_id -> DbUuid,
            task_execution_id -> Nullable<DbUuid>,
            recovery_type -> Text,
            recovered_at -> DbTimestamp,
            details -> Nullable<Text>,
            created_at -> DbTimestamp,
            updated_at -> DbTimestamp,
        }
    }

    diesel::table! {
        use diesel::sql_types::*;
        use crate::database::universal_types::{DbUuid, DbTimestamp, DbBool, DbBinary};

        task_execution_metadata (id) {
            id -> DbUuid,
            task_execution_id -> DbUuid,
            pipeline_execution_id -> DbUuid,
            task_name -> Text,
            context_id -> Nullable<DbUuid>,
            created_at -> DbTimestamp,
            updated_at -> DbTimestamp,
        }
    }

    diesel::table! {
        use diesel::sql_types::*;
        use crate::database::universal_types::{DbUuid, DbTimestamp, DbBool, DbBinary};

        workflow_registry (id) {
            id -> DbUuid,
            created_at -> DbTimestamp,
            data -> DbBinary,
        }
    }

    diesel::table! {
        use diesel::sql_types::*;
        use crate::database::universal_types::{DbUuid, DbTimestamp, DbBool, DbBinary};

        workflow_packages (id) {
            id -> DbUuid,
            registry_id -> DbUuid,
            package_name -> Text,
            version -> Text,
            description -> Nullable<Text>,
            author -> Nullable<Text>,
            metadata -> Text,
            storage_type -> Text,
            created_at -> DbTimestamp,
            updated_at -> DbTimestamp,
        }
    }

    diesel::table! {
        use diesel::sql_types::*;
        use crate::database::universal_types::{DbUuid, DbTimestamp, DbBool, DbBinary};

        cron_schedules (id) {
            id -> DbUuid,
            workflow_name -> Text,
            cron_expression -> Text,
            timezone -> Text,
            enabled -> DbBool,
            catchup_policy -> Text,
            start_date -> Nullable<DbTimestamp>,
            end_date -> Nullable<DbTimestamp>,
            next_run_at -> DbTimestamp,
            last_run_at -> Nullable<DbTimestamp>,
            created_at -> DbTimestamp,
            updated_at -> DbTimestamp,
        }
    }

    diesel::table! {
        use diesel::sql_types::*;
        use crate::database::universal_types::{DbUuid, DbTimestamp, DbBool, DbBinary};

        cron_executions (id) {
            id -> DbUuid,
            schedule_id -> DbUuid,
            pipeline_execution_id -> Nullable<DbUuid>,
            scheduled_time -> DbTimestamp,
            claimed_at -> DbTimestamp,
            created_at -> DbTimestamp,
            updated_at -> DbTimestamp,
        }
    }

    diesel::table! {
        use diesel::sql_types::*;
        use crate::database::universal_types::{DbUuid, DbTimestamp, DbBool, DbBinary};

        trigger_schedules (id) {
            id -> DbUuid,
            trigger_name -> Text,
            workflow_name -> Text,
            poll_interval_ms -> Integer,
            allow_concurrent -> DbBool,
            enabled -> DbBool,
            last_poll_at -> Nullable<DbTimestamp>,
            created_at -> DbTimestamp,
            updated_at -> DbTimestamp,
        }
    }

    diesel::table! {
        use diesel::sql_types::*;
        use crate::database::universal_types::{DbUuid, DbTimestamp, DbBool, DbBinary};

        trigger_executions (id) {
            id -> DbUuid,
            trigger_name -> Text,
            context_hash -> Text,
            pipeline_execution_id -> Nullable<DbUuid>,
            started_at -> DbTimestamp,
            completed_at -> Nullable<DbTimestamp>,
            created_at -> DbTimestamp,
            updated_at -> DbTimestamp,
        }
    }

    diesel::joinable!(pipeline_executions -> contexts (context_id));
    diesel::joinable!(task_executions -> pipeline_executions (pipeline_execution_id));
    diesel::joinable!(task_execution_metadata -> task_executions (task_execution_id));
    diesel::joinable!(task_execution_metadata -> pipeline_executions (pipeline_execution_id));
    diesel::joinable!(task_execution_metadata -> contexts (context_id));
    diesel::joinable!(recovery_events -> pipeline_executions (pipeline_execution_id));
    diesel::joinable!(recovery_events -> task_executions (task_execution_id));
    diesel::joinable!(cron_executions -> cron_schedules (schedule_id));
    diesel::joinable!(cron_executions -> pipeline_executions (pipeline_execution_id));
    diesel::joinable!(trigger_executions -> pipeline_executions (pipeline_execution_id));

    diesel::allow_tables_to_appear_in_same_query!(
        contexts,
        cron_executions,
        cron_schedules,
        pipeline_executions,
        recovery_events,
        task_executions,
        task_execution_metadata,
        trigger_executions,
        trigger_schedules,
        workflow_packages,
        workflow_registry,
    );
}

// =============================================================================
// Legacy Backend-Specific Schemas (to be removed after migration)
// =============================================================================

#[cfg(feature = "postgres")]
mod postgres_schema {
    // PostgreSQL schema using native types
    diesel::table! {
        contexts (id) {
            id -> Uuid,
            value -> Varchar,
            created_at -> Timestamp,
            updated_at -> Timestamp,
        }
    }

    diesel::table! {
        pipeline_executions (id) {
            id -> Uuid,
            pipeline_name -> Varchar,
            pipeline_version -> Varchar,
            status -> Varchar,
            context_id -> Nullable<Uuid>,
            started_at -> Timestamp,
            completed_at -> Nullable<Timestamp>,
            error_details -> Nullable<Text>,
            recovery_attempts -> Int4,
            last_recovery_at -> Nullable<Timestamp>,
            paused_at -> Nullable<Timestamp>,
            pause_reason -> Nullable<Text>,
            created_at -> Timestamp,
            updated_at -> Timestamp,
        }
    }

    diesel::table! {
        task_executions (id) {
            id -> Uuid,
            pipeline_execution_id -> Uuid,
            task_name -> Varchar,
            status -> Varchar,
            started_at -> Nullable<Timestamp>,
            completed_at -> Nullable<Timestamp>,
            attempt -> Int4,
            max_attempts -> Int4,
            error_details -> Nullable<Text>,
            trigger_rules -> Text,
            task_configuration -> Text,
            retry_at -> Nullable<Timestamp>,
            last_error -> Nullable<Text>,
            recovery_attempts -> Int4,
            last_recovery_at -> Nullable<Timestamp>,
            created_at -> Timestamp,
            updated_at -> Timestamp,
        }
    }

    diesel::table! {
        recovery_events (id) {
            id -> Uuid,
            pipeline_execution_id -> Uuid,
            task_execution_id -> Nullable<Uuid>,
            recovery_type -> Varchar,
            recovered_at -> Timestamp,
            details -> Nullable<Text>,
            created_at -> Timestamp,
            updated_at -> Timestamp,
        }
    }

    diesel::table! {
        task_execution_metadata (id) {
            id -> Uuid,
            task_execution_id -> Uuid,
            pipeline_execution_id -> Uuid,
            task_name -> Varchar,
            context_id -> Nullable<Uuid>,
            created_at -> Timestamp,
            updated_at -> Timestamp,
        }
    }

    diesel::table! {
        workflow_registry (id) {
            id -> Uuid,
            created_at -> Timestamp,
            data -> Bytea,
        }
    }

    diesel::table! {
        workflow_packages (id) {
            id -> Uuid,
            registry_id -> Uuid,
            package_name -> Varchar,
            version -> Varchar,
            description -> Nullable<Text>,
            author -> Nullable<Varchar>,
            metadata -> Text,
            storage_type -> Varchar,
            created_at -> Timestamp,
            updated_at -> Timestamp,
        }
    }

    diesel::table! {
        cron_schedules (id) {
            id -> Uuid,
            workflow_name -> Varchar,
            cron_expression -> Varchar,
            timezone -> Varchar,
            enabled -> Bool,
            catchup_policy -> Varchar,
            start_date -> Nullable<Timestamp>,
            end_date -> Nullable<Timestamp>,
            next_run_at -> Timestamp,
            last_run_at -> Nullable<Timestamp>,
            created_at -> Timestamp,
            updated_at -> Timestamp,
        }
    }

    diesel::table! {
        cron_executions (id) {
            id -> Uuid,
            schedule_id -> Uuid,
            pipeline_execution_id -> Nullable<Uuid>,
            scheduled_time -> Timestamp,
            claimed_at -> Timestamp,
            created_at -> Timestamp,
            updated_at -> Timestamp,
        }
    }

    diesel::table! {
        trigger_schedules (id) {
            id -> Uuid,
            trigger_name -> Varchar,
            workflow_name -> Varchar,
            poll_interval_ms -> Int4,
            allow_concurrent -> Bool,
            enabled -> Bool,
            last_poll_at -> Nullable<Timestamp>,
            created_at -> Timestamp,
            updated_at -> Timestamp,
        }
    }

    diesel::table! {
        trigger_executions (id) {
            id -> Uuid,
            trigger_name -> Varchar,
            context_hash -> Varchar,
            pipeline_execution_id -> Nullable<Uuid>,
            started_at -> Timestamp,
            completed_at -> Nullable<Timestamp>,
            created_at -> Timestamp,
            updated_at -> Timestamp,
        }
    }

    #[cfg(feature = "auth")]
    diesel::table! {
        auth_tokens (token_id) {
            token_id -> Varchar,
            name -> Varchar,
            role -> Varchar,
            tenant_id -> Varchar,
            created_at -> Timestamptz,
            expires_at -> Nullable<Timestamptz>,
            last_used_at -> Nullable<Timestamptz>,
            created_by_token_id -> Nullable<Varchar>,
            status -> Varchar,
        }
    }

    #[cfg(feature = "auth")]
    diesel::table! {
        auth_audit_log (id) {
            id -> Int4,
            timestamp -> Timestamptz,
            action -> Varchar,
            token_id -> Nullable<Varchar>,
            actor_token_id -> Nullable<Varchar>,
            tenant_id -> Nullable<Varchar>,
            details -> Nullable<Json>,
            ip_address -> Nullable<Inet>,
            user_agent -> Nullable<Varchar>,
            resource -> Nullable<Varchar>,
            action_type -> Nullable<Varchar>,
        }
    }

    diesel::joinable!(pipeline_executions -> contexts (context_id));
    diesel::joinable!(task_executions -> pipeline_executions (pipeline_execution_id));
    diesel::joinable!(task_execution_metadata -> task_executions (task_execution_id));
    diesel::joinable!(task_execution_metadata -> pipeline_executions (pipeline_execution_id));
    diesel::joinable!(task_execution_metadata -> contexts (context_id));
    diesel::joinable!(recovery_events -> pipeline_executions (pipeline_execution_id));
    diesel::joinable!(recovery_events -> task_executions (task_execution_id));
    diesel::joinable!(cron_executions -> cron_schedules (schedule_id));
    diesel::joinable!(cron_executions -> pipeline_executions (pipeline_execution_id));
    diesel::joinable!(trigger_executions -> pipeline_executions (pipeline_execution_id));
    diesel::joinable!(workflow_packages -> workflow_registry (registry_id));

    // Auth table relationships - using allow_tables_to_appear_in_same_query instead
    // of joinable! to avoid conflicts with multiple foreign keys to same table.
    // Manual join syntax should be used in queries when joining auth tables.

    #[cfg(not(feature = "auth"))]
    diesel::allow_tables_to_appear_in_same_query!(
        contexts,
        cron_executions,
        cron_schedules,
        pipeline_executions,
        recovery_events,
        task_executions,
        task_execution_metadata,
        trigger_executions,
        trigger_schedules,
        workflow_packages,
        workflow_registry,
    );

    #[cfg(feature = "auth")]
    diesel::allow_tables_to_appear_in_same_query!(
        auth_audit_log,
        auth_tokens,
        contexts,
        cron_executions,
        cron_schedules,
        pipeline_executions,
        recovery_events,
        task_executions,
        task_execution_metadata,
        trigger_executions,
        trigger_schedules,
        workflow_packages,
        workflow_registry,
    );
}

#[cfg(feature = "sqlite")]
mod sqlite_schema {
    // SQLite schema with appropriate type mappings
    diesel::table! {
        contexts (id) {
            id -> Binary,
            value -> Text,
            created_at -> Text,
            updated_at -> Text,
        }
    }

    diesel::table! {
        pipeline_executions (id) {
            id -> Binary,
            pipeline_name -> Text,
            pipeline_version -> Text,
            status -> Text,
            context_id -> Nullable<Binary>,
            started_at -> Text,
            completed_at -> Nullable<Text>,
            error_details -> Nullable<Text>,
            recovery_attempts -> Integer,
            last_recovery_at -> Nullable<Text>,
            paused_at -> Nullable<Text>,
            pause_reason -> Nullable<Text>,
            created_at -> Text,
            updated_at -> Text,
        }
    }

    diesel::table! {
        task_executions (id) {
            id -> Binary,
            pipeline_execution_id -> Binary,
            task_name -> Text,
            status -> Text,
            started_at -> Nullable<Text>,
            completed_at -> Nullable<Text>,
            attempt -> Integer,
            max_attempts -> Integer,
            error_details -> Nullable<Text>,
            trigger_rules -> Text,
            task_configuration -> Text,
            retry_at -> Nullable<Text>,
            last_error -> Nullable<Text>,
            recovery_attempts -> Integer,
            last_recovery_at -> Nullable<Text>,
            created_at -> Text,
            updated_at -> Text,
        }
    }

    diesel::table! {
        recovery_events (id) {
            id -> Binary,
            pipeline_execution_id -> Binary,
            task_execution_id -> Nullable<Binary>,
            recovery_type -> Text,
            recovered_at -> Text,
            details -> Nullable<Text>,
            created_at -> Text,
            updated_at -> Text,
        }
    }

    diesel::table! {
        task_execution_metadata (id) {
            id -> Binary,
            task_execution_id -> Binary,
            pipeline_execution_id -> Binary,
            task_name -> Text,
            context_id -> Nullable<Binary>,
            created_at -> Text,
            updated_at -> Text,
        }
    }

    diesel::table! {
        workflow_registry (id) {
            id -> Binary,
            created_at -> Text,
            data -> Binary,
        }
    }

    diesel::table! {
        workflow_packages (id) {
            id -> Binary,
            registry_id -> Binary,
            package_name -> Text,
            version -> Text,
            description -> Nullable<Text>,
            author -> Nullable<Text>,
            metadata -> Text,
            storage_type -> Text,
            created_at -> Text,
            updated_at -> Text,
        }
    }

    diesel::table! {
        cron_schedules (id) {
            id -> Binary,
            workflow_name -> Text,
            cron_expression -> Text,
            timezone -> Text,
            enabled -> Integer,
            catchup_policy -> Text,
            start_date -> Nullable<Text>,
            end_date -> Nullable<Text>,
            next_run_at -> Text,
            last_run_at -> Nullable<Text>,
            created_at -> Text,
            updated_at -> Text,
        }
    }

    diesel::table! {
        cron_executions (id) {
            id -> Binary,
            schedule_id -> Binary,
            pipeline_execution_id -> Nullable<Binary>,
            scheduled_time -> Text,
            claimed_at -> Text,
            created_at -> Text,
            updated_at -> Text,
        }
    }

    diesel::table! {
        trigger_schedules (id) {
            id -> Binary,
            trigger_name -> Text,
            workflow_name -> Text,
            poll_interval_ms -> Integer,
            allow_concurrent -> Integer,
            enabled -> Integer,
            last_poll_at -> Nullable<Text>,
            created_at -> Text,
            updated_at -> Text,
        }
    }

    diesel::table! {
        trigger_executions (id) {
            id -> Binary,
            trigger_name -> Text,
            context_hash -> Text,
            pipeline_execution_id -> Nullable<Binary>,
            started_at -> Text,
            completed_at -> Nullable<Text>,
            created_at -> Text,
            updated_at -> Text,
        }
    }

    diesel::joinable!(pipeline_executions -> contexts (context_id));
    diesel::joinable!(task_executions -> pipeline_executions (pipeline_execution_id));
    diesel::joinable!(task_execution_metadata -> task_executions (task_execution_id));
    diesel::joinable!(task_execution_metadata -> pipeline_executions (pipeline_execution_id));
    diesel::joinable!(task_execution_metadata -> contexts (context_id));
    diesel::joinable!(recovery_events -> pipeline_executions (pipeline_execution_id));
    diesel::joinable!(recovery_events -> task_executions (task_execution_id));
    diesel::joinable!(cron_executions -> cron_schedules (schedule_id));
    diesel::joinable!(cron_executions -> pipeline_executions (pipeline_execution_id));
    diesel::joinable!(trigger_executions -> pipeline_executions (pipeline_execution_id));

    diesel::allow_tables_to_appear_in_same_query!(
        contexts,
        cron_executions,
        cron_schedules,
        pipeline_executions,
        recovery_events,
        task_executions,
        task_execution_metadata,
        trigger_executions,
        trigger_schedules,
    );
}

// Re-export the appropriate schema based on feature flags

// Unified schema - use this for new code (works with both backends)
pub mod unified {
    pub use super::unified_schema::*;
}

// Legacy backend-specific modules (to be removed after migration)
// Use schema::postgres::* or schema::sqlite::* for legacy code
#[cfg(feature = "postgres")]
pub mod postgres {
    pub use super::postgres_schema::*;
}

#[cfg(feature = "sqlite")]
pub mod sqlite {
    pub use super::sqlite_schema::*;
}
