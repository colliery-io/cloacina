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

//! CRUD operations for cron executions.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use super::CronExecutionDAL;
use crate::dal::unified::models::{NewUnifiedCronExecution, UnifiedCronExecution};
use crate::database::schema::unified::cron_executions;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::cron_execution::{CronExecution, NewCronExecution};

impl<'a> CronExecutionDAL<'a> {
    #[cfg(feature = "postgres")]
    pub(super) async fn create_postgres(
        &self,
        new_execution: NewCronExecution,
    ) -> Result<CronExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedCronExecution {
            id,
            schedule_id: new_execution.schedule_id,
            pipeline_execution_id: new_execution.pipeline_execution_id,
            scheduled_time: new_execution.scheduled_time,
            claimed_at: now,
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(cron_executions::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedCronExecution = conn
            .interact(move |conn| cron_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn create_sqlite(
        &self,
        new_execution: NewCronExecution,
    ) -> Result<CronExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedCronExecution {
            id,
            schedule_id: new_execution.schedule_id,
            pipeline_execution_id: new_execution.pipeline_execution_id,
            scheduled_time: new_execution.scheduled_time,
            claimed_at: now,
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(cron_executions::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedCronExecution = conn
            .interact(move |conn| cron_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn update_pipeline_execution_id_postgres(
        &self,
        cron_execution_id: UniversalUuid,
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
            diesel::update(cron_executions::table.find(cron_execution_id))
                .set((
                    cron_executions::pipeline_execution_id.eq(Some(pipeline_execution_id)),
                    cron_executions::updated_at.eq(now),
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
        cron_execution_id: UniversalUuid,
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
            diesel::update(cron_executions::table.find(cron_execution_id))
                .set((
                    cron_executions::pipeline_execution_id.eq(Some(pipeline_execution_id)),
                    cron_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn delete_older_than_postgres(
        &self,
        older_than: DateTime<Utc>,
    ) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let cutoff_ts = UniversalTimestamp::from(older_than);
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

    #[cfg(feature = "sqlite")]
    pub(super) async fn delete_older_than_sqlite(
        &self,
        older_than: DateTime<Utc>,
    ) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let cutoff_ts = UniversalTimestamp::from(older_than);
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
}
