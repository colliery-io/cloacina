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

//! Safety-net sweeper for the delivery substrate
//! (CLOACI-I-0115 / S-0012 / A-0006, task T-0628).
//!
//! The sweeper is **what makes the substrate at-least-once.** The relay
//! (T-0626) drains and pushes on every wake, and the WS handler (T-0627)
//! resets stuck rows back to `pending` on reconnect — but a recipient that
//! never reconnects, a NOTIFY lost during a LISTEN reconnect, or a replica
//! that crashed between commit and ack would otherwise leave rows stranded.
//! The sweeper periodically scans the outbox for rows that have been open
//! past a threshold and pushes them back through the relay path.
//!
//! ## Multi-replica (OQ-D)
//!
//! Every replica may run a sweeper; ownership is **not** needed because
//! [`crate::dal::unified::delivery_outbox::DeliveryOutboxDAL::reset_to_pending`]
//! is an atomic compare-and-set. Concurrent sweepers racing on the same row
//! produce exactly one `Ok(())` and N-1 `InvalidStateTransition`s — safe, just
//! ignored. There is no need for a per-replica claim column or an elected
//! sweeper for the v1 substrate.
//!
//! ## Cost / NFR-1.2.1
//!
//! Sweep cost is `O(stuck rows + 1 count query)` every `sweep_interval`
//! (default 30s). With the relay handling the steady-state via NOTIFY-driven
//! wakes, the sweeper is genuinely a backstop — its traffic is a tiny fraction
//! of what the former "anything ready?" poll loops produced.

use std::time::Duration;

use tokio::sync::watch;
use tracing::{debug, warn};

use crate::dal::DAL;
use crate::database::universal_types::UniversalTimestamp;
use crate::delivery::WakeHandle;
use crate::error::ValidationError;
use crate::models::delivery_outbox::DeliveryState;

/// Tunables for the sweeper.
#[derive(Debug, Clone)]
pub struct SweeperConfig {
    /// How often the sweeper wakes to scan.
    pub sweep_interval: Duration,
    /// Rows whose `created_at` is older than `now - stuck_threshold` are
    /// considered stuck and eligible for redelivery.
    pub stuck_threshold: Duration,
    /// Maximum rows examined per sweep, keeping a single tick bounded.
    pub batch_limit: i64,
}

impl Default for SweeperConfig {
    fn default() -> Self {
        Self {
            sweep_interval: Duration::from_secs(30),
            stuck_threshold: Duration::from_secs(60),
            batch_limit: 256,
        }
    }
}

/// Periodic backstop scan over the delivery outbox.
pub struct DeliverySweeper {
    dal: DAL,
    wake: WakeHandle,
    config: SweeperConfig,
}

impl DeliverySweeper {
    /// Construct with [`SweeperConfig::default`].
    pub fn new(dal: DAL, wake: WakeHandle) -> Self {
        Self::with_config(dal, wake, SweeperConfig::default())
    }

    pub fn with_config(dal: DAL, wake: WakeHandle, config: SweeperConfig) -> Self {
        Self { dal, wake, config }
    }

    /// Run until `shutdown` flips to `true` (or the sender drops).
    pub async fn run(self, mut shutdown: watch::Receiver<bool>) {
        let mut ticker = tokio::time::interval(self.config.sweep_interval);
        // The first tick fires immediately; we want to wait one interval so
        // startup doesn't race the relay's own catch-up drain.
        ticker.tick().await;
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match self.sweep_once().await {
                        Ok(reset) => debug!(reset, "delivery sweep complete"),
                        Err(e) => warn!(error = %e, "delivery sweep failed"),
                    }
                }
                res = shutdown.changed() => {
                    if res.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
            }
        }
        debug!("delivery sweeper stopped");
    }

    /// One sweep pass. Returns the number of rows reset (`delivered → pending`).
    /// Pending-past-threshold rows are left alone — a wake at the end of the
    /// sweep tells the relay to retry them via its normal drain path.
    pub async fn sweep_once(&self) -> Result<usize, ValidationError> {
        let cutoff_chrono = chrono::Utc::now()
            - chrono::Duration::from_std(self.config.stuck_threshold)
                .unwrap_or_else(|_| chrono::Duration::seconds(60));
        let cutoff = UniversalTimestamp(cutoff_chrono);

        let stuck = self
            .dal
            .delivery_outbox()
            .list_stuck(cutoff, self.config.batch_limit)
            .await?;

        let mut reset = 0usize;
        for row in &stuck {
            // Delivered-past-threshold rows = pushed but not acked. Reset to
            // pending so the relay re-pushes (idempotent for the recipient per
            // the envelope contract). Pending-past-threshold rows are left as-is
            // — the wake below tells the relay to re-drain.
            if row.state() == Some(DeliveryState::Delivered) {
                match self.dal.delivery_outbox().reset_to_pending(row.id).await {
                    Ok(()) => reset += 1,
                    // Race with another sweeper or with an ack: row already
                    // moved out of `delivered`. Safe to ignore.
                    Err(e) => debug!(
                        id = row.id,
                        error = %e,
                        "sweep reset skipped (state changed concurrently)"
                    ),
                }
            }
        }

        // One wake per non-empty sweep coalesces multiple redeliveries into a
        // single drain pass (relay's Notify holds at most one permit).
        if !stuck.is_empty() {
            self.wake.wake();
        }

        metrics::counter!("cloacina_delivery_outbox_sweep_runs_total").increment(1);
        if reset > 0 {
            metrics::counter!("cloacina_delivery_outbox_sweep_redeliveries_total")
                .increment(reset as u64);
        }
        match self.dal.delivery_outbox().count_open().await {
            Ok(n) => metrics::gauge!("cloacina_delivery_outbox_open").set(n as f64),
            Err(e) => debug!(error = %e, "count_open failed during sweep"),
        }

        Ok(reset)
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::delivery::{DeliveryOutcome, DeliveryRelay, DeliverySink};
    use crate::models::delivery_outbox::{DeliveryOutbox, NewDeliveryOutbox};
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    async fn unique_dal() -> DAL {
        let url = format!(
            "file:delivery_sweeper_test_{}?mode=memory&cache=shared",
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
            payload: b"x".to_vec(),
        }
    }

    /// Minimal sink so we can construct a real relay (and thus a real wake
    /// handle) for the sweeper. Drops everything; sweeper tests don't read it.
    struct NullSink;
    #[async_trait]
    impl DeliverySink for NullSink {
        async fn deliver(
            &self,
            _row: &DeliveryOutbox,
        ) -> Result<DeliveryOutcome, crate::delivery::DeliveryError> {
            Ok(DeliveryOutcome::NoRoute)
        }
    }

    fn make_sweeper(dal: DAL, threshold: Duration) -> DeliverySweeper {
        let relay = DeliveryRelay::new(dal.clone(), Arc::new(NullSink));
        let wake = relay.wake_handle();
        DeliverySweeper::with_config(
            dal,
            wake,
            SweeperConfig {
                sweep_interval: Duration::from_secs(30),
                stuck_threshold: threshold,
                batch_limit: 256,
            },
        )
    }

    #[tokio::test]
    async fn sweep_resets_stuck_delivered_rows() {
        let dal = unique_dal().await;
        let row = dal
            .delivery_outbox()
            .enqueue(work("agent:1"))
            .await
            .unwrap();
        dal.delivery_outbox().mark_delivered(row.id).await.unwrap();

        // Threshold = 0 → "stuck" means "older than now", which the row is.
        let sweeper = make_sweeper(dal.clone(), Duration::ZERO);
        let reset = sweeper.sweep_once().await.unwrap();
        assert_eq!(reset, 1);

        // Row is back in pending; visible via the relay's drain query.
        let pending = dal.delivery_outbox().list_pending(10).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, row.id);
    }

    #[tokio::test]
    async fn sweep_skips_fresh_rows() {
        let dal = unique_dal().await;
        let row = dal
            .delivery_outbox()
            .enqueue(work("agent:1"))
            .await
            .unwrap();
        dal.delivery_outbox().mark_delivered(row.id).await.unwrap();

        // 1-hour threshold → the just-enqueued row is not yet stuck.
        let sweeper = make_sweeper(dal.clone(), Duration::from_secs(3600));
        let reset = sweeper.sweep_once().await.unwrap();
        assert_eq!(reset, 0, "fresh row should not be considered stuck");

        // State unchanged.
        let open = dal
            .delivery_outbox()
            .list_open_for_recipient("agent:1", 10)
            .await
            .unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].delivery_state, "delivered");
    }

    #[tokio::test]
    async fn second_sweep_is_idempotent() {
        let dal = unique_dal().await;
        let row = dal
            .delivery_outbox()
            .enqueue(work("agent:1"))
            .await
            .unwrap();
        dal.delivery_outbox().mark_delivered(row.id).await.unwrap();

        let sweeper = make_sweeper(dal.clone(), Duration::ZERO);
        assert_eq!(sweeper.sweep_once().await.unwrap(), 1);
        // Row is now pending → second sweep finds it in `list_stuck` (still
        // open + past cutoff) but skips it (not in delivered state). Reset = 0.
        assert_eq!(sweeper.sweep_once().await.unwrap(), 0);

        // Still exactly one pending row, no duplicates created.
        assert_eq!(
            dal.delivery_outbox().list_pending(10).await.unwrap().len(),
            1
        );
    }

    /// Concurrent sweepers racing on the same stuck row: at most one wins the
    /// reset CAS, neither errors out to the caller — both report 0 or 1.
    #[tokio::test]
    async fn concurrent_sweepers_are_race_safe() {
        let dal = unique_dal().await;
        let _r = dal
            .delivery_outbox()
            .enqueue(work("agent:1"))
            .await
            .unwrap();
        dal.delivery_outbox().mark_delivered(_r.id).await.unwrap();

        let a = make_sweeper(dal.clone(), Duration::ZERO);
        let b = make_sweeper(dal.clone(), Duration::ZERO);

        let (ra, rb) = tokio::join!(a.sweep_once(), b.sweep_once());
        let total = ra.unwrap() + rb.unwrap();
        assert_eq!(total, 1, "exactly one sweeper should win the reset CAS");

        // Either way the row ends up in pending exactly once.
        assert_eq!(
            dal.delivery_outbox().list_pending(10).await.unwrap().len(),
            1
        );

        // Sanity: the per-sweep metric counter doesn't need verification here;
        // metrics-format tests cover registration.
        let _ = Mutex::new(()); // suppress unused-import warning for Mutex
    }
}
