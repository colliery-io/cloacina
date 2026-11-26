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

//! WebSocket delivery sink for the interservice communication substrate
//! (CLOACI-I-0115 / T-0627). The `cloacina-server`-side bridge between the
//! relay (which has no knowledge of transports) and live WS recipient
//! connections.
//!
//! Wiring: the per-replica [`cloacina::delivery::DeliveryRelay`] is constructed
//! with an `Arc<WsDeliverySink>`; the relay drains `pending` rows and calls
//! `sink.deliver(row)`. The sink looks up the row's
//! `(recipient, tenant_id)` in its registry — if a WS handler is currently
//! connected for that key, the push lands in its mpsc channel and the relay
//! marks the row `delivered`. If no one is connected, the sink returns
//! [`DeliveryOutcome::NoRoute`] and the row stays `pending`; another
//! replica's NOTIFY-woken relay (or the safety-net sweeper from T-0628)
//! handles it. This is the connection-ownership routing pattern called out
//! in [[CLOACI-A-0006]] — no roster needed.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use cloacina::delivery::{DeliveryError, DeliveryOutcome, DeliverySink, ServerMessage};
use cloacina::models::delivery_outbox::DeliveryOutbox;
use tokio::sync::mpsc;
use tracing::debug;

/// Per-connection mpsc channel depth. Bounded so a stalled recipient applies
/// backpressure (`try_send` returns `Full` → row stays `pending` for the next
/// wake or sweeper retry) instead of letting the relay buffer unboundedly.
const CHANNEL_DEPTH: usize = 32;

/// Composite registry key. `tenant_id` is `Option<String>` because the
/// `delivery_outbox.tenant_id` column is nullable for globally-scoped rows.
type Key = (String, Option<String>);

/// Identifies one registered connection, so that a handler that exits removes
/// its own entry and never the entry of a connection that replaced it
/// (CLOACINA-T-0952).
pub type ConnectionId = u64;

/// Recipients whose frames go to every connected subscriber
/// (CLOACINA-T-0952, CLOACINA-T-1051). The rule per recipient kind is
/// [`cloacina::security::ownership::RecipientKind::is_shared`]:
///
/// - `ops_metrics:*`: ephemeral snapshots pushed with
///   [`WsDeliverySink::push_direct`], so two browser tabs or two people on
///   one tenant can each hold a socket.
/// - `exec_events:<id>`: the events of an execution, which more than one
///   person of the tenant can follow. A second follower does not cut off the
///   first. The rows are durable: each connected follower gets a row when it
///   is pushed, and the first ack settles it.
///
/// Every other recipient is single-consumer (`agent:<id>`): its rows are
/// work for one process, and a newer connection replaces the older one.
pub fn is_broadcast_recipient(recipient: &str) -> bool {
    cloacina::security::ownership::RecipientKind::of(recipient).is_shared()
}

struct Connection {
    id: ConnectionId,
    tx: mpsc::Sender<ServerMessage>,
}

/// In-memory registry of `(recipient, tenant_id) → connections` for
/// currently-connected WS recipients on this replica. A single-consumer key
/// holds at most one connection; a broadcast key holds any number.
#[derive(Default)]
pub struct WsDeliverySink {
    by_key: Mutex<HashMap<Key, Vec<Connection>>>,
    next_id: AtomicU64,
}

fn key(recipient: &str, tenant_id: Option<&str>) -> Key {
    (recipient.to_string(), tenant_id.map(|s| s.to_string()))
}

impl WsDeliverySink {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a connection for a recipient. Returns its id (pass it to
    /// [`unregister`](Self::unregister)) and the mpsc receiver the handler
    /// awaits to receive pushes.
    ///
    /// For a single-consumer recipient this replaces any prior connection for
    /// the same key: the older send half drops, the previous handler's
    /// `rx.recv()` returns `None`, and that handler unwinds. For a broadcast
    /// recipient ([`is_broadcast_recipient`]) the connection is added beside
    /// the others.
    pub fn register(
        &self,
        recipient: &str,
        tenant_id: Option<&str>,
    ) -> (ConnectionId, mpsc::Receiver<ServerMessage>) {
        let (tx, rx) = mpsc::channel(CHANNEL_DEPTH);
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let key = key(recipient, tenant_id);
        let mut g = self.by_key.lock().unwrap_or_else(|e| e.into_inner());
        let conns = g.entry(key.clone()).or_default();
        if !is_broadcast_recipient(recipient) && !conns.is_empty() {
            debug!(
                recipient = %key.0,
                tenant = ?key.1,
                "delivery_sink: replaced existing recipient connection"
            );
            conns.clear();
        }
        conns.push(Connection { id, tx });
        (id, rx)
    }

    /// Deregister one connection. Idempotent: a connection that a newer one
    /// already replaced is gone, and the newer one stays.
    pub fn unregister(&self, recipient: &str, tenant_id: Option<&str>, id: ConnectionId) {
        let key = key(recipient, tenant_id);
        let mut g = self.by_key.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(conns) = g.get_mut(&key) {
            conns.retain(|c| c.id != id);
            if conns.is_empty() {
                g.remove(&key);
            }
        }
    }

    /// Whether a recipient is currently connected on this replica. Lets a
    /// publisher skip gathering ephemeral data nobody is listening for
    /// (CLOACI-T-0718).
    pub fn has_recipient(&self, recipient: &str, tenant_id: Option<&str>) -> bool {
        self.by_key
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&key(recipient, tenant_id))
            .is_some_and(|c| !c.is_empty())
    }

    /// The live senders of a key, cloned out of the lock.
    fn senders(
        &self,
        recipient: &str,
        tenant_id: Option<&str>,
    ) -> Vec<(ConnectionId, mpsc::Sender<ServerMessage>)> {
        self.by_key
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&key(recipient, tenant_id))
            .map(|c| c.iter().map(|c| (c.id, c.tx.clone())).collect())
            .unwrap_or_default()
    }

    /// Push a frame straight to the connected subscribers of a recipient,
    /// bypassing the durable outbox + relay (CLOACI-T-0718). For *ephemeral*
    /// latest-snapshot data (operational metrics) that must NOT accrue rows in
    /// `delivery_outbox` — the substrate's durable path is reserved for
    /// must-not-lose work (execution events, work packets). Returns `true` if
    /// at least one connection received it, `false` if nobody is listening
    /// (no-op) or every channel is full/closed — the next snapshot supersedes a
    /// dropped one anyway.
    pub fn push_direct(
        &self,
        recipient: &str,
        tenant_id: Option<&str>,
        msg: ServerMessage,
    ) -> bool {
        let mut any = false;
        for (id, sender) in self.senders(recipient, tenant_id) {
            match sender.try_send(msg.clone()) {
                Ok(()) => any = true,
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    self.unregister(recipient, tenant_id, id);
                }
                Err(mpsc::error::TrySendError::Full(_)) => {}
            }
        }
        any
    }

    /// Current number of registered connections — useful for metrics (T-0628).
    pub fn len(&self) -> usize {
        self.by_key
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .map(Vec::len)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl DeliverySink for WsDeliverySink {
    async fn deliver(&self, row: &DeliveryOutbox) -> Result<DeliveryOutcome, DeliveryError> {
        // A single-consumer key holds one connection. A shared key
        // (`exec_events:<id>`) holds any number: the row goes to each of
        // them, and it counts as delivered when at least one took it.
        let senders = self.senders(&row.recipient, row.tenant_id.as_deref());
        if senders.is_empty() {
            return Ok(DeliveryOutcome::NoRoute);
        }
        let frame = cloacina::delivery::envelope::push_from_row(row);
        let mut delivered = false;
        for (id, sender) in senders {
            match sender.try_send(frame.clone()) {
                Ok(()) => delivered = true,
                // Closed: recipient disconnected between the lookup and the
                // send; drop the stale entry.
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    self.unregister(&row.recipient, row.tenant_id.as_deref(), id);
                }
                // Full: recipient is slow. With no other taker the row stays
                // pending (backpressure on the relay).
                Err(mpsc::error::TrySendError::Full(_)) => {}
            }
        }
        if delivered {
            Ok(DeliveryOutcome::Delivered)
        } else {
            Ok(DeliveryOutcome::NoRoute)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina::database::universal_types::UniversalTimestamp;

    fn row(recipient: &str, tenant: Option<&str>) -> DeliveryOutbox {
        DeliveryOutbox {
            id: 1,
            recipient: recipient.to_string(),
            kind: "work".to_string(),
            tenant_id: tenant.map(|s| s.to_string()),
            payload: b"x".to_vec(),
            delivery_state: "pending".to_string(),
            delivery_attempts: 0,
            created_at: UniversalTimestamp::now(),
            delivered_at: None,
            acked_at: None,
        }
    }

    #[tokio::test]
    async fn deliver_to_unregistered_recipient_is_no_route() {
        let sink = WsDeliverySink::new();
        let outcome = sink.deliver(&row("agent:1", Some("t1"))).await.unwrap();
        assert_eq!(outcome, DeliveryOutcome::NoRoute);
    }

    #[tokio::test]
    async fn deliver_to_registered_recipient_pushes_frame() {
        let sink = WsDeliverySink::new();
        let (_, mut rx) = sink.register("agent:1", Some("t1"));
        let outcome = sink.deliver(&row("agent:1", Some("t1"))).await.unwrap();
        assert_eq!(outcome, DeliveryOutcome::Delivered);
        let msg = rx.recv().await.expect("frame should arrive");
        match msg {
            ServerMessage::Push {
                id,
                recipient,
                tenant_id,
                ..
            } => {
                assert_eq!(id, 1);
                assert_eq!(recipient, "agent:1");
                assert_eq!(tenant_id.as_deref(), Some("t1"));
            }
            other => panic!("expected Push, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn tenant_isolation_recipient_string_alone_is_not_enough() {
        let sink = WsDeliverySink::new();
        // Same recipient name in two different tenants → distinct keys.
        let (t1_id, mut t1) = sink.register("agent:1", Some("t1"));
        let _t2 = sink.register("agent:1", Some("t2"));

        let outcome = sink.deliver(&row("agent:1", Some("t1"))).await.unwrap();
        assert_eq!(outcome, DeliveryOutcome::Delivered);
        // t1 received it; t2 must not.
        let _ = t1.recv().await.expect("t1 receives");
        // No way to assert t2 didn't, except by deregistering t1 and re-checking:
        sink.unregister("agent:1", Some("t1"), t1_id);
        let outcome2 = sink.deliver(&row("agent:1", Some("t1"))).await.unwrap();
        assert_eq!(outcome2, DeliveryOutcome::NoRoute);
    }

    #[tokio::test]
    async fn full_channel_returns_no_route_for_backpressure() {
        let sink = WsDeliverySink::new();
        let (_, _rx) = sink.register("agent:1", None);
        // Saturate the channel (depth=32) without draining.
        for _ in 0..CHANNEL_DEPTH {
            let outcome = sink.deliver(&row("agent:1", None)).await.unwrap();
            assert_eq!(outcome, DeliveryOutcome::Delivered);
        }
        // Next send must report NoRoute (relay leaves the row pending for retry).
        let outcome = sink.deliver(&row("agent:1", None)).await.unwrap();
        assert_eq!(outcome, DeliveryOutcome::NoRoute);
    }

    fn ops_frame() -> ServerMessage {
        ServerMessage::push(
            0,
            "ops_metrics",
            "ops_metrics:global",
            Some("t1".to_string()),
            b"{}",
        )
    }

    // CLOACINA-T-0952: two tabs on one tenant both get the ops metrics.
    #[tokio::test]
    async fn broadcast_recipient_keeps_every_subscriber() {
        let sink = WsDeliverySink::new();
        let (_, mut a) = sink.register("ops_metrics:global", Some("t1"));
        let (_, mut b) = sink.register("ops_metrics:global", Some("t1"));
        assert_eq!(sink.len(), 2);

        assert!(sink.push_direct("ops_metrics:global", Some("t1"), ops_frame()));
        assert_eq!(a.recv().await, Some(ops_frame()));
        assert_eq!(b.recv().await, Some(ops_frame()));
    }

    // CLOACINA-T-0952: one subscriber leaving does not cut off the other.
    #[tokio::test]
    async fn broadcast_unregister_removes_only_that_connection() {
        let sink = WsDeliverySink::new();
        let (a_id, a) = sink.register("ops_metrics:global", Some("t1"));
        let (_, mut b) = sink.register("ops_metrics:global", Some("t1"));
        drop(a);
        sink.unregister("ops_metrics:global", Some("t1"), a_id);

        assert!(sink.has_recipient("ops_metrics:global", Some("t1")));
        assert!(sink.push_direct("ops_metrics:global", Some("t1"), ops_frame()));
        assert_eq!(b.recv().await, Some(ops_frame()));
    }

    // CLOACINA-T-0952: a closed subscriber is pruned and the rest still receive.
    #[tokio::test]
    async fn broadcast_push_prunes_closed_subscriber() {
        let sink = WsDeliverySink::new();
        let (_, a) = sink.register("ops_metrics:global", Some("t1"));
        let (_, mut b) = sink.register("ops_metrics:global", Some("t1"));
        drop(a);

        assert!(sink.push_direct("ops_metrics:global", Some("t1"), ops_frame()));
        assert_eq!(b.recv().await, Some(ops_frame()));
        assert_eq!(sink.len(), 1);
    }

    // CLOACINA-T-1051: two followers of one execution in one tenant both get
    // each event; the second does not cut off the first.
    #[tokio::test]
    async fn execution_events_go_to_every_follower_of_the_tenant() {
        let sink = WsDeliverySink::new();
        let (first_id, mut first) = sink.register("exec_events:e1", Some("t1"));
        let (_, mut second) = sink.register("exec_events:e1", Some("t1"));
        // A follower with the same id in another tenant is another key.
        let (_, mut other_tenant) = sink.register("exec_events:e1", Some("t2"));
        assert_eq!(sink.len(), 3);

        let outcome = sink
            .deliver(&row("exec_events:e1", Some("t1")))
            .await
            .unwrap();
        assert_eq!(outcome, DeliveryOutcome::Delivered);
        assert!(
            first.try_recv().is_ok(),
            "the first follower is not cut off"
        );
        assert!(second.try_recv().is_ok());
        assert!(
            other_tenant.try_recv().is_err(),
            "another tenant gets nothing"
        );

        // One follower leaves: the other still gets the events.
        drop(first);
        sink.unregister("exec_events:e1", Some("t1"), first_id);
        let outcome = sink
            .deliver(&row("exec_events:e1", Some("t1")))
            .await
            .unwrap();
        assert_eq!(outcome, DeliveryOutcome::Delivered);
        assert!(second.try_recv().is_ok());
    }

    // A follower that went away without unregistering is pruned; with no
    // follower left the row stays pending.
    #[tokio::test]
    async fn execution_events_with_no_live_follower_are_no_route() {
        let sink = WsDeliverySink::new();
        let (_, gone) = sink.register("exec_events:e1", Some("t1"));
        drop(gone);
        let outcome = sink
            .deliver(&row("exec_events:e1", Some("t1")))
            .await
            .unwrap();
        assert_eq!(outcome, DeliveryOutcome::NoRoute);
        assert_eq!(sink.len(), 0);
    }

    // The queue stays single-consumer: a new connection replaces the old one.
    #[tokio::test]
    async fn single_consumer_recipient_is_replaced() {
        let sink = WsDeliverySink::new();
        let (_, mut old) = sink.register("agent:1", Some("t1"));
        let (_, mut new) = sink.register("agent:1", Some("t1"));
        assert_eq!(sink.len(), 1);
        assert_eq!(old.recv().await, None, "old handler unwinds");

        let outcome = sink.deliver(&row("agent:1", Some("t1"))).await.unwrap();
        assert_eq!(outcome, DeliveryOutcome::Delivered);
        assert!(new.recv().await.is_some());
    }

    // CLOACINA-T-0952: the replaced handler's unregister on its way out must
    // not remove the connection that replaced it.
    #[tokio::test]
    async fn replaced_handler_unregister_keeps_the_new_connection() {
        let sink = WsDeliverySink::new();
        let (old_id, _old) = sink.register("agent:1", Some("t1"));
        let (_, mut new) = sink.register("agent:1", Some("t1"));
        sink.unregister("agent:1", Some("t1"), old_id);

        assert!(sink.has_recipient("agent:1", Some("t1")));
        let outcome = sink.deliver(&row("agent:1", Some("t1"))).await.unwrap();
        assert_eq!(outcome, DeliveryOutcome::Delivered);
        assert!(new.recv().await.is_some());
    }
}
