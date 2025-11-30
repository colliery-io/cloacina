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

use super::DAL;
use crate::database::universal_types::UniversalUuid;
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
        use crate::dal::postgres_dal::models::{NewPgCronSchedule, PgCronSchedule};
        use crate::database::schema::postgres::cron_schedules;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let pg_new = NewPgCronSchedule {
            workflow_name: new_schedule.workflow_name,
            cron_expression: new_schedule.cron_expression,
            timezone: new_schedule.timezone.unwrap_or_else(|| "UTC".to_string()),
            enabled: new_schedule.enabled.map(|b| b.into()).unwrap_or(true),
            catchup_policy: new_schedule
                .catchup_policy
                .unwrap_or_else(|| "skip".to_string()),
            start_date: new_schedule.start_date.map(|t| t.0.naive_utc()),
            end_date: new_schedule.end_date.map(|t| t.0.naive_utc()),
            next_run_at: new_schedule.next_run_at.0.naive_utc(),
        };

        let pg_schedule: PgCronSchedule = conn
            .interact(move |conn| {
                diesel::insert_into(cron_schedules::table)
                    .values(&pg_new)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_schedule.into())
    }

    async fn create_sqlite(
        &self,
        new_schedule: NewCronSchedule,
    ) -> Result<CronSchedule, ValidationError> {
        use crate::dal::sqlite_dal::models::{
            current_timestamp_string, uuid_to_blob, NewSqliteCronSchedule, SqliteCronSchedule,
        };
        use crate::database::schema::sqlite::cron_schedules;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let id_blob = uuid_to_blob(&id.0);
        let now = current_timestamp_string();

        let sqlite_new = NewSqliteCronSchedule {
            id: id_blob.clone(),
            workflow_name: new_schedule.workflow_name,
            cron_expression: new_schedule.cron_expression,
            timezone: new_schedule.timezone.unwrap_or_else(|| "UTC".to_string()),
            enabled: if new_schedule.enabled.map(|b| b.into()).unwrap_or(true) {
                1
            } else {
                0
            },
            catchup_policy: new_schedule
                .catchup_policy
                .unwrap_or_else(|| "skip".to_string()),
            start_date: new_schedule.start_date.map(|t| t.0.to_rfc3339()),
            end_date: new_schedule.end_date.map(|t| t.0.to_rfc3339()),
            next_run_at: new_schedule.next_run_at.0.to_rfc3339(),
            created_at: now.clone(),
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(cron_schedules::table)
                .values(&sqlite_new)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let sqlite_schedule: SqliteCronSchedule = conn
            .interact(move |conn| cron_schedules::table.find(id_blob).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(sqlite_schedule.into())
    }

    /// Retrieves a cron schedule by its ID.
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<CronSchedule, ValidationError> {
        match self.dal.backend() {
            BackendType::Postgres => self.get_by_id_postgres(id).await,
            BackendType::Sqlite => self.get_by_id_sqlite(id).await,
        }
    }

    async fn get_by_id_postgres(&self, id: UniversalUuid) -> Result<CronSchedule, ValidationError> {
        use crate::dal::postgres_dal::models::PgCronSchedule;
        use crate::database::schema::postgres::cron_schedules;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id = id.0;
        let pg_schedule: PgCronSchedule = conn
            .interact(move |conn| cron_schedules::table.find(uuid_id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_schedule.into())
    }

    async fn get_by_id_sqlite(&self, id: UniversalUuid) -> Result<CronSchedule, ValidationError> {
        use crate::dal::sqlite_dal::models::{uuid_to_blob, SqliteCronSchedule};
        use crate::database::schema::sqlite::cron_schedules;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let sqlite_schedule: SqliteCronSchedule = conn
            .interact(move |conn| cron_schedules::table.find(id_blob).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(sqlite_schedule.into())
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
        use crate::dal::postgres_dal::models::PgCronSchedule;
        use crate::database::schema::postgres::cron_schedules;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now_ts = now.naive_utc();
        let pg_schedules: Vec<PgCronSchedule> = conn
            .interact(move |conn| {
                diesel::sql_query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE").execute(conn)?;

                let schedules = cron_schedules::table
                    .filter(cron_schedules::enabled.eq(true))
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

        Ok(pg_schedules.into_iter().map(Into::into).collect())
    }

    async fn get_due_schedules_sqlite(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        use crate::dal::sqlite_dal::models::SqliteCronSchedule;
        use crate::database::schema::sqlite::cron_schedules;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now_ts = now.to_rfc3339();
        let sqlite_schedules: Vec<SqliteCronSchedule> = conn
            .interact(move |conn| {
                cron_schedules::table
                    .filter(cron_schedules::enabled.eq(1))
                    .filter(cron_schedules::next_run_at.le(now_ts.clone()))
                    .filter(
                        cron_schedules::start_date
                            .is_null()
                            .or(cron_schedules::start_date.le(now_ts.clone())),
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

        Ok(sqlite_schedules.into_iter().map(Into::into).collect())
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
        use crate::database::schema::postgres::cron_schedules;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id = id.0;
        let last_run_ts = last_run.naive_utc();
        let next_run_ts = next_run.naive_utc();

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(uuid_id))
                .set((
                    cron_schedules::last_run_at.eq(Some(last_run_ts)),
                    cron_schedules::next_run_at.eq(next_run_ts),
                    cron_schedules::updated_at.eq(diesel::dsl::now),
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
        use crate::dal::sqlite_dal::models::{current_timestamp_string, uuid_to_blob};
        use crate::database::schema::sqlite::cron_schedules;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let last_run_ts = last_run.to_rfc3339();
        let next_run_ts = next_run.to_rfc3339();
        let now_ts = current_timestamp_string();

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(id_blob))
                .set((
                    cron_schedules::last_run_at.eq(Some(last_run_ts)),
                    cron_schedules::next_run_at.eq(next_run_ts),
                    cron_schedules::updated_at.eq(now_ts),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
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
                self.update_expression_and_timezone_postgres(id, cron_expression, timezone, next_run)
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
        use crate::database::schema::postgres::cron_schedules;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id = id.0;
        let next_run_ts = next_run.naive_utc();
        let cron_expr_owned = cron_expression.map(|s| s.to_string());
        let timezone_owned = timezone.map(|s| s.to_string());

        conn.interact(move |conn| {
            let query = diesel::update(cron_schedules::table.find(uuid_id));

            if let (Some(ref expr), Some(ref tz)) = (&cron_expr_owned, &timezone_owned) {
                query
                    .set((
                        cron_schedules::cron_expression.eq(expr),
                        cron_schedules::timezone.eq(tz),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(diesel::dsl::now),
                    ))
                    .execute(conn)
            } else if let Some(ref expr) = &cron_expr_owned {
                query
                    .set((
                        cron_schedules::cron_expression.eq(expr),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(diesel::dsl::now),
                    ))
                    .execute(conn)
            } else if let Some(ref tz) = &timezone_owned {
                query
                    .set((
                        cron_schedules::timezone.eq(tz),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(diesel::dsl::now),
                    ))
                    .execute(conn)
            } else {
                query
                    .set((
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(diesel::dsl::now),
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
        use crate::dal::sqlite_dal::models::{current_timestamp_string, uuid_to_blob};
        use crate::database::schema::sqlite::cron_schedules;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let next_run_ts = next_run.to_rfc3339();
        let now_ts = current_timestamp_string();
        let cron_expr_owned = cron_expression.map(|s| s.to_string());
        let timezone_owned = timezone.map(|s| s.to_string());

        conn.interact(move |conn| {
            let query = diesel::update(cron_schedules::table.find(id_blob));

            if let (Some(ref expr), Some(ref tz)) = (&cron_expr_owned, &timezone_owned) {
                query
                    .set((
                        cron_schedules::cron_expression.eq(expr),
                        cron_schedules::timezone.eq(tz),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now_ts),
                    ))
                    .execute(conn)
            } else if let Some(ref expr) = &cron_expr_owned {
                query
                    .set((
                        cron_schedules::cron_expression.eq(expr),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now_ts),
                    ))
                    .execute(conn)
            } else if let Some(ref tz) = &timezone_owned {
                query
                    .set((
                        cron_schedules::timezone.eq(tz),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now_ts),
                    ))
                    .execute(conn)
            } else {
                query
                    .set((
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now_ts),
                    ))
                    .execute(conn)
            }
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
        use crate::database::schema::postgres::cron_schedules;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id = id.0;

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(uuid_id))
                .set((
                    cron_schedules::enabled.eq(true),
                    cron_schedules::updated_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn enable_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        use crate::dal::sqlite_dal::models::{current_timestamp_string, uuid_to_blob};
        use crate::database::schema::sqlite::cron_schedules;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let now_ts = current_timestamp_string();

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(id_blob))
                .set((
                    cron_schedules::enabled.eq(1),
                    cron_schedules::updated_at.eq(now_ts),
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
        use crate::database::schema::postgres::cron_schedules;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id = id.0;

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(uuid_id))
                .set((
                    cron_schedules::enabled.eq(false),
                    cron_schedules::updated_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn disable_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        use crate::dal::sqlite_dal::models::{current_timestamp_string, uuid_to_blob};
        use crate::database::schema::sqlite::cron_schedules;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let now_ts = current_timestamp_string();

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(id_blob))
                .set((
                    cron_schedules::enabled.eq(0),
                    cron_schedules::updated_at.eq(now_ts),
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
        use crate::database::schema::postgres::cron_schedules;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id = id.0;
        conn.interact(move |conn| {
            diesel::delete(cron_schedules::table.find(uuid_id)).execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    async fn delete_sqlite(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        use crate::dal::sqlite_dal::models::uuid_to_blob;
        use crate::database::schema::sqlite::cron_schedules;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        conn.interact(move |conn| diesel::delete(cron_schedules::table.find(id_blob)).execute(conn))
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
        use crate::dal::postgres_dal::models::PgCronSchedule;
        use crate::database::schema::postgres::cron_schedules;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let pg_schedules: Vec<PgCronSchedule> = conn
            .interact(move |conn| {
                let mut query = cron_schedules::table.into_boxed();

                if enabled_only {
                    query = query.filter(cron_schedules::enabled.eq(true));
                }

                query
                    .order(cron_schedules::workflow_name.asc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_schedules.into_iter().map(Into::into).collect())
    }

    async fn list_sqlite(
        &self,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        use crate::dal::sqlite_dal::models::SqliteCronSchedule;
        use crate::database::schema::sqlite::cron_schedules;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let sqlite_schedules: Vec<SqliteCronSchedule> = conn
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

        Ok(sqlite_schedules.into_iter().map(Into::into).collect())
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
        use crate::dal::postgres_dal::models::PgCronSchedule;
        use crate::database::schema::postgres::cron_schedules;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let workflow_name = workflow_name.to_string();
        let pg_schedules: Vec<PgCronSchedule> = conn
            .interact(move |conn| {
                cron_schedules::table
                    .filter(cron_schedules::workflow_name.eq(workflow_name))
                    .order(cron_schedules::created_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_schedules.into_iter().map(Into::into).collect())
    }

    async fn find_by_workflow_sqlite(
        &self,
        workflow_name: &str,
    ) -> Result<Vec<CronSchedule>, ValidationError> {
        use crate::dal::sqlite_dal::models::SqliteCronSchedule;
        use crate::database::schema::sqlite::cron_schedules;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let workflow_name = workflow_name.to_string();
        let sqlite_schedules: Vec<SqliteCronSchedule> = conn
            .interact(move |conn| {
                cron_schedules::table
                    .filter(cron_schedules::workflow_name.eq(workflow_name))
                    .order(cron_schedules::created_at.desc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(sqlite_schedules.into_iter().map(Into::into).collect())
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
        use crate::database::schema::postgres::cron_schedules;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id = id.0;
        let next_run_ts = next_run.naive_utc();

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(uuid_id))
                .set((
                    cron_schedules::next_run_at.eq(next_run_ts),
                    cron_schedules::updated_at.eq(diesel::dsl::now),
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
        use crate::dal::sqlite_dal::models::{current_timestamp_string, uuid_to_blob};
        use crate::database::schema::sqlite::cron_schedules;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let next_run_ts = next_run.to_rfc3339();
        let now_ts = current_timestamp_string();

        conn.interact(move |conn| {
            diesel::update(cron_schedules::table.find(id_blob))
                .set((
                    cron_schedules::next_run_at.eq(next_run_ts),
                    cron_schedules::updated_at.eq(now_ts),
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
        use crate::database::schema::postgres::cron_schedules;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let uuid_id = id.0;
        let current_ts = current_time.naive_utc();
        let last_run_ts = last_run.naive_utc();
        let next_run_ts = next_run.naive_utc();

        let updated_rows = conn
            .interact(move |conn| {
                diesel::sql_query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE").execute(conn)?;

                let updated_rows = diesel::update(cron_schedules::table.find(uuid_id))
                    .filter(cron_schedules::next_run_at.le(current_ts))
                    .filter(cron_schedules::enabled.eq(true))
                    .set((
                        cron_schedules::last_run_at.eq(Some(last_run_ts)),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(diesel::dsl::now),
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
        use crate::dal::sqlite_dal::models::{current_timestamp_string, uuid_to_blob};
        use crate::database::schema::sqlite::cron_schedules;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id_blob = uuid_to_blob(&id.0);
        let current_ts = current_time.to_rfc3339();
        let last_run_ts = last_run.to_rfc3339();
        let next_run_ts = next_run.to_rfc3339();
        let now_ts = current_timestamp_string();

        let updated_rows = conn
            .interact(move |conn| {
                diesel::update(cron_schedules::table.find(id_blob))
                    .filter(cron_schedules::next_run_at.le(current_ts))
                    .filter(cron_schedules::enabled.eq(1))
                    .set((
                        cron_schedules::last_run_at.eq(Some(last_run_ts)),
                        cron_schedules::next_run_at.eq(next_run_ts),
                        cron_schedules::updated_at.eq(now_ts),
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
        use crate::database::schema::postgres::cron_schedules;

        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let count = conn
            .interact(move |conn| {
                let mut query = cron_schedules::table.into_boxed();

                if enabled_only {
                    query = query.filter(cron_schedules::enabled.eq(true));
                }

                query.count().first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }

    async fn count_sqlite(&self, enabled_only: bool) -> Result<i64, ValidationError> {
        use crate::database::schema::sqlite::cron_schedules;

        let conn = self
            .dal
            .pool()
            .expect_sqlite()
            .get()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

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
