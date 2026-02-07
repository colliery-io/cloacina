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

//! Unified Task Outbox DAL with runtime backend selection
//!
//! This module provides operations for the task outbox, which is used for
//! work distribution. The outbox is transient - entries are deleted immediately
//! when workers claim tasks.
//!
//! Note: The primary outbox insertion happens in `mark_ready()` within the same
//! transaction as the status update. This DAL provides additional operations
//! for claiming and cleanup.

use super::models::{NewUnifiedTaskOutbox, UnifiedTaskOutbox};
use super::DAL;
use crate::database::schema::unified::task_outbox;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::task_outbox::{NewTaskOutbox, TaskOutbox};
use diesel::prelude::*;

/// Data access layer for task outbox operations with runtime backend selection.
///
/// The outbox provides reliable work distribution by:
/// 1. Inserting entries atomically with task status updates
/// 2. Enabling push notifications (Postgres LISTEN/NOTIFY)
/// 3. Supporting polling for SQLite
/// 4. Deleting entries when tasks are claimed
#[derive(Clone)]
pub struct TaskOutboxDAL<'a> {
    dal: &'a DAL,
}

impl<'a> TaskOutboxDAL<'a> {
    /// Creates a new TaskOutboxDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new outbox entry.
    ///
    /// Note: Prefer using the transactional insertion in `mark_ready()` instead
    /// of calling this directly, to ensure atomicity with status updates.
    pub async fn create(&self, new_entry: NewTaskOutbox) -> Result<TaskOutbox, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_entry).await,
            self.create_sqlite(new_entry).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn create_postgres(
        &self,
        new_entry: NewTaskOutbox,
    ) -> Result<TaskOutbox, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let new_unified = NewUnifiedTaskOutbox {
            task_execution_id: new_entry.task_execution_id,
            created_at: now,
        };

        let result: UnifiedTaskOutbox = conn
            .interact(move |conn| {
                diesel::insert_into(task_outbox::table)
                    .values(&new_unified)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(TaskOutbox {
            id: result.id,
            task_execution_id: result.task_execution_id,
            created_at: result.created_at,
        })
    }

    #[cfg(feature = "sqlite")]
    async fn create_sqlite(&self, new_entry: NewTaskOutbox) -> Result<TaskOutbox, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        let new_unified = NewUnifiedTaskOutbox {
            task_execution_id: new_entry.task_execution_id,
            created_at: now,
        };

        let result: UnifiedTaskOutbox = conn
            .interact(move |conn| {
                diesel::insert_into(task_outbox::table)
                    .values(&new_unified)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(TaskOutbox {
            id: result.id,
            task_execution_id: result.task_execution_id,
            created_at: result.created_at,
        })
    }

    /// Deletes an outbox entry by task execution ID.
    ///
    /// This is called when a task is claimed to remove it from the work queue.
    pub async fn delete_by_task(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.delete_by_task_postgres(task_execution_id).await,
            self.delete_by_task_sqlite(task_execution_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn delete_by_task_postgres(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            diesel::delete(
                task_outbox::table.filter(task_outbox::task_execution_id.eq(task_execution_id)),
            )
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn delete_by_task_sqlite(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            diesel::delete(
                task_outbox::table.filter(task_outbox::task_execution_id.eq(task_execution_id)),
            )
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Lists all pending outbox entries (for polling-based claiming).
    ///
    /// Returns entries ordered by creation time (oldest first).
    pub async fn list_pending(&self, limit: i64) -> Result<Vec<TaskOutbox>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_pending_postgres(limit).await,
            self.list_pending_sqlite(limit).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn list_pending_postgres(&self, limit: i64) -> Result<Vec<TaskOutbox>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedTaskOutbox> = conn
            .interact(move |conn| {
                task_outbox::table
                    .order(task_outbox::created_at.asc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results
            .into_iter()
            .map(|r| TaskOutbox {
                id: r.id,
                task_execution_id: r.task_execution_id,
                created_at: r.created_at,
            })
            .collect())
    }

    #[cfg(feature = "sqlite")]
    async fn list_pending_sqlite(&self, limit: i64) -> Result<Vec<TaskOutbox>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedTaskOutbox> = conn
            .interact(move |conn| {
                task_outbox::table
                    .order(task_outbox::created_at.asc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results
            .into_iter()
            .map(|r| TaskOutbox {
                id: r.id,
                task_execution_id: r.task_execution_id,
                created_at: r.created_at,
            })
            .collect())
    }

    /// Counts pending outbox entries (for monitoring).
    pub async fn count_pending(&self) -> Result<i64, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.count_pending_postgres().await,
            self.count_pending_sqlite().await
        )
    }

    #[cfg(feature = "postgres")]
    async fn count_pending_postgres(&self) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let count: i64 = conn
            .interact(move |conn| task_outbox::table.count().get_result(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }

    #[cfg(feature = "sqlite")]
    async fn count_pending_sqlite(&self) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let count: i64 = conn
            .interact(move |conn| task_outbox::table.count().get_result(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }

    /// Deletes stale outbox entries older than the specified timestamp.
    ///
    /// This is used for cleanup of orphaned entries that were never claimed
    /// (e.g., due to task failures or system crashes).
    pub async fn delete_older_than(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<i64, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.delete_older_than_postgres(cutoff).await,
            self.delete_older_than_sqlite(cutoff).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn delete_older_than_postgres(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let deleted: usize = conn
            .interact(move |conn| {
                diesel::delete(task_outbox::table.filter(task_outbox::created_at.lt(cutoff)))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(deleted as i64)
    }

    #[cfg(feature = "sqlite")]
    async fn delete_older_than_sqlite(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let deleted: usize = conn
            .interact(move |conn| {
                diesel::delete(task_outbox::table.filter(task_outbox::created_at.lt(cutoff)))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(deleted as i64)
    }
}
