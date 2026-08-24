/*
 *  Copyright 2025 Colliery Software
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

//! Routing reactive events to the replica that owns the reactor
//! (CLOACI-T-0851, [`ADR CLOACI-A-0012`] Decision item 2).
//!
//! # How this works, and why there is no roster
//!
//! Under per-reactor leadership only ONE replica runs a given reactor and its
//! accumulators. An event injected over WS/REST lands on whichever replica
//! terminated the connection, which is frequently not the owner.
//!
//! Rather than discovering the owner's address and forwarding point-to-point,
//! a non-owner writes the event to the **delivery outbox** addressed to the
//! accumulator. Every replica's relay drains that outbox and hands rows to its
//! sinks; the sink below tries to deliver into the LOCAL endpoint registry and
//! answers [`DeliveryOutcome::NoRoute`] when it cannot. The row then stays
//! pending for another replica's relay or the sweeper.
//!
//! **The endpoint registry IS the routing table.** A replica has an accumulator
//! registered if and only if it runs the reactor hosting it — which, under
//! leadership, means it owns it. So "can I deliver this?" needs no ownership
//! query, no lock check, and no roster of peers: it is exactly "do I have this
//! accumulator". That also means routing cannot disagree with ownership, since
//! there is only one fact being consulted rather than two that could drift.
//!
//! This is the mechanism the delivery substrate was built for —
//! `DeliveryOutcome::NoRoute` already documents itself as "how
//! connection-ownership routing falls out without the relay needing a roster".
//! Reactive events are simply another addressed recipient alongside
//! `agent:<uuid>` and `exec_events:<exec_id>`.

use crate::delivery::{DeliveryError, DeliveryOutcome, DeliverySink};
use crate::models::delivery_outbox::DeliveryOutbox;

/// `kind` discriminator for reactive-event rows.
pub const REACTOR_EVENT_KIND: &str = "reactor_event";

/// Recipient prefix for an accumulator-addressed row.
const ACCUMULATOR_RECIPIENT_PREFIX: &str = "accumulator:";

/// Build the outbox `recipient` key for an accumulator.
///
/// The tenant is NOT encoded here: outbox rows carry `tenant_id` as its own
/// column, and duplicating it in the recipient string would create two
/// spellings of one scope that could disagree.
pub fn accumulator_recipient(accumulator_name: &str) -> String {
    format!("{ACCUMULATOR_RECIPIENT_PREFIX}{accumulator_name}")
}

/// Parse an accumulator recipient key, returning `None` for rows addressed to
/// anything else.
///
/// Returning `None` rather than erroring matters: every replica's relay hands
/// EVERY pending row to EVERY sink, so this sink constantly sees rows belonging
/// to other subsystems. Those are not errors and must not be logged as such.
pub fn parse_accumulator_recipient(recipient: &str) -> Option<&str> {
    recipient.strip_prefix(ACCUMULATOR_RECIPIENT_PREFIX)
}

/// Delivers reactive events into this replica's accumulators, when it has them.
pub struct AccumulatorDeliverySink {
    registry: super::registry::EndpointRegistry,
}

impl AccumulatorDeliverySink {
    pub fn new(registry: super::registry::EndpointRegistry) -> Self {
        Self { registry }
    }
}

#[async_trait::async_trait]
impl DeliverySink for AccumulatorDeliverySink {
    async fn deliver(&self, row: &DeliveryOutbox) -> Result<DeliveryOutcome, DeliveryError> {
        // Not ours. NoRoute, not an error — see parse_accumulator_recipient.
        if row.kind != REACTOR_EVENT_KIND {
            return Ok(DeliveryOutcome::NoRoute);
        }
        let Some(name) = parse_accumulator_recipient(&row.recipient) else {
            return Ok(DeliveryOutcome::NoRoute);
        };

        let scope = match row.tenant_id.as_deref() {
            Some(t) => super::registry::EndpointScope::tenant(t),
            None => super::registry::EndpointScope::untenanted(),
        };

        match self
            .registry
            .send_to_accumulator(name, scope, row.payload.clone())
            .await
        {
            // Delivered locally: this replica owns the reactor hosting it.
            Ok(n) if n > 0 => Ok(DeliveryOutcome::Delivered),
            // Registered but nothing accepted it. Treat as NOT delivered so the
            // row is retried rather than silently dropped — an event that
            // vanishes between replicas is precisely the split-buffer failure
            // this routing exists to prevent.
            Ok(_) => Ok(DeliveryOutcome::NoRoute),
            // We do not host this accumulator, so we are not the owner. This is
            // the ordinary case on every non-owning replica and is exactly what
            // NoRoute means.
            Err(super::registry::RegistryError::AccumulatorNotFound(_)) => {
                Ok(DeliveryOutcome::NoRoute)
            }
            // Ambiguity is a real misconfiguration (the same accumulator name
            // resolvable in multiple tenants) and must not be silently retried
            // forever as though it were a routing miss.
            Err(e) => Err(DeliveryError::Sink(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipient_round_trips() {
        let r = accumulator_recipient("sensor_window");
        assert_eq!(r, "accumulator:sensor_window");
        assert_eq!(parse_accumulator_recipient(&r), Some("sensor_window"));
    }

    /// Rows belonging to other subsystems share the outbox. Mistaking one for a
    /// malformed accumulator recipient would turn routine traffic into errors.
    #[test]
    fn other_subsystems_recipients_are_not_ours() {
        assert_eq!(parse_accumulator_recipient("agent:abc-123"), None);
        assert_eq!(parse_accumulator_recipient("exec_events:xyz"), None);
        assert_eq!(parse_accumulator_recipient(""), None);
    }

    /// An accumulator name containing the prefix must not confuse the parser.
    #[test]
    fn only_the_leading_prefix_is_stripped() {
        let r = accumulator_recipient("accumulator:weird");
        assert_eq!(parse_accumulator_recipient(&r), Some("accumulator:weird"));
    }
}
