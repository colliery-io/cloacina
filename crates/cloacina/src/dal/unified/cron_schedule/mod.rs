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

//! Unified Cron Schedule DAL with runtime backend selection
//!
//! This module provides operations for CronSchedule entities that work with
//! both PostgreSQL and SQLite backends, selecting the appropriate implementation
//! at runtime based on the database connection type.

mod crud;
mod queries;
mod state;

use super::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::error::ValidationError;
use crate::models::cron_schedule::{CronSchedule, NewCronSchedule};
use chrono::{DateTime, Utc};

/// Data access layer for cron schedule operations with runtime backend selection.
#[derive(Clone)]
pub struct CronScheduleDAL<'a> {
    dal: &'a DAL,
}

impl<'a> CronScheduleDAL<'a> {
    /// Creates a new CronScheduleDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new cron schedule record in the database.
    pub async fn create(
        &self,
        new_schedule: NewCronSchedule,
    ) -> Result<CronSchedule, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_schedule).await,
            self.create_sqlite(new_schedule).await
        )
    }

    /// Retrieves a cron schedule by its ID.
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<CronSchedule, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_id_postgres(id).await,
            self.get_by_id_sqlite(id).await
        )
    }

    /// Retrieves all enabled cron schedules that are due for execution.
    pub async fn get_due_schedules(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_due_schedules_postgres(now).await,
            self.get_due_schedules_sqlite(now).await
        )
    }

    /// Updates the last run and next run times for a cron schedule.
    pub async fn update_schedule_times(
        &self,
        id: UniversalUuid,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.update_schedule_times_postgres(id, last_run, next_run)
                .await,
            self.update_schedule_times_sqlite(id, last_run, next_run)
                .await
        )
    }

    /// Enables a cron schedule.
    pub async fn enable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.enable_postgres(id).await,
            self.enable_sqlite(id).await
        )
    }

    /// Disables a cron schedule.
    pub async fn disable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.disable_postgres(id).await,
            self.disable_sqlite(id).await
        )
    }

    /// Deletes a cron schedule from the database.
    pub async fn delete(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.delete_postgres(id).await,
            self.delete_sqlite(id).await
        )
    }

    /// Lists all cron schedules with optional filtering.
    pub async fn list(
        &self,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_postgres(enabled_only, limit, offset).await,
            self.list_sqlite(enabled_only, limit, offset).await
        )
    }

    /// Finds cron schedules by workflow name.
    pub async fn find_by_workflow(
        &self,
        workflow_name: &str,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.find_by_workflow_postgres(workflow_name).await,
            self.find_by_workflow_sqlite(workflow_name).await
        )
    }

    /// Updates the next run time for a cron schedule.
    pub async fn update_next_run(
        &self,
        id: UniversalUuid,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.update_next_run_postgres(id, next_run).await,
            self.update_next_run_sqlite(id, next_run).await
        )
    }

    /// Atomically claims and updates a cron schedule's timing.
    pub async fn claim_and_update(
        &self,
        id: UniversalUuid,
        current_time: DateTime<Utc>,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<bool, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.claim_and_update_postgres(id, current_time, last_run, next_run)
                .await,
            self.claim_and_update_sqlite(id, current_time, last_run, next_run)
                .await
        )
    }

    /// Counts the total number of cron schedules.
    pub async fn count(&self, enabled_only: bool) -> Result<i64, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.count_postgres(enabled_only).await,
            self.count_sqlite(enabled_only).await
        )
    }

    /// Updates the cron expression, timezone, and next run time for a schedule.
    pub async fn update_expression_and_timezone(
        &self,
        id: UniversalUuid,
        cron_expression: Option<&str>,
        timezone: Option<&str>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.update_expression_and_timezone_postgres(id, cron_expression, timezone, next_run)
                .await,
            self.update_expression_and_timezone_sqlite(id, cron_expression, timezone, next_run)
                .await
        )
    }
}
