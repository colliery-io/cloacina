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

use super::models::{NewUnifiedCronSchedule, UnifiedCronSchedule};
use super::DAL;
use crate::database::schema::unified::cron_schedules;
use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
use crate::database::BackendType;
use crate::error::ValidationError;
use crate::models::cron_schedule::{CronSchedule, NewCronSchedule};
use chrono::{DateTime, Utc};
use diesel::prelude::*;

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
        match self.dal.backend() {
            BackendType::Postgres => self.create_postgres(new_schedule).await,
            BackendType::Sqlite => self.create_sqlite(new_schedule).await,
        }
    }

    async fn create_postgres(
        &self,
        new_schedule: NewCronSchedule,
    ) -> Result<CronSchedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedCronSchedule {
            id,
            workflow_name: new_schedule.workflow_name,
            cron_expression: new_schedule.cron_expression,
            timezone: new_schedule.timezone.unwrap_or_else(|| "UTC".to_string()),
            enabled: UniversalBool::from(new_schedule.enabled.map(|b| b.into()).unwrap_or(true)),
            catchup_policy: new_schedule
                .catchup_policy
                .unwrap_or_else(|| "skip".to_string()),
            start_date: new_schedule.start_date,
            end_date: new_schedule.end_date,
            next_run_at: new_schedule.next_run_at,
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(cron_schedules::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedCronSchedule = conn
            .interact(move |conn| cron_schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    async fn create_sqlite(
        &self,
        new_schedule: NewCronSchedule,
    ) -> Result<CronSchedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedCronSchedule {
            id,
            workflow_name: new_schedule.workflow_name,
            cron_expression: new_schedule.cron_expression,
            timezone: new_schedule.timezone.unwrap_or_else(|| "UTC".to_string()),
            enabled: UniversalBool::from(new_schedule.enabled.map(|b| b.into()).unwrap_or(true)),
            catchup_policy: new_schedule
                .catchup_policy
                .unwrap_or_else(|| "skip".to_string()),
            start_date: new_schedule.start_date,
            end_date: new_schedule.end_date,
            next_run_at: new_schedule.next_run_at,
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(cron_schedules::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedCronSchedule = conn
            .interact(move |conn| cron_schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    /// Retrieves a cron schedule by its ID.
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<CronSchedule, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.get_by_id_postgres(id).await,
            BackendType::Sqlite => self.get_by_id_sqlite(id).await,
        }
    }

    async fn get_by_id_postgres(&self, id: UniversalUuid) -> Result<CronSchedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedCronSchedule = conn
            .interact(move |conn| cron_schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    async fn get_by_id_sqlite(&self, id: UniversalUuid) -> Result<CronSchedule, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedCronSchedule = conn
            .interact(move |conn| cron_schedules::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    /// Retrieves all enabled cron schedules that are due for execution.
    pub async fn get_due_schedules(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.get_due_schedules_postgres(now).await,
            BackendType::Sqlite => self.get_due_schedules_sqlite(now).await,
        }
    }

    async fn get_due_schedules_postgres(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now_ts = UniversalTimestamp::from(now);
        let enabled_true = UniversalBool::from(true);
        let results: Vec<UnifiedCronSchedule> = conn
            .interact(move |conn| {
                diesel::sql_query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE").execute(conn)?;

                let schedules = cron_schedules::table
                    .filter(cron_schedules::enabled.eq(enabled_true))
                    .filter(cron_schedules::next_run_at.le(now_ts))
                    .filter(
                        cron_schedules::start_date
                            .is_null()
                            .or(cron_schedules::start_date.le(now_ts)),
                    )
                    .filter(
                        cron_schedules::end_date
                            .is_null()
                            .or(cron_schedules::end_date.ge(now_ts)),
                    )
                    .order(cron_schedules::next_run_at.asc())
                    .load(conn)?;

                Ok::<_, diesel::result::Error>(schedules)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    async fn get_due_schedules_sqlite(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now_ts = UniversalTimestamp::from(now);
        let enabled_true = UniversalBool::from(true);
        let results: Vec<UnifiedCronSchedule> = conn
            .interact(move |conn| {
                cron_schedules::table
                    .filter(cron_schedules::enabled.eq(enabled_true))
                    .filter(cron_schedules::next_run_at.le(now_ts))
                    .filter(
                        cron_schedules::start_date
                            .is_null()
                            .or(cron_schedules::start_date.le(now_ts)),
                    )
                    .filter(
                        cron_schedules::end_date
                            .is_null()
                            .or(cron_schedules::end_date.ge(now_ts)),
                    )
                    .order(cron_schedules::next_run_at.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Updates the last run and next run times for a cron schedule.
    pub async fn update_schedule_times(
        &self,
        id: UniversalUuid,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.update_schedule_times_postgres(id, last_run, next_run)
                    .await
            }
            BackendType::Sqlite => {
                self.update_schedule_times_sqlite(id, last_run, next_run)
                    .await
            }
        }
    }

    async fn update_schedule_times_postgres(
        &self,
        id: UniversalUuid,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let last_run_ts = UniversalTimestamp::from(last_run);
        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::last_run_at.eq(Some(last_run_ts)),
                    cron_schedules::next_run_at.eq(next_run_ts),
                    cron_schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn update_schedule_times_sqlite(
        &self,
        id: UniversalUuid,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let last_run_ts = UniversalTimestamp::from(last_run);
        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::last_run_at.eq(Some(last_run_ts)),
                    cron_schedules::next_run_at.eq(next_run_ts),
                    cron_schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Enables a cron schedule.
    pub async fn enable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.enable_postgres(id).await,
            BackendType::Sqlite => self.enable_sqlite(id).await,
        }
    }

    async fn enable_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let enabled_true = UniversalBool::from(true);

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::enabled.eq(enabled_true),
                    cron_schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn enable_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let enabled_true = UniversalBool::from(true);

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::enabled.eq(enabled_true),
                    cron_schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Disables a cron schedule.
    pub async fn disable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.disable_postgres(id).await,
            BackendType::Sqlite => self.disable_sqlite(id).await,
        }
    }

    async fn disable_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let enabled_false = UniversalBool::from(false);

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::enabled.eq(enabled_false),
                    cron_schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn disable_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let enabled_false = UniversalBool::from(false);

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::enabled.eq(enabled_false),
                    cron_schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Deletes a cron schedule from the database.
    pub async fn delete(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.delete_postgres(id).await,
            BackendType::Sqlite => self.delete_sqlite(id).await,
        }
    }

    async fn delete_postgres(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| diesel::delete(cron_schedules::table.find(id)).execute(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn delete_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| diesel::delete(cron_schedules::table.find(id)).execute(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Lists all cron schedules with optional filtering.
    pub async fn list(
        &self,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.list_postgres(enabled_only, limit, offset).await,
            BackendType::Sqlite => self.list_sqlite(enabled_only, limit, offset).await,
        }
    }

    async fn list_postgres(
        &self,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let enabled_true = UniversalBool::from(true);
        let results: Vec<UnifiedCronSchedule> = conn
            .interact(move |conn| {
                let mut query = cron_schedules::table.into_boxed();

                if enabled_only {
                    query = query.filter(cron_schedules::enabled.eq(enabled_true));
                }

                query
                    .order(cron_schedules::workflow_name.asc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    async fn list_sqlite(
        &self,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let enabled_true = UniversalBool::from(true);
        let results: Vec<UnifiedCronSchedule> = conn
            .interact(move |conn| {
                let mut query = cron_schedules::table.into_boxed();

                if enabled_only {
                    query = query.filter(cron_schedules::enabled.eq(enabled_true));
                }

                query
                    .order(cron_schedules::workflow_name.asc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Finds cron schedules by workflow name.
    pub async fn find_by_workflow(
        &self,
        workflow_name: &str,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.find_by_workflow_postgres(workflow_name).await,
            BackendType::Sqlite => self.find_by_workflow_sqlite(workflow_name).await,
        }
    }

    async fn find_by_workflow_postgres(
        &self,
        workflow_name: &str,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let workflow_name = workflow_name.to_string();
        let results: Vec<UnifiedCronSchedule> = conn
            .interact(move |conn| {
                cron_schedules::table
                    .filter(cron_schedules::workflow_name.eq(workflow_name))
                    .order(cron_schedules::created_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    async fn find_by_workflow_sqlite(
        &self,
        workflow_name: &str,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let workflow_name = workflow_name.to_string();
        let results: Vec<UnifiedCronSchedule> = conn
            .interact(move |conn| {
                cron_schedules::table
                    .filter(cron_schedules::workflow_name.eq(workflow_name))
                    .order(cron_schedules::created_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Updates the next run time for a cron schedule.
    pub async fn update_next_run(
        &self,
        id: UniversalUuid,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.update_next_run_postgres(id, next_run).await,
            BackendType::Sqlite => self.update_next_run_sqlite(id, next_run).await,
        }
    }

    async fn update_next_run_postgres(
        &self,
        id: UniversalUuid,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::next_run_at.eq(next_run_ts),
                    cron_schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn update_next_run_sqlite(
        &self,
        id: UniversalUuid,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(id))
                .set((
                    cron_schedules::next_run_at.eq(next_run_ts),
                    cron_schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Atomically claims and updates a cron schedule's timing.
    pub async fn claim_and_update(
        &self,
        id: UniversalUuid,
        current_time: DateTime<Utc>,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<bool, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.claim_and_update_postgres(id, current_time, last_run, next_run)
                    .await
            }
            BackendType::Sqlite => {
                self.claim_and_update_sqlite(id, current_time, last_run, next_run)
                    .await
            }
        }
    }

    async fn claim_and_update_postgres(
        &self,
        id: UniversalUuid,
        current_time: DateTime<Utc>,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let current_ts = UniversalTimestamp::from(current_time);
        let last_run_ts = UniversalTimestamp::from(last_run);
        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();
        let enabled_true = UniversalBool::from(true);

        let updated_rows = conn
            .interact(move |conn| {
                diesel::sql_query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE").execute(conn)?;

                let updated_rows = diesel::update(cron_schedules::table.find(id))
                    .filter(cron_schedules::next_run_at.le(current_ts))
                    .filter(cron_schedules::enabled.eq(enabled_true))
                    .set((
                        cron_schedules::last_run_at.eq(Some(last_run_ts)),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                Ok::<_, diesel::result::Error>(updated_rows)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(updated_rows == 1)
    }

    async fn claim_and_update_sqlite(
        &self,
        id: UniversalUuid,
        current_time: DateTime<Utc>,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let current_ts = UniversalTimestamp::from(current_time);
        let last_run_ts = UniversalTimestamp::from(last_run);
        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();
        let enabled_true = UniversalBool::from(true);

        let updated_rows = conn
            .interact(move |conn| {
                diesel::update(cron_schedules::table.find(id))
                    .filter(cron_schedules::next_run_at.le(current_ts))
                    .filter(cron_schedules::enabled.eq(enabled_true))
                    .set((
                        cron_schedules::last_run_at.eq(Some(last_run_ts)),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(updated_rows == 1)
    }

    /// Counts the total number of cron schedules.
    pub async fn count(&self, enabled_only: bool) -> Result<i64, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.count_postgres(enabled_only).await,
            BackendType::Sqlite => self.count_sqlite(enabled_only).await,
        }
    }

    async fn count_postgres(&self, enabled_only: bool) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let enabled_true = UniversalBool::from(true);
        let count = conn
            .interact(move |conn| {
                let mut query = cron_schedules::table.into_boxed();

                if enabled_only {
                    query = query.filter(cron_schedules::enabled.eq(enabled_true));
                }

                query.count().first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }

    async fn count_sqlite(&self, enabled_only: bool) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let enabled_true = UniversalBool::from(true);
        let count = conn
            .interact(move |conn| {
                let mut query = cron_schedules::table.into_boxed();

                if enabled_only {
                    query = query.filter(cron_schedules::enabled.eq(enabled_true));
                }

                query.count().first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }

    /// Updates the cron expression, timezone, and next run time for a schedule.
    pub async fn update_expression_and_timezone(
        &self,
        id: UniversalUuid,
        cron_expression: Option<&str>,
        timezone: Option<&str>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => {
                self.update_expression_and_timezone_postgres(
                    id,
                    cron_expression,
                    timezone,
                    next_run,
                )
                .await
            }
            BackendType::Sqlite => {
                self.update_expression_and_timezone_sqlite(id, cron_expression, timezone, next_run)
                    .await
            }
        }
    }

    async fn update_expression_and_timezone_postgres(
        &self,
        id: UniversalUuid,
        cron_expression: Option<&str>,
        timezone: Option<&str>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();
        let cron_expr_owned = cron_expression.map(|s| s.to_string());
        let timezone_owned = timezone.map(|s| s.to_string());

        conn.interact(move |conn| {
            let query = diesel::update(cron_schedules::table.find(id));

            if let (Some(ref expr), Some(ref tz)) = (&cron_expr_owned, &timezone_owned) {
                query
                    .set((
                        cron_schedules::cron_expression.eq(expr),
                        cron_schedules::timezone.eq(tz),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            } else if let Some(ref expr) = &cron_expr_owned {
                query
                    .set((
                        cron_schedules::cron_expression.eq(expr),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            } else if let Some(ref tz) = &timezone_owned {
                query
                    .set((
                        cron_schedules::timezone.eq(tz),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            } else {
                query
                    .set((
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            }
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn update_expression_and_timezone_sqlite(
        &self,
        id: UniversalUuid,
        cron_expression: Option<&str>,
        timezone: Option<&str>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();
        let cron_expr_owned = cron_expression.map(|s| s.to_string());
        let timezone_owned = timezone.map(|s| s.to_string());

        conn.interact(move |conn| {
            let query = diesel::update(cron_schedules::table.find(id));

            if let (Some(ref expr), Some(ref tz)) = (&cron_expr_owned, &timezone_owned) {
                query
                    .set((
                        cron_schedules::cron_expression.eq(expr),
                        cron_schedules::timezone.eq(tz),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            } else if let Some(ref expr) = &cron_expr_owned {
                query
                    .set((
                        cron_schedules::cron_expression.eq(expr),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            } else if let Some(ref tz) = &timezone_owned {
                query
                    .set((
                        cron_schedules::timezone.eq(tz),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            } else {
                query
                    .set((
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            }
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }
}
