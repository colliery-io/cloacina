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

//! Query operations for cron executions.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use super::CronExecutionDAL;
use crate::dal::unified::models::UnifiedCronExecution;
use crate::database::schema::unified::cron_executions;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::cron_execution::CronExecution;

impl<'a> CronExecutionDAL<'a> {
    #[cfg(feature = "postgres")]
    pub(super) async fn get_by_id_postgres(
        &self,
        id: UniversalUuid,
    ) -> Result<CronExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedCronExecution = conn
            .interact(move |conn| cron_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_by_id_sqlite(
        &self,
        id: UniversalUuid,
    ) -> Result<CronExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedCronExecution = conn
            .interact(move |conn| cron_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_by_schedule_id_postgres(
        &self,
        schedule_id: UniversalUuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedCronExecution> = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::schedule_id.eq(schedule_id))
                    .order(cron_executions::scheduled_time.desc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_by_schedule_id_sqlite(
        &self,
        schedule_id: UniversalUuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedCronExecution> = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::schedule_id.eq(schedule_id))
                    .order(cron_executions::scheduled_time.desc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_by_pipeline_execution_id_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Option<CronExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: Option<UnifiedCronExecution> = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::pipeline_execution_id.eq(Some(pipeline_execution_id)))
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.map(Into::into))
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_by_pipeline_execution_id_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Option<CronExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: Option<UnifiedCronExecution> = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::pipeline_execution_id.eq(Some(pipeline_execution_id)))
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.map(Into::into))
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_by_time_range_postgres(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let start_ts = UniversalTimestamp::from(start_time);
        let end_ts = UniversalTimestamp::from(end_time);

        let results: Vec<UnifiedCronExecution> = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::scheduled_time.ge(start_ts))
                    .filter(cron_executions::scheduled_time.lt(end_ts))
                    .order(cron_executions::scheduled_time.desc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_by_time_range_sqlite(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let start_ts = UniversalTimestamp::from(start_time);
        let end_ts = UniversalTimestamp::from(end_time);

        let results: Vec<UnifiedCronExecution> = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::scheduled_time.ge(start_ts))
                    .filter(cron_executions::scheduled_time.lt(end_ts))
                    .order(cron_executions::scheduled_time.desc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_latest_by_schedule_postgres(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<Option<CronExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: Option<UnifiedCronExecution> = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::schedule_id.eq(schedule_id))
                    .order(cron_executions::scheduled_time.desc())
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.map(Into::into))
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_latest_by_schedule_sqlite(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<Option<CronExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: Option<UnifiedCronExecution> = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::schedule_id.eq(schedule_id))
                    .order(cron_executions::scheduled_time.desc())
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.map(Into::into))
    }
}
