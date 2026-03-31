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

//! Unified Schedule DAL with runtime backend selection
//!
//! This module provides operations for the unified `schedules` table that
//! replaces the separate `cron_schedules` and `trigger_schedules` tables.
//! Works with both PostgreSQL and SQLite backends, selecting the appropriate
//! implementation at runtime based on the database connection type.

mod crud;

use super::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::error::ValidationError;
use crate::models::schedule::{NewSchedule, Schedule};
use chrono::{DateTime, Utc};

/// Data access layer for unified schedule operations with runtime backend selection.
#[derive(Clone)]
pub struct ScheduleDAL<'a> {
    dal: &'a DAL,
}

impl<'a> ScheduleDAL<'a> {
    /// Creates a new ScheduleDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new schedule record in the database.
    pub async fn create(&self, new_schedule: NewSchedule) -> Result<Schedule, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_schedule).await,
            self.create_sqlite(new_schedule).await
        )
    }

    /// Retrieves a schedule by its ID.
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<Schedule, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_id_postgres(id).await,
            self.get_by_id_sqlite(id).await
        )
    }

    /// Lists schedules with optional filtering by type and enabled status.
    pub async fn list(
        &self,
        schedule_type: Option<&str>,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let schedule_type_owned = schedule_type.map(|s| s.to_string());
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_postgres(schedule_type_owned.clone(), enabled_only, limit, offset)
                .await,
            self.list_sqlite(schedule_type_owned, enabled_only, limit, offset)
                .await
        )
    }

    /// Enables a schedule.
    pub async fn enable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.enable_postgres(id).await,
            self.enable_sqlite(id).await
        )
    }

    /// Disables a schedule.
    pub async fn disable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.disable_postgres(id).await,
            self.disable_sqlite(id).await
        )
    }

    /// Deletes a schedule from the database.
    pub async fn delete(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.delete_postgres(id).await,
            self.delete_sqlite(id).await
        )
    }

    /// Retrieves all enabled cron schedules that are due for execution.
    pub async fn get_due_cron_schedules(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<Schedule>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_due_cron_schedules_postgres(now).await,
            self.get_due_cron_schedules_sqlite(now).await
        )
    }

    /// Atomically claims and updates a cron schedule's timing.
    pub async fn claim_and_update_cron(
        &self,
        id: UniversalUuid,
        current_time: DateTime<Utc>,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<bool, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.claim_and_update_cron_postgres(id, current_time, last_run, next_run)
                .await,
            self.claim_and_update_cron_sqlite(id, current_time, last_run, next_run)
                .await
        )
    }

    /// Updates the last run and next run times for a schedule.
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

    /// Retrieves all enabled trigger schedules.
    pub async fn get_enabled_triggers(&self) -> Result<Vec<Schedule>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_enabled_triggers_postgres().await,
            self.get_enabled_triggers_sqlite().await
        )
    }

    /// Updates the last poll time for a trigger schedule.
    pub async fn update_last_poll(
        &self,
        id: UniversalUuid,
        last_poll_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.update_last_poll_postgres(id, last_poll_at).await,
            self.update_last_poll_sqlite(id, last_poll_at).await
        )
    }

    /// Upserts a trigger schedule by trigger_name.
    pub async fn upsert_trigger(
        &self,
        new_schedule: NewSchedule,
    ) -> Result<Schedule, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.upsert_trigger_postgres(new_schedule).await,
            self.upsert_trigger_sqlite(new_schedule).await
        )
    }

    /// Retrieves a schedule by its trigger name.
    pub async fn get_by_trigger_name(
        &self,
        name: &str,
    ) -> Result<Option<Schedule>, ValidationError> {
        let name_owned = name.to_string();
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_trigger_name_postgres(name_owned.clone()).await,
            self.get_by_trigger_name_sqlite(name_owned).await
        )
    }

    /// Finds schedules by workflow name.
    pub async fn find_by_workflow(
        &self,
        workflow_name: &str,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let name_owned = workflow_name.to_string();
        crate::dispatch_backend!(
            self.dal.backend(),
            self.find_by_workflow_postgres(name_owned.clone()).await,
            self.find_by_workflow_sqlite(name_owned).await
        )
    }

    /// Updates the cron expression and timezone for a cron schedule.
    pub async fn update_cron_expression_and_timezone(
        &self,
        id: UniversalUuid,
        cron_expression: Option<&str>,
        timezone: Option<&str>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let cron_expression_owned = cron_expression.map(|s| s.to_string());
        let timezone_owned = timezone.map(|s| s.to_string());
        crate::dispatch_backend!(
            self.dal.backend(),
            self.update_cron_expression_and_timezone_postgres(
                id,
                cron_expression_owned.clone(),
                timezone_owned.clone(),
                next_run
            )
            .await,
            self.update_cron_expression_and_timezone_sqlite(
                id,
                cron_expression_owned,
                timezone_owned,
                next_run
            )
            .await
        )
    }
}
