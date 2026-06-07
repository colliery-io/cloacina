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

//! Delivery Outbox Model
//!
//! Domain types for the delivery outbox — the durable, ack-tracked,
//! recipient-addressed push-delivery outbox of the interservice communication
//! substrate (spec CLOACI-S-0012, decided in CLOACI-A-0006).
//!
//! Unlike [`crate::models::task_outbox`] (a transient, competing-consumer
//! claim queue deleted on claim), delivery-outbox rows are addressed to a
//! specific recipient, carry a payload, and are retained until acked. The
//! substrate is Postgres-only at runtime.

use crate::database::universal_types::UniversalTimestamp;
use serde::{Deserialize, Serialize};

/// Delivery lifecycle of an outbox row.
///
/// `Pending` → `Delivered` (pushed to recipient, awaiting ack) → `Acked`
/// (recipient confirmed receipt). `Delivered` → `Pending` is the sweeper /
/// reconnect redelivery path (T-0628 / T-0627).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryState {
    Pending,
    Delivered,
    Acked,
}

impl DeliveryState {
    /// String form persisted in the `delivery_state` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            DeliveryState::Pending => "pending",
            DeliveryState::Delivered => "delivered",
            DeliveryState::Acked => "acked",
        }
    }

    /// Parse from the persisted string form.
    pub fn from_db(s: &str) -> Option<DeliveryState> {
        match s {
            "pending" => Some(DeliveryState::Pending),
            "delivered" => Some(DeliveryState::Delivered),
            "acked" => Some(DeliveryState::Acked),
            _ => None,
        }
    }

    /// Whether `self → next` is a permitted transition.
    ///
    /// - `Pending → Delivered`: relay pushed the row.
    /// - `Delivered → Acked`: recipient confirmed.
    /// - `Delivered → Pending`: redelivery (sweeper reclaim / reconnect resync).
    ///
    /// `Acked` is terminal. All other transitions are rejected.
    pub fn can_transition_to(self, next: DeliveryState) -> bool {
        use DeliveryState::*;
        matches!(
            (self, next),
            (Pending, Delivered) | (Delivered, Acked) | (Delivered, Pending)
        )
    }
}

/// A delivery-outbox row (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryOutbox {
    /// Auto-incrementing primary key (also the replay-ordering cursor).
    pub id: i64,
    /// Addressed recipient key (e.g. `agent:<uuid>`, `exec_events:<exec_id>`).
    pub recipient: String,
    /// Payload discriminator (e.g. `work`, `execution_event`).
    pub kind: String,
    /// Tenant scope, when applicable (matches the server's `Nullable<Text>` tenant id).
    pub tenant_id: Option<String>,
    /// Opaque payload bytes. NOTIFY never carries this — only the row id.
    pub payload: Vec<u8>,
    /// Current delivery lifecycle state (see [`DeliveryState`]).
    pub delivery_state: String,
    /// Number of delivery attempts so far (incremented on each (re)delivery).
    pub delivery_attempts: i32,
    /// When the row was enqueued.
    pub created_at: UniversalTimestamp,
    /// When it was last pushed to the recipient.
    pub delivered_at: Option<UniversalTimestamp>,
    /// When the recipient acked.
    pub acked_at: Option<UniversalTimestamp>,
}

impl DeliveryOutbox {
    /// Typed view of the persisted `delivery_state` string.
    pub fn state(&self) -> Option<DeliveryState> {
        DeliveryState::from_db(&self.delivery_state)
    }
}

/// Structure for enqueuing a new delivery-outbox row.
///
/// `delivery_state` defaults to `pending` and counters/timestamps are set by
/// the DAL; callers supply only the addressing and payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDeliveryOutbox {
    pub recipient: String,
    pub kind: String,
    pub tenant_id: Option<String>,
    pub payload: Vec<u8>,
}
