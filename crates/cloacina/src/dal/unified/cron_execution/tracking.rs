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

//! Tracking and statistics operations for cron executions.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use super::{CronExecutionDAL, CronExecutionStats};
use crate::dal::unified::models::UnifiedCronExecution;
use crate::database::schema::unified::{cron_executions, pipeline_executions};
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::cron_execution::CronExecution;

impl<'a> CronExecutionDAL<'a> {
    #[cfg(feature = "postgres")]
    pub(super) async fn find_lost_executions_postgres(
        &self,
        older_than_minutes: i32,
    ) -> Result<Vec<CronExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let cutoff_time = UniversalTimestamp::from(
            Utc::now() - chrono::Duration::minutes(older_than_minutes as i64),
        );

        let results: Vec<UnifiedCronExecution> = conn
            .interact(move |conn| {
                cron_executions::table
                    .left_join(
                        pipeline_executions::table.on(cron_executions::pipeline_execution_id
                            .eq(pipeline_executions::id.nullable())),
                    )
                    .filter(pipeline_executions::id.is_null())
                    .filter(cron_executions::claimed_at.lt(cutoff_time))
                    .select(cron_executions::all_columns)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn find_lost_executions_sqlite(
        &self,
        older_than_minutes: i32,
    ) -> Result<Vec<CronExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let cutoff_time = UniversalTimestamp::from(
            Utc::now() - chrono::Duration::minutes(older_than_minutes as i64),
        );

        let results: Vec<UnifiedCronExecution> = conn
            .interact(move |conn| {
                cron_executions::table
                    .left_join(
                        pipeline_executions::table.on(cron_executions::pipeline_execution_id
                            .eq(pipeline_executions::id.nullable())),
                    )
                    .filter(pipeline_executions::id.is_null())
                    .filter(cron_executions::claimed_at.lt(cutoff_time))
                    .select(cron_executions::all_columns)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn count_by_schedule_postgres(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let count = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::schedule_id.eq(schedule_id))
                    .count()
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn count_by_schedule_sqlite(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let count: i64 = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::schedule_id.eq(schedule_id))
                    .count()
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn execution_exists_postgres(
        &self,
        schedule_id: UniversalUuid,
        scheduled_time: DateTime<Utc>,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let scheduled_ts = UniversalTimestamp::from(scheduled_time);

        let count: i64 = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::schedule_id.eq(schedule_id))
                    .filter(cron_executions::scheduled_time.eq(scheduled_ts))
                    .count()
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count > 0)
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn execution_exists_sqlite(
        &self,
        schedule_id: UniversalUuid,
        scheduled_time: DateTime<Utc>,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let scheduled_ts = UniversalTimestamp::from(scheduled_time);

        let count: i64 = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::schedule_id.eq(schedule_id))
                    .filter(cron_executions::scheduled_time.eq(scheduled_ts))
                    .count()
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count > 0)
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_execution_stats_postgres(
        &self,
        since: DateTime<Utc>,
    ) -> Result<CronExecutionStats, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let since_ts = UniversalTimestamp::from(since);
        let lost_cutoff = UniversalTimestamp::from(Utc::now() - chrono::Duration::minutes(10));

        let (total_executions, successful_executions, lost_executions) = conn
            .interact(move |conn| {
                let total_executions = cron_executions::table
                    .filter(cron_executions::claimed_at.ge(since_ts))
                    .count()
                    .first(conn)?;

                let successful_executions = cron_executions::table
                    .filter(cron_executions::claimed_at.ge(since_ts))
                    .filter(cron_executions::pipeline_execution_id.is_not_null())
                    .count()
                    .first(conn)?;

                let lost_executions = cron_executions::table
                    .left_join(
                        pipeline_executions::table.on(cron_executions::pipeline_execution_id
                            .eq(pipeline_executions::id.nullable())),
                    )
                    .filter(pipeline_executions::id.is_null())
                    .filter(cron_executions::claimed_at.ge(since_ts))
                    .filter(cron_executions::claimed_at.lt(lost_cutoff))
                    .count()
                    .first(conn)?;

                Ok::<(i64, i64, i64), diesel::result::Error>((
                    total_executions,
                    successful_executions,
                    lost_executions,
                ))
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

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

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_execution_stats_sqlite(
        &self,
        since: DateTime<Utc>,
    ) -> Result<CronExecutionStats, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let since_ts = UniversalTimestamp::from(since);

        let total_executions: i64 = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::claimed_at.ge(since_ts))
                    .count()
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let since_ts = UniversalTimestamp::from(since);
        let successful_executions: i64 = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::claimed_at.ge(since_ts))
                    .filter(cron_executions::pipeline_execution_id.is_not_null())
                    .count()
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let since_ts = UniversalTimestamp::from(since);
        let lost_cutoff = UniversalTimestamp::from(Utc::now() - chrono::Duration::minutes(10));
        let lost_executions: i64 = conn
            .interact(move |conn| {
                cron_executions::table
                    .left_join(
                        pipeline_executions::table.on(cron_executions::pipeline_execution_id
                            .eq(pipeline_executions::id.nullable())),
                    )
                    .filter(pipeline_executions::id.is_null())
                    .filter(cron_executions::claimed_at.ge(since_ts))
                    .filter(cron_executions::claimed_at.lt(lost_cutoff))
                    .count()
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

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
