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

//! Task heartbeat operations — claim, heartbeat, and orphan detection.

use super::TaskExecutionDAL;
use crate::database::schema::unified::task_executions;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use diesel::prelude::*;

impl<'a> TaskExecutionDAL<'a> {
    /// Atomically claim a task for execution.
    ///
    /// Sets status to "Running", records the executor identity, and starts
    /// the heartbeat clock. Returns true if the claim succeeded (task was
    /// still in Ready state), false if another executor claimed it first.
    pub async fn claim_task(
        &self,
        task_id: UniversalUuid,
        executor_id: &str,
    ) -> Result<bool, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.claim_task_postgres(task_id, executor_id).await,
            self.claim_task_sqlite(task_id, executor_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn claim_task_postgres(
        &self,
        task_id: UniversalUuid,
        executor_id: &str,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let executor_id = executor_id.to_string();

        let rows = conn
            .interact(move |conn| {
                diesel::update(task_executions::table.find(task_id))
                    .filter(task_executions::status.eq("Ready"))
                    .set((
                        task_executions::status.eq("Running"),
                        task_executions::claimed_by.eq(Some(&executor_id)),
                        task_executions::heartbeat_at.eq(Some(now)),
                        task_executions::started_at.eq(Some(now)),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(rows == 1)
    }

    #[cfg(feature = "sqlite")]
    async fn claim_task_sqlite(
        &self,
        task_id: UniversalUuid,
        executor_id: &str,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let executor_id = executor_id.to_string();

        let rows = conn
            .interact(move |conn| {
                diesel::update(task_executions::table.find(task_id))
                    .filter(task_executions::status.eq("Ready"))
                    .set((
                        task_executions::status.eq("Running"),
                        task_executions::claimed_by.eq(Some(&executor_id)),
                        task_executions::heartbeat_at.eq(Some(now)),
                        task_executions::started_at.eq(Some(now)),
                        task_executions::updated_at.eq(now),
                    ))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(rows == 1)
    }

    /// Update heartbeat timestamp for a running task.
    ///
    /// Returns true if the heartbeat was accepted (task is still claimed by
    /// this executor). Returns false if the task was recovered and re-assigned
    /// — the executor should stop executing.
    pub async fn heartbeat(
        &self,
        task_id: UniversalUuid,
        executor_id: &str,
    ) -> Result<bool, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.heartbeat_postgres(task_id, executor_id).await,
            self.heartbeat_sqlite(task_id, executor_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn heartbeat_postgres(
        &self,
        task_id: UniversalUuid,
        executor_id: &str,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let executor_id = executor_id.to_string();

        let rows = conn
            .interact(move |conn| {
                diesel::update(task_executions::table.find(task_id))
                    .filter(task_executions::claimed_by.eq(Some(&executor_id)))
                    .set(task_executions::heartbeat_at.eq(Some(now)))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(rows == 1)
    }

    #[cfg(feature = "sqlite")]
    async fn heartbeat_sqlite(
        &self,
        task_id: UniversalUuid,
        executor_id: &str,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let executor_id = executor_id.to_string();

        let rows = conn
            .interact(move |conn| {
                diesel::update(task_executions::table.find(task_id))
                    .filter(task_executions::claimed_by.eq(Some(&executor_id)))
                    .set(task_executions::heartbeat_at.eq(Some(now)))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(rows == 1)
    }

    /// Find orphaned tasks — Running with stale heartbeats.
    ///
    /// In startup mode, `cutoff` should be `sweeper_start_time - orphan_threshold`
    /// to only recover tasks from previous sessions.
    /// In normal mode, `cutoff` should be `NOW() - orphan_threshold`.
    pub async fn find_stale_heartbeats(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<Vec<crate::models::task_execution::TaskExecution>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.find_stale_heartbeats_postgres(cutoff).await,
            self.find_stale_heartbeats_sqlite(cutoff).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn find_stale_heartbeats_postgres(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<Vec<crate::models::task_execution::TaskExecution>, ValidationError> {
        use crate::dal::unified::models::UnifiedTaskExecution;

        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::status.eq("Running"))
                    .filter(task_executions::heartbeat_at.lt(Some(cutoff)))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn find_stale_heartbeats_sqlite(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<Vec<crate::models::task_execution::TaskExecution>, ValidationError> {
        use crate::dal::unified::models::UnifiedTaskExecution;

        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedTaskExecution> = conn
            .interact(move |conn| {
                task_executions::table
                    .filter(task_executions::status.eq("Running"))
                    .filter(task_executions::heartbeat_at.lt(Some(cutoff)))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }
}
