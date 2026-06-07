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

//! Delivery relay for the interservice communication substrate
//! (spec CLOACI-S-0012, ADR CLOACI-A-0006, task T-0626).
//!
//! The relay turns the durable [`crate::dal::unified::delivery_outbox`] into an
//! event-driven push: when woken, it drains `pending` rows, hands each to a
//! [`DeliverySink`], and marks delivered ones. Two wake sources feed it:
//!
//! - **In-process** ([`WakeHandle`]): the producer signals its own replica's
//!   relay immediately after enqueue — no DB round-trip.
//! - **Cross-replica** (`LISTEN`/`NOTIFY`, Postgres): a `tokio-postgres`
//!   connection LISTENing on the `delivery_outbox` channel forwards each
//!   notification to a [`WakeHandle`]. Wired in increment 2 of T-0626; the
//!   NOTIFY side already fires via the `delivery_outbox_notify` trigger.
//!
//! There is **no steady-state polling**: the relay blocks on its wake signal.
//! The safety-net sweeper (T-0628) is the only periodic scan, and exists purely
//! to backstop a missed NOTIFY or a crash — not as the delivery path.
//!
//! Postgres-only at runtime; the drain loop is backend-agnostic so it is
//! unit-tested on SQLite.

pub mod envelope;
pub mod sweeper;
pub use envelope::{ClientMessage, EnvelopeError, ServerMessage, DELIVERY_PROTOCOL_VERSION};
pub use sweeper::{DeliverySweeper, SweeperConfig};

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{watch, Notify};
use tracing::{debug, error, warn};

use crate::dal::DAL;
use crate::models::delivery_outbox::DeliveryOutbox;

/// Default number of rows drained per wake before yielding back to the wait.
const DEFAULT_DRAIN_BATCH: i64 = 256;

/// Errors a [`DeliverySink`] can report. Transient by contract: a failed
/// delivery leaves the row `pending` for the next wake or the sweeper.
#[derive(Debug, thiserror::Error)]
pub enum DeliveryError {
    #[error("sink delivery failed: {0}")]
    Sink(String),
}

/// Outcome of handing a row to a sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryOutcome {
    /// Pushed to the recipient; the relay marks the row `delivered`.
    Delivered,
    /// The recipient is not reachable from this replica right now (no owned
    /// connection). The row stays `pending`; another replica's relay (woken by
    /// NOTIFY) or the sweeper will pick it up. This is how connection-ownership
    /// routing falls out without the relay needing a roster.
    NoRoute,
}

/// Transport that actually delivers an outbox row to its addressed recipient.
///
/// T-0626 ships only a test sink; the WebSocket sink (push over the substrate
/// envelope, await ack) lands in T-0627.
#[async_trait]
pub trait DeliverySink: Send + Sync {
    async fn deliver(&self, row: &DeliveryOutbox) -> Result<DeliveryOutcome, DeliveryError>;
}

/// A cloneable handle producers (and the LISTEN task) use to wake the relay.
#[derive(Clone)]
pub struct WakeHandle {
    notify: Arc<Notify>,
}

impl WakeHandle {
    /// Wake the relay to drain. Coalesces: many wakes between drains collapse
    /// into a single drain (which reads all pending rows anyway). A wake that
    /// arrives with no waiter is retained as one permit, so a signal racing the
    /// tail of a drain is not lost.
    pub fn wake(&self) {
        self.notify.notify_one();
    }
}

/// Drains the delivery outbox on demand and pushes rows to a [`DeliverySink`].
pub struct DeliveryRelay {
    dal: DAL,
    sink: Arc<dyn DeliverySink>,
    notify: Arc<Notify>,
    drain_batch: i64,
}

impl DeliveryRelay {
    /// Creates a relay over the given DAL and sink.
    pub fn new(dal: DAL, sink: Arc<dyn DeliverySink>) -> Self {
        Self {
            dal,
            sink,
            notify: Arc::new(Notify::new()),
            drain_batch: DEFAULT_DRAIN_BATCH,
        }
    }

    /// Overrides the per-wake drain batch size (defaults to [`DEFAULT_DRAIN_BATCH`]).
    pub fn with_drain_batch(mut self, batch: i64) -> Self {
        self.drain_batch = batch;
        self
    }

    /// Returns a wake handle for producers / the LISTEN task.
    pub fn wake_handle(&self) -> WakeHandle {
        WakeHandle {
            notify: self.notify.clone(),
        }
    }

    /// Drains one batch of `pending` rows: deliver each via the sink, mark the
    /// delivered ones. Rows the sink can't route (or that error) stay `pending`.
    /// Returns the count marked `delivered`.
    pub async fn drain_once(&self) -> Result<usize, crate::error::ValidationError> {
        let rows = self
            .dal
            .delivery_outbox()
            .list_pending(self.drain_batch)
            .await?;

        let mut delivered = 0usize;
        for row in rows {
            match self.sink.deliver(&row).await {
                Ok(DeliveryOutcome::Delivered) => {
                    match self.dal.delivery_outbox().mark_delivered(row.id).await {
                        Ok(()) => delivered += 1,
                        Err(crate::error::ValidationError::InvalidStateTransition { .. }) => {
                            // Benign race: the recipient acked the row (now
                            // `acked`), a sweeper reset it, or another replica
                            // handled it between our drain read and this CAS.
                            // Either way the row has already advanced; nothing
                            // to do here.
                            debug!(id = row.id, "mark_delivered skipped — row already advanced");
                        }
                        Err(e) => warn!(
                            id = row.id,
                            error = %e,
                            "delivery_outbox: mark_delivered failed; row stays pending for retry"
                        ),
                    }
                }
                Ok(DeliveryOutcome::NoRoute) => debug!(
                    id = row.id,
                    recipient = %row.recipient,
                    "delivery_outbox: no local route; leaving pending"
                ),
                Err(e) => warn!(
                    id = row.id,
                    error = %e,
                    "delivery_outbox: sink delivery failed; leaving pending"
                ),
            }
        }
        Ok(delivered)
    }

    /// Runs the relay until `shutdown` flips to `true`. Drains once on startup
    /// (catch-up for anything enqueued before the relay was listening), then
    /// drains on every wake. No periodic timer — purely event-driven.
    pub async fn run(self, mut shutdown: watch::Receiver<bool>) {
        if let Err(e) = self.drain_once().await {
            error!(error = %e, "delivery_outbox: initial catch-up drain failed");
        }
        loop {
            tokio::select! {
                _ = self.notify.notified() => {
                    if let Err(e) = self.drain_once().await {
                        error!(error = %e, "delivery_outbox: drain failed");
                    }
                }
                res = shutdown.changed() => {
                    // Sender dropped or shutdown requested.
                    if res.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
            }
        }
        debug!("delivery_outbox: relay shut down");
    }
}

/// Runs a Postgres `LISTEN` loop on `channel`, waking `wake` on every
/// notification — the cross-replica wake of [[CLOACI-A-0006]]. Also wakes once
/// on each successful (re)connect, so anything enqueued while disconnected is
/// caught up. Reconnects with a fixed backoff until `shutdown` flips to `true`.
///
/// `conn_str` is a libpq-style URL plumbed from server config — `Database` does
/// not retain it. NoTls only in v1: the substrate targets the server's
/// local/in-cluster Postgres; TLS is a follow-up.
#[cfg(feature = "postgres")]
pub async fn run_pg_listener(
    conn_str: String,
    channel: String,
    wake: WakeHandle,
    mut shutdown: watch::Receiver<bool>,
) {
    use std::time::Duration;
    const BACKOFF: Duration = Duration::from_secs(1);

    loop {
        if *shutdown.borrow() {
            break;
        }
        match listen_once(&conn_str, &channel, &wake, &mut shutdown).await {
            Ok(()) => break, // clean shutdown
            Err(e) => {
                warn!(error = %e, "delivery_outbox: LISTEN connection lost; reconnecting after backoff");
                tokio::select! {
                    _ = tokio::time::sleep(BACKOFF) => {}
                    _ = shutdown.changed() => {}
                }
            }
        }
    }
    debug!("delivery_outbox: LISTEN loop stopped");
}

/// One LISTEN session. Returns `Ok(())` only on requested shutdown; any
/// connection loss returns `Err` so the caller reconnects.
#[cfg(feature = "postgres")]
async fn listen_once(
    conn_str: &str,
    channel: &str,
    wake: &WakeHandle,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<(), DeliveryError> {
    use futures::StreamExt;
    use std::pin::Pin;

    let (client, mut connection) = tokio_postgres::connect(conn_str, tokio_postgres::NoTls)
        .await
        .map_err(|e| DeliveryError::Sink(format!("connect: {e}")))?;

    // The connection's async-message stream surfaces notifications; forward each
    // as a unit wake-signal over a channel so the main loop can also watch shutdown.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let driver = tokio::spawn(async move {
        let mut stream =
            futures::stream::poll_fn(move |cx| Pin::new(&mut connection).poll_message(cx));
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(tokio_postgres::AsyncMessage::Notification(_)) => {
                    if tx.send(()).is_err() {
                        break;
                    }
                }
                Ok(_) => {}      // notices, etc. — ignored
                Err(_) => break, // connection error → end driver → main loop reconnects
            }
        }
    });

    client
        .batch_execute(&format!("LISTEN {channel}"))
        .await
        .map_err(|e| DeliveryError::Sink(format!("LISTEN: {e}")))?;

    // Catch-up wake for anything enqueued before this session was listening.
    wake.wake();

    let outcome = loop {
        tokio::select! {
            maybe = rx.recv() => match maybe {
                Some(()) => wake.wake(),
                // Driver ended → connection dropped. Surface as error to reconnect.
                None => break Err(DeliveryError::Sink("LISTEN connection closed".to_string())),
            },
            res = shutdown.changed() => {
                // Either an explicit shutdown signal or the sender was dropped
                // (process exit). Both mean "stop the LISTEN cleanly".
                if res.is_err() || *shutdown.borrow() {
                    break Ok(());
                }
            }
        }
    };

    driver.abort();
    drop(client); // hold the client (and thus the LISTEN) until here
    outcome
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::models::delivery_outbox::NewDeliveryOutbox;
    use std::sync::Mutex;
    use std::time::Duration;

    async fn unique_dal() -> DAL {
        let url = format!(
            "file:delivery_relay_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    fn work(recipient: &str) -> NewDeliveryOutbox {
        NewDeliveryOutbox {
            recipient: recipient.to_string(),
            kind: "work".to_string(),
            tenant_id: None,
            payload: b"payload".to_vec(),
        }
    }

    /// A sink that records the ids it was asked to deliver, with a configurable
    /// outcome.
    struct CollectingSink {
        seen: Mutex<Vec<i64>>,
        outcome: DeliveryOutcome,
    }

    impl CollectingSink {
        fn new(outcome: DeliveryOutcome) -> Arc<Self> {
            Arc::new(Self {
                seen: Mutex::new(Vec::new()),
                outcome,
            })
        }
        fn seen(&self) -> Vec<i64> {
            self.seen.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl DeliverySink for CollectingSink {
        async fn deliver(&self, row: &DeliveryOutbox) -> Result<DeliveryOutcome, DeliveryError> {
            self.seen.lock().unwrap().push(row.id);
            Ok(self.outcome)
        }
    }

    #[tokio::test]
    async fn test_drain_delivers_pending_and_marks_delivered() {
        let dal = unique_dal().await;
        let r1 = dal
            .delivery_outbox()
            .enqueue(work("agent:1"))
            .await
            .unwrap();

        let sink = CollectingSink::new(DeliveryOutcome::Delivered);
        let relay = DeliveryRelay::new(dal.clone(), sink.clone());

        let delivered = relay.drain_once().await.unwrap();
        assert_eq!(delivered, 1);
        assert_eq!(sink.seen(), vec![r1.id]);
        // Delivered rows leave the pending set, so a second drain is a no-op.
        assert_eq!(relay.drain_once().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_no_route_leaves_row_pending() {
        let dal = unique_dal().await;
        dal.delivery_outbox()
            .enqueue(work("agent:1"))
            .await
            .unwrap();

        let sink = CollectingSink::new(DeliveryOutcome::NoRoute);
        let relay = DeliveryRelay::new(dal.clone(), sink.clone());

        assert_eq!(relay.drain_once().await.unwrap(), 0);
        // Row was offered to the sink but stays pending for another replica.
        assert_eq!(sink.seen().len(), 1);
        assert_eq!(
            dal.delivery_outbox().list_pending(10).await.unwrap().len(),
            1
        );
    }

    #[tokio::test]
    async fn test_in_process_wake_triggers_drain() {
        let dal = unique_dal().await;
        let sink = CollectingSink::new(DeliveryOutcome::Delivered);
        let relay = DeliveryRelay::new(dal.clone(), sink.clone());
        let wake = relay.wake_handle();

        let (tx, rx) = watch::channel(false);
        let handle = tokio::spawn(relay.run(rx));

        // Enqueue after the relay is running, then wake it.
        let r = dal
            .delivery_outbox()
            .enqueue(work("agent:1"))
            .await
            .unwrap();
        wake.wake();

        // The drain is async; wait briefly for the row to be delivered.
        let mut got = false;
        for _ in 0..50 {
            if sink.seen().contains(&r.id) {
                got = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(got, "wake did not trigger delivery");

        tx.send(true).unwrap();
        let _ = handle.await;
    }
}
