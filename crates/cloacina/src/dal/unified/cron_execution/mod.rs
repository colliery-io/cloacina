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

//! Unified Cron Execution DAL with runtime backend selection
//!
//! This module provides operations for CronExecution audit records that work with
//! both PostgreSQL and SQLite backends, selecting the appropriate implementation
//! at runtime based on the database connection type.

mod crud;
mod queries;
mod tracking;

use super::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::error::ValidationError;
use crate::models::cron_execution::{CronExecution, NewCronExecution};
use chrono::{DateTime, Utc};

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

/// Data access layer for cron execution operations with runtime backend selection.
#[derive(Clone)]
pub struct CronExecutionDAL<'a> {
    dal: &'a DAL,
}

impl<'a> CronExecutionDAL<'a> {
    /// Creates a new CronExecutionDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new cron execution audit record in the database.
    pub async fn create(
        &self,
        new_execution: NewCronExecution,
    ) -> Result<CronExecution, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_execution).await,
            self.create_sqlite(new_execution).await
        )
    }

    /// Updates the pipeline execution ID for an existing cron execution record.
    pub async fn update_pipeline_execution_id(
        &self,
        cron_execution_id: UniversalUuid,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.update_pipeline_execution_id_postgres(cron_execution_id, pipeline_execution_id)
                .await,
            self.update_pipeline_execution_id_sqlite(cron_execution_id, pipeline_execution_id)
                .await
        )
    }

    /// Finds "lost" executions that need recovery.
    pub async fn find_lost_executions(
        &self,
        older_than_minutes: i32,
    ) -> Result<Vec<CronExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.find_lost_executions_postgres(older_than_minutes).await,
            self.find_lost_executions_sqlite(older_than_minutes).await
        )
    }

    /// Retrieves a cron execution record by its ID.
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<CronExecution, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_id_postgres(id).await,
            self.get_by_id_sqlite(id).await
        )
    }

    /// Retrieves all cron execution records for a specific schedule.
    pub async fn get_by_schedule_id(
        &self,
        schedule_id: UniversalUuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_schedule_id_postgres(schedule_id, limit, offset)
                .await,
            self.get_by_schedule_id_sqlite(schedule_id, limit, offset)
                .await
        )
    }

    /// Retrieves the cron execution record for a specific pipeline execution.
    pub async fn get_by_pipeline_execution_id(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Option<CronExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_pipeline_execution_id_postgres(pipeline_execution_id)
                .await,
            self.get_by_pipeline_execution_id_sqlite(pipeline_execution_id)
                .await
        )
    }

    /// Retrieves cron execution records within a time range.
    pub async fn get_by_time_range(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_time_range_postgres(start_time, end_time, limit, offset)
                .await,
            self.get_by_time_range_sqlite(start_time, end_time, limit, offset)
                .await
        )
    }

    /// Counts the total number of executions for a specific schedule.
    pub async fn count_by_schedule(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<i64, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.count_by_schedule_postgres(schedule_id).await,
            self.count_by_schedule_sqlite(schedule_id).await
        )
    }

    /// Checks if an execution already exists for a specific schedule and time.
    pub async fn execution_exists(
        &self,
        schedule_id: UniversalUuid,
        scheduled_time: DateTime<Utc>,
    ) -> Result<bool, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.execution_exists_postgres(schedule_id, scheduled_time)
                .await,
            self.execution_exists_sqlite(schedule_id, scheduled_time)
                .await
        )
    }

    /// Retrieves the most recent execution for a specific schedule.
    pub async fn get_latest_by_schedule(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<Option<CronExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_latest_by_schedule_postgres(schedule_id).await,
            self.get_latest_by_schedule_sqlite(schedule_id).await
        )
    }

    /// Deletes old execution records beyond a certain age.
    pub async fn delete_older_than(
        &self,
        older_than: DateTime<Utc>,
    ) -> Result<usize, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.delete_older_than_postgres(older_than).await,
            self.delete_older_than_sqlite(older_than).await
        )
    }

    /// Gets execution statistics for monitoring and alerting.
    pub async fn get_execution_stats(
        &self,
        since: DateTime<Utc>,
    ) -> Result<CronExecutionStats, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_execution_stats_postgres(since).await,
            self.get_execution_stats_sqlite(since).await
        )
    }
}
