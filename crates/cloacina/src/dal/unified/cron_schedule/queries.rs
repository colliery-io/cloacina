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

//! Query operations for cron schedules.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use super::CronScheduleDAL;
use crate::dal::unified::models::UnifiedCronSchedule;
use crate::database::schema::unified::cron_schedules;
use crate::database::universal_types::{UniversalBool, UniversalTimestamp};
use crate::error::ValidationError;
use crate::models::cron_schedule::CronSchedule;

impl<'a> CronScheduleDAL<'a> {
    #[cfg(feature = "postgres")]
    pub(super) async fn get_due_schedules_postgres(
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

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_due_schedules_sqlite(
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

    #[cfg(feature = "postgres")]
    pub(super) async fn list_postgres(
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

    #[cfg(feature = "sqlite")]
    pub(super) async fn list_sqlite(
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

    #[cfg(feature = "postgres")]
    pub(super) async fn find_by_workflow_postgres(
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

    #[cfg(feature = "sqlite")]
    pub(super) async fn find_by_workflow_sqlite(
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

    #[cfg(feature = "postgres")]
    pub(super) async fn count_postgres(&self, enabled_only: bool) -> Result<i64, ValidationError> {
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

    #[cfg(feature = "sqlite")]
    pub(super) async fn count_sqlite(&self, enabled_only: bool) -> Result<i64, ValidationError> {
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
}
