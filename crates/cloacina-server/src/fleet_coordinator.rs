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

//! Cross-request rendezvous between [`crate::fleet_executor::FleetExecutor`]
//! and the agent's `POST /v1/agent/result` handler (CLOACI-T-0633).
//!
//! `FleetExecutor::execute` is an async call that must block until the agent
//! reports back, but the agent's result arrives on a separate HTTP request
//! handled by [`crate::routes::agent::report_result`]. They meet here: the
//! executor calls [`FleetCoordinator::register_pending`] to get a oneshot
//! receiver before enqueueing the work packet, then `await`s. The result
//! handler calls [`FleetCoordinator::forward`] to push the result through.
//!
//! The waits are an in-memory `Mutex<HashMap>` per replica, and an agent
//! posts its report to whichever replica the Service picks. So a report can
//! reach a replica that did not dispatch the task (CLOACINA-T-1015).
//! [`FleetCoordinator::accept`] is the entry point of the result handler: it
//! forwards to a local wait, or stores the report in the `fleet_result_inbox`
//! table. Each replica runs [`run_result_inbox`], which takes the stored
//! reports that its own waits accept ([`FleetCoordinator::collect_stored`])
//! on a `NOTIFY` and on a slow tick, and deletes the reports nobody took.
//!
//! The rules of CLOACINA-A-0016 and CLOACINA-A-0017 hold on both paths: a
//! report resolves only its own dispatch, the terminal write stays with the
//! executor that holds the claim, and a report nobody waits for changes no
//! state.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cloacina::dal::DAL;
use cloacina::database::universal_types::{UniversalTimestamp, UniversalUuid};
use cloacina::delivery::WakeHandle;
use cloacina::error::ValidationError;
use cloacina::fleet::AgentResultRequest;
use tokio::sync::{oneshot, watch};
use tracing::{debug, info, warn};

/// The Postgres `NOTIFY` channel of the `fleet_result_inbox` insert trigger.
pub const RESULT_INBOX_CHANNEL: &str = "fleet_result_inbox";

/// How often a replica with waits reads the inbox with no `NOTIFY`. This is
/// the backstop for a notification that was missed (the listener reconnects
/// after a connection loss), so it bounds how late a stored report can be.
pub const RESULT_INBOX_CHECK_INTERVAL: Duration = Duration::from_secs(5);

/// How long a stored report is kept. The replica that waits takes a report
/// within one check interval, so a row this old has no wait anywhere.
pub const RESULT_INBOX_RETENTION: Duration = Duration::from_secs(15 * 60);

/// How often a replica deletes the stored reports past the retention.
const RESULT_INBOX_SWEEP_INTERVAL: Duration = Duration::from_secs(60);

/// The most ids in one inbox read.
const RESULT_INBOX_READ_CHUNK: usize = 500;

/// What a replica did with an incoming report ([`FleetCoordinator::accept`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accepted {
    /// A wait on this replica got the report.
    Local,
    /// This replica has no wait for the report: it is stored for the replica
    /// that has one.
    Stored,
}

/// Whether a wait for `want` takes a report that carries `got`
/// (CLOACINA-A-0016): the ids are equal, or one side has none.
fn dispatch_matches(want: &Option<String>, got: &Option<String>) -> bool {
    match (want, got) {
        (Some(want), Some(got)) => want == got,
        _ => true,
    }
}

/// One wait of an executor for an agent report.
struct Pending {
    /// The dispatch this wait is for (CLOACINA-T-1011). `None` accepts any
    /// report for the id (graph firings, which have one dispatch per id).
    dispatch_id: Option<String>,
    tx: oneshot::Sender<AgentResultRequest>,
}

#[derive(Default)]
pub struct FleetCoordinator {
    pending: Mutex<HashMap<UniversalUuid, Pending>>,
}

impl FleetCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a pending result expectation. Returns the receiver the
    /// executor awaits. If the executor is restarted or cancels, drop the
    /// receiver — the sender drops too and any later [`forward`] for that
    /// id is reported back to the caller via `Err(result)`.
    pub fn register_pending(
        &self,
        task_execution_id: UniversalUuid,
    ) -> oneshot::Receiver<AgentResultRequest> {
        self.register(task_execution_id, None)
    }

    /// Register a wait for one named dispatch of a task (CLOACINA-T-1011,
    /// CLOACINA-A-0016).
    /// [`forward`](Self::forward) gives this wait only a report that carries
    /// the same `dispatch_id`, or no id at all (an agent from before the
    /// field). A report for an older dispatch of the task is an orphan and
    /// leaves this wait in place.
    pub fn register_dispatch(
        &self,
        task_execution_id: UniversalUuid,
        dispatch_id: String,
    ) -> oneshot::Receiver<AgentResultRequest> {
        self.register(task_execution_id, Some(dispatch_id))
    }

    fn register(
        &self,
        task_execution_id: UniversalUuid,
        dispatch_id: Option<String>,
    ) -> oneshot::Receiver<AgentResultRequest> {
        let (tx, rx) = oneshot::channel();
        let mut g = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(_prev) = g.insert(task_execution_id, Pending { dispatch_id, tx }) {
            warn!(
                task_id = %task_execution_id,
                "FleetCoordinator: replaced an existing pending entry — \
                 the prior executor invocation will see its oneshot drop"
            );
        }
        rx
    }

    /// Forward an incoming agent result to the executor waiting on it.
    ///
    /// Returns `Ok(())` when a waiting executor received the result.
    /// Returns `Err(result)` when no executor was waiting (orphan report —
    /// caller may log + drop or retain as bookkeeping; v1 just logs).
    pub fn forward(
        &self,
        task_execution_id: UniversalUuid,
        result: AgentResultRequest,
    ) -> Result<(), AgentResultRequest> {
        let pending = {
            let mut g = self.pending.lock().unwrap_or_else(|e| e.into_inner());
            let other_dispatch = match g.get(&task_execution_id) {
                Some(p) => !dispatch_matches(&p.dispatch_id, &result.dispatch_id),
                None => false,
            };
            if other_dispatch {
                debug!(
                    task_id = %task_execution_id,
                    "FleetCoordinator: report is for a different dispatch of this task (orphan); \
                     the pending wait stays"
                );
                return Err(result);
            }
            g.remove(&task_execution_id)
        };
        match pending {
            Some(p) => p.tx.send(result).map_err(|r| {
                debug!(
                    task_id = %task_execution_id,
                    "FleetCoordinator: pending receiver dropped before forward; \
                     likely executor canceled or timed out"
                );
                r
            }),
            None => {
                debug!(
                    task_id = %task_execution_id,
                    "FleetCoordinator: no pending entry for incoming result (orphan)"
                );
                Err(result)
            }
        }
    }

    /// Take in a report that reached this replica (CLOACINA-T-1015).
    ///
    /// A wait on this replica gets it ([`forward`](Self::forward)). With no
    /// such wait the report is stored in `inbox` (the admin database, shared
    /// by all replicas) for the replica that waits. `Err` means the report is
    /// neither forwarded nor stored: the caller must tell the agent, so the
    /// agent sends it again.
    pub async fn accept(
        &self,
        inbox: &DAL,
        task_execution_id: UniversalUuid,
        report: AgentResultRequest,
    ) -> Result<Accepted, String> {
        let report = match self.forward(task_execution_id, report) {
            Ok(()) => return Ok(Accepted::Local),
            Err(report) => report,
        };
        let payload =
            serde_json::to_vec(&report).map_err(|e| format!("serialize agent report: {e}"))?;
        inbox
            .fleet_result_inbox()
            .store(task_execution_id, report.dispatch_id.clone(), payload)
            .await
            .map_err(|e| format!("store agent report: {e}"))?;
        Ok(Accepted::Stored)
    }

    /// Take the stored reports that the waits of this replica accept, and
    /// resolve those waits. Returns how many waits were resolved.
    ///
    /// A row is taken only when a wait here would accept it, and forwarded
    /// only when this replica's delete removed the row, so a report reaches
    /// one wait. A row for another dispatch of a task that waits here is left
    /// alone; it goes away by age.
    pub async fn collect_stored(&self, inbox: &DAL) -> Result<usize, ValidationError> {
        let ids = self.waiting_ids();
        let mut resolved = 0usize;
        for chunk in ids.chunks(RESULT_INBOX_READ_CHUNK) {
            let rows = inbox.fleet_result_inbox().list_for(chunk).await?;
            for row in rows {
                if !self.would_accept(row.task_execution_id, &row.dispatch_id) {
                    continue;
                }
                if !inbox.fleet_result_inbox().take(row.id).await? {
                    continue;
                }
                let report: AgentResultRequest = match serde_json::from_slice(&row.payload) {
                    Ok(r) => r,
                    Err(e) => {
                        warn!(
                            task_id = %row.task_execution_id,
                            error = %e,
                            "fleet result inbox: stored report does not parse; dropped"
                        );
                        continue;
                    }
                };
                match self.forward(row.task_execution_id, report) {
                    Ok(()) => {
                        debug!(
                            task_id = %row.task_execution_id,
                            "fleet result inbox: report from another replica resolved a wait here"
                        );
                        resolved += 1;
                    }
                    Err(orphan) => {
                        // The wait ended between the check and the forward.
                        metrics::counter!("cloacina_fleet_orphan_results_total").increment(1);
                        info!(
                            task_id = %row.task_execution_id,
                            agent_id = %orphan.agent_id,
                            attempt = orphan.attempt,
                            dispatch_id = ?orphan.dispatch_id,
                            "stored agent result is for no pending dispatch (orphan): no state changed"
                        );
                    }
                }
            }
        }
        Ok(resolved)
    }

    /// The ids this replica waits for.
    fn waiting_ids(&self) -> Vec<UniversalUuid> {
        let g = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        g.keys().copied().collect()
    }

    /// Whether a wait on this replica takes a report with this dispatch id.
    fn would_accept(&self, task_execution_id: UniversalUuid, dispatch_id: &Option<String>) -> bool {
        let g = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        g.get(&task_execution_id)
            .is_some_and(|p| dispatch_matches(&p.dispatch_id, dispatch_id))
    }

    /// Drop a pending entry without sending. Called by the executor when it
    /// bails before the oneshot could be awaited (e.g. enqueue failure).
    pub fn cancel(&self, task_execution_id: UniversalUuid) {
        let mut g = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        g.remove(&task_execution_id);
    }

    /// Pending count — useful for metrics/debug.
    pub fn pending_count(&self) -> usize {
        self.pending.lock().unwrap_or_else(|e| e.into_inner()).len()
    }
}

/// Delete the stored reports older than `retention`. Each one is a report
/// nobody waited for (CLOACINA-A-0017): counted in
/// `cloacina_fleet_orphan_results_total`. Returns how many were deleted.
pub async fn sweep_result_inbox(
    inbox: &DAL,
    retention: Duration,
) -> Result<usize, ValidationError> {
    let retention = chrono::Duration::from_std(retention).unwrap_or(chrono::Duration::days(365));
    let cutoff = UniversalTimestamp(chrono::Utc::now() - retention);
    let deleted = inbox.fleet_result_inbox().delete_older_than(cutoff).await?;
    if deleted > 0 {
        metrics::counter!("cloacina_fleet_orphan_results_total").increment(deleted as u64);
        info!(
            deleted,
            "fleet result inbox: deleted stored agent results that no dispatch waited for \
             (orphans): no state changed"
        );
    }
    Ok(deleted)
}

/// The per-replica loop over the fleet result inbox (CLOACINA-T-1015).
///
/// It calls [`FleetCoordinator::collect_stored`] when `wake` fires (the
/// Postgres `NOTIFY` of an insert, forwarded by
/// `cloacina::delivery::run_pg_listener` on [`RESULT_INBOX_CHANNEL`]) and
/// every [`RESULT_INBOX_CHECK_INTERVAL`], and [`sweep_result_inbox`] every
/// minute. A read with no waits on this replica touches no database. Runs
/// until `shutdown` flips to `true` or its sender is dropped.
pub async fn run_result_inbox(
    coordinator: Arc<FleetCoordinator>,
    inbox: DAL,
    wake: WakeHandle,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut check = tokio::time::interval(RESULT_INBOX_CHECK_INTERVAL);
    check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut sweep = tokio::time::interval(RESULT_INBOX_SWEEP_INTERVAL);
    sweep.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = wake.notified() => {}
            _ = check.tick() => {}
            _ = sweep.tick() => {
                if let Err(e) = sweep_result_inbox(&inbox, RESULT_INBOX_RETENTION).await {
                    warn!(error = %e, "fleet result inbox: sweep failed");
                }
                continue;
            }
            res = shutdown.changed() => {
                if res.is_err() || *shutdown.borrow() {
                    break;
                }
                continue;
            }
        }
        if let Err(e) = coordinator.collect_stored(&inbox).await {
            warn!(error = %e, "fleet result inbox: read failed; the next tick tries again");
        }
    }
    debug!("fleet result inbox: loop stopped");
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina::fleet::{AGENT_PROTOCOL_VERSION, AgentOutcome};

    fn synthetic_result(task_id: UniversalUuid) -> AgentResultRequest {
        AgentResultRequest {
            protocol_version: AGENT_PROTOCOL_VERSION,
            agent_id: "agent-a".into(),
            task_execution_id: task_id.0.to_string(),
            attempt: 1,
            dispatch_id: None,
            duration_ms: 10,
            outcome: AgentOutcome::Success {
                context: serde_json::json!({}),
            },
        }
    }

    #[tokio::test]
    async fn register_then_forward_delivers_to_receiver() {
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let rx = coord.register_pending(task_id);
        let result = synthetic_result(task_id);
        assert!(coord.forward(task_id, result.clone()).is_ok());
        let got = rx.await.unwrap();
        assert_eq!(got.task_execution_id, result.task_execution_id);
    }

    #[tokio::test]
    async fn forward_without_pending_is_err_orphan() {
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let result = synthetic_result(task_id);
        let returned = coord.forward(task_id, result).unwrap_err();
        assert_eq!(returned.task_execution_id, task_id.0.to_string());
    }

    #[tokio::test]
    async fn cancel_removes_pending_so_subsequent_forward_is_orphan() {
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let _rx = coord.register_pending(task_id);
        coord.cancel(task_id);
        let result = synthetic_result(task_id);
        assert!(coord.forward(task_id, result).is_err());
    }

    #[tokio::test]
    async fn report_of_another_dispatch_is_orphan_and_the_wait_stays() {
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let rx = coord.register_dispatch(task_id, "second".into());

        let mut stale = synthetic_result(task_id);
        stale.dispatch_id = Some("first".into());
        assert!(coord.forward(task_id, stale).is_err());
        assert_eq!(coord.pending_count(), 1);

        let mut current = synthetic_result(task_id);
        current.dispatch_id = Some("second".into());
        assert!(coord.forward(task_id, current).is_ok());
        assert_eq!(rx.await.unwrap().dispatch_id.as_deref(), Some("second"));
    }

    #[tokio::test]
    async fn report_with_no_dispatch_id_resolves_a_dispatch_wait() {
        // An agent from before the field sends no id: accept it as before.
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let rx = coord.register_dispatch(task_id, "d".into());
        assert!(coord.forward(task_id, synthetic_result(task_id)).is_ok());
        assert!(rx.await.is_ok());
    }

    // ── CLOACINA-T-1015: a report that reaches another replica ───────────
    //
    // Two replicas are two coordinators over one database. SQLite in memory
    // stands in for the shared Postgres: no NOTIFY, so the tests call
    // `collect_stored` as the tick of `run_result_inbox` does.

    #[cfg(feature = "sqlite")]
    async fn shared_inbox() -> DAL {
        let url = format!(
            "file:fleet_coordinator_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = cloacina::database::Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    #[cfg(feature = "sqlite")]
    fn report_of(task_id: UniversalUuid, dispatch: &str) -> AgentResultRequest {
        let mut r = synthetic_result(task_id);
        r.dispatch_id = Some(dispatch.into());
        r
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn report_to_another_replica_resolves_the_wait_of_the_dispatching_replica() {
        let inbox = shared_inbox().await;
        let dispatching = FleetCoordinator::new();
        let other = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let rx = dispatching.register_dispatch(task_id, "d1".into());

        // The agent's POST lands on the replica that did not dispatch.
        let accepted = other
            .accept(&inbox, task_id, report_of(task_id, "d1"))
            .await
            .unwrap();
        assert_eq!(accepted, Accepted::Stored);
        assert_eq!(dispatching.pending_count(), 1);

        // The replica with no wait takes nothing.
        assert_eq!(other.collect_stored(&inbox).await.unwrap(), 0);
        // The replica that waits takes the report, one time.
        assert_eq!(dispatching.collect_stored(&inbox).await.unwrap(), 1);
        let got = rx.await.expect("the wait is resolved");
        assert_eq!(got.dispatch_id.as_deref(), Some("d1"));
        assert_eq!(dispatching.pending_count(), 0);
        assert!(
            inbox
                .fleet_result_inbox()
                .list_for(&[task_id])
                .await
                .unwrap()
                .is_empty(),
            "a taken report leaves no row"
        );
        assert_eq!(dispatching.collect_stored(&inbox).await.unwrap(), 0);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn report_to_the_dispatching_replica_is_not_stored() {
        let inbox = shared_inbox().await;
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let rx = coord.register_dispatch(task_id, "d1".into());
        let accepted = coord
            .accept(&inbox, task_id, report_of(task_id, "d1"))
            .await
            .unwrap();
        assert_eq!(accepted, Accepted::Local);
        assert!(rx.await.is_ok());
        assert!(
            inbox
                .fleet_result_inbox()
                .list_for(&[task_id])
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn stored_report_of_an_older_dispatch_leaves_the_wait_and_goes_by_age() {
        let inbox = shared_inbox().await;
        let dispatching = FleetCoordinator::new();
        let other = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        // The task was dispatched again: the wait is for the second dispatch.
        let mut rx = dispatching.register_dispatch(task_id, "second".into());

        other
            .accept(&inbox, task_id, report_of(task_id, "first"))
            .await
            .unwrap();
        assert_eq!(dispatching.collect_stored(&inbox).await.unwrap(), 0);
        assert_eq!(dispatching.pending_count(), 1);
        assert!(rx.try_recv().is_err(), "the wait is not resolved");
        assert_eq!(
            inbox
                .fleet_result_inbox()
                .list_for(&[task_id])
                .await
                .unwrap()
                .len(),
            1,
            "the row of the older dispatch is not taken"
        );

        // The report of the second dispatch arrives and is taken.
        other
            .accept(&inbox, task_id, report_of(task_id, "second"))
            .await
            .unwrap();
        assert_eq!(dispatching.collect_stored(&inbox).await.unwrap(), 1);
        assert_eq!(rx.await.unwrap().dispatch_id.as_deref(), Some("second"));

        // The orphan is still there until the age sweep.
        assert_eq!(
            sweep_result_inbox(&inbox, Duration::from_secs(3600))
                .await
                .unwrap(),
            0
        );
        assert_eq!(sweep_result_inbox(&inbox, Duration::ZERO).await.unwrap(), 1);
        assert!(
            inbox
                .fleet_result_inbox()
                .list_for(&[task_id])
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn two_replicas_that_wait_for_one_id_get_one_report_between_them() {
        // Not a state the server makes (one executor holds the claim), but the
        // delete must still hand a row to one caller.
        let inbox = shared_inbox().await;
        let a = FleetCoordinator::new();
        let b = FleetCoordinator::new();
        let reporter = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let _rx_a = a.register_dispatch(task_id, "d".into());
        let _rx_b = b.register_dispatch(task_id, "d".into());
        reporter
            .accept(&inbox, task_id, report_of(task_id, "d"))
            .await
            .unwrap();
        let (ra, rb) = tokio::join!(a.collect_stored(&inbox), b.collect_stored(&inbox));
        assert_eq!(ra.unwrap() + rb.unwrap(), 1);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn the_inbox_loop_resolves_a_wait_on_a_wake() {
        let inbox = shared_inbox().await;
        let dispatching = Arc::new(FleetCoordinator::new());
        let other = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let rx = dispatching.register_dispatch(task_id, "d1".into());

        let wake = WakeHandle::new();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let loop_task = tokio::spawn(run_result_inbox(
            dispatching.clone(),
            inbox.clone(),
            wake.clone(),
            shutdown_rx,
        ));

        other
            .accept(&inbox, task_id, report_of(task_id, "d1"))
            .await
            .unwrap();
        wake.wake();
        let got = tokio::time::timeout(Duration::from_secs(10), rx)
            .await
            .expect("the loop resolves the wait")
            .unwrap();
        assert_eq!(got.dispatch_id.as_deref(), Some("d1"));

        shutdown_tx.send(true).unwrap();
        tokio::time::timeout(Duration::from_secs(5), loop_task)
            .await
            .expect("the loop stops on shutdown")
            .unwrap();
    }

    #[test]
    fn pending_count_tracks_inserts_and_removals() {
        let coord = FleetCoordinator::new();
        assert_eq!(coord.pending_count(), 0);
        let a = UniversalUuid::new_v4();
        let b = UniversalUuid::new_v4();
        let _ra = coord.register_pending(a);
        let _rb = coord.register_pending(b);
        assert_eq!(coord.pending_count(), 2);
        coord.cancel(a);
        assert_eq!(coord.pending_count(), 1);
    }
}
