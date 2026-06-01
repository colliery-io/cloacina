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

//! Cross-connection LISTEN/NOTIFY integration test for the delivery substrate
//! (CLOACI-I-0115 / T-0626, increment 2). Requires a live Postgres.
//!
//! Verifies the load-bearing wake of ADR A-0006: an insert on the DAL's pool
//! connection fires the `delivery_outbox` NOTIFY trigger on commit, which a
//! *separate* `tokio-postgres` LISTEN connection receives and uses to wake the
//! relay — proving the cross-replica path, not just an in-process channel.

use crate::fixtures::get_or_init_postgres_fixture;
use async_trait::async_trait;
use cloacina::dal::DAL;
use cloacina::delivery::{
    run_pg_listener, DeliveryError, DeliveryOutcome, DeliveryRelay, DeliverySink,
};
use cloacina::models::delivery_outbox::{DeliveryOutbox, NewDeliveryOutbox};
use serial_test::serial;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::watch;

struct CollectingSink {
    seen: Mutex<Vec<i64>>,
}

#[async_trait]
impl DeliverySink for CollectingSink {
    async fn deliver(&self, row: &DeliveryOutbox) -> Result<DeliveryOutcome, DeliveryError> {
        self.seen.lock().unwrap().push(row.id);
        Ok(DeliveryOutcome::Delivered)
    }
}

#[tokio::test]
#[serial]
async fn test_notify_wakes_relay_across_connections() {
    let fixture = get_or_init_postgres_fixture().await;
    let (dal, url) = {
        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        (DAL::new(guard.get_database()), guard.get_database_url())
    };

    let sink = Arc::new(CollectingSink {
        seen: Mutex::new(Vec::new()),
    });
    let relay = DeliveryRelay::new(dal.clone(), sink.clone());
    let wake = relay.wake_handle();

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    // Relay drains + delivers; gets woken only via the wake handle.
    let relay_handle = tokio::spawn(relay.run(shutdown_rx.clone()));
    // LISTEN connection is independent of the DAL pool — this is the cross-connection path.
    let listener_handle = tokio::spawn(run_pg_listener(
        url,
        "delivery_outbox".to_string(),
        wake,
        shutdown_rx,
    ));

    // Let the listener register LISTEN and let the relay's startup catch-up
    // drain (and the listener's connect-time catch-up wake) settle, so the
    // post-insert delivery can only be driven by a real NOTIFY.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Enqueue via the DAL pool connection. The AFTER INSERT trigger fires
    // NOTIFY on commit; the separate LISTEN connection must receive it and wake
    // the relay. No in-process wake is issued by this test.
    let row = dal
        .delivery_outbox()
        .enqueue(NewDeliveryOutbox {
            recipient: "agent:1".to_string(),
            kind: "work".to_string(),
            tenant_id: None,
            payload: b"x".to_vec(),
        })
        .await
        .expect("enqueue should succeed");

    let mut delivered = false;
    for _ in 0..100 {
        if sink.seen.lock().unwrap().contains(&row.id) {
            delivered = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Clean up before asserting so a failure doesn't leak tasks.
    let _ = shutdown_tx.send(true);
    let _ = relay_handle.await;
    let _ = listener_handle.await;

    assert!(
        delivered,
        "NOTIFY on the DAL connection did not wake the LISTEN-driven relay to deliver the row"
    );
}
