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
//!
//! A wait also knows where its packet went (CLOACINA-T-1050): the tenant,
//! the agent and the outbox row. A report resolves it only when
//! [`DispatchTarget::check_report`] says so. The result route hands in a
//! [`CheckedReport`]: the report plus the tenant of the agent row the caller
//! key was checked against. A stored report holds both, so the replica that
//! takes it runs the same check as for a local report.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cloacina::dal::DAL;
use cloacina::database::universal_types::{UniversalTimestamp, UniversalUuid};
use cloacina::delivery::WakeHandle;
use cloacina::error::ValidationError;
use cloacina::fleet::{
    AGENT_PROTOCOL_VERSION, AgentOutcome, AgentResultRequest, AgentUndecodableRequest,
    FailureClassification, GRAPH_PACKET_KIND, WORK_PACKET_KIND,
};
use cloacina::security::ownership::{
    AgentOwner, DeliveryScope, DispatchTarget, Refusal, ReportCheck, ReportOrigin,
};
use serde::{Deserialize, Serialize};
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
    /// This replica has no wait the report resolves: it is stored. The
    /// replica that waits for it takes it, or it goes by age.
    Stored,
    /// A wait on this replica is for this dispatch, and the report comes
    /// from another tenant or from an agent the packet is not addressed to
    /// (CLOACINA-T-1050). Nothing is stored and no state changes.
    Refused(Refusal),
}

/// An agent report whose sender was checked: the caller key holds the agent
/// the report names ([`cloacina::security::ownership::Principal::may_act_as_agent`]).
/// `agent_tenant` is the tenant of that agent's roster row.
///
/// This is also the payload of a `fleet_result_inbox` row. A payload that
/// does not parse as this type (one that a server from before
/// CLOACINA-T-1050 stored, with no check of the sender) is never taken.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckedReport {
    pub agent_tenant: Option<String>,
    pub report: AgentResultRequest,
}

impl CheckedReport {
    /// Build one from the roster row the caller key was checked against.
    pub fn new(agent: &AgentOwner, report: AgentResultRequest) -> Self {
        Self {
            agent_tenant: agent.tenant_id.clone(),
            report,
        }
    }

    fn origin(&self) -> ReportOrigin<'_> {
        ReportOrigin {
            tenant_id: self.agent_tenant.as_deref(),
            agent_id: &self.report.agent_id,
            dispatch_id: self.report.dispatch_id.as_deref(),
        }
    }
}

/// One wait of an executor for an agent report.
struct Pending {
    /// Tells this wait from a later wait for the same id.
    wait_id: u64,
    /// Where the packet went: tenant, agent, dispatch id.
    target: DispatchTarget,
    /// The `delivery_outbox` row of the packet, once it is enqueued. It is
    /// the record of a re-target: `reassign_open_rows` rewrites its
    /// recipient when the agent is evicted.
    packet_row: Option<i64>,
    tx: oneshot::Sender<AgentResultRequest>,
}

/// What a report is to the waits of this replica.
enum Judged {
    /// No wait here has this id.
    NoWait,
    /// The wait `wait_id` takes the report.
    Accept { wait_id: u64 },
    /// A wait here has this id and does not take the report.
    Refuse(Refusal),
}

#[derive(Default)]
pub struct FleetCoordinator {
    pending: Mutex<HashMap<UniversalUuid, Pending>>,
    next_wait_id: AtomicU64,
}

impl FleetCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a wait for the report of one dispatch, and get the receiver
    /// the executor awaits. `id` is the task execution id, or the firing id
    /// of a graph firing. `target` says where the packet goes: only a report
    /// that [`DispatchTarget::check_report`] accepts resolves the wait
    /// (CLOACINA-A-0016, CLOACINA-T-1050).
    ///
    /// Register before the packet is enqueued, so a fast agent cannot report
    /// before the wait exists; then call
    /// [`packet_enqueued`](Self::packet_enqueued). If the executor drops the
    /// receiver, a later report for the id is an orphan.
    pub fn register_dispatch(
        &self,
        id: UniversalUuid,
        target: DispatchTarget,
    ) -> oneshot::Receiver<AgentResultRequest> {
        let (tx, rx) = oneshot::channel();
        let wait_id = self.next_wait_id.fetch_add(1, Ordering::Relaxed);
        let mut g = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        let pending = Pending {
            wait_id,
            target,
            packet_row: None,
            tx,
        };
        if let Some(_prev) = g.insert(id, pending) {
            warn!(
                task_id = %id,
                "FleetCoordinator: replaced an existing pending entry — \
                 the prior executor invocation will see its oneshot drop"
            );
        }
        rx
    }

    /// Tell the wait for `id` which outbox row holds its packet. A report
    /// from another agent than the one the packet was enqueued for is taken
    /// only when this row is addressed to that agent (a packet re-targeted
    /// after an eviction). Without the row such a report is refused.
    pub fn packet_enqueued(&self, id: UniversalUuid, outbox_row_id: i64) {
        let mut g = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(p) = g.get_mut(&id) {
            p.packet_row = Some(outbox_row_id);
        }
    }

    /// The outbox row of the packet the wait for `id` is for, once
    /// [`packet_enqueued`](Self::packet_enqueued) named it. `None` with no
    /// wait for `id` on this replica.
    pub fn packet_row(&self, id: UniversalUuid) -> Option<i64> {
        let g = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        g.get(&id).and_then(|p| p.packet_row)
    }

    /// Whether a wait on this replica takes `report`. Reads the outbox row
    /// of the packet when the report comes from another agent than the one
    /// the packet was enqueued for. `Err`: that read failed, so there is no
    /// answer yet.
    async fn judge(
        &self,
        db: &DAL,
        id: UniversalUuid,
        report: &CheckedReport,
    ) -> Result<Judged, ValidationError> {
        let (wait_id, check, packet_row) = {
            let g = self.pending.lock().unwrap_or_else(|e| e.into_inner());
            match g.get(&id) {
                None => return Ok(Judged::NoWait),
                Some(p) => (
                    p.wait_id,
                    p.target.check_report(&report.origin()),
                    p.packet_row,
                ),
            }
        };
        match check {
            ReportCheck::Accept => Ok(Judged::Accept { wait_id }),
            ReportCheck::Refuse(why) => Ok(Judged::Refuse(why)),
            ReportCheck::IfRetargeted => {
                let Some(row_id) = packet_row else {
                    return Ok(Judged::Refuse(Refusal::NotTheTarget));
                };
                let scope =
                    DeliveryScope::of_agent(&report.report.agent_id, report.agent_tenant.clone());
                match db.delivery_outbox().get(row_id).await? {
                    Some(row) if scope.owns_row(&row.recipient, row.tenant_id.as_deref()) => {
                        Ok(Judged::Accept { wait_id })
                    }
                    _ => Ok(Judged::Refuse(Refusal::NotTheTarget)),
                }
            }
        }
    }

    /// Give `report` to the wait `wait_id` for `id`. `Err(report)`: that
    /// wait is gone (it ended, or a newer wait replaced it), or the executor
    /// dropped its receiver.
    fn resolve(
        &self,
        id: UniversalUuid,
        wait_id: u64,
        report: AgentResultRequest,
    ) -> Result<(), AgentResultRequest> {
        let pending = {
            let mut g = self.pending.lock().unwrap_or_else(|e| e.into_inner());
            match g.get(&id) {
                Some(p) if p.wait_id == wait_id => g.remove(&id),
                _ => None,
            }
        };
        match pending {
            Some(p) => p.tx.send(report).map_err(|r| {
                debug!(
                    task_id = %id,
                    "FleetCoordinator: pending receiver dropped before forward; \
                     likely executor canceled or timed out"
                );
                r
            }),
            None => Err(report),
        }
    }

    /// Take in a report that reached this replica (CLOACINA-T-1015).
    ///
    /// - A wait on this replica that the report resolves gets it:
    ///   [`Accepted::Local`].
    /// - With no wait here for the id, or a wait for another dispatch of the
    ///   task, the report is stored in `db` (the admin database, shared by
    ///   all replicas) for the replica that waits: [`Accepted::Stored`]. A
    ///   stored report nobody takes goes by age and changes no state.
    /// - A wait here for this dispatch that the report may not resolve
    ///   (another tenant, or an agent the packet is not addressed to):
    ///   [`Accepted::Refused`], logged and counted in
    ///   `cloacina_fleet_refused_results_total`.
    ///
    /// `Err` means the report is neither forwarded nor stored: the caller
    /// must tell the agent, so the agent sends it again.
    pub async fn accept(
        &self,
        db: &DAL,
        id: UniversalUuid,
        report: CheckedReport,
    ) -> Result<Accepted, String> {
        let judged = self
            .judge(db, id, &report)
            .await
            .map_err(|e| format!("read the packet row of the dispatch: {e}"))?;
        let report = match judged {
            Judged::Accept { wait_id } => {
                let CheckedReport {
                    agent_tenant,
                    report,
                } = report;
                match self.resolve(id, wait_id, report) {
                    Ok(()) => return Ok(Accepted::Local),
                    Err(report) => CheckedReport {
                        agent_tenant,
                        report,
                    },
                }
            }
            Judged::NoWait => report,
            // Another dispatch of the task can wait on another replica.
            Judged::Refuse(Refusal::OtherDispatch | Refusal::NoDispatchId) => {
                debug!(
                    task_id = %id,
                    "FleetCoordinator: report is for a different dispatch of this task (orphan); \
                     the pending wait stays"
                );
                report
            }
            Judged::Refuse(why) => {
                refused(id, &report, why);
                return Ok(Accepted::Refused(why));
            }
        };
        let payload =
            serde_json::to_vec(&report).map_err(|e| format!("serialize agent report: {e}"))?;
        db.fleet_result_inbox()
            .store(id, report.report.dispatch_id.clone(), payload)
            .await
            .map_err(|e| format!("store agent report: {e}"))?;
        Ok(Accepted::Stored)
    }

    /// Take the stored reports that the waits of this replica accept, and
    /// resolve those waits. Returns how many waits were resolved.
    ///
    /// A stored report passes the same check as a local one
    /// ([`DispatchTarget::check_report`]). A row is taken only when a wait
    /// here accepts it, and forwarded only when this replica's delete removed
    /// the row, so a report reaches one wait. Any other row is left alone
    /// and goes away by age: a row for another dispatch of a task that waits
    /// here, a row from another tenant or the wrong agent, and a row whose
    /// payload is not a [`CheckedReport`].
    pub async fn collect_stored(&self, inbox: &DAL) -> Result<usize, ValidationError> {
        let ids = self.waiting_ids();
        let mut resolved = 0usize;
        for chunk in ids.chunks(RESULT_INBOX_READ_CHUNK) {
            let rows = inbox.fleet_result_inbox().list_for(chunk).await?;
            for row in rows {
                let id = row.task_execution_id;
                let checked: CheckedReport = match serde_json::from_slice(&row.payload) {
                    Ok(r) => r,
                    Err(e) => {
                        debug!(
                            task_id = %id,
                            error = %e,
                            "fleet result inbox: stored report is not a checked report; left to \
                             go by age"
                        );
                        continue;
                    }
                };
                let wait_id = match self.judge(inbox, id, &checked).await? {
                    Judged::Accept { wait_id } => wait_id,
                    Judged::NoWait
                    | Judged::Refuse(Refusal::OtherDispatch | Refusal::NoDispatchId) => continue,
                    Judged::Refuse(why) => {
                        debug!(
                            task_id = %id,
                            agent_id = %checked.report.agent_id,
                            reason = %why,
                            "fleet result inbox: stored report may not resolve the wait here; \
                             left to go by age"
                        );
                        continue;
                    }
                };
                if !inbox.fleet_result_inbox().take(row.id).await? {
                    continue;
                }
                match self.resolve(id, wait_id, checked.report) {
                    Ok(()) => {
                        debug!(
                            task_id = %id,
                            "fleet result inbox: report from another replica resolved a wait here"
                        );
                        resolved += 1;
                    }
                    Err(orphan) => {
                        // The wait ended between the check and the forward.
                        metrics::counter!("cloacina_fleet_orphan_results_total").increment(1);
                        info!(
                            task_id = %id,
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

/// Log and count a report that a wait for its own dispatch refused
/// (CLOACINA-T-1050): the sender is in another tenant than the dispatch, or
/// is an agent the packet is not addressed to.
fn refused(id: UniversalUuid, report: &CheckedReport, why: Refusal) {
    metrics::counter!("cloacina_fleet_refused_results_total", "reason" => why.as_str())
        .increment(1);
    warn!(
        task_id = %id,
        agent_id = %report.report.agent_id,
        agent_tenant = ?report.agent_tenant,
        dispatch_id = ?report.report.dispatch_id,
        reason = %why,
        "agent result refused: the sender is not the agent this dispatch went to; no state changed"
    );
}

/// What became of an agent's "cannot decode" message
/// ([`fail_undecodable_packet`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Undecodable {
    /// The task or the firing of the packet got a failure report; this is
    /// what [`FleetCoordinator::accept`] did with it.
    Failed(Accepted),
    /// The row is acked already: the work of the packet ended. Nothing
    /// changed.
    AlreadyAcked,
    /// No row has the id. Nothing changed.
    NoRow,
    /// The row is addressed to another recipient or tenant than the agent
    /// that sent the message. Nothing changed.
    NotOwned,
    /// The row is no work packet, or the server cannot read the ids of its
    /// own packet. Nothing changed.
    NotAPacket,
}

/// The ids a report needs, read from the payload of a packet row.
fn packet_ids(kind: &str, payload: &[u8]) -> Option<(UniversalUuid, i32, Option<String>)> {
    let packet: serde_json::Value = serde_json::from_slice(payload).ok()?;
    let text = |key: &str| packet.get(key).and_then(|v| v.as_str());
    if kind == WORK_PACKET_KIND {
        let id = uuid::Uuid::parse_str(text("task_execution_id")?).ok()?;
        let attempt = packet.get("attempt").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
        Some((
            UniversalUuid(id),
            attempt,
            text("dispatch_id").map(str::to_string),
        ))
    } else if kind == GRAPH_PACKET_KIND {
        let id = uuid::Uuid::parse_str(text("firing_id")?).ok()?;
        Some((UniversalUuid(id), 1, None))
    } else if kind == cloacina::fleet::POLL_PACKET_KIND {
        // CLOACINA-T-1077: a poll ends with the failure; nothing fires.
        let id = uuid::Uuid::parse_str(text("poll_id")?).ok()?;
        Some((UniversalUuid(id), 1, None))
    } else {
        None
    }
}

/// The longest decode error of an agent that goes into the task's error.
const UNDECODABLE_ERROR_MAX_CHARS: usize = 300;

/// End the wait for a packet that its agent cannot decode (CLOACINA-T-1058).
///
/// The agent has no task id (it could not read the packet), so it names the
/// delivery-outbox row. `agent` is the roster row the caller key was checked
/// against. The row must be addressed to that agent in that agent's tenant
/// ([`DeliveryScope::owns_row`]), as an ack must be (CLOACINA-T-1052): a
/// message for any other row changes nothing, is logged, and is counted in
/// `cloacina_fleet_refused_undecodable_total`.
///
/// For its own row the server reads the task or firing id from the packet
/// it wrote, and hands [`FleetCoordinator::accept`] a failure report in the
/// agent's name. So the rules of a report apply: only the dispatch the
/// packet belongs to is resolved, on this replica or through the inbox. The
/// failure is a task error: the task's retry policy decides the next run; a
/// graph firing ends with the error. Each one is counted in
/// `cloacina_fleet_undecodable_packets_total`.
///
/// `Err`: the row could not be read, or the report was neither forwarded
/// nor stored. The agent sends the message again.
pub async fn fail_undecodable_packet(
    coordinator: &FleetCoordinator,
    db: &DAL,
    agent: &AgentOwner,
    req: &AgentUndecodableRequest,
) -> Result<Undecodable, String> {
    let Some(row) = db
        .delivery_outbox()
        .get(req.push_id)
        .await
        .map_err(|e| format!("read the packet row: {e}"))?
    else {
        return Ok(Undecodable::NoRow);
    };
    let scope = DeliveryScope::of_agent(&req.agent_id, agent.tenant_id.clone());
    if !scope.owns_row(&row.recipient, row.tenant_id.as_deref()) {
        metrics::counter!("cloacina_fleet_refused_undecodable_total").increment(1);
        warn!(
            push_id = req.push_id,
            agent_id = %req.agent_id,
            agent_tenant = ?agent.tenant_id,
            "undecodable-packet message refused: the row is not addressed to this agent; \
             nothing changed"
        );
        return Ok(Undecodable::NotOwned);
    }
    if row.delivery_state == cloacina::models::delivery_outbox::DeliveryState::Acked.as_str() {
        return Ok(Undecodable::AlreadyAcked);
    }
    let Some((id, attempt, dispatch_id)) = packet_ids(&row.kind, &row.payload) else {
        warn!(
            push_id = req.push_id,
            agent_id = %req.agent_id,
            kind = %row.kind,
            "undecodable-packet message for a row that is no readable work packet; \
             nothing changed"
        );
        return Ok(Undecodable::NotAPacket);
    };
    metrics::counter!("cloacina_fleet_undecodable_packets_total", "kind" => row.kind.clone())
        .increment(1);
    let agent_error: String = req
        .error
        .chars()
        .take(UNDECODABLE_ERROR_MAX_CHARS)
        .collect();
    let message = format!(
        "agent {} (version {}) could not decode the packet in delivery row {}: {}. \
         The server (version {}) and the agent are probably different versions: \
         run the same version on both.",
        req.agent_id,
        req.agent_version.as_deref().unwrap_or("unknown"),
        req.push_id,
        agent_error,
        env!("CARGO_PKG_VERSION"),
    );
    warn!(
        push_id = req.push_id,
        task_id = %id,
        agent_id = %req.agent_id,
        kind = %row.kind,
        error = %agent_error,
        "agent could not decode a packet: failing its dispatch (version mismatch?)"
    );
    let report = AgentResultRequest {
        protocol_version: AGENT_PROTOCOL_VERSION,
        agent_id: req.agent_id.clone(),
        task_execution_id: id.0.to_string(),
        attempt,
        dispatch_id,
        duration_ms: 0,
        outcome: AgentOutcome::Failure {
            message,
            classification: FailureClassification::TaskError,
        },
    };
    coordinator
        .accept(db, id, CheckedReport::new(agent, report))
        .await
        .map(Undecodable::Failed)
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

    const TENANT: &str = "tenant-a";
    const AGENT: &str = "agent-a";

    /// CLOACINA-T-1077: the server reads the poll id of its own poll packet,
    /// so an agent that cannot decode one ends the poll's wait at once.
    #[test]
    fn the_poll_id_of_a_poll_packet_is_read() {
        let id = uuid::Uuid::new_v4();
        let payload = serde_json::to_vec(&serde_json::json!({
            "poll_id": id.to_string(),
            "trigger_name": "watch_inbox",
        }))
        .unwrap();
        let (read, attempt, dispatch) =
            packet_ids(cloacina::fleet::POLL_PACKET_KIND, &payload).expect("ids");
        assert_eq!(read, UniversalUuid(id));
        assert_eq!(attempt, 1);
        assert!(dispatch.is_none());
    }

    /// A task dispatch `dispatch` to [`AGENT`] in [`TENANT`].
    fn task_target(dispatch: &str) -> DispatchTarget {
        DispatchTarget {
            tenant_id: Some(TENANT.to_string()),
            agent_id: AGENT.to_string(),
            dispatch_id: Some(dispatch.to_string()),
        }
    }

    /// A graph firing to [`AGENT`] in [`TENANT`]: no dispatch id.
    fn firing_target() -> DispatchTarget {
        DispatchTarget {
            tenant_id: Some(TENANT.to_string()),
            agent_id: AGENT.to_string(),
            dispatch_id: None,
        }
    }

    /// A report as the result route hands it in: sent by `agent`, whose
    /// roster row is in `tenant`.
    fn report_from(
        tenant: Option<&str>,
        agent: &str,
        task_id: UniversalUuid,
        dispatch: Option<&str>,
    ) -> CheckedReport {
        CheckedReport {
            agent_tenant: tenant.map(str::to_string),
            report: AgentResultRequest {
                protocol_version: AGENT_PROTOCOL_VERSION,
                agent_id: agent.to_string(),
                task_execution_id: task_id.0.to_string(),
                attempt: 1,
                dispatch_id: dispatch.map(str::to_string),
                duration_ms: 10,
                outcome: AgentOutcome::Success {
                    context: serde_json::json!({}),
                },
            },
        }
    }

    /// The report of the agent the packet went to.
    fn report_of(task_id: UniversalUuid, dispatch: &str) -> CheckedReport {
        report_from(Some(TENANT), AGENT, task_id, Some(dispatch))
    }

    // ── with no database ──────────────────────────────────────────────────
    //
    // These three need no row: a report that a local wait takes, or refuses
    // for its tenant or its agent, reads and writes nothing. They run with
    // no `sqlite` feature too. The handle is never connected.

    fn unconnected_db() -> DAL {
        DAL::new(cloacina::database::Database::new(
            "postgres://cloacina:cloacina@localhost:15432/cloacina",
            "cloacina",
            1,
        ))
    }

    #[tokio::test]
    async fn local_wait_takes_the_report_of_its_dispatch_agent_and_tenant() {
        let db = unconnected_db();
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let rx = coord.register_dispatch(task_id, task_target("d1"));
        let accepted = coord
            .accept(&db, task_id, report_of(task_id, "d1"))
            .await
            .unwrap();
        assert_eq!(accepted, Accepted::Local);
        assert!(rx.await.is_ok());
        assert_eq!(coord.pending_count(), 0);
    }

    #[tokio::test]
    async fn local_wait_refuses_a_report_from_another_tenant() {
        let db = unconnected_db();
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let mut rx = coord.register_dispatch(task_id, task_target("d1"));
        for dispatch in [Some("d1"), None] {
            let accepted = coord
                .accept(
                    &db,
                    task_id,
                    report_from(Some("tenant-b"), "agent-b", task_id, dispatch),
                )
                .await
                .unwrap();
            assert_eq!(accepted, Accepted::Refused(Refusal::OtherTenant));
        }
        assert_eq!(coord.pending_count(), 1);
        assert!(rx.try_recv().is_err(), "the wait is not resolved");
    }

    #[tokio::test]
    async fn local_wait_refuses_a_report_from_another_agent_of_the_tenant() {
        let db = unconnected_db();
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let mut rx = coord.register_dispatch(task_id, task_target("d1"));
        let accepted = coord
            .accept(
                &db,
                task_id,
                report_from(Some(TENANT), "agent-other", task_id, Some("d1")),
            )
            .await
            .unwrap();
        assert_eq!(accepted, Accepted::Refused(Refusal::NotTheTarget));
        assert_eq!(coord.pending_count(), 1);
        assert!(rx.try_recv().is_err(), "the wait is not resolved");
    }

    // Two replicas are two coordinators over one database. SQLite in memory
    // stands in for the shared Postgres: no NOTIFY, so the tests call
    // `collect_stored` as the tick of `run_result_inbox` does.
    #[cfg(feature = "sqlite")]
    async fn shared_db() -> DAL {
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
    async fn stored_rows(db: &DAL, task_id: UniversalUuid) -> usize {
        db.fleet_result_inbox()
            .list_for(&[task_id])
            .await
            .unwrap()
            .len()
    }

    /// Enqueue a work packet for `agent` in `tenant`, as the executor does.
    #[cfg(feature = "sqlite")]
    async fn enqueue_packet(db: &DAL, tenant: &str, agent: &str) -> i64 {
        db.delivery_outbox()
            .enqueue(cloacina::models::delivery_outbox::NewDeliveryOutbox {
                recipient: format!("{}{agent}", cloacina::fleet::AGENT_RECIPIENT_PREFIX),
                kind: "work_packet".to_string(),
                tenant_id: Some(tenant.to_string()),
                payload: b"{}".to_vec(),
            })
            .await
            .unwrap()
            .id
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn the_report_of_the_dispatch_resolves_the_wait() {
        let db = shared_db().await;
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let rx = coord.register_dispatch(task_id, task_target("d1"));
        let accepted = coord
            .accept(&db, task_id, report_of(task_id, "d1"))
            .await
            .unwrap();
        assert_eq!(accepted, Accepted::Local);
        assert_eq!(rx.await.unwrap().dispatch_id.as_deref(), Some("d1"));
        assert_eq!(stored_rows(&db, task_id).await, 0, "not stored");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn a_report_nobody_waits_for_is_stored() {
        let db = shared_db().await;
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let accepted = coord
            .accept(&db, task_id, report_of(task_id, "d1"))
            .await
            .unwrap();
        assert_eq!(accepted, Accepted::Stored);
        assert_eq!(stored_rows(&db, task_id).await, 1);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn cancel_removes_the_wait_so_a_later_report_is_an_orphan() {
        let db = shared_db().await;
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let _rx = coord.register_dispatch(task_id, task_target("d1"));
        coord.cancel(task_id);
        let accepted = coord
            .accept(&db, task_id, report_of(task_id, "d1"))
            .await
            .unwrap();
        assert_eq!(accepted, Accepted::Stored);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn report_of_another_dispatch_is_orphan_and_the_wait_stays() {
        let db = shared_db().await;
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let rx = coord.register_dispatch(task_id, task_target("second"));

        let stale = coord
            .accept(&db, task_id, report_of(task_id, "first"))
            .await
            .unwrap();
        assert_eq!(stale, Accepted::Stored);
        assert_eq!(coord.pending_count(), 1);

        let current = coord
            .accept(&db, task_id, report_of(task_id, "second"))
            .await
            .unwrap();
        assert_eq!(current, Accepted::Local);
        assert_eq!(rx.await.unwrap().dispatch_id.as_deref(), Some("second"));
    }

    /// CLOACINA-T-1050: no agent from before the dispatch id exists, so a
    /// report with no id does not resolve a task dispatch.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn report_with_no_dispatch_id_is_an_orphan_for_a_task_dispatch() {
        let db = shared_db().await;
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let mut rx = coord.register_dispatch(task_id, task_target("d1"));
        let accepted = coord
            .accept(
                &db,
                task_id,
                report_from(Some(TENANT), AGENT, task_id, None),
            )
            .await
            .unwrap();
        assert_eq!(accepted, Accepted::Stored);
        assert_eq!(coord.pending_count(), 1);
        assert!(rx.try_recv().is_err(), "the wait is not resolved");
        // Stored, never taken: it goes by age.
        assert_eq!(coord.collect_stored(&db).await.unwrap(), 0);
        assert_eq!(stored_rows(&db, task_id).await, 1);
    }

    /// CLOACINA-T-1050, across tenants: an agent of tenant B, with the right
    /// task id and even the right dispatch id, cannot resolve a wait of
    /// tenant A. Neither can a key with no tenant.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn a_report_from_another_tenant_cannot_resolve_the_wait() {
        let db = shared_db().await;
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let mut rx = coord.register_dispatch(task_id, task_target("d1"));
        coord.packet_enqueued(task_id, enqueue_packet(&db, TENANT, AGENT).await);

        for (tenant, agent) in [
            (Some("tenant-b"), "agent-b"),
            // The same agent id, claimed to be in another tenant.
            (Some("tenant-b"), AGENT),
            (None, "agent-of-no-tenant"),
        ] {
            let accepted = coord
                .accept(
                    &db,
                    task_id,
                    report_from(tenant, agent, task_id, Some("d1")),
                )
                .await
                .unwrap();
            assert_eq!(accepted, Accepted::Refused(Refusal::OtherTenant));
        }
        // With no dispatch id it is refused the same way, and not stored.
        let accepted = coord
            .accept(
                &db,
                task_id,
                report_from(Some("tenant-b"), "agent-b", task_id, None),
            )
            .await
            .unwrap();
        assert_eq!(accepted, Accepted::Refused(Refusal::OtherTenant));

        assert_eq!(coord.pending_count(), 1);
        assert!(rx.try_recv().is_err(), "the wait is not resolved");
        assert_eq!(stored_rows(&db, task_id).await, 0);

        // The real report still resolves it.
        let accepted = coord
            .accept(&db, task_id, report_of(task_id, "d1"))
            .await
            .unwrap();
        assert_eq!(accepted, Accepted::Local);
        assert!(rx.await.is_ok());
    }

    /// CLOACINA-T-1050, inside one tenant: a registered agent of the same
    /// tenant that was not sent the packet cannot resolve the wait.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn a_report_from_another_agent_of_the_tenant_cannot_resolve_the_wait() {
        let db = shared_db().await;
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let mut rx = coord.register_dispatch(task_id, task_target("d1"));

        // Before the executor has named the outbox row.
        let accepted = coord
            .accept(
                &db,
                task_id,
                report_from(Some(TENANT), "agent-other", task_id, Some("d1")),
            )
            .await
            .unwrap();
        assert_eq!(accepted, Accepted::Refused(Refusal::NotTheTarget));

        // With the row: it is addressed to AGENT, not to the sender.
        coord.packet_enqueued(task_id, enqueue_packet(&db, TENANT, AGENT).await);
        let accepted = coord
            .accept(
                &db,
                task_id,
                report_from(Some(TENANT), "agent-other", task_id, Some("d1")),
            )
            .await
            .unwrap();
        assert_eq!(accepted, Accepted::Refused(Refusal::NotTheTarget));

        assert_eq!(coord.pending_count(), 1);
        assert!(rx.try_recv().is_err(), "the wait is not resolved");
        assert_eq!(stored_rows(&db, task_id).await, 0);
    }

    /// CLOACINA-T-1050: when the agent is evicted, `reassign_open_rows`
    /// re-targets its packet. The report of the new agent resolves the wait;
    /// a third agent of the tenant still cannot.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn the_report_of_the_agent_a_packet_was_retargeted_to_resolves_the_wait() {
        let db = shared_db().await;
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let mut rx = coord.register_dispatch(task_id, task_target("d1"));
        coord.packet_enqueued(task_id, enqueue_packet(&db, TENANT, AGENT).await);

        let moved = db
            .delivery_outbox()
            .reassign_open_rows(
                &format!("agent:{AGENT}"),
                &format!("agent:{}", "agent-live"),
            )
            .await
            .unwrap();
        assert_eq!(moved, 1);

        let third = coord
            .accept(
                &db,
                task_id,
                report_from(Some(TENANT), "agent-third", task_id, Some("d1")),
            )
            .await
            .unwrap();
        assert_eq!(third, Accepted::Refused(Refusal::NotTheTarget));
        assert!(rx.try_recv().is_err());

        let accepted = coord
            .accept(
                &db,
                task_id,
                report_from(Some(TENANT), "agent-live", task_id, Some("d1")),
            )
            .await
            .unwrap();
        assert_eq!(accepted, Accepted::Local);
        assert_eq!(rx.await.unwrap().agent_id, "agent-live");
    }

    /// The same, when the report of the new agent reaches another replica.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn a_stored_report_of_a_retargeted_packet_resolves_the_wait() {
        let db = shared_db().await;
        let dispatching = FleetCoordinator::new();
        let other = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let rx = dispatching.register_dispatch(task_id, task_target("d1"));
        dispatching.packet_enqueued(task_id, enqueue_packet(&db, TENANT, AGENT).await);
        db.delivery_outbox()
            .reassign_open_rows(&format!("agent:{AGENT}"), "agent:agent-live")
            .await
            .unwrap();

        let accepted = other
            .accept(
                &db,
                task_id,
                report_from(Some(TENANT), "agent-live", task_id, Some("d1")),
            )
            .await
            .unwrap();
        assert_eq!(accepted, Accepted::Stored);
        assert_eq!(dispatching.collect_stored(&db).await.unwrap(), 1);
        assert_eq!(rx.await.unwrap().agent_id, "agent-live");
    }

    /// Graph firings keep their own key (the firing id) and have no dispatch
    /// id. The tenant and agent check is the same.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn a_graph_firing_takes_only_the_report_of_its_agent_and_tenant() {
        let db = shared_db().await;
        let coord = FleetCoordinator::new();
        let firing_id = UniversalUuid::new_v4();
        let mut rx = coord.register_dispatch(firing_id, firing_target());
        coord.packet_enqueued(firing_id, enqueue_packet(&db, TENANT, AGENT).await);

        let other_tenant = coord
            .accept(
                &db,
                firing_id,
                report_from(Some("tenant-b"), "agent-b", firing_id, None),
            )
            .await
            .unwrap();
        assert_eq!(other_tenant, Accepted::Refused(Refusal::OtherTenant));
        let other_agent = coord
            .accept(
                &db,
                firing_id,
                report_from(Some(TENANT), "agent-other", firing_id, None),
            )
            .await
            .unwrap();
        assert_eq!(other_agent, Accepted::Refused(Refusal::NotTheTarget));
        assert!(rx.try_recv().is_err());

        let accepted = coord
            .accept(
                &db,
                firing_id,
                report_from(Some(TENANT), AGENT, firing_id, None),
            )
            .await
            .unwrap();
        assert_eq!(accepted, Accepted::Local);
        assert!(rx.await.is_ok());
    }

    // ── CLOACINA-T-1015: a report that reaches another replica ───────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn report_to_another_replica_resolves_the_wait_of_the_dispatching_replica() {
        let inbox = shared_db().await;
        let dispatching = FleetCoordinator::new();
        let other = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let rx = dispatching.register_dispatch(task_id, task_target("d1"));

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
        assert_eq!(
            stored_rows(&inbox, task_id).await,
            0,
            "a taken report leaves no row"
        );
        assert_eq!(dispatching.collect_stored(&inbox).await.unwrap(), 0);
    }

    /// CLOACINA-T-1050: a stored report carries the tenant and the agent of
    /// its sender, and the replica that waits checks them. A report of
    /// another tenant, or of another agent of the tenant, that reached
    /// another replica is not taken.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn a_stored_report_is_checked_by_the_replica_that_takes_it() {
        let inbox = shared_db().await;
        let dispatching = FleetCoordinator::new();
        let other = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let mut rx = dispatching.register_dispatch(task_id, task_target("d1"));
        dispatching.packet_enqueued(task_id, enqueue_packet(&inbox, TENANT, AGENT).await);

        for forged in [
            report_from(Some("tenant-b"), "agent-b", task_id, Some("d1")),
            report_from(Some("tenant-b"), AGENT, task_id, Some("d1")),
            report_from(Some(TENANT), "agent-other", task_id, Some("d1")),
            report_from(Some(TENANT), AGENT, task_id, None),
        ] {
            let accepted = other.accept(&inbox, task_id, forged).await.unwrap();
            assert_eq!(accepted, Accepted::Stored, "the other replica has no wait");
        }
        assert_eq!(dispatching.collect_stored(&inbox).await.unwrap(), 0);
        assert_eq!(dispatching.pending_count(), 1);
        assert!(rx.try_recv().is_err(), "the wait is not resolved");
        assert_eq!(
            stored_rows(&inbox, task_id).await,
            4,
            "refused rows are not taken"
        );

        other
            .accept(&inbox, task_id, report_of(task_id, "d1"))
            .await
            .unwrap();
        assert_eq!(dispatching.collect_stored(&inbox).await.unwrap(), 1);
        assert_eq!(rx.await.unwrap().agent_id, AGENT);
    }

    /// A row that a server from before CLOACINA-T-1050 stored holds the bare
    /// report, with no check of its sender. It is never taken.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn a_stored_report_in_the_old_shape_is_not_taken() {
        let inbox = shared_db().await;
        let dispatching = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let mut rx = dispatching.register_dispatch(task_id, task_target("d1"));
        let bare = serde_json::to_vec(&report_of(task_id, "d1").report).unwrap();
        inbox
            .fleet_result_inbox()
            .store(task_id, Some("d1".to_string()), bare)
            .await
            .unwrap();
        assert_eq!(dispatching.collect_stored(&inbox).await.unwrap(), 0);
        assert!(rx.try_recv().is_err());
        assert_eq!(stored_rows(&inbox, task_id).await, 1);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn stored_report_of_an_older_dispatch_leaves_the_wait_and_goes_by_age() {
        let inbox = shared_db().await;
        let dispatching = FleetCoordinator::new();
        let other = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        // The task was dispatched again: the wait is for the second dispatch.
        let mut rx = dispatching.register_dispatch(task_id, task_target("second"));

        other
            .accept(&inbox, task_id, report_of(task_id, "first"))
            .await
            .unwrap();
        assert_eq!(dispatching.collect_stored(&inbox).await.unwrap(), 0);
        assert_eq!(dispatching.pending_count(), 1);
        assert!(rx.try_recv().is_err(), "the wait is not resolved");
        assert_eq!(
            stored_rows(&inbox, task_id).await,
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
        assert_eq!(stored_rows(&inbox, task_id).await, 0);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn two_replicas_that_wait_for_one_id_get_one_report_between_them() {
        // Not a state the server makes (one executor holds the claim), but the
        // delete must still hand a row to one caller.
        let inbox = shared_db().await;
        let a = FleetCoordinator::new();
        let b = FleetCoordinator::new();
        let reporter = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let _rx_a = a.register_dispatch(task_id, task_target("d"));
        let _rx_b = b.register_dispatch(task_id, task_target("d"));
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
        let inbox = shared_db().await;
        let dispatching = Arc::new(FleetCoordinator::new());
        let other = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let rx = dispatching.register_dispatch(task_id, task_target("d1"));

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

    // ── CLOACINA-T-1058: a packet its agent cannot decode ─────────────────

    #[cfg(feature = "sqlite")]
    fn owner(tenant: &str) -> AgentOwner {
        AgentOwner {
            tenant_id: Some(tenant.to_string()),
            owner_key_id: Some(uuid::Uuid::new_v4()),
        }
    }

    #[cfg(feature = "sqlite")]
    fn cannot_decode(agent: &str, push_id: i64) -> AgentUndecodableRequest {
        AgentUndecodableRequest {
            protocol_version: AGENT_PROTOCOL_VERSION,
            agent_id: agent.to_string(),
            push_id,
            error: "deserialize WorkPacket JSON: missing field `artifact`".to_string(),
            agent_version: Some("0.0.1".to_string()),
        }
    }

    /// Enqueue a packet row of `kind` for [`AGENT`] in [`TENANT`].
    #[cfg(feature = "sqlite")]
    async fn enqueue_row(db: &DAL, kind: &str, payload: serde_json::Value) -> i64 {
        db.delivery_outbox()
            .enqueue(cloacina::models::delivery_outbox::NewDeliveryOutbox {
                recipient: format!("{}{AGENT}", cloacina::fleet::AGENT_RECIPIENT_PREFIX),
                kind: kind.to_string(),
                tenant_id: Some(TENANT.to_string()),
                payload: serde_json::to_vec(&payload).unwrap(),
            })
            .await
            .unwrap()
            .id
    }

    /// The row of a task packet, as the executor writes it (the fields the
    /// lookup reads).
    #[cfg(feature = "sqlite")]
    async fn enqueue_task_packet(db: &DAL, task_id: UniversalUuid, dispatch: &str) -> i64 {
        enqueue_row(
            db,
            WORK_PACKET_KIND,
            serde_json::json!({
                "protocol_version": AGENT_PROTOCOL_VERSION,
                "task_execution_id": task_id.0.to_string(),
                "attempt": 3,
                "dispatch_id": dispatch,
            }),
        )
        .await
    }

    #[cfg(feature = "sqlite")]
    async fn row_state(db: &DAL, id: i64) -> String {
        db.delivery_outbox()
            .get(id)
            .await
            .unwrap()
            .unwrap()
            .delivery_state
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn an_undecodable_task_packet_fails_its_dispatch_with_a_version_error() {
        let db = shared_db().await;
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let rx = coord.register_dispatch(task_id, task_target("d1"));
        let row = enqueue_task_packet(&db, task_id, "d1").await;
        coord.packet_enqueued(task_id, row);

        let got =
            fail_undecodable_packet(&coord, &db, &owner(TENANT), &cannot_decode(AGENT, row)).await;
        assert_eq!(got, Ok(Undecodable::Failed(Accepted::Local)));

        // The executor gets a report for the task found in the row.
        let report = rx.await.expect("the wait is resolved at once");
        assert_eq!(report.task_execution_id, task_id.0.to_string());
        assert_eq!(report.attempt, 3);
        assert_eq!(report.dispatch_id.as_deref(), Some("d1"));
        assert_eq!(report.agent_id, AGENT);
        match report.outcome {
            AgentOutcome::Failure {
                message,
                classification,
            } => {
                // A task error: the retry policy of the task decides.
                assert_eq!(classification, FailureClassification::TaskError);
                assert!(message.contains("could not decode"), "{message}");
                assert!(message.contains("different versions"), "{message}");
                assert!(message.contains("missing field `artifact`"), "{message}");
                assert!(message.contains("version 0.0.1"), "{message}");
                assert!(
                    message.contains(&format!("delivery row {row}")),
                    "{message}"
                );
            }
            other => panic!("expected a failure, got {other:?}"),
        }
        assert_eq!(coord.pending_count(), 0);
        // The ack is the agent's: the message does not ack the row.
        assert_eq!(row_state(&db, row).await, "pending");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn an_undecodable_graph_packet_ends_its_firing_with_the_error() {
        let db = shared_db().await;
        let coord = FleetCoordinator::new();
        let firing_id = UniversalUuid::new_v4();
        let rx = coord.register_dispatch(firing_id, firing_target());
        let row = enqueue_row(
            &db,
            GRAPH_PACKET_KIND,
            serde_json::json!({"firing_id": firing_id.0.to_string(), "graph_name": "g"}),
        )
        .await;
        coord.packet_enqueued(firing_id, row);

        let got =
            fail_undecodable_packet(&coord, &db, &owner(TENANT), &cannot_decode(AGENT, row)).await;
        assert_eq!(got, Ok(Undecodable::Failed(Accepted::Local)));
        let report = rx.await.expect("the firing's wait is resolved at once");
        assert_eq!(report.task_execution_id, firing_id.0.to_string());
        assert_eq!(report.dispatch_id, None);
        assert!(matches!(report.outcome, AgentOutcome::Failure { .. }));
    }

    /// The message reaches a replica that does not wait for the task: the
    /// failure is stored, and the replica that dispatched the task takes it.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn an_undecodable_packet_fails_a_dispatch_of_another_replica() {
        let db = shared_db().await;
        let dispatcher = FleetCoordinator::new();
        let other = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let rx = dispatcher.register_dispatch(task_id, task_target("d1"));
        let row = enqueue_task_packet(&db, task_id, "d1").await;
        dispatcher.packet_enqueued(task_id, row);

        let got =
            fail_undecodable_packet(&other, &db, &owner(TENANT), &cannot_decode(AGENT, row)).await;
        assert_eq!(got, Ok(Undecodable::Failed(Accepted::Stored)));
        assert_eq!(dispatcher.collect_stored(&db).await.unwrap(), 1);
        assert!(matches!(
            rx.await.unwrap().outcome,
            AgentOutcome::Failure { .. }
        ));
    }

    /// The owner check of an ack (CLOACINA-T-1052) holds for this message: an
    /// agent of another tenant, another agent of the tenant, and the same
    /// agent name in another tenant cannot fail the dispatch of a row that
    /// is not theirs.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn an_undecodable_message_for_a_row_of_another_agent_changes_nothing() {
        let db = shared_db().await;
        let coord = FleetCoordinator::new();
        let task_id = UniversalUuid::new_v4();
        let mut rx = coord.register_dispatch(task_id, task_target("d1"));
        let row = enqueue_task_packet(&db, task_id, "d1").await;
        coord.packet_enqueued(task_id, row);

        let no_tenant = AgentOwner {
            tenant_id: None,
            owner_key_id: Some(uuid::Uuid::new_v4()),
        };
        for (agent, agent_row) in [
            ("agent-b", owner("tenant-b")),
            ("agent-other", owner(TENANT)),
            (AGENT, owner("tenant-b")),
            (AGENT, no_tenant),
        ] {
            let got =
                fail_undecodable_packet(&coord, &db, &agent_row, &cannot_decode(agent, row)).await;
            assert_eq!(got, Ok(Undecodable::NotOwned), "{agent} {agent_row:?}");
        }
        assert_eq!(coord.pending_count(), 1);
        assert!(rx.try_recv().is_err(), "the wait is not resolved");
        assert_eq!(stored_rows(&db, task_id).await, 0, "nothing stored");
        assert_eq!(row_state(&db, row).await, "pending");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn an_undecodable_message_for_no_packet_changes_nothing() {
        let db = shared_db().await;
        let coord = FleetCoordinator::new();
        let agent_row = owner(TENANT);

        // No row has the id.
        let got =
            fail_undecodable_packet(&coord, &db, &agent_row, &cannot_decode(AGENT, 9_999)).await;
        assert_eq!(got, Ok(Undecodable::NoRow));

        // The agent's own row, of a kind that is no work packet.
        let other_kind = enqueue_row(&db, "work_packet", serde_json::json!({})).await;
        let got =
            fail_undecodable_packet(&coord, &db, &agent_row, &cannot_decode(AGENT, other_kind))
                .await;
        assert_eq!(got, Ok(Undecodable::NotAPacket));

        // The agent's own packet, acked already: its work ended.
        let task_id = UniversalUuid::new_v4();
        let mut rx = coord.register_dispatch(task_id, task_target("d1"));
        let acked = enqueue_task_packet(&db, task_id, "d1").await;
        assert!(
            db.delivery_outbox()
                .mark_acked_for(acked, &format!("agent:{AGENT}"), Some(TENANT))
                .await
                .unwrap()
        );
        let got =
            fail_undecodable_packet(&coord, &db, &agent_row, &cannot_decode(AGENT, acked)).await;
        assert_eq!(got, Ok(Undecodable::AlreadyAcked));
        assert!(rx.try_recv().is_err(), "the wait is not resolved");
    }

    #[test]
    fn pending_count_tracks_inserts_and_removals() {
        let coord = FleetCoordinator::new();
        assert_eq!(coord.pending_count(), 0);
        let a = UniversalUuid::new_v4();
        let b = UniversalUuid::new_v4();
        let _ra = coord.register_dispatch(a, task_target("d"));
        let _rb = coord.register_dispatch(b, firing_target());
        assert_eq!(coord.pending_count(), 2);
        coord.cancel(a);
        assert_eq!(coord.pending_count(), 1);
    }
}
