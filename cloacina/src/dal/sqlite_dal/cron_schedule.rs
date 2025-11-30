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

//! Cron Schedule Data Access Layer
//!
//! This module provides the data access layer for managing cron schedules in the database.
//! It handles all database operations related to time-based workflow scheduling.
//!
//! Key features:
//! - Schedule creation, modification, and deletion
//! - Efficient polling for due schedules
//! - Schedule state management (enabled/disabled)
//! - Time window support (start/end dates)
//! - Timezone-aware scheduling
//!
//! The module uses Diesel ORM for database operations and supports atomic updates
//! for schedule timing information.

use super::DAL;
use super::models::{NewSqliteCronSchedule, SqliteCronSchedule, uuid_to_blob, current_timestamp_string};
use crate::database::schema::sqlite::cron_schedules;
use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::cron_schedule::{CronSchedule, NewCronSchedule};
use chrono::{DateTime, Utc};
use diesel::prelude::*;

/// Data Access Layer for cron schedule operations.
///
/// This struct provides methods for managing cron schedules, including creation,
/// retrieval, updates, and polling for due schedules. It maintains a reference
/// to the main DAL for database connection management.
pub struct CronScheduleDAL<'a> {
    pub dal: &'a DAL,
}

impl<'a> CronScheduleDAL<'a> {
    /// Creates a new cron schedule record in the database.
    ///
    /// # Arguments
    /// * `new_schedule` - A `NewCronSchedule` struct containing the schedule details
    ///
    /// # Returns
    /// * `Result<CronSchedule, ValidationError>` - The created cron schedule record
    pub async fn create(
        &self,
        new_schedule: NewCronSchedule,
    ) -> Result<CronSchedule, ValidationError> {
        let conn = self.dal.pool.get().await?;

        // Generate ID and timestamps manually for SQLite
        let id = UniversalUuid::new_v4();
        let now = current_timestamp_string();

        // Create SQLite-specific model with converted types
        let new_sqlite_schedule = NewSqliteCronSchedule {
            id: uuid_to_blob(&id.0),
            workflow_name: new_schedule.workflow_name,
            cron_expression: new_schedule.cron_expression,
            timezone: new_schedule.timezone.unwrap_or_else(|| "UTC".to_string()),
            enabled: if new_schedule.enabled.unwrap_or_else(|| UniversalBool::from(true)).0 { 1 } else { 0 },
            catchup_policy: new_schedule.catchup_policy.unwrap_or_else(|| "skip".to_string()),
            start_date: new_schedule.start_date.map(|ts| ts.0.to_rfc3339()),
            end_date: new_schedule.end_date.map(|ts| ts.0.to_rfc3339()),
            next_run_at: new_schedule.next_run_at.0.to_rfc3339(),
            created_at: now.clone(),
            updated_at: now,
        };

        // Insert with SQLite-specific types
        conn.interact(move |conn| {
            diesel::insert_into(cron_schedules::table)
                .values(&new_sqlite_schedule)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        // Retrieve the created record
        let id_blob = uuid_to_blob(&id.0);
        let schedule: SqliteCronSchedule = conn
            .interact(move |conn| {
                cron_schedules::table
                    .filter(cron_schedules::id.eq(&id_blob))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(schedule.into())
    }

    /// Retrieves a cron schedule by its ID.
    ///
    /// # Arguments
    /// * `id` - UUID of the cron schedule
    ///
    /// # Returns
    /// * `Result<CronSchedule, ValidationError>` - The cron schedule record
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<CronSchedule, ValidationError> {
        let conn = self.dal.pool.get().await?;

        let id_blob = uuid_to_blob(&id.0);
        let schedule: SqliteCronSchedule = conn
            .interact(move |conn| {
                cron_schedules::table
                    .filter(cron_schedules::id.eq(&id_blob))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(schedule.into())
    }

    /// Retrieves all enabled cron schedules that are due for execution.
    ///
    /// This method finds schedules where:
    /// - enabled = 1 (true in SQLite)
    /// - next_run_at <= now
    /// - start_date is NULL or <= now
    /// - end_date is NULL or >= now
    ///
    /// # Arguments
    /// * `now` - Current timestamp to check against
    ///
    /// # Returns
    /// * `Result<Vec<CronSchedule>, ValidationError>` - List of due schedules
    pub async fn get_due_schedules(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        let conn = self.dal.pool.get().await?;
        let now_ts = UniversalTimestamp(now);

        let now_str = now_ts.0.to_rfc3339();
        let schedules: Vec<SqliteCronSchedule> = conn
            .interact(move |conn| {
                cron_schedules::table
                    .filter(cron_schedules::enabled.eq(1))
                    .filter(cron_schedules::next_run_at.le(now_str.clone()))
                    .filter(
                        cron_schedules::start_date
                            .is_null()
                            .or(cron_schedules::start_date.le(now_str.clone())),
                    )
                    .filter(
                        cron_schedules::end_date
                            .is_null()
                            .or(cron_schedules::end_date.ge(now_str)),
                    )
                    .order(cron_schedules::next_run_at.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(schedules.into_iter().map(|s| s.into()).collect())
    }

    /// Updates the last run and next run times for a cron schedule.
    ///
    /// This method atomically updates both timing fields and the updated_at timestamp.
    ///
    /// # Arguments
    /// * `id` - UUID of the cron schedule
    /// * `last_run` - Timestamp when the schedule was last executed
    /// * `next_run` - Timestamp when the schedule should next execute
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub async fn update_schedule_times(
        &self,
        id: UniversalUuid,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self.dal.pool.get().await?;
        let id_blob = uuid_to_blob(&id.0);
        let last_run_str = last_run.to_rfc3339();
        let next_run_str = next_run.to_rfc3339();
        let now_str = current_timestamp_string();

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.filter(cron_schedules::id.eq(&id_blob)))
                .set((
                    cron_schedules::last_run_at.eq(Some(last_run_str)),
                    cron_schedules::next_run_at.eq(next_run_str),
                    cron_schedules::updated_at.eq(now_str),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Updates the cron expression, timezone, and next run time for a schedule.
    ///
    /// # Arguments
    /// * `id` - UUID of the cron schedule
    /// * `cron_expression` - Optional new cron expression
    /// * `timezone` - Optional new timezone
    /// * `next_run` - New next run time (calculated from updated expression/timezone)
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub async fn update_expression_and_timezone(
        &self,
        id: UniversalUuid,
        cron_expression: Option<&str>,
        timezone: Option<&str>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self.dal.pool.get().await?;
        let id_blob = uuid_to_blob(&id.0);
        let next_run_str = next_run.to_rfc3339();
        let now_str = current_timestamp_string();

        // Convert optional references to owned strings for the closure
        let cron_expr_owned = cron_expression.map(|s| s.to_string());
        let timezone_owned = timezone.map(|s| s.to_string());

        // Build the update values dynamically based on what's provided
        conn.interact(move |conn| {
            let query = diesel::update(cron_schedules::table.filter(cron_schedules::id.eq(&id_blob)));

            if let (Some(ref expr), Some(ref tz)) = (&cron_expr_owned, &timezone_owned) {
                // Update both expression and timezone
                query
                    .set((
                        cron_schedules::cron_expression.eq(expr),
                        cron_schedules::timezone.eq(tz),
                        cron_schedules::next_run_at.eq(&next_run_str),
                        cron_schedules::updated_at.eq(&now_str),
                    ))
                    .execute(conn)
            } else if let Some(ref expr) = &cron_expr_owned {
                // Update only expression
                query
                    .set((
                        cron_schedules::cron_expression.eq(expr),
                        cron_schedules::next_run_at.eq(&next_run_str),
                        cron_schedules::updated_at.eq(&now_str),
                    ))
                    .execute(conn)
            } else if let Some(ref tz) = &timezone_owned {
                // Update only timezone
                query
                    .set((
                        cron_schedules::timezone.eq(tz),
                        cron_schedules::next_run_at.eq(&next_run_str),
                        cron_schedules::updated_at.eq(&now_str),
                    ))
                    .execute(conn)
            } else {
                // Update only next run time
                query
                    .set((
                        cron_schedules::next_run_at.eq(&next_run_str),
                        cron_schedules::updated_at.eq(&now_str),
                    ))
                    .execute(conn)
            }
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Enables a cron schedule.
    ///
    /// # Arguments
    /// * `id` - UUID of the cron schedule
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub async fn enable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self.dal.pool.get().await?;
        let id_blob = uuid_to_blob(&id.0);
        let now_str = current_timestamp_string();

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.filter(cron_schedules::id.eq(&id_blob)))
                .set((
                    cron_schedules::enabled.eq(1),
                    cron_schedules::updated_at.eq(now_str),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Disables a cron schedule.
    ///
    /// # Arguments
    /// * `id` - UUID of the cron schedule
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub async fn disable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self.dal.pool.get().await?;
        let id_blob = uuid_to_blob(&id.0);
        let now_str = current_timestamp_string();

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.filter(cron_schedules::id.eq(&id_blob)))
                .set((
                    cron_schedules::enabled.eq(0),
                    cron_schedules::updated_at.eq(now_str),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Deletes a cron schedule from the database.
    ///
    /// # Arguments
    /// * `id` - UUID of the cron schedule
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub async fn delete(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self.dal.pool.get().await?;
        let id_blob = uuid_to_blob(&id.0);

        conn.interact(move |conn| {
            diesel::delete(cron_schedules::table.filter(cron_schedules::id.eq(&id_blob)))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }

    /// Lists all cron schedules with optional filtering.
    ///
    /// # Arguments
    /// * `enabled_only` - If true, only return enabled schedules
    /// * `limit` - Maximum number of schedules to return
    /// * `offset` - Number of schedules to skip
    ///
    /// # Returns
    /// * `Result<Vec<CronSchedule>, ValidationError>` - List of cron schedules
    pub async fn list(
        &self,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        let conn = self.dal.pool.get().await?;

        let schedules: Vec<SqliteCronSchedule> = conn
            .interact(move |conn| {
                let mut query = cron_schedules::table.into_boxed();

                if enabled_only {
                    query = query.filter(cron_schedules::enabled.eq(1));
                }

                query
                    .order(cron_schedules::workflow_name.asc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(schedules.into_iter().map(|s| s.into()).collect())
    }

    /// Finds cron schedules by workflow name.
    ///
    /// # Arguments
    /// * `workflow_name` - Name of the workflow to search for
    ///
    /// # Returns
    /// * `Result<Vec<CronSchedule>, ValidationError>` - List of matching schedules
    pub async fn find_by_workflow(
        &self,
        workflow_name: &str,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        let conn = self.dal.pool.get().await?;
        let workflow_name = workflow_name.to_string();

        let schedules: Vec<SqliteCronSchedule> = conn
            .interact(move |conn| {
                cron_schedules::table
                    .filter(cron_schedules::workflow_name.eq(workflow_name))
                    .order(cron_schedules::created_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(schedules.into_iter().map(|s| s.into()).collect())
    }

    /// Updates the next run time for a cron schedule.
    ///
    /// This is useful for recalculating schedule times without updating the last run time.
    ///
    /// # Arguments
    /// * `id` - UUID of the cron schedule
    /// * `next_run` - New next run timestamp
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub async fn update_next_run(
        &self,
        id: UniversalUuid,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self.dal.pool.get().await?;
        let id_blob = uuid_to_blob(&id.0);
        let next_run_str = next_run.to_rfc3339();
        let now_str = current_timestamp_string();

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.filter(cron_schedules::id.eq(&id_blob)))
                .set((
                    cron_schedules::next_run_at.eq(next_run_str),
                    cron_schedules::updated_at.eq(now_str),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Atomically claims and updates a cron schedule's timing.
    ///
    /// This method implements the atomic claiming pattern by only updating the schedule
    /// timing if the schedule is still due. This prevents duplicate executions across
    /// multiple scheduler instances or during service interruptions.
    ///
    /// # Arguments
    /// * `id` - UUID of the cron schedule
    /// * `current_time` - Current timestamp to compare against next_run_at
    /// * `last_run` - Timestamp when the schedule was executed
    /// * `next_run` - Timestamp when the schedule should next execute
    ///
    /// # Returns
    /// * `Result<bool, ValidationError>` - True if claim was successful, false if schedule was no longer due
    ///
    /// # Example
    /// ```rust
    /// let now = Utc::now();
    /// let next_run = evaluator.next_execution(now)?;
    ///
    /// if dal.cron_schedule().claim_and_update(schedule_id, now, now, next_run)? {
    ///     // Successfully claimed the schedule, proceed with execution
    ///     execute_workflow(&schedule).await?;
    /// } else {
    ///     // Schedule was already claimed by another instance, skip
    /// }
    /// ```
    pub async fn claim_and_update(
        &self,
        id: UniversalUuid,
        current_time: DateTime<Utc>,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<bool, ValidationError> {
        let conn = self.dal.pool.get().await?;
        let id_blob = uuid_to_blob(&id.0);
        let current_str = current_time.to_rfc3339();
        let last_run_str = last_run.to_rfc3339();
        let next_run_str = next_run.to_rfc3339();
        let now_str = current_timestamp_string();

        // Atomic update: only update if schedule is still due and enabled
        let updated_rows = conn
            .interact(move |conn| {
                diesel::update(cron_schedules::table.filter(cron_schedules::id.eq(&id_blob)))
                    .filter(cron_schedules::next_run_at.le(current_str))
                    .filter(cron_schedules::enabled.eq(1))
                    .set((
                        cron_schedules::last_run_at.eq(Some(last_run_str)),
                        cron_schedules::next_run_at.eq(next_run_str),
                        cron_schedules::updated_at.eq(now_str),
                    ))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        // Return true if exactly one row was updated (successful claim)
        Ok(updated_rows == 1)
    }

    /// Counts the total number of cron schedules.
    ///
    /// # Arguments
    /// * `enabled_only` - If true, only count enabled schedules
    ///
    /// # Returns
    /// * `Result<i64, ValidationError>` - Total count of schedules
    pub async fn count(&self, enabled_only: bool) -> Result<i64, ValidationError> {
        let conn = self.dal.pool.get().await?;

        let count = conn
            .interact(move |conn| {
                let mut query = cron_schedules::table.into_boxed();

                if enabled_only {
                    query = query.filter(cron_schedules::enabled.eq(1));
                }

                query.count().first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(count)
    }
}
