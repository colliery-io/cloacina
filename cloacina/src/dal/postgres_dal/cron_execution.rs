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

use super::models::{NewPgCronExecution, PgCronExecution};
use super::DAL;
use crate::database::schema::postgres::cron_executions;
use crate::database::universal_types::UniversalUuid;
use crate::error::ValidationError;
use crate::models::cron_execution::{CronExecution, NewCronExecution};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

/// Data Access Layer for cron execution operations.
pub struct CronExecutionDAL<'a> {
    pub dal: &'a DAL,
}

impl<'a> CronExecutionDAL<'a> {
    /// Creates a new cron execution audit record in the database.
    pub async fn create(
        &self,
        new_execution: NewCronExecution,
    ) -> Result<CronExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let pg_new = NewPgCronExecution {
            schedule_id: new_execution.schedule_id.0,
            pipeline_execution_id: new_execution.pipeline_execution_id.map(|u| u.0),
            scheduled_time: new_execution.scheduled_time.0.naive_utc(),
            claimed_at: Utc::now().naive_utc(),
        };

        let pg_execution: PgCronExecution = conn
            .interact(move |conn| {
                diesel::insert_into(cron_executions::table)
                    .values(&pg_new)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_execution.into())
    }

    /// Updates the pipeline execution ID for an existing cron execution record.
    pub async fn update_pipeline_execution_id(
        &self,
        cron_execution_id: UniversalUuid,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let uuid_id: Uuid = cron_execution_id.0;
        let pipeline_uuid: Uuid = pipeline_execution_id.0;

        conn.interact(move |conn| {
            diesel::update(cron_executions::table.find(uuid_id))
                .set((
                    cron_executions::pipeline_execution_id.eq(pipeline_uuid),
                    cron_executions::updated_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Finds "lost" executions that need recovery.
    pub async fn find_lost_executions(
        &self,
        older_than_minutes: i32,
    ) -> Result<Vec<CronExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let cutoff_time = (Utc::now() - chrono::Duration::minutes(older_than_minutes as i64)).naive_utc();

        let pg_executions: Vec<PgCronExecution> = conn
            .interact(move |conn| {
                cron_executions::table
                    .left_join(
                        crate::database::schema::postgres::pipeline_executions::table
                            .on(cron_executions::pipeline_execution_id
                                .eq(crate::database::schema::postgres::pipeline_executions::id.nullable())),
                    )
                    .filter(crate::database::schema::postgres::pipeline_executions::id.is_null())
                    .filter(cron_executions::claimed_at.lt(cutoff_time))
                    .select(cron_executions::all_columns)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_executions.into_iter().map(Into::into).collect())
    }

    /// Retrieves a cron execution record by its ID.
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<CronExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let uuid_id: Uuid = id.0;

        let pg_execution: PgCronExecution = conn
            .interact(move |conn| cron_executions::table.find(uuid_id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(pg_execution.into())
    }

    /// Retrieves all cron execution records for a specific schedule.
    pub async fn get_by_schedule_id(
        &self,
        schedule_id: UniversalUuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let uuid_id: Uuid = schedule_id.0;

        let pg_executions: Vec<PgCronExecution> = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::schedule_id.eq(uuid_id))
                    .order(cron_executions::scheduled_time.desc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_executions.into_iter().map(Into::into).collect())
    }

    /// Retrieves the cron execution record for a specific pipeline execution.
    pub async fn get_by_pipeline_execution_id(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Option<CronExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let uuid_id: Uuid = pipeline_execution_id.0;

        let pg_execution: Option<PgCronExecution> = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::pipeline_execution_id.eq(uuid_id))
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_execution.map(Into::into))
    }

    /// Retrieves cron execution records within a time range.
    pub async fn get_by_time_range(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CronExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let start_ts = start_time.naive_utc();
        let end_ts = end_time.naive_utc();

        let pg_executions: Vec<PgCronExecution> = conn
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

        Ok(pg_executions.into_iter().map(Into::into).collect())
    }

    /// Counts the total number of executions for a specific schedule.
    pub async fn count_by_schedule(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let uuid_id: Uuid = schedule_id.0;

        let count = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::schedule_id.eq(uuid_id))
                    .count()
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }

    /// Checks if an execution already exists for a specific schedule and time.
    pub async fn execution_exists(
        &self,
        schedule_id: UniversalUuid,
        scheduled_time: DateTime<Utc>,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let uuid_id: Uuid = schedule_id.0;
        let scheduled_ts = scheduled_time.naive_utc();

        let count: i64 = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::schedule_id.eq(uuid_id))
                    .filter(cron_executions::scheduled_time.eq(scheduled_ts))
                    .count()
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count > 0)
    }

    /// Retrieves the most recent execution for a specific schedule.
    pub async fn get_latest_by_schedule(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<Option<CronExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let uuid_id: Uuid = schedule_id.0;

        let pg_execution: Option<PgCronExecution> = conn
            .interact(move |conn| {
                cron_executions::table
                    .filter(cron_executions::schedule_id.eq(uuid_id))
                    .order(cron_executions::scheduled_time.desc())
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(pg_execution.map(Into::into))
    }

    /// Deletes old execution records beyond a certain age.
    pub async fn delete_older_than(
        &self,
        older_than: DateTime<Utc>,
    ) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let cutoff_ts = older_than.naive_utc();

        let deleted_count = conn
            .interact(move |conn| {
                diesel::delete(cron_executions::table)
                    .filter(cron_executions::scheduled_time.lt(cutoff_ts))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(deleted_count)
    }

    /// Gets execution statistics for monitoring and alerting.
    pub async fn get_execution_stats(
        &self,
        since: DateTime<Utc>,
    ) -> Result<CronExecutionStats, ValidationError> {
        let conn = self
            .dal
            .database
            .get_connection_with_schema()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let since_ts = since.naive_utc();
        let lost_cutoff = (Utc::now() - chrono::Duration::minutes(10)).naive_utc();

        let (total_executions, successful_executions, lost_executions) = conn
            .interact(move |conn| {
                let total_executions = cron_executions::table
                    .filter(cron_executions::claimed_at.ge(since_ts))
                    .count()
                    .first(conn)?;

                let successful_executions = cron_executions::table
                    .inner_join(crate::database::schema::postgres::pipeline_executions::table)
                    .filter(cron_executions::claimed_at.ge(since_ts))
                    .count()
                    .first(conn)?;

                let lost_executions = cron_executions::table
                    .left_join(
                        crate::database::schema::postgres::pipeline_executions::table
                            .on(cron_executions::pipeline_execution_id
                                .eq(crate::database::schema::postgres::pipeline_executions::id.nullable())),
                    )
                    .filter(crate::database::schema::postgres::pipeline_executions::id.is_null())
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
