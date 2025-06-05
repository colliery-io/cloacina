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

//! Cron Execution Data Access Layer for PostgreSQL
//!
//! This module provides the data access layer for managing cron execution audit records
//! in PostgreSQL. It handles the audit trail that tracks every handoff from the cron
//! scheduler to the pipeline executor, ensuring guaranteed execution reliability.
//!
//! Key features:
//! - Audit trail creation for successful schedule claims
//! - Recovery detection for lost/failed executions
//! - Query capabilities for execution history and monitoring
//! - Integration with cron schedules and pipeline executions
//! - Duplicate prevention through unique constraints

use super::DAL;
use crate::database::schema::cron_executions;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::cron_execution::{CronExecution, NewCronExecution};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

/// Data Access Layer for cron execution operations.
///
/// This struct provides methods for managing cron execution audit records,
/// including creation, retrieval, and recovery detection. It is critical
/// for implementing guaranteed execution reliability in the cron scheduling system.
pub struct CronExecutionDAL<'a> {
    pub dal: &'a DAL,
}

impl<'a> CronExecutionDAL<'a> {
    /// Creates a new cron execution audit record in the database.
    ///
    /// This method is called BEFORE handing off to the pipeline executor
    /// to create a durable record of execution intent. This enables recovery
    /// if the handoff fails or the system crashes.
    ///
    /// # Arguments
    /// * `new_execution` - A `NewCronExecution` struct containing the execution details
    ///
    /// # Returns
    /// * `Result<CronExecution, ValidationError>` - The created cron execution record
    ///
    /// # Errors
    /// * Returns `ValidationError` if a duplicate execution exists (same schedule_id + scheduled_time)
    /// * Returns database errors for connection or constraint violations
    ///
    /// # Example
    /// ```rust
    /// // Create audit record BEFORE handoff
    /// let new_execution = NewCronExecution::new(
    ///     schedule_id,
    ///     None, // pipeline_execution_id will be set after handoff
    ///     scheduled_time,
    /// );
    /// let audit_record = dal.cron_execution().create(new_execution)?;
    ///
    /// // Now hand off to executor
    /// let result = executor.execute(workflow_name, context).await?;
    ///
    /// // Update audit record with pipeline execution ID
    /// dal.cron_execution().update_pipeline_execution_id(
    ///     audit_record.id,
    ///     result.execution_id
    /// )?;
    /// ```
    pub async fn create(
        &self,
        mut new_execution: NewCronExecution,
    ) -> Result<CronExecution, ValidationError> {
        let mut conn = self.dal.pool.get().await?;

        // For PostgreSQL, use database defaults for timestamps by setting to None
        // The database will automatically set CURRENT_TIMESTAMP
        new_execution.claimed_at = None;
        new_execution.created_at = None;
        new_execution.updated_at = None;

        let execution: CronExecution = diesel::insert_into(cron_executions::table)
            .values(&new_execution)
            .get_result(&mut *conn)?;

        Ok(execution)
    }

    /// Updates the pipeline execution ID for an existing cron execution record.
    ///
    /// This method is called after successfully handing off to the pipeline executor
    /// to link the audit record with the actual pipeline execution.
    ///
    /// # Arguments
    /// * `cron_execution_id` - UUID of the cron execution audit record
    /// * `pipeline_execution_id` - UUID of the created pipeline execution
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub async fn update_pipeline_execution_id(
        &self,
        cron_execution_id: UniversalUuid,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let mut conn = self.dal.pool.get().await?;
        let uuid_id: Uuid = cron_execution_id.into();
        let pipeline_uuid: Uuid = pipeline_execution_id.into();
        let now_ts = UniversalTimestamp::now();

        diesel::update(cron_executions::table.find(uuid_id))
            .set((
                cron_executions::pipeline_execution_id.eq(pipeline_uuid),
                cron_executions::updated_at.eq(now_ts),
            ))
            .execute(&mut *conn)?;

        Ok(())
    }

    /// Finds "lost" executions that need recovery.
    ///
    /// This method identifies cron executions that were claimed and recorded
    /// but either failed to hand off to the pipeline executor or the pipeline
    /// execution failed to start properly.
    ///
    /// # Arguments
    /// * `older_than_minutes` - Consider executions lost if claimed more than this many minutes ago
    ///
    /// # Returns
    /// * `Result<Vec<CronExecution>, ValidationError>` - List of lost executions needing recovery
    ///
    /// # Recovery Logic
    /// An execution is considered "lost" if:
    /// 1. It has a cron_executions record (was claimed)
    /// 2. BUT has no corresponding pipeline_executions record (handoff failed)
    /// 3. AND was claimed more than X minutes ago (not just in progress)
    ///
    /// # Example
    /// ```rust
    /// // Find executions lost more than 5 minutes ago
    /// let lost_executions = dal.cron_execution().find_lost_executions(5)?;
    ///
    /// for lost in lost_executions {
    ///     warn!("Lost execution found: schedule {} at {}",
    ///           lost.schedule_id, lost.scheduled_time());
    ///     // Retry the execution
    ///     retry_lost_execution(lost).await?;
    /// }
    /// ```
    pub async fn find_lost_executions(
        &self,
        older_than_minutes: i32,
    ) -> Result<Vec<CronExecution>, ValidationError> {
        let mut conn = self.dal.pool.get().await?;
        let cutoff_time =
            UniversalTimestamp(Utc::now() - chrono::Duration::minutes(older_than_minutes as i64));

        // Find cron executions that don't have corresponding pipeline executions
        // and were claimed before the cutoff time
        let lost_executions = cron_executions::table
            .left_join(
                crate::database::schema::pipeline_executions::table
                    .on(cron_executions::pipeline_execution_id
                        .eq(crate::database::schema::pipeline_executions::id.nullable())),
            )
            .filter(crate::database::schema::pipeline_executions::id.is_null())
            .filter(cron_executions::claimed_at.lt(cutoff_time))
            .select(cron_executions::all_columns)
            .load(&mut *conn)?;

        Ok(lost_executions)
    }

    /// Retrieves a cron execution record by its ID.
    ///
    /// # Arguments
    /// * `id` - UUID of the cron execution record
    ///
    /// # Returns
    /// * `Result<CronExecution, ValidationError>` - The cron execution record
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<CronExecution, ValidationError> {
        let mut conn = self.dal.pool.get().await?;
        let uuid_id: Uuid = id.into();

        let execution = cron_executions::table.find(uuid_id).first(&mut *conn)?;
        Ok(execution)
    }

    /// Retrieves all cron execution records for a specific schedule.
    ///
    /// This method is useful for auditing the execution history of a particular
    /// cron schedule and understanding execution patterns.
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the cron schedule
    /// * `limit` - Maximum number of executions to return
    /// * `offset` - Number of executions to skip
    ///
    /// # Returns
    /// * `Result<Vec<CronExecution>, ValidationError>` - List of execution records
    pub async fn get_by_schedule_id(
        &self,
        schedule_id: UniversalUuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronExecution>, ValidationError> {
        let mut conn = self.dal.pool.get().await?;
        let uuid_id: Uuid = schedule_id.into();

        let executions = cron_executions::table
            .filter(cron_executions::schedule_id.eq(uuid_id))
            .order(cron_executions::scheduled_time.desc())
            .limit(limit)
            .offset(offset)
            .load(&mut *conn)?;

        Ok(executions)
    }

    /// Retrieves the cron execution record for a specific pipeline execution.
    ///
    /// This method enables reverse lookup from pipeline executions back to
    /// their originating cron schedules for debugging and monitoring.
    ///
    /// # Arguments
    /// * `pipeline_execution_id` - UUID of the pipeline execution
    ///
    /// # Returns
    /// * `Result<Option<CronExecution>, ValidationError>` - The cron execution record if it exists
    pub async fn get_by_pipeline_execution_id(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Option<CronExecution>, ValidationError> {
        let mut conn = self.dal.pool.get().await?;
        let uuid_id: Uuid = pipeline_execution_id.into();

        let execution = cron_executions::table
            .filter(cron_executions::pipeline_execution_id.eq(uuid_id))
            .first(&mut *conn)
            .optional()?;

        Ok(execution)
    }

    /// Retrieves cron execution records within a time range.
    ///
    /// This method is useful for analyzing execution patterns, detecting
    /// delays, and generating reports on scheduling performance.
    ///
    /// # Arguments
    /// * `start_time` - Start of the time range (inclusive)
    /// * `end_time` - End of the time range (exclusive)
    /// * `limit` - Maximum number of executions to return
    /// * `offset` - Number of executions to skip
    ///
    /// # Returns
    /// * `Result<Vec<CronExecution>, ValidationError>` - List of execution records in the time range
    pub async fn get_by_time_range(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronExecution>, ValidationError> {
        let mut conn = self.dal.pool.get().await?;
        let start_ts = UniversalTimestamp(start_time);
        let end_ts = UniversalTimestamp(end_time);

        let executions = cron_executions::table
            .filter(cron_executions::scheduled_time.ge(start_ts))
            .filter(cron_executions::scheduled_time.lt(end_ts))
            .order(cron_executions::scheduled_time.desc())
            .limit(limit)
            .offset(offset)
            .load(&mut *conn)?;

        Ok(executions)
    }

    /// Counts the total number of executions for a specific schedule.
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the cron schedule
    ///
    /// # Returns
    /// * `Result<i64, ValidationError>` - Total count of executions
    pub async fn count_by_schedule(&self, schedule_id: UniversalUuid) -> Result<i64, ValidationError> {
        let mut conn = self.dal.pool.get().await?;
        let uuid_id: Uuid = schedule_id.into();

        let count = cron_executions::table
            .filter(cron_executions::schedule_id.eq(uuid_id))
            .count()
            .first(&mut *conn)?;

        Ok(count)
    }

    /// Checks if an execution already exists for a specific schedule and time.
    ///
    /// This method can be used to detect potential duplicate executions
    /// before attempting to create new audit records.
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the cron schedule
    /// * `scheduled_time` - The scheduled execution time
    ///
    /// # Returns
    /// * `Result<bool, ValidationError>` - True if execution already exists
    pub async fn execution_exists(
        &self,
        schedule_id: UniversalUuid,
        scheduled_time: DateTime<Utc>,
    ) -> Result<bool, ValidationError> {
        let mut conn = self.dal.pool.get().await?;
        let uuid_id: Uuid = schedule_id.into();
        let scheduled_ts = UniversalTimestamp(scheduled_time);

        let count: i64 = cron_executions::table
            .filter(cron_executions::schedule_id.eq(uuid_id))
            .filter(cron_executions::scheduled_time.eq(scheduled_ts))
            .count()
            .first(&mut *conn)?;

        Ok(count > 0)
    }

    /// Retrieves the most recent execution for a specific schedule.
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the cron schedule
    ///
    /// # Returns
    /// * `Result<Option<CronExecution>, ValidationError>` - The most recent execution if any exists
    pub async fn get_latest_by_schedule(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<Option<CronExecution>, ValidationError> {
        let mut conn = self.dal.pool.get().await?;
        let uuid_id: Uuid = schedule_id.into();

        let execution = cron_executions::table
            .filter(cron_executions::schedule_id.eq(uuid_id))
            .order(cron_executions::scheduled_time.desc())
            .first(&mut *conn)
            .optional()?;

        Ok(execution)
    }

    /// Deletes old execution records beyond a certain age.
    ///
    /// This method can be used for cleanup and retention management
    /// to prevent the audit table from growing indefinitely.
    ///
    /// # Arguments
    /// * `older_than` - Delete records with scheduled_time older than this timestamp
    ///
    /// # Returns
    /// * `Result<usize, ValidationError>` - Number of records deleted
    pub async fn delete_older_than(&self, older_than: DateTime<Utc>) -> Result<usize, ValidationError> {
        let mut conn = self.dal.pool.get().await?;
        let cutoff_ts = UniversalTimestamp(older_than);

        let deleted_count = diesel::delete(cron_executions::table)
            .filter(cron_executions::scheduled_time.lt(cutoff_ts))
            .execute(&mut *conn)?;

        Ok(deleted_count)
    }

    /// Gets execution statistics for monitoring and alerting.
    ///
    /// This method provides aggregated statistics about cron execution
    /// patterns for monitoring dashboard and alerting systems.
    ///
    /// # Arguments
    /// * `since` - Only include executions since this timestamp
    ///
    /// # Returns
    /// * `Result<CronExecutionStats, ValidationError>` - Aggregated statistics
    pub async fn get_execution_stats(
        &self,
        since: DateTime<Utc>,
    ) -> Result<CronExecutionStats, ValidationError> {
        let mut conn = self.dal.pool.get().await?;
        let since_ts = UniversalTimestamp(since);

        // Get total executions
        let total_executions = cron_executions::table
            .filter(cron_executions::claimed_at.ge(since_ts))
            .count()
            .first(&mut *conn)?;

        // Get successful executions (those with pipeline executions)
        let successful_executions = cron_executions::table
            .inner_join(crate::database::schema::pipeline_executions::table)
            .filter(cron_executions::claimed_at.ge(since_ts))
            .count()
            .first(&mut *conn)?;

        // Get lost executions (no pipeline execution after 10 minutes)
        let lost_cutoff = UniversalTimestamp(Utc::now() - chrono::Duration::minutes(10));
        let lost_executions = cron_executions::table
            .left_join(
                crate::database::schema::pipeline_executions::table
                    .on(cron_executions::pipeline_execution_id
                        .eq(crate::database::schema::pipeline_executions::id.nullable())),
            )
            .filter(crate::database::schema::pipeline_executions::id.is_null())
            .filter(cron_executions::claimed_at.ge(since_ts))
            .filter(cron_executions::claimed_at.lt(lost_cutoff))
            .count()
            .first(&mut *conn)?;

        Ok(CronExecutionStats {
            total_executions,
            successful_executions,
            lost_executions,
            success_rate: if total_executions > 0 {
                (successful_executions as f64 / total_executions as f64) * 100.0
            } else {
                0.0
            },
        })
    }
}

/// Statistics about cron execution performance
#[derive(Debug)]
pub struct CronExecutionStats {
    /// Total number of executions attempted
    pub total_executions: i64,
    /// Number of executions that successfully handed off to pipeline executor
    pub successful_executions: i64,
    /// Number of executions that were lost (claimed but never executed)
    pub lost_executions: i64,
    /// Success rate as a percentage
    pub success_rate: f64,
}
