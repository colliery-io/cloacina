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

//! Unified Execution Event DAL with runtime backend selection
//!
//! This module provides CRUD operations for ExecutionEvent entities that work with
//! both PostgreSQL and SQLite backends, selecting the appropriate implementation
//! at runtime based on the database connection type.
//!
//! Execution events form an append-only audit trail of all task and workflow
//! state transitions for debugging, compliance, and replay capability.

use super::models::{NewUnifiedDeliveryOutbox, NewUnifiedExecutionEvent, UnifiedExecutionEvent};
use super::DAL;
use crate::database::schema::unified::{delivery_outbox, execution_events};
use crate::database::universal_types::{UniversalBinary, UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::execution_event::{ExecutionEvent, ExecutionEventType, NewExecutionEvent};
use diesel::prelude::*;

/// Substrate (CLOACI-I-0115 / T-0629): build the `delivery_outbox` row that
/// gets inserted in the same transaction as the event, so a subscribed CLI /
/// future SDK receives the event over the substrate WS. Recipient is keyed by
/// `workflow_execution_id` (convention: `exec_events:<uuid>`). Payload is JSON
/// of the fields the consumer displays — sequence_num is intentionally omitted
/// (substrate row id provides arrival ordering).
fn build_event_outbox_row(
    event_id: UniversalUuid,
    new_event: &NewExecutionEvent,
    now: UniversalTimestamp,
) -> Result<NewUnifiedDeliveryOutbox, ValidationError> {
    let payload = serde_json::json!({
        "id": event_id.0.to_string(),
        "workflow_execution_id": new_event.workflow_execution_id.0.to_string(),
        "task_execution_id": new_event.task_execution_id.map(|u| u.0.to_string()),
        "event_type": new_event.event_type.as_str(),
        "event_data": new_event.event_data.as_deref(),
        "created_at": now.0.to_rfc3339(),
    });
    let bytes = serde_json::to_vec(&payload).map_err(|e| {
        ValidationError::ConnectionPool(format!("execution_event outbox payload: {}", e))
    })?;
    Ok(NewUnifiedDeliveryOutbox {
        recipient: format!("exec_events:{}", new_event.workflow_execution_id.0),
        kind: "execution_event".to_string(),
        tenant_id: new_event.tenant_id.clone(),
        payload: UniversalBinary::from(bytes),
        delivery_state: "pending".to_string(),
        delivery_attempts: 0,
        created_at: now,
    })
}

/// Data access layer for execution event operations with runtime backend selection.
///
/// This DAL provides methods for creating and querying execution events,
/// which track all state transitions for tasks and workflow executions.
#[derive(Clone)]
pub struct ExecutionEventDAL<'a> {
    dal: &'a DAL,
}

// Several methods on this impl block are kept as future admin/ops
// surface (per T-0565): `list_by_task`, `list_by_type`, `get_recent`,
// `count_by_workflow`. They have zero in-tree callers today but are
// preserved as the "deliberately complete CRUD surface" the audit
// flagged. Re-promote to `pub` if a real consumer arrives.
#[allow(dead_code)]
impl<'a> ExecutionEventDAL<'a> {
    /// Creates a new ExecutionEventDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new execution event record.
    ///
    /// Events are append-only and never updated after creation. Each event
    /// receives a monotonically increasing sequence number for ordering.
    pub async fn create(
        &self,
        new_event: NewExecutionEvent,
    ) -> Result<ExecutionEvent, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_event).await,
            self.create_sqlite(new_event).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn create_postgres(
        &self,
        new_event: NewExecutionEvent,
    ) -> Result<ExecutionEvent, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        // Build the outbox row BEFORE moving fields into `new_unified`
        // (`NewExecutionEvent` is not Copy and we need its fields still
        // accessible for the substrate payload).
        let outbox_row = build_event_outbox_row(id, &new_event, now)?;
        let new_unified = NewUnifiedExecutionEvent {
            id,
            workflow_execution_id: new_event.workflow_execution_id,
            task_execution_id: new_event.task_execution_id,
            event_type: new_event.event_type,
            event_data: new_event.event_data,
            worker_id: new_event.worker_id,
            created_at: now,
            request_id: new_event.request_id,
            runner_id: new_event.runner_id,
            tenant_id: new_event.tenant_id,
        };
        let result: UnifiedExecutionEvent = conn
            .interact(move |conn| {
                conn.transaction::<_, diesel::result::Error, _>(|conn| {
                    let event: UnifiedExecutionEvent = diesel::insert_into(execution_events::table)
                        .values(&new_unified)
                        .get_result(conn)?;
                    diesel::insert_into(delivery_outbox::table)
                        .values(&outbox_row)
                        .execute(conn)?;
                    Ok(event)
                })
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    async fn create_sqlite(
        &self,
        new_event: NewExecutionEvent,
    ) -> Result<ExecutionEvent, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        // Build the outbox row BEFORE moving fields into `new_unified`
        // (`NewExecutionEvent` is not Copy and we need its fields still
        // accessible for the substrate payload).
        let outbox_row = build_event_outbox_row(id, &new_event, now)?;
        let new_unified = NewUnifiedExecutionEvent {
            id,
            workflow_execution_id: new_event.workflow_execution_id,
            task_execution_id: new_event.task_execution_id,
            event_type: new_event.event_type,
            event_data: new_event.event_data,
            worker_id: new_event.worker_id,
            created_at: now,
            request_id: new_event.request_id,
            runner_id: new_event.runner_id,
            tenant_id: new_event.tenant_id,
        };

        // Wrap both inserts + the post-commit select in a single interact so
        // they share one SQLite connection (and thus one transaction for the
        // inserts). SQLite doesn't support RETURNING here, so we fetch the
        // row by id after the txn commits — sequence_num is auto-assigned.
        let result: UnifiedExecutionEvent = conn
            .interact(move |conn| {
                conn.transaction::<_, diesel::result::Error, _>(|conn| {
                    diesel::insert_into(execution_events::table)
                        .values(&new_unified)
                        .execute(conn)?;
                    diesel::insert_into(delivery_outbox::table)
                        .values(&outbox_row)
                        .execute(conn)?;
                    Ok(())
                })?;
                execution_events::table
                    .filter(execution_events::id.eq(id))
                    .first::<UnifiedExecutionEvent>(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    /// Gets all execution events for a specific workflow execution, ordered by sequence.
    pub async fn list_by_workflow(
        &self,
        workflow_execution_id: UniversalUuid,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_by_workflow_postgres(workflow_execution_id).await,
            self.list_by_workflow_sqlite(workflow_execution_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn list_by_workflow_postgres(
        &self,
        workflow_execution_id: UniversalUuid,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::workflow_execution_id.eq(workflow_execution_id))
                    .order(execution_events::sequence_num.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn list_by_workflow_sqlite(
        &self,
        workflow_execution_id: UniversalUuid,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::workflow_execution_id.eq(workflow_execution_id))
                    .order(execution_events::sequence_num.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Gets all execution events for a specific task execution, ordered by sequence.
    pub(crate) async fn list_by_task(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_by_task_postgres(task_execution_id).await,
            self.list_by_task_sqlite(task_execution_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn list_by_task_postgres(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::task_execution_id.eq(task_execution_id))
                    .order(execution_events::sequence_num.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn list_by_task_sqlite(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::task_execution_id.eq(task_execution_id))
                    .order(execution_events::sequence_num.asc())
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Gets execution events by type for monitoring and analysis.
    pub(crate) async fn list_by_type(
        &self,
        event_type: ExecutionEventType,
        limit: i64,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_by_type_postgres(event_type, limit).await,
            self.list_by_type_sqlite(event_type, limit).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn list_by_type_postgres(
        &self,
        event_type: ExecutionEventType,
        limit: i64,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let event_type_str = event_type.as_str().to_string();
        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::event_type.eq(event_type_str))
                    .order(execution_events::created_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn list_by_type_sqlite(
        &self,
        event_type: ExecutionEventType,
        limit: i64,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let event_type_str = event_type.as_str().to_string();
        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::event_type.eq(event_type_str))
                    .order(execution_events::created_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Gets recent execution events for monitoring purposes.
    pub(crate) async fn get_recent(
        &self,
        limit: i64,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_recent_postgres(limit).await,
            self.get_recent_sqlite(limit).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_recent_postgres(
        &self,
        limit: i64,
    ) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .order(execution_events::created_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn get_recent_sqlite(&self, limit: i64) -> Result<Vec<ExecutionEvent>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let results: Vec<UnifiedExecutionEvent> = conn
            .interact(move |conn| {
                execution_events::table
                    .order(execution_events::created_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Deletes execution events older than the specified timestamp.
    ///
    /// Used for retention policy enforcement to prevent unbounded table growth.
    /// Returns the number of deleted events.
    pub async fn delete_older_than(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<usize, ValidationError> {
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
    ) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let deleted: usize = conn
            .interact(move |conn| {
                diesel::delete(
                    execution_events::table.filter(execution_events::created_at.lt(cutoff)),
                )
                .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(deleted)
    }

    #[cfg(feature = "sqlite")]
    async fn delete_older_than_sqlite(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<usize, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let deleted: usize = conn
            .interact(move |conn| {
                diesel::delete(
                    execution_events::table.filter(execution_events::created_at.lt(cutoff)),
                )
                .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(deleted)
    }

    /// Counts total execution events for a workflow execution.
    pub(crate) async fn count_by_workflow(
        &self,
        workflow_execution_id: UniversalUuid,
    ) -> Result<i64, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.count_by_workflow_postgres(workflow_execution_id).await,
            self.count_by_workflow_sqlite(workflow_execution_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn count_by_workflow_postgres(
        &self,
        workflow_execution_id: UniversalUuid,
    ) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let count: i64 = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::workflow_execution_id.eq(workflow_execution_id))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }

    #[cfg(feature = "sqlite")]
    async fn count_by_workflow_sqlite(
        &self,
        workflow_execution_id: UniversalUuid,
    ) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let count: i64 = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::workflow_execution_id.eq(workflow_execution_id))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }

    /// Counts execution events older than the specified timestamp.
    ///
    /// Used for dry-run mode to preview how many events would be deleted.
    pub async fn count_older_than(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<i64, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.count_older_than_postgres(cutoff).await,
            self.count_older_than_sqlite(cutoff).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn count_older_than_postgres(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let count: i64 = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::created_at.lt(cutoff))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }

    #[cfg(feature = "sqlite")]
    async fn count_older_than_sqlite(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<i64, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let count: i64 = conn
            .interact(move |conn| {
                execution_events::table
                    .filter(execution_events::created_at.lt(cutoff))
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count)
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod substrate_producer_tests {
    use super::*;
    use crate::database::Database;
    use crate::models::workflow_execution::NewWorkflowExecution;

    async fn unique_dal() -> DAL {
        let url = format!(
            "file:event_outbox_producer_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    async fn make_wf_exec(dal: &DAL) -> UniversalUuid {
        dal.workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "exec-events-producer-test".into(),
                workflow_version: "1.0".into(),
                status: "Running".into(),
                context_id: None,
            })
            .await
            .unwrap()
            .id
    }

    /// Creating an execution event also enqueues a `delivery_outbox` row with
    /// the matching recipient + tenant + JSON payload — the in-txn enqueue
    /// promised in T-0625 (REQ-1.1.1), here landed on its first real producer.
    #[tokio::test]
    async fn event_create_also_enqueues_delivery_outbox_row() {
        let dal = unique_dal().await;
        let wf_id = make_wf_exec(&dal).await;

        let event = dal
            .execution_event()
            .create(NewExecutionEvent {
                workflow_execution_id: wf_id,
                task_execution_id: None,
                event_type: "task_started".into(),
                event_data: Some(r#"{"task":"alpha"}"#.into()),
                worker_id: None,
                request_id: None,
                runner_id: None,
                tenant_id: Some("t1".into()),
            })
            .await
            .unwrap();

        let expected_recipient = format!("exec_events:{}", wf_id.0);
        let pending = dal.delivery_outbox().list_pending(10).await.unwrap();
        assert_eq!(pending.len(), 1, "exactly one outbox row per event");
        let row = &pending[0];
        assert_eq!(row.recipient, expected_recipient);
        assert_eq!(row.kind, "execution_event");
        assert_eq!(row.tenant_id.as_deref(), Some("t1"));

        // Payload is JSON with the displayable event fields. Decode to check.
        let parsed: serde_json::Value = serde_json::from_slice(&row.payload).unwrap();
        assert_eq!(parsed["id"], event.id.0.to_string());
        assert_eq!(parsed["event_type"], "task_started");
        assert_eq!(parsed["event_data"], r#"{"task":"alpha"}"#);
        assert_eq!(parsed["workflow_execution_id"], wf_id.0.to_string());
    }

    /// Two events for the same workflow produce two distinct outbox rows under
    /// the same recipient — proves the producer doesn't dedupe or replace.
    #[tokio::test]
    async fn two_events_produce_two_outbox_rows_for_same_recipient() {
        let dal = unique_dal().await;
        let wf_id = make_wf_exec(&dal).await;

        for et in &["task_started", "task_completed"] {
            dal.execution_event()
                .create(NewExecutionEvent {
                    workflow_execution_id: wf_id,
                    task_execution_id: None,
                    event_type: (*et).into(),
                    event_data: None,
                    worker_id: None,
                    request_id: None,
                    runner_id: None,
                    tenant_id: None,
                })
                .await
                .unwrap();
        }

        let recipient = format!("exec_events:{}", wf_id.0);
        let rows = dal
            .delivery_outbox()
            .list_open_for_recipient(&recipient, 10)
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
        // Substrate row id is monotonic — preserves event ordering for the recipient.
        assert!(rows[0].id < rows[1].id);
    }
}
