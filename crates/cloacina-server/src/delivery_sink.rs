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

/// In-memory registry of `(recipient, tenant_id) → sender` for currently-
/// connected WS recipients on this replica.
#[derive(Default)]
pub struct WsDeliverySink {
    by_key: Mutex<HashMap<Key, mpsc::Sender<ServerMessage>>>,
}

impl WsDeliverySink {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a recipient. Replaces any prior entry for the same key (the
    /// older socket is implicitly evicted — its send half drops, the previous
    /// handler's `rx.recv()` returns `None`, and that handler unwinds). Returns
    /// the mpsc receiver the handler awaits to receive pushes.
    pub fn register(
        &self,
        recipient: &str,
        tenant_id: Option<&str>,
    ) -> mpsc::Receiver<ServerMessage> {
        let (tx, rx) = mpsc::channel(CHANNEL_DEPTH);
        let key = (recipient.to_string(), tenant_id.map(|s| s.to_string()));
        let mut g = self.by_key.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(_old) = g.insert(key.clone(), tx) {
            debug!(
                recipient = %key.0,
                tenant = ?key.1,
                "delivery_sink: replaced existing recipient connection"
            );
        }
        rx
    }

    /// Deregister a recipient. Idempotent. Safe to call after a connection
    /// drops or after [`register`] returned (the old entry was already evicted).
    pub fn unregister(&self, recipient: &str, tenant_id: Option<&str>) {
        let key = (recipient.to_string(), tenant_id.map(|s| s.to_string()));
        let mut g = self.by_key.lock().unwrap_or_else(|e| e.into_inner());
        g.remove(&key);
    }

    /// Whether a recipient is currently connected on this replica. Lets a
    /// publisher skip gathering ephemeral data nobody is listening for
    /// (CLOACI-T-0718).
    pub fn has_recipient(&self, recipient: &str, tenant_id: Option<&str>) -> bool {
        let key = (recipient.to_string(), tenant_id.map(|s| s.to_string()));
        self.by_key
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(&key)
    }

    /// Push a frame straight to a connected recipient, bypassing the durable
    /// outbox + relay (CLOACI-T-0718). For *ephemeral* latest-snapshot data
    /// (operational metrics) that must NOT accrue rows in `delivery_outbox` —
    /// the substrate's durable path is reserved for must-not-lose work
    /// (execution events, work packets). Returns `true` if a connection
    /// received it, `false` if nobody is listening (no-op) or the channel is
    /// full/closed — the next snapshot supersedes a dropped one anyway.
    pub fn push_direct(
        &self,
        recipient: &str,
        tenant_id: Option<&str>,
        msg: ServerMessage,
    ) -> bool {
        let key = (recipient.to_string(), tenant_id.map(|s| s.to_string()));
        let sender = {
            let g = self.by_key.lock().unwrap_or_else(|e| e.into_inner());
            g.get(&key).cloned()
        };
        let Some(sender) = sender else {
            return false;
        };
        match sender.try_send(msg) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.unregister(recipient, tenant_id);
                false
            }
            Err(mpsc::error::TrySendError::Full(_)) => false,
        }
    }

    /// Current registry depth — useful for metrics (T-0628).
    pub fn len(&self) -> usize {
        self.by_key.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl DeliverySink for WsDeliverySink {
    async fn deliver(&self, row: &DeliveryOutbox) -> Result<DeliveryOutcome, DeliveryError> {
        let key: Key = (row.recipient.clone(), row.tenant_id.clone());
        let sender = {
            let g = self.by_key.lock().unwrap_or_else(|e| e.into_inner());
            g.get(&key).cloned()
        };
        let Some(sender) = sender else {
            return Ok(DeliveryOutcome::NoRoute);
        };
        let frame = cloacina::delivery::envelope::push_from_row(row);
        match sender.try_send(frame) {
            Ok(()) => Ok(DeliveryOutcome::Delivered),
            // Closed: recipient disconnected between the lookup and the send;
            // drop the stale entry and treat as NoRoute.
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.unregister(&row.recipient, row.tenant_id.as_deref());
                Ok(DeliveryOutcome::NoRoute)
            }
            // Full: recipient is slow; backpressure the relay (row stays pending).
            Err(mpsc::error::TrySendError::Full(_)) => Ok(DeliveryOutcome::NoRoute),
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
        let mut rx = sink.register("agent:1", Some("t1"));
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
        let mut t1 = sink.register("agent:1", Some("t1"));
        let _t2 = sink.register("agent:1", Some("t2"));

        let outcome = sink.deliver(&row("agent:1", Some("t1"))).await.unwrap();
        assert_eq!(outcome, DeliveryOutcome::Delivered);
        // t1 received it; t2 must not.
        let _ = t1.recv().await.expect("t1 receives");
        // No way to assert t2 didn't, except by deregistering t1 and re-checking:
        sink.unregister("agent:1", Some("t1"));
        let outcome2 = sink.deliver(&row("agent:1", Some("t1"))).await.unwrap();
        assert_eq!(outcome2, DeliveryOutcome::NoRoute);
    }

    #[tokio::test]
    async fn full_channel_returns_no_route_for_backpressure() {
        let sink = WsDeliverySink::new();
        let _rx = sink.register("agent:1", None);
        // Saturate the channel (depth=32) without draining.
        for _ in 0..CHANNEL_DEPTH {
            let outcome = sink.deliver(&row("agent:1", None)).await.unwrap();
            assert_eq!(outcome, DeliveryOutcome::Delivered);
        }
        // Next send must report NoRoute (relay leaves the row pending for retry).
        let outcome = sink.deliver(&row("agent:1", None)).await.unwrap();
        assert_eq!(outcome, DeliveryOutcome::NoRoute);
    }
}
