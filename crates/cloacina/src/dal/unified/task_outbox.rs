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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::models::task_execution::NewTaskExecution;
    use crate::models::task_outbox::NewTaskOutbox;
    use crate::models::workflow_execution::NewWorkflowExecution;

    #[cfg(feature = "sqlite")]
    async fn unique_dal() -> DAL {
        let url = format!(
            "file:outbox_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    /// Helper: create a workflow execution + task, mark it ready (which inserts into outbox),
    /// and return the task execution ID.
    #[cfg(feature = "sqlite")]
    async fn create_ready_task(dal: &DAL, task_name: &str) -> UniversalUuid {
        let wf_exec = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "test_workflow".into(),
                workflow_version: "1.0".into(),
                status: "Running".into(),
                context_id: None,
            })
            .await
            .unwrap();

        let task = dal
            .task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: wf_exec.id,
                task_name: task_name.into(),
                status: "NotStarted".into(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: r#"{"type":"Always"}"#.into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap();

        dal.task_execution().mark_ready(task.id).await.unwrap();

        task.id
    }

    // ── create + list_pending ──────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_create_outbox_entry() {
        let dal = unique_dal().await;
        let task_id = create_ready_task(&dal, "task_create_test").await;

        let pending = dal.task_outbox().list_pending(10).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].task_execution_id, task_id);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_pending_empty() {
        let dal = unique_dal().await;
        let pending = dal.task_outbox().list_pending(10).await.unwrap();
        assert!(pending.is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_pending_respects_limit() {
        let dal = unique_dal().await;
        create_ready_task(&dal, "task_a").await;
        create_ready_task(&dal, "task_b").await;
        create_ready_task(&dal, "task_c").await;

        let page = dal.task_outbox().list_pending(2).await.unwrap();
        assert_eq!(page.len(), 2);

        let all = dal.task_outbox().list_pending(100).await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_pending_ordered_oldest_first() {
        let dal = unique_dal().await;
        create_ready_task(&dal, "first").await;
        create_ready_task(&dal, "second").await;

        let pending = dal.task_outbox().list_pending(10).await.unwrap();
        assert_eq!(pending.len(), 2);
        // Verify ordered oldest first (created_at[0] <= created_at[1])
        let t0: chrono::DateTime<chrono::Utc> = pending[0].created_at.into();
        let t1: chrono::DateTime<chrono::Utc> = pending[1].created_at.into();
        assert!(t0 <= t1);
    }

    // ── count_pending ──────────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_count_pending_empty() {
        let dal = unique_dal().await;
        let count = dal.task_outbox().count_pending().await.unwrap();
        assert_eq!(count, 0);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_count_pending_after_inserts() {
        let dal = unique_dal().await;
        create_ready_task(&dal, "t1").await;
        create_ready_task(&dal, "t2").await;

        let count = dal.task_outbox().count_pending().await.unwrap();
        assert_eq!(count, 2);
    }

    // ── delete_by_task ─────────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_by_task() {
        let dal = unique_dal().await;
        let task_id = create_ready_task(&dal, "to_delete").await;

        // Verify it exists
        let count_before = dal.task_outbox().count_pending().await.unwrap();
        assert_eq!(count_before, 1);

        // Delete it
        dal.task_outbox().delete_by_task(task_id).await.unwrap();

        let count_after = dal.task_outbox().count_pending().await.unwrap();
        assert_eq!(count_after, 0);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_by_task_nonexistent() {
        let dal = unique_dal().await;
        // Deleting a nonexistent entry should not error
        let bogus = UniversalUuid::new_v4();
        dal.task_outbox().delete_by_task(bogus).await.unwrap();
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_by_task_only_removes_target() {
        let dal = unique_dal().await;
        let task_a = create_ready_task(&dal, "keep_me").await;
        let task_b = create_ready_task(&dal, "delete_me").await;

        dal.task_outbox().delete_by_task(task_b).await.unwrap();

        let remaining = dal.task_outbox().list_pending(10).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].task_execution_id, task_a);
    }

    // ── delete_older_than ──────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_older_than() {
        let dal = unique_dal().await;
        create_ready_task(&dal, "old_task").await;

        // Use a cutoff in the future so all current entries are "older than" it
        let future_cutoff =
            UniversalTimestamp::from(chrono::Utc::now() + chrono::Duration::hours(1));

        let deleted = dal
            .task_outbox()
            .delete_older_than(future_cutoff)
            .await
            .unwrap();
        assert_eq!(deleted, 1);

        let count = dal.task_outbox().count_pending().await.unwrap();
        assert_eq!(count, 0);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_older_than_keeps_recent() {
        let dal = unique_dal().await;
        create_ready_task(&dal, "recent_task").await;

        // Use a cutoff in the past so nothing is older
        let past_cutoff = UniversalTimestamp::from(chrono::Utc::now() - chrono::Duration::hours(1));

        let deleted = dal
            .task_outbox()
            .delete_older_than(past_cutoff)
            .await
            .unwrap();
        assert_eq!(deleted, 0);

        let count = dal.task_outbox().count_pending().await.unwrap();
        assert_eq!(count, 1);
    }

    // ── direct create (bypassing mark_ready) ───────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_direct_create() {
        let dal = unique_dal().await;
        // Create a workflow execution + task first (FK constraint)
        let wf_exec = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "p".into(),
                workflow_version: "1".into(),
                status: "Running".into(),
                context_id: None,
            })
            .await
            .unwrap();
        let task = dal
            .task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: wf_exec.id,
                task_name: "direct".into(),
                status: "NotStarted".into(),
                attempt: 1,
                max_attempts: 1,
                trigger_rules: r#"{"type":"Always"}"#.into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap();

        let entry = dal
            .task_outbox()
            .create(NewTaskOutbox {
                task_execution_id: task.id,
            })
            .await
            .unwrap();

        assert_eq!(entry.task_execution_id, task.id);
        assert_eq!(dal.task_outbox().count_pending().await.unwrap(), 1);
    }

    // ── integration: mark_ready populates outbox ───────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_mark_ready_populates_outbox() {
        let dal = unique_dal().await;
        let wf_exec = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "p".into(),
                workflow_version: "1".into(),
                status: "Running".into(),
                context_id: None,
            })
            .await
            .unwrap();
        let task = dal
            .task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: wf_exec.id,
                task_name: "ready_test".into(),
                status: "NotStarted".into(),
                attempt: 1,
                max_attempts: 1,
                trigger_rules: r#"{"type":"Always"}"#.into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap();

        // Before mark_ready: no outbox entries
        assert_eq!(dal.task_outbox().count_pending().await.unwrap(), 0);

        dal.task_execution().mark_ready(task.id).await.unwrap();

        // After mark_ready: exactly one outbox entry
        let pending = dal.task_outbox().list_pending(10).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].task_execution_id, task.id);
    }
}
