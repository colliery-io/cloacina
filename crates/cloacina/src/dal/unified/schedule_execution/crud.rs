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

//! CRUD operations for unified schedule executions.

use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;

use super::ScheduleExecutionDAL;
use crate::dal::unified::models::{NewUnifiedScheduleExecution, UnifiedScheduleExecution};
use crate::database::schema::unified::schedule_executions;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::schedule::{NewScheduleExecution, ScheduleExecution};

// =============================================================================
// PostgreSQL implementations
// =============================================================================

impl<'a> ScheduleExecutionDAL<'a> {
    #[cfg(feature = "postgres")]
    pub(super) async fn create_postgres(
        &self,
        new_execution: NewScheduleExecution,
    ) -> Result<ScheduleExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedScheduleExecution {
            id,
            schedule_id: new_execution.schedule_id,
            pipeline_execution_id: new_execution.pipeline_execution_id,
            scheduled_time: new_execution.scheduled_time,
            claimed_at: new_execution.claimed_at,
            context_hash: new_execution.context_hash,
            started_at: now,
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(schedule_executions::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedScheduleExecution = conn
            .interact(move |conn| schedule_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn create_sqlite(
        &self,
        new_execution: NewScheduleExecution,
    ) -> Result<ScheduleExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedScheduleExecution {
            id,
            schedule_id: new_execution.schedule_id,
            pipeline_execution_id: new_execution.pipeline_execution_id,
            scheduled_time: new_execution.scheduled_time,
            claimed_at: new_execution.claimed_at,
            context_hash: new_execution.context_hash,
            started_at: now,
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(schedule_executions::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedScheduleExecution = conn
            .interact(move |conn| schedule_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_by_id_postgres(
        &self,
        id: UniversalUuid,
    ) -> Result<ScheduleExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedScheduleExecution = conn
            .interact(move |conn| schedule_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_by_id_sqlite(
        &self,
        id: UniversalUuid,
    ) -> Result<ScheduleExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedScheduleExecution = conn
            .interact(move |conn| schedule_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn list_by_schedule_postgres(
        &self,
        schedule_id: UniversalUuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ScheduleExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedScheduleExecution> = conn
            .interact(move |conn| {
                schedule_executions::table
                    .filter(schedule_executions::schedule_id.eq(schedule_id))
                    .order(schedule_executions::created_at.desc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn list_by_schedule_sqlite(
        &self,
        schedule_id: UniversalUuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ScheduleExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedScheduleExecution> = conn
            .interact(move |conn| {
                schedule_executions::table
                    .filter(schedule_executions::schedule_id.eq(schedule_id))
                    .order(schedule_executions::created_at.desc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn complete_postgres(
        &self,
        id: UniversalUuid,
        completed_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let completed_ts = UniversalTimestamp::from(completed_at);
        let now = UniversalTimestamp::now();

        conn.interact(move |conn| {
            diesel::update(schedule_executions::table.find(id))
                .set((
                    schedule_executions::completed_at.eq(Some(completed_ts)),
                    schedule_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn complete_sqlite(
        &self,
        id: UniversalUuid,
        completed_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let completed_ts = UniversalTimestamp::from(completed_at);
        let now = UniversalTimestamp::now();

        conn.interact(move |conn| {
            diesel::update(schedule_executions::table.find(id))
                .set((
                    schedule_executions::completed_at.eq(Some(completed_ts)),
                    schedule_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn has_active_execution_postgres(
        &self,
        schedule_id: UniversalUuid,
        context_hash: String,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let count: i64 = conn
            .interact(move |conn| {
                schedule_executions::table
                    .filter(schedule_executions::schedule_id.eq(schedule_id))
                    .filter(schedule_executions::context_hash.eq(context_hash))
                    .filter(schedule_executions::completed_at.is_null())
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count > 0)
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn has_active_execution_sqlite(
        &self,
        schedule_id: UniversalUuid,
        context_hash: String,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let count: i64 = conn
            .interact(move |conn| {
                schedule_executions::table
                    .filter(schedule_executions::schedule_id.eq(schedule_id))
                    .filter(schedule_executions::context_hash.eq(context_hash))
                    .filter(schedule_executions::completed_at.is_null())
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count > 0)
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn update_pipeline_execution_id_postgres(
        &self,
        id: UniversalUuid,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();

        conn.interact(move |conn| {
            diesel::update(schedule_executions::table.find(id))
                .set((
                    schedule_executions::pipeline_execution_id.eq(Some(pipeline_execution_id)),
                    schedule_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn update_pipeline_execution_id_sqlite(
        &self,
        id: UniversalUuid,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();

        conn.interact(move |conn| {
            diesel::update(schedule_executions::table.find(id))
                .set((
                    schedule_executions::pipeline_execution_id.eq(Some(pipeline_execution_id)),
                    schedule_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn find_lost_executions_postgres(
        &self,
        older_than_minutes: i32,
    ) -> Result<Vec<ScheduleExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let cutoff = Utc::now() - Duration::minutes(older_than_minutes as i64);
        let cutoff_ts = UniversalTimestamp::from(cutoff);

        let results: Vec<UnifiedScheduleExecution> = conn
            .interact(move |conn| {
                schedule_executions::table
                    .filter(schedule_executions::completed_at.is_null())
                    .filter(schedule_executions::started_at.lt(cutoff_ts))
                    .order(schedule_executions::started_at.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn find_lost_executions_sqlite(
        &self,
        older_than_minutes: i32,
    ) -> Result<Vec<ScheduleExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let cutoff = Utc::now() - Duration::minutes(older_than_minutes as i64);
        let cutoff_ts = UniversalTimestamp::from(cutoff);

        let results: Vec<UnifiedScheduleExecution> = conn
            .interact(move |conn| {
                schedule_executions::table
                    .filter(schedule_executions::completed_at.is_null())
                    .filter(schedule_executions::started_at.lt(cutoff_ts))
                    .order(schedule_executions::started_at.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_latest_by_schedule_postgres(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<Option<ScheduleExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: Option<UnifiedScheduleExecution> = conn
            .interact(move |conn| {
                schedule_executions::table
                    .filter(schedule_executions::schedule_id.eq(schedule_id))
                    .order(schedule_executions::created_at.desc())
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.map(|r| r.into()))
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_latest_by_schedule_sqlite(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<Option<ScheduleExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: Option<UnifiedScheduleExecution> = conn
            .interact(move |conn| {
                schedule_executions::table
                    .filter(schedule_executions::schedule_id.eq(schedule_id))
                    .order(schedule_executions::created_at.desc())
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.map(|r| r.into()))
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_execution_stats_postgres(
        &self,
        since: DateTime<Utc>,
    ) -> Result<super::ScheduleExecutionStats, ValidationError> {
        use crate::database::schema::unified::pipeline_executions;

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let since_ts = UniversalTimestamp::from(since);
        let lost_cutoff = UniversalTimestamp::from(Utc::now() - Duration::minutes(10));

        let (total_executions, successful_executions, lost_executions) = conn
            .interact(move |conn| {
                let total_executions: i64 = schedule_executions::table
                    .filter(schedule_executions::started_at.ge(since_ts))
                    .count()
                    .first(conn)?;

                let successful_executions: i64 = schedule_executions::table
                    .filter(schedule_executions::started_at.ge(since_ts))
                    .filter(schedule_executions::pipeline_execution_id.is_not_null())
                    .count()
                    .first(conn)?;

                let lost_executions: i64 = schedule_executions::table
                    .left_join(
                        pipeline_executions::table.on(schedule_executions::pipeline_execution_id
                            .eq(pipeline_executions::id.nullable())),
                    )
                    .filter(pipeline_executions::id.is_null())
                    .filter(schedule_executions::started_at.ge(since_ts))
                    .filter(schedule_executions::started_at.lt(lost_cutoff))
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

        Ok(super::ScheduleExecutionStats {
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
    ) -> Result<super::ScheduleExecutionStats, ValidationError> {
        use crate::database::schema::unified::pipeline_executions;

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let since_ts = UniversalTimestamp::from(since);

        let total_executions: i64 = conn
            .interact(move |conn| {
                schedule_executions::table
                    .filter(schedule_executions::started_at.ge(since_ts))
                    .count()
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let since_ts = UniversalTimestamp::from(since);
        let successful_executions: i64 = conn
            .interact(move |conn| {
                schedule_executions::table
                    .filter(schedule_executions::started_at.ge(since_ts))
                    .filter(schedule_executions::pipeline_execution_id.is_not_null())
                    .count()
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let since_ts = UniversalTimestamp::from(since);
        let lost_cutoff = UniversalTimestamp::from(Utc::now() - Duration::minutes(10));
        let lost_executions: i64 = conn
            .interact(move |conn| {
                schedule_executions::table
                    .left_join(
                        pipeline_executions::table.on(schedule_executions::pipeline_execution_id
                            .eq(pipeline_executions::id.nullable())),
                    )
                    .filter(pipeline_executions::id.is_null())
                    .filter(schedule_executions::started_at.ge(since_ts))
                    .filter(schedule_executions::started_at.lt(lost_cutoff))
                    .count()
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(super::ScheduleExecutionStats {
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
