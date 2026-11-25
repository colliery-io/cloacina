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
//! v1 (Tier A): in-memory `Mutex<HashMap>` per replica. The agent's result
//! POST must land on the same replica that dispatched the work — agents
//! authenticate against and POST to a specific server URL, so this matches
//! the connection-ownership model the substrate already uses (CLOACI-A-0006).
//! Multi-replica fleet-wide rendezvous (an agent dispatched by replica A but
//! whose result lands on replica B) is future work, parallel to the substrate's
//! cross-replica fan-out direction.

use std::collections::HashMap;
use std::sync::Mutex;

use cloacina::database::universal_types::UniversalUuid;
use cloacina::fleet::AgentResultRequest;
use tokio::sync::oneshot;
use tracing::{debug, warn};

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
                Some(p) => match (&p.dispatch_id, &result.dispatch_id) {
                    (Some(want), Some(got)) => want != got,
                    _ => false,
                },
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
