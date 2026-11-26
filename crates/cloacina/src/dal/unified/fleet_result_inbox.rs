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

//! Fleet result inbox DAL (CLOACINA-T-1015).
//!
//! An agent posts its report to any server replica, but the wait for it lives
//! in the memory of the replica that dispatched the task. A replica that gets
//! a report it has no wait for [`store`](FleetResultInboxDAL::store)s it here.
//! The replica that waits reads the rows of its waits
//! ([`list_for`](FleetResultInboxDAL::list_for)) and
//! [`take`](FleetResultInboxDAL::take)s the one it accepts. `take` is a delete
//! by row id: of several callers, one gets `true`.
//!
//! A row nobody takes is a report nobody waits for. It changes no state
//! (CLOACINA-A-0017) and goes away by age
//! ([`delete_older_than`](FleetResultInboxDAL::delete_older_than)).
//!
//! The fleet runs on Postgres only (CLOACINA-A-0006); the SQLite arms exist
//! for schema parity and tests.

use super::DAL;
use super::models::{NewUnifiedFleetResultInbox, UnifiedFleetResultInbox};
use crate::database::schema::unified::fleet_result_inbox;
use crate::database::universal_types::{UniversalBinary, UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use diesel::prelude::*;

/// One stored agent report.
#[derive(Debug, Clone)]
pub struct StoredFleetResult {
    /// Row id: the handle for [`FleetResultInboxDAL::take`].
    pub id: UniversalUuid,
    /// The key of the wait: a task execution id or a graph firing id.
    pub task_execution_id: UniversalUuid,
    /// The dispatch the report is for (CLOACINA-A-0016), when the agent sent one.
    pub dispatch_id: Option<String>,
    /// The report (`AgentResultRequest`) as JSON.
    pub payload: Vec<u8>,
    pub created_at: UniversalTimestamp,
}

/// Data access layer for the fleet result inbox.
#[derive(Clone)]
pub struct FleetResultInboxDAL<'a> {
    dal: &'a DAL,
}

impl<'a> FleetResultInboxDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Store a report. Two reports for the same dispatch are two rows: the
    /// wait takes one, the other goes away by age.
    pub async fn store(
        &self,
        task_execution_id: UniversalUuid,
        dispatch_id: Option<String>,
        payload: Vec<u8>,
    ) -> Result<UniversalUuid, ValidationError> {
        let id = UniversalUuid::new_v4();
        let row = NewUnifiedFleetResultInbox {
            id,
            task_execution_id,
            dispatch_id,
            payload: UniversalBinary::new(payload),
            created_at: UniversalTimestamp::now(),
        };
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(fleet_result_inbox::table)
                .values(&row)
                .execute(conn)
        })?;
        Ok(id)
    }

    /// The stored reports for any of `task_execution_ids`, oldest first.
    pub async fn list_for(
        &self,
        task_execution_ids: &[UniversalUuid],
    ) -> Result<Vec<StoredFleetResult>, ValidationError> {
        if task_execution_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids = task_execution_ids.to_vec();
        let rows: Vec<UnifiedFleetResultInbox> = crate::interact_on_backend!(self.dal, |conn| {
            fleet_result_inbox::table
                .filter(fleet_result_inbox::task_execution_id.eq_any(ids))
                .order(fleet_result_inbox::created_at.asc())
                .load(conn)
        })?;
        Ok(rows
            .into_iter()
            .map(|r| StoredFleetResult {
                id: r.id,
                task_execution_id: r.task_execution_id,
                dispatch_id: r.dispatch_id,
                payload: r.payload.into_inner(),
                created_at: r.created_at,
            })
            .collect())
    }

    /// Take a stored report: delete its row. `true` when this call removed
    /// the row, `false` when it was already gone (another replica took it).
    pub async fn take(&self, id: UniversalUuid) -> Result<bool, ValidationError> {
        let affected: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(fleet_result_inbox::table.filter(fleet_result_inbox::id.eq(id)))
                .execute(conn)
        })?;
        Ok(affected == 1)
    }

    /// Delete the reports stored before `cutoff`. Returns how many: each one
    /// is a report nobody waited for.
    pub async fn delete_older_than(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<usize, ValidationError> {
        let affected: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(
                fleet_result_inbox::table.filter(fleet_result_inbox::created_at.lt(cutoff)),
            )
            .execute(conn)
        })?;
        Ok(affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    #[cfg(feature = "sqlite")]
    async fn unique_dal() -> DAL {
        let url = format!(
            "file:fleet_result_inbox_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_store_then_list_for_returns_only_the_asked_ids() {
        let dal = unique_dal().await;
        let inbox = dal.fleet_result_inbox();
        let a = UniversalUuid::new_v4();
        let b = UniversalUuid::new_v4();
        inbox
            .store(a, Some("d1".into()), b"report-a".to_vec())
            .await
            .unwrap();
        inbox.store(b, None, b"report-b".to_vec()).await.unwrap();

        let rows = inbox.list_for(&[a]).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].task_execution_id, a);
        assert_eq!(rows[0].dispatch_id.as_deref(), Some("d1"));
        assert_eq!(rows[0].payload, b"report-a".to_vec());

        assert_eq!(inbox.list_for(&[a, b]).await.unwrap().len(), 2);
        assert!(inbox.list_for(&[]).await.unwrap().is_empty());
        assert!(
            inbox
                .list_for(&[UniversalUuid::new_v4()])
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_take_gives_the_row_to_one_caller() {
        let dal = unique_dal().await;
        let inbox = dal.fleet_result_inbox();
        let task = UniversalUuid::new_v4();
        let id = inbox.store(task, None, b"r".to_vec()).await.unwrap();

        assert!(inbox.take(id).await.unwrap());
        assert!(!inbox.take(id).await.unwrap());
        assert!(inbox.list_for(&[task]).await.unwrap().is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_two_reports_of_one_dispatch_are_two_rows() {
        let dal = unique_dal().await;
        let inbox = dal.fleet_result_inbox();
        let task = UniversalUuid::new_v4();
        let first = inbox
            .store(task, Some("d".into()), b"1".to_vec())
            .await
            .unwrap();
        let second = inbox
            .store(task, Some("d".into()), b"2".to_vec())
            .await
            .unwrap();
        assert_ne!(first, second);
        assert_eq!(inbox.list_for(&[task]).await.unwrap().len(), 2);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_older_than_removes_only_old_rows() {
        let dal = unique_dal().await;
        let inbox = dal.fleet_result_inbox();
        let task = UniversalUuid::new_v4();
        inbox.store(task, None, b"r".to_vec()).await.unwrap();

        let before = UniversalTimestamp(chrono::Utc::now() - chrono::Duration::minutes(1));
        assert_eq!(inbox.delete_older_than(before).await.unwrap(), 0);
        assert_eq!(inbox.list_for(&[task]).await.unwrap().len(), 1);

        let after = UniversalTimestamp(chrono::Utc::now() + chrono::Duration::minutes(1));
        assert_eq!(inbox.delete_older_than(after).await.unwrap(), 1);
        assert!(inbox.list_for(&[task]).await.unwrap().is_empty());
    }
}
